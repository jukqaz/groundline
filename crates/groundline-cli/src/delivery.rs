//! Write-once private delivery receipts from bounded, checked local artifacts.
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use groundline_contracts::{ContractError, delivery};
use groundline_runtime::local_file::{create_private_new, open_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_EVIDENCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RECEIPTS: usize = 1000;
const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;

fn error(code: &str) -> ContractError {
    ContractError(format!("delivery_{code}"))
}

fn parent_or_current(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, ContractError> {
    let mut file =
        open_bounded_regular_file(path, 1, maximum).map_err(|_| error("invalid_input_file"))?;
    let mut data = Vec::new();
    Read::by_ref(&mut file)
        .take(maximum + 1)
        .read_to_end(&mut data)
        .map_err(|_| error("input_unavailable"))?;
    if data.is_empty() || data.len() as u64 > maximum {
        return Err(error("invalid_input_file"));
    }
    Ok(data)
}

fn artifact(
    row: &mut Value,
    manifest_dir: &Path,
    role: &str,
    evidence_bytes: &mut u64,
) -> Result<(), ContractError> {
    let object = row
        .as_object_mut()
        .ok_or_else(|| error("invalid_manifest"))?;
    let artifact_path = object
        .remove("artifact_path")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| error("artifact_path_required"))?;
    let declared = object
        .get("evidence_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| error("artifact_digest_required"))?;
    let path = PathBuf::from(artifact_path);
    let path = if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    };
    let remaining = MAX_EVIDENCE_BYTES.saturating_sub(*evidence_bytes);
    if remaining == 0 {
        return Err(error("evidence_too_large"));
    }
    let data = bounded(&path, MAX_ARTIFACT_BYTES.min(remaining))?;
    *evidence_bytes += data.len() as u64;
    let actual = format!("{:x}", Sha256::digest(&data));
    if declared != actual {
        return Err(error("artifact_digest_mismatch"));
    }
    // A proposal can only be linked to its actual selected pair. Native
    // observations remain operator-supplied even when this local file matches.
    if role == "recommendation" {
        let proposal: Value =
            serde_json::from_slice(&data).map_err(|_| error("invalid_recommendation_artifact"))?;
        if proposal["kind"] != "groundline-routing-proposal"
            || proposal["schema"] != 2
            || proposal["suggestion"]["model"] != row["model"]
            || proposal["suggestion"]["effort"] != row["effort"]
        {
            return Err(error("recommendation_mismatch"));
        }
    }
    // Other evidence points to an existing native result, request, test log or
    // acceptance artifact. Its interpretation appears once in the manifest;
    // copying it into another operator-authored JSON adds no authenticity.
    Ok(())
}

/// Validate all supplied evidence before creating the output. The output is a
/// new owner-private file; an existing file, link, or directory is never replaced.
pub(crate) fn record(input: &Path, output: &Path) -> Result<Value, ContractError> {
    let input_bytes = bounded(input, MAX_MANIFEST_BYTES)?;
    let mut receipt: Value =
        serde_json::from_slice(&input_bytes).map_err(|_| error("invalid_manifest"))?;
    if receipt["kind"] != "groundline-delivery-manifest" || receipt["schema"] != 2 {
        return Err(error("invalid_manifest"));
    }
    if receipt.get("activation_verified").is_some()
        || receipt
            .get("observed_selection_matches_requested")
            .is_some()
        || receipt["verification"]
            .get("authenticity_verified")
            .is_some()
    {
        return Err(error("derived_fields_not_allowed"));
    }
    let manifest_dir = parent_or_current(input);
    let mut evidence_bytes = 0;
    for role in ["recommendation", "requested", "effective"] {
        if !receipt[role].is_null() {
            artifact(&mut receipt[role], manifest_dir, role, &mut evidence_bytes)?;
        }
    }
    artifact(
        &mut receipt["verification"],
        manifest_dir,
        "verification",
        &mut evidence_bytes,
    )?;
    if let Some(entries) = receipt["resources"]["entries"].as_array_mut() {
        if entries.len() > delivery::MAX_RESOURCE_ENTRIES {
            return Err(error("invalid_resources"));
        }
        for entry in entries {
            if !entry["effective"].is_null() {
                artifact(
                    &mut entry["effective"],
                    manifest_dir,
                    "effective",
                    &mut evidence_bytes,
                )?;
            }
        }
    }
    receipt["kind"] = json!("groundline-delivery-receipt");
    receipt["schema"] = json!(1);
    receipt["activation_verified"] = json!(false);
    receipt["verification"]["authenticity_verified"] = json!(false);
    receipt["observed_selection_matches_requested"] =
        match (&receipt["requested"], &receipt["effective"]) {
            (requested, effective) if !requested.is_null() && !effective.is_null() => json!(
                requested["model"] == effective["model"]
                    && requested["effort"] == effective["effort"]
            ),
            _ => Value::Null,
        };
    delivery::validate_receipt(&receipt)?;
    let mut data =
        serde_json::to_vec_pretty(&receipt).map_err(|_| error("serialization_failed"))?;
    data.push(b'\n');
    if data.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(error("receipt_too_large"));
    }
    let parent = parent_or_current(output);
    let meta = fs::symlink_metadata(parent).map_err(|_| error("invalid_output_directory"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(error("invalid_output_directory"));
    }
    let mut file = create_private_new(output).map_err(|_| error("output_exists_or_unavailable"))?;
    let result = file.write_all(&data).and_then(|_| file.sync_all());
    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(output);
        return Err(error("output_write_failed"));
    }
    Ok(json!({
        "kind":"groundline-delivery-recording",
        "schema":1,
        "status":"PASS",
        "receipt_sha256":format!("{:x}", Sha256::digest(&data)),
        "phase":receipt["phase"],
        "outcome":receipt["verification"]["status"],
        "observed_selection_matches_requested":receipt["observed_selection_matches_requested"],
        "activation_verified":false,
        "network_performed":false,
        "mutation_performed":true,
        "raw_content_emitted":false,
        "private_paths_emitted":false
    }))
}

fn read_error(code: &str) -> ContractError {
    ContractError(format!("delivery_read_{code}"))
}

/// The routing importer and observational summary share one bounded, immutable
/// reader. Collection ownership is checked before either caller filters rows.
pub(crate) fn read_receipts(directory: &Path) -> Result<Vec<Value>, ContractError> {
    let metadata =
        fs::symlink_metadata(directory).map_err(|_| read_error("directory_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(read_error("invalid_directory"));
    }
    let mut paths = Vec::new();
    for (index, entry) in fs::read_dir(directory)
        .map_err(|_| read_error("directory_unavailable"))?
        .enumerate()
    {
        if index >= MAX_RECEIPTS {
            return Err(read_error("too_many_entries"));
        }
        let entry = entry.map_err(|_| read_error("entry_unavailable"))?;
        if entry.path().extension().is_some_and(|e| e == "json") {
            paths.push(entry.path());
        }
    }
    paths.sort();
    let mut total_bytes = 0_usize;
    let mut receipts = Vec::new();
    for path in paths {
        let bytes =
            bounded(&path, MAX_MANIFEST_BYTES).map_err(|_| read_error("invalid_receipt_file"))?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| read_error("directory_too_large"))?;
        if total_bytes > MAX_DIRECTORY_BYTES {
            return Err(read_error("directory_too_large"));
        }
        let receipt = serde_json::from_slice(&bytes).map_err(|_| read_error("invalid_receipt"))?;
        receipts.push(receipt);
    }
    delivery::validate_receipt_collection(&receipts).map_err(|error| match error.0.as_str() {
        "delivery_duplicate_delivery" => read_error("duplicate_delivery"),
        "delivery_overlapping_response_ownership" => read_error("overlapping_response_ownership"),
        _ => read_error("invalid_receipt"),
    })?;
    Ok(receipts)
}

pub(crate) fn summarize(directory: &Path) -> Result<Value, ContractError> {
    let receipts = read_receipts(directory).map_err(|error| {
        ContractError(error.0.replacen("delivery_read_", "delivery_summary_", 1))
    })?;
    delivery::summarize_receipts(&receipts)
        .map_err(|error| ContractError(error.0.replacen("delivery_", "delivery_summary_", 1)))
}
