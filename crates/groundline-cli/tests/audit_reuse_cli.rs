use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn audit() -> Value {
    let end = Utc::now() - Duration::minutes(1);
    json!({"kind":"groundline-codex-weekly-audit","schema":1,"status":"PARTIAL",
        "collection_complete":false,"mutation_performed":false,
        "raw_content_emitted":false,"private_paths_emitted":false,
        "thread_ids_emitted":false,"rollout_paths_emitted":false,"secret_value_printed":false,
        "scope":{"generated_at":end.to_rfc3339(),"requested_window_start":(end-Duration::days(7)).to_rfc3339(),
            "requested_window_end":end.to_rfc3339(),"completed_root_sample_count":14},
        "coverage":{"denominator_complete":false,"eligible_root_count":null,"selection_coverage":null,
            "recommendation_evidence_complete":false,"window_impact_of_store_discrepancies":"unknown",
            "store_integrity":{"unindexed_rollout_file_count":3,"stale_rollout_path_row_count":null,"duplicate_thread_id_count":2}},
        "root":{"task_latency":{"completed_count":128},"coverage":{"rollout_count":14},
            "provider_reported_usage":{"rollout_count_with_usage":12,"total_tokens":100},
            "tools":{"verification_success_count":7,"verification_unresolved_count":2}}})
}

fn run(path: &Path, extra: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .env(
            "CODEX_HOME",
            path.parent().unwrap().join("nonexistent-codex-home"),
        )
        .args(["audit", "review", "--input"])
        .arg(path)
        .args(["--json"])
        .args(extra)
        .output()
        .unwrap();
    let result = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.success(), result)
}

#[test]
fn saved_audit_and_review_preserve_unknowns_and_window_without_native_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.json");
    let original = audit();
    let bytes = serde_json::to_vec(&original).unwrap();
    fs::write(&path, &bytes).unwrap();
    let (ok, review) = run(&path, &[]);
    assert!(ok, "{review}");
    assert!(review.get("audit").is_none());
    assert_eq!(review["execution"]["audit_runs"], 0);
    assert_eq!(review["execution"]["recommendation_runs"], 1);
    assert_eq!(review["execution"]["native_history_bytes_read"], 0);
    assert_eq!(review["execution"]["model_calls"], 0);
    for key in [
        "requested_window_start",
        "requested_window_end",
        "generated_at",
    ] {
        assert_eq!(review["reuse"]["source_scope"][key], original["scope"][key]);
    }
    assert!(review["reuse"]["source_execution"].is_null());
    assert_eq!(review["reuse"]["history_changed_since_snapshot"], "unknown");
    assert_eq!(
        review["reuse"]["source_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(review["status"], "PARTIAL");
    assert!(review["readout"]["selection_coverage"].is_null());
    assert_eq!(
        review["readout"]["store_integrity"]["unindexed_rollout_file_count"],
        3
    );
    assert!(
        review["report_ko"]
            .as_str()
            .unwrap()
            .contains("오래된 DB 경로 미관측개")
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(!dir.path().join("nonexistent-codex-home").exists());

    let combined = dir.path().join("review.json");
    let saved_review = groundline_contracts::weekly_review::review(original.clone()).unwrap();
    let full = serde_json::to_vec(&saved_review).unwrap();
    fs::write(&combined, &full).unwrap();
    let (ok, reused) = run(&combined, &[]);
    assert!(ok, "{reused}");
    assert!(reused["recommendation"].is_object());
    assert_eq!(reused["execution"]["recommendation_runs"], 1);
    assert_eq!(
        reused["reuse"]["source_execution"],
        saved_review["execution"]
    );
    assert_eq!(
        reused["reuse"]["source_scope"],
        review["reuse"]["source_scope"]
    );
    let summary = reused;
    assert_eq!(summary["kind"], "groundline-codex-weekly-review-summary");
    assert_eq!(summary["usable_as_full_audit"], false);
    assert_eq!(summary["full_evidence_in_saved_input"], true);
    assert!(summary.get("audit").is_none());
    assert!(summary["recommendation"].is_object());
    assert_eq!(summary["readout"], review["readout"]);
    assert_eq!(fs::read(&combined).unwrap(), full);
    fs::write(&path, serde_json::to_vec(&summary).unwrap()).unwrap();
    assert_eq!(run(&path, &[]).1["error"], "audit_reuse_invalid_snapshot");
}

#[test]
fn summary_never_echoes_saved_free_form_text_or_untyped_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.json");
    let marker = "private-arbitrary-user-content";
    let mut original = audit();
    original["scope"]["private_path"] = json!(marker);
    original["scope"]["runtime_family"] = json!(marker);
    original["scope"]["eligible_root_count"] = json!(marker);
    original["coverage"]["store_integrity"]["unindexed_rollout_file_count"] = json!(marker);
    original["coverage"]["store_integrity"]["private_path"] = json!(marker);
    let mut review = groundline_contracts::weekly_review::review(original).unwrap();
    review["report_ko"] = json!(marker);
    review["readout"]["private_path"] = json!(marker);
    review["execution"]["private_path"] = json!(marker);
    review["execution"]["audit_runs"] = json!(marker);
    fs::write(&path, serde_json::to_vec(&review).unwrap()).unwrap();
    let (ok, summary) = run(&path, &[]);
    assert!(ok, "{summary}");
    assert!(!summary.to_string().contains(marker));
    assert!(summary["reuse"]["source_execution"]["audit_runs"].is_null());
    assert_eq!(summary["execution"]["recommendation_runs"], 1);
    assert_eq!(summary["reuse"]["recommendation_policy_revalidated"], true);
}

#[test]
fn freshness_uses_observation_window_not_new_wrapper_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.json");
    let mut value = audit();
    value["scope"]["requested_window_end"] = json!((Utc::now() - Duration::hours(25)).to_rfc3339());
    value["scope"]["generated_at"] = json!(Utc::now().to_rfc3339());
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let (ok, result) = run(&path, &[]);
    assert!(!ok);
    assert_eq!(result["error"], "audit_reuse_stale_snapshot");
    assert_eq!(result["mutation_performed"], false);
    value["scope"]["requested_window_end"] = json!((Utc::now() + Duration::hours(1)).to_rfc3339());
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(run(&path, &[]).1["error"], "audit_reuse_invalid_window");
}

#[test]
fn rejects_unsafe_wrong_kind_oversized_and_linked_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.json");
    for (field, value) in [
        ("raw_content_emitted", json!(true)),
        ("kind", json!("groundline-codex-activity-audit")),
    ] {
        let mut unsafe_audit = audit();
        unsafe_audit[field] = value;
        fs::write(&path, serde_json::to_vec(&unsafe_audit).unwrap()).unwrap();
        let (ok, result) = run(&path, &[]);
        assert!(!ok);
        assert_eq!(result["error"], "audit_reuse_invalid_snapshot");
    }
    fs::write(&path, vec![b' '; 2 * 1024 * 1024 + 1]).unwrap();
    assert_eq!(run(&path, &[]).1["error"], "audit_reuse_invalid_input_file");
    #[cfg(unix)]
    {
        let link = dir.path().join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert_eq!(run(&link, &[]).1["error"], "audit_reuse_invalid_input_file");
    }
}

#[test]
fn combined_snapshot_cannot_replay_a_forged_historical_recommendation() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("audit.json");
    let combined = dir.path().join("review.json");
    let observation = audit();
    fs::write(&raw, serde_json::to_vec(&observation).unwrap()).unwrap();
    let mut saved = groundline_contracts::weekly_review::review(observation).unwrap();
    saved["recommendation"] =
        json!({"status":"PASS", "recommended_change":{"code":"invented_change"}});
    saved["status"] = json!("PASS");
    fs::write(&combined, serde_json::to_vec(&saved).unwrap()).unwrap();
    let (_, current) = run(&raw, &[]);
    let (_, refreshed) = run(&combined, &[]);
    assert_eq!(refreshed["status"], current["status"]);
    assert_eq!(refreshed["recommendation"], current["recommendation"]);
    assert!(refreshed["recommendation"]["recommended_change"].is_null());
    assert!(!refreshed.to_string().contains("invented_change"));
    assert_eq!(refreshed["readout"], current["readout"]);
    assert_eq!(refreshed["report_ko"], current["report_ko"]);
    assert_eq!(refreshed["execution"]["recommendation_runs"], 1);
    assert_eq!(refreshed["execution"]["native_history_bytes_read"], 0);
    assert_eq!(
        refreshed["reuse"]["recommendation_policy_revalidated"],
        true
    );
}

#[test]
fn refreshed_action_is_delivered_without_requiring_a_second_recommendation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.json");
    let mut observed = audit();
    observed["status"] = json!("PASS");
    observed["collection_complete"] = json!(true);
    observed["coverage"]["recommendation_evidence_complete"] = json!(true);
    observed["scope"]["sample_sufficient"] = json!(true);
    observed["root"]["activity"] = json!({});
    observed["root"]["model_effort"] = json!({"counts":{"gpt-6-sol|medium":14}});
    observed["root"]["prompt_shape"] = json!({});
    observed["root"]["boundary_signals"] = json!({});
    observed["root"]["tools"]["call_count"] = json!(20);
    observed["root"]["tools"]["calls_in_exact_repeated_groups"] = json!(4);
    let mut snapshot = groundline_contracts::weekly_review::review(observed).unwrap();
    snapshot["recommendation"]["recommended_change"]["code"] = json!("preserve_current_workflow");
    fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    let (ok, current) = run(&path, &[]);
    assert!(ok, "{current}");
    assert_eq!(
        current["recommendation"]["recommended_change"]["code"],
        "diagnose_before_retry"
    );
    assert!(current["recommendation"]["recommended_change"]["proposed_agent_actions"].is_array());
    assert_eq!(
        current["recommendation"]["recommendation_scope"],
        "selected_completed_root_sample"
    );
    assert_eq!(
        current["recommendation"]["quality_contract"]["generalization_to_unselected_roots_allowed"],
        false
    );
    assert_eq!(current["execution"]["recommendation_runs"], 1);
    assert_eq!(current["execution"]["native_history_bytes_read"], 0);
    assert!(!dir.path().join("nonexistent-codex-home").exists());
}
