//! Read immutable, private delivery receipts without changing their evidence.
use std::path::Path;

use chrono::{DateTime, Duration};
use groundline_contracts::{ContractError, delivery};
use serde_json::{Value, json};

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
    let receipts = crate::delivery::read_receipts(directory).map_err(|error| {
        ContractError(error.0.replacen("delivery_read_", "routing_deliveries_", 1))
    })?;
    let mut outcomes = Vec::new();
    let mut outside_window = 0;
    let mut other_cohorts = 0;
    let mut unattributed_matched = 0;
    let mut resources_incomplete = 0;
    let mut verification_unknown = 0;
    let mut selection_mismatch = 0;
    for receipt in &receipts {
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
            // Resource-level child selections describe owned usage, not the
            // root's effective execution. Never fill a missing root from them.
            if let Some(outcome) = delivery::outcome_from_receipt(receipt)? {
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
            "receipts_read": receipts.len(), "included_outcomes": included,
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
