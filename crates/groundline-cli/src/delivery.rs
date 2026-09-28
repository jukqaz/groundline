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

fn error(code: &str) -> ContractError {
    ContractError(format!("delivery_{code}"))
}

#[cfg(windows)]
fn reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn reparse(_: &fs::Metadata) -> bool {
    false
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

fn artifact(row: &mut Value, manifest_dir: &Path, role: &str) -> Result<(), ContractError> {
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
    let data = bounded(&path, MAX_ARTIFACT_BYTES)?;
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
            || proposal["schema"] != 1
            || proposal["suggestion"]["model"] != row["model"]
            || proposal["suggestion"]["effort"] != row["effort"]
        {
            return Err(error("recommendation_mismatch"));
        }
    } else if role == "requested" || role == "effective" {
        let observation: Value =
            serde_json::from_slice(&data).map_err(|_| error("invalid_selection_artifact"))?;
        let expected_kind = if role == "effective" {
            "groundline-native-selection-observation"
        } else {
            "groundline-selection-request"
        };
        if observation["kind"] != expected_kind
            || observation["schema"] != 1
            || observation["model"] != row["model"]
            || observation["effort"] != row["effort"]
            || observation["source"] != "operator_supplied"
        {
            return Err(error("selection_artifact_mismatch"));
        }
    } else if role == "verification" {
        let observation: Value =
            serde_json::from_slice(&data).map_err(|_| error("invalid_verification_artifact"))?;
        if observation["kind"] != "groundline-delivery-verification-observation"
            || observation["schema"] != 1
            || observation["status"] != row["status"]
            || observation["evidence_kind"] != row["evidence_kind"]
            || observation["source"] != "operator_supplied"
        {
            return Err(error("verification_artifact_mismatch"));
        }
    }
    Ok(())
}

/// Validate all supplied evidence before creating the output. The output is a
/// new owner-private file; an existing file, link, or directory is never replaced.
pub(crate) fn record(input: &Path, output: &Path) -> Result<Value, ContractError> {
    let input_bytes = bounded(input, MAX_MANIFEST_BYTES)?;
    let mut receipt: Value =
        serde_json::from_slice(&input_bytes).map_err(|_| error("invalid_manifest"))?;
    if receipt["kind"] != "groundline-delivery-manifest" || receipt["schema"] != 1 {
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
    for role in ["recommendation", "requested", "effective"] {
        if !receipt[role].is_null() {
            artifact(&mut receipt[role], manifest_dir, role)?;
        }
    }
    artifact(&mut receipt["verification"], manifest_dir, "verification")?;
    receipt["kind"] = json!("groundline-delivery-receipt");
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
    if !meta.is_dir() || meta.file_type().is_symlink() || reparse(&meta) {
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
