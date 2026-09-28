//! Read immutable, private delivery receipts without changing their evidence.
use std::{collections::BTreeSet, fs, path::Path};

use chrono::{DateTime, Duration};
use groundline_contracts::{ContractError, delivery};
use serde_json::{Value, json};

const MAX_RECEIPTS: usize = 1000;
const MAX_RECEIPT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;

fn error(code: &str) -> ContractError {
    ContractError(format!("routing_deliveries_{code}"))
}

pub(super) struct Imported {
    pub(super) summary: Value,
    pub(super) unattributed_matched: usize,
}

pub(super) fn read(packet: &mut Value, directory: &Path) -> Result<Imported, ContractError> {
    // Mixing manually aggregated outcomes with response-level receipts cannot
    // prove disjoint ownership. Use exactly one evidence source for a proposal.
    if packet["outcomes"]
        .as_array()
        .is_none_or(|rows| !rows.is_empty())
    {
        return Err(error("outcomes_must_be_empty"));
    }
    let phase = packet["task"]["phase"]
        .as_str()
        .filter(|phase| delivery::PHASES.contains(phase))
        .ok_or_else(|| error("current_phase_required"))?
        .to_owned();
    let generated = packet["generated_at_utc"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .ok_or_else(|| error("invalid_packet_time"))?;
    let metadata = fs::symlink_metadata(directory).map_err(|_| error("directory_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || reparse(&metadata) {
        return Err(error("invalid_directory"));
    }
    let mut paths = Vec::new();
    for (index, entry) in fs::read_dir(directory)
        .map_err(|_| error("directory_unavailable"))?
        .enumerate()
    {
        if index >= MAX_RECEIPTS {
            return Err(error("too_many_entries"));
        }
        let entry = entry.map_err(|_| error("entry_unavailable"))?;
        // This is a deliberately selected receipt directory, not a recursive
        // search of conversation history or the user's account state.
        if entry.path().extension().is_some_and(|e| e == "json") {
            paths.push(entry.path());
        }
    }
    paths.sort();
    let mut total_bytes = 0;
    let mut unit_ids = BTreeSet::new();
    let mut response_ids = BTreeSet::new();
    let mut outcomes = Vec::new();
    let mut outside_window = 0;
    let mut other_cohorts = 0;
    let mut unattributed_matched = 0;
    let mut resources_incomplete = 0;
    let mut verification_unknown = 0;
    let mut selection_mismatch = 0;
    for path in &paths {
        let bytes = crate::load_bounded(path, MAX_RECEIPT_BYTES)
            .map_err(|_| error("invalid_receipt_file"))?;
        total_bytes += bytes.len();
        if total_bytes > MAX_DIRECTORY_BYTES {
            return Err(error("directory_too_large"));
        }
        let receipt = crate::parse_object(&bytes).map_err(|_| error("invalid_receipt"))?;
        delivery::validate_receipt(&receipt).map_err(|_| error("invalid_receipt"))?;
        if !unit_ids.insert(receipt["unit_hash"].as_str().unwrap().to_owned()) {
            return Err(error("duplicate_delivery"));
        }
        for entry in receipt["resources"]["entries"].as_array().unwrap() {
            if !response_ids.insert(entry["response_hash"].as_str().unwrap().to_owned()) {
                return Err(error("overlapping_response_ownership"));
            }
        }
        let completed = DateTime::parse_from_rfc3339(receipt["completed_at_utc"].as_str().unwrap())
            .map_err(|_| error("invalid_receipt_time"))?;
        if completed > generated {
            return Err(error("receipt_after_packet"));
        }
        if generated - completed > Duration::days(30) {
            outside_window += 1;
        } else if receipt["cohort_sha256"] != packet["cohort_sha256"] {
            other_cohorts += 1;
        } else {
            if receipt["phase"] != phase {
                return Err(error("cohort_phase_mismatch"));
            }
            resources_incomplete += usize::from(receipt["resources"]["complete"] != true);
            verification_unknown += usize::from(receipt["verification"]["status"] == "unknown");
            selection_mismatch +=
                usize::from(receipt["observed_selection_matches_requested"] == false);
            if let Some(outcome) = delivery::outcome_from_receipt(&receipt)? {
                outcomes.push(outcome);
            } else {
                // Unknown actual execution must remain visible. Dropping these
                // records would cherry-pick successful/attributable work.
                unattributed_matched += 1;
            }
        }
    }
    let included = outcomes.len();
    packet["outcomes"] = json!(outcomes);
    Ok(Imported {
        summary: json!({
            "status": if unattributed_matched + resources_incomplete + verification_unknown > 0 { "PARTIAL" } else { "AVAILABLE" },
            "receipts_read": paths.len(), "included_outcomes": included,
            "outside_window_count": outside_window, "other_cohort_count": other_cohorts,
            "unattributed_matched_count": unattributed_matched,
            "resources_incomplete_matched_count": resources_incomplete,
            "verification_unknown_matched_count": verification_unknown,
            "selection_mismatch_matched_count": selection_mismatch,
            "phase": phase,
            "source": "private_immutable_delivery_receipts",
            "selection_and_acceptance_source": "operator_observed_local_artifacts",
            "source_authenticity_verified": false,
            "artifact_hashes_rechecked_at_route": false,
            "raw_content_emitted": false, "private_paths_emitted": false,
            "mutation_performed": false
        }),
        unattributed_matched,
    })
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
