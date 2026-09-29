//! Reuse an explicit saved snapshot, without pretending it describes new history.
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use chrono::{DateTime, Duration, Utc};
use groundline_contracts::{ContractError, weekly_review};
use groundline_runtime::local_file::open_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_BYTES: u64 = 2 * 1024 * 1024;
const PRIVACY_FIELDS: &[&str] = &[
    "raw_content_emitted",
    "private_paths_emitted",
    "thread_ids_emitted",
    "secret_value_printed",
];

fn error(code: &str) -> ContractError {
    ContractError(format!("audit_reuse_{code}"))
}

fn timestamp(audit: &Value, key: &str) -> Result<DateTime<Utc>, ContractError> {
    audit["scope"][key]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
        .ok_or_else(|| error("invalid_window"))
}

pub(crate) fn reuse(input: &Path, max_age_hours: u16) -> Result<Value, ContractError> {
    let started = Instant::now();
    if !(1..=24).contains(&max_age_hours) {
        return Err(error("invalid_max_age"));
    }
    // Keep full evidence in a real saved input; only typed aggregates are emitted.
    let file =
        open_bounded_regular_file(input, 1, MAX_BYTES).map_err(|_| error("invalid_input_file"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("input_unavailable"))?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_BYTES {
        return Err(error("invalid_input_file"));
    }
    let supplied: Value = serde_json::from_slice(&bytes).map_err(|_| error("invalid_json"))?;
    let combined = supplied["kind"] == "groundline-codex-weekly-review";
    if supplied["schema"] != 1
        || PRIVACY_FIELDS.iter().any(|key| supplied[*key] != false)
        || (!combined && supplied["kind"] != "groundline-codex-weekly-audit")
    {
        return Err(error("invalid_snapshot"));
    }
    let audit = if combined {
        &supplied["audit"]
    } else {
        &supplied
    };
    if audit["kind"] != "groundline-codex-weekly-audit"
        || audit["schema"] != 1
        || PRIVACY_FIELDS.iter().any(|key| audit[*key] != false)
        || audit["rollout_paths_emitted"] != false
    {
        return Err(error("invalid_snapshot"));
    }
    let now = Utc::now();
    let start = timestamp(audit, "requested_window_start")?;
    let end = timestamp(audit, "requested_window_end")?;
    let generated = timestamp(audit, "generated_at")?;
    if start >= end
        || end > now + Duration::minutes(5)
        || generated > now + Duration::minutes(5)
        || generated < end
    {
        return Err(error("invalid_window"));
    }
    // A fresh wrapper date cannot rejuvenate an old observation window.
    if now.signed_duration_since(end) > Duration::hours(i64::from(max_age_hours)) {
        return Err(error("stale_snapshot"));
    }
    let mut original_scope = json!({
        "requested_window_start":start.to_rfc3339(),
        "requested_window_end":end.to_rfc3339(),
        "generated_at":generated.to_rfc3339(),
        "runtime_family":match audit["scope"]["runtime_family"].as_str() {
            Some("all") => Some("all"), Some("codex_app") => Some("codex_app"),
            Some("codex_cli") => Some("codex_cli"), _ => None,
        }
    });
    for key in [
        "requested_days",
        "completed_root_sample_count",
        "selected_root_count",
        "eligible_root_count",
    ] {
        original_scope[key] = json!(audit["scope"][key].as_u64());
    }
    let original_execution = supplied.get("execution").map(|value| {
        json!({
            "audit_runs":value["audit_runs"].as_u64(),
            "recommendation_runs":value["recommendation_runs"].as_u64(),
            "audit_completed":value["audit_completed"].as_bool(),
            "recommendation_completed":value["recommendation_completed"].as_bool(),
        })
    });
    // Reuse observations, not a historical policy decision. This deterministic
    // calculation performs no native history reads or model calls.
    let review = weekly_review::review(audit.clone())?;
    // Save historical execution counts as provenance, not this invocation's cost.
    let metadata = json!({
        "source_sha256":format!("{:x}", Sha256::digest(&bytes)),
        "source_bytes":bytes.len(), "source_kind":if combined {"groundline-codex-weekly-review"} else {"groundline-codex-weekly-audit"},
        "source_scope":original_scope, "source_execution":original_execution,
        "max_age_hours":max_age_hours, "age_seconds":now.signed_duration_since(end).num_seconds().max(0),
        "window_advanced":false, "current_history_rescanned":false,
        "history_changed_since_snapshot":"unknown", "input_modified":false,
        "recommendation_policy_revalidated":true,
        "scope_caveat":"Saved observation window only; current local history may have changed."
    });
    let execution = json!({
        "audit_runs":0, "recommendation_runs":1,
        "audit_reused":true, "recommendation_reused":false,
        "audit_completed":false, "recommendation_completed":review["recommendation"]["status"]=="PASS",
        "model_calls":0, "native_history_bytes_read":0,
        "saved_evidence_bytes_read":bytes.len(), "elapsed_ms":started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        "cost_scope":"This offline command only; parent analysis and external model costs are not measured."
    });
    Ok(json!({
        "kind":"groundline-codex-weekly-review-summary", "schema":1,
        "status":review["status"], "report_ko":review["report_ko"],
        // This is the freshly computed decision, never copied from supplied text.
        "recommendation":review["recommendation"],
        "readout":review["readout"], "execution":execution, "reuse":metadata,
        "full_evidence_in_saved_input":true, "usable_as_full_audit":false,
        "automatic_application_allowed":false,
        "mutation_performed":false, "network_performed":false,
        "raw_content_emitted":false, "private_paths_emitted":false,
        "thread_ids_emitted":false, "secret_value_printed":false
    }))
}
