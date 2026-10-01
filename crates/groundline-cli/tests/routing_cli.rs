use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Seek;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn fixture() -> (TempDir, Value, Value, Value) {
    let dir = tempfile::tempdir().unwrap();
    let now = Utc::now().to_rfc3339();
    let catalog = json!({"models":[
        {"slug":"gpt-6-sol","default_reasoning_level":"medium","supported_reasoning_levels":[{"effort":"medium"}]},
        {"slug":"gpt-6-luna","default_reasoning_level":"low","supported_reasoning_levels":[{"effort":"low"}]}
    ]});
    let packet = json!({"kind":"groundline-routing-evidence","schema":2,
        "generated_at_utc":now,"catalog_checked_at_utc":now,
        "catalog_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&catalog).unwrap())),
        "quality_status":"PASS","cohort_sha256":"a".repeat(64),
        "task":{"kind":"implementation","phase":"implementation","complexity":"routine","evidence_sha256":"b".repeat(64)},
        "current":{"model":"gpt-6-sol","effort":"medium","explicit":false},
        "objective":"balanced","outcomes":[]});
    let audit = json!({"kind":"groundline-codex-weekly-audit","schema":1,"status":"PASS",
        "collection_complete":true,
        "coverage":{"recommendation_evidence_complete":true},
        "scope":{"generated_at":now,"completed_root_sample_count":10,"selected_root_count":10,
            "minimum_root_sample_count":5,"sample_sufficient":true},
        "root":{"activity":{},"model_effort":{"counts":{"gpt-6-sol|medium":10}},
            "task_latency":{},"prompt_shape":{},"tools":{},"boundary_signals":{},
            "provider_reported_usage":{"rollout_count_with_usage":10}},
        "raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,
        "rollout_paths_emitted":false,"secret_value_printed":false});
    (dir, packet, catalog, audit)
}

fn complete_audit(audit: &mut Value) {
    audit["root"]["status"] = json!("PASS");
    audit["root"]["collection_complete"] = json!(true);
    audit["root"]["coverage"] = json!({"rollout_count":10});
    audit["root"]["provider_reported_usage"]["rollout_count_without_usage"] = json!(0);
    for key in [
        "collection_issue_count",
        "ownership_unavailable_rollout_count",
    ] {
        audit["root"][key] = json!(0);
    }
    for key in [
        "root_truncated_count",
        "unreadable_completed_root_count",
        "delegated_truncated_count",
        "guardian_truncated_count",
        "unreadable_delegated_count",
        "unreadable_guardian_count",
    ] {
        audit["scope"][key] = json!(0);
    }
}

fn direct_outcomes(packet: &mut Value) {
    let completed = (Utc::now() - Duration::seconds(1)).to_rfc3339();
    packet["outcomes"] = json!(
        (0..20)
            .map(|index| {
                let (model, effort, tokens) = if index < 10 {
                    ("gpt-6-sol", "medium", 100)
                } else {
                    ("gpt-6-luna", "low", 70)
                };
                json!({
                    "unit_hash":format!("{index:064x}"),
                    "evidence_sha256":format!("{:064x}", index + 100),
                    "evidence_kind":"runtime_check",
                    "cohort_sha256":packet["cohort_sha256"],
                    "model":model, "effort":effort, "outcome":"verified", "rework":false,
                    "owned_total_tokens":tokens, "wall_duration_ms":1000,
                    "owned_resources_complete":true, "completed_at_utc":completed
                })
            })
            .collect::<Vec<_>>()
    );
}

fn report_fixture() -> Value {
    let mut report = json!({
        "schema_version": 3,
        "kind": "groundline-insights-weekly-report",
        "status": "PASS",
        "reason_code": "accepted",
        "generated_at_utc": "2026-08-27T00:00:00Z",
        "requested_days": 7,
        "source_contract": {
            "dataset": "basic_active",
            "time_basis": "utc",
            "metric_time_field": "period_end_or_generated_at",
            "freshness_time_field": "received_at",
            "roster_source": "enrolled_installation_registry",
            "analysis_mode": "descriptive_single_period",
            "query_set_version": 3,
            "basic_aggregate_only": true
        },
        "collection_health": {
            "enrolled_installation_count": 1,
            "metadata_known_installation_count": 1,
            "metadata_unknown_installation_count": 0,
            "observed_installation_count": 1,
            "reporting_installation_count": 1,
            "recent_installation_count": 1,
            "never_reported_installation_count": 0,
            "pending_initial_report_installation_count": 0,
            "overdue_never_reported_installation_count": 0,
            "stale_observed_installation_count": 0,
            "current_package_claim_installation_count": 1,
            "current_package_claim_unobserved_installation_count": 0,
            "current_observed_installation_count": 1,
            "current_reporting_installation_count": 1,
            "current_recent_installation_count": 1,
            "policy_latest_version": "0.18.0",
            "roster_status": "AVAILABLE",
            "latest_received_at_utc": "2026-08-27T00:00:00Z",
            "freshness_status": "FRESH",
            "freshness_threshold_hours": 48,
            "initial_report_grace_hours": 24,
            "stored_event_row_count": 2,
            "deduplicated_event_count": 2,
            "duplicate_event_row_count": 0,
            "ttl_expired_event_row_count": 0,
            "quarantined_event_count": 0,
            "delayed_delivery_event_count": 0,
            "overdue_delivery_event_count": 0,
            "clock_skew_event_count": 0,
            "delivery_delay_threshold_hours": 6,
            "delivery_overdue_threshold_hours": 24,
            "clock_skew_tolerance_minutes": 5
        },
        "coverage": {
            "event_count": 2,
            "eligible_root_count": 5,
            "selected_root_count": 5,
            "observed_root_count": 5,
            "completed_turn_count": 5,
            "unreadable_root_count": 0,
            "root_truncated_count": 0,
            "non_root_truncated_count": 0,
            "originator_unclassified_count": 0,
            "originator_source_fallback_count": 0,
            "root_usage_applicable_event_count": 2,
            "root_usage_missing_event_count": 0,
            "root_usage_fallback_event_count": 0,
            "delegated_usage_applicable_event_count": 0,
            "delegated_usage_missing_event_count": 0,
            "delegated_usage_fallback_event_count": 0,
            "guardian_usage_applicable_event_count": 0,
            "guardian_usage_missing_event_count": 0,
            "guardian_usage_fallback_event_count": 0,
            "guardian_incomplete_excluded_count": 0,
            "completed_root_coverage_applicable_event_count": 2,
            "completed_root_coverage_capable_event_count": 2,
            "completed_root_selection_coverage": 1.0,
            "latency_capable_event_count": 2,
            "boundary_count_capable_event_count": 2,
            "guardian_attribution_applicable_event_count": 0,
            "guardian_attribution_capable_event_count": 0,
            "component_nonpass_event_count": 0
        },
        "weekly_metrics": {
            "tokens": {
                "input": 100,
                "cached_input": 80,
                "non_cached_input": 20,
                "output": 10,
                "reasoning_output": 2,
                "total": 110,
                "delegated_total": 0,
                "guardian_total": 0
            },
            "workflow": {
                "compactions": 0,
                "compactions_per_observed_root": 0.0,
                "long_turn_count": 0,
                "long_turn_rate": 0.0,
                "exact_repeated_call_groups": 0,
                "calls_in_exact_repeated_groups": 0,
                "repeated_call_rate": null,
                "failure_signal_count": 0,
                "failure_signal_rate": null,
                "tool_call_count": 0,
                "user_messages_with_text": 0,
                "short_message_count": 0,
                "short_message_rate": null,
                "broad_scope_message_count": 0,
                "broad_scope_message_rate": null,
                "boundary_review_root_count": 0,
                "long_lived_root_count": 0
            },
            "verification": {
                "tool_call_count": 0,
                "success_count": 0,
                "failure_count": 0,
                "unresolved_count": 0,
                "outcome_coverage": null
            },
            "guardian": {
                "review_count": 0,
                "workspace_attributed_review_count": 0,
                "workspace_attribution_coverage": null
            }
        },
        "cohorts": {
            "event_distributions": {
                "schema_version": [{"value": "5", "event_count": 2}],
                "groundline_version": [{"value": "0.18.0", "event_count": 2}],
                "os_family": [{"value": "macos", "event_count": 2}],
                "runtime_family": [{"value": "codex_app", "event_count": 2}],
                "execution_mode": [{"value": "desktop", "event_count": 2}]
            },
            "installation_distributions": {
                "groundline_version": [{"value": "0.18.0", "installation_count": 1}],
                "os_family": [{"value": "macos", "installation_count": 1}],
                "runtime_family": [{"value": "codex_app", "installation_count": 1}],
                "execution_mode": [{"value": "desktop", "installation_count": 1}]
            },
            "model_effort_context_distribution": [],
            "model_effort_token_efficiency": {
                "status": "UNAVAILABLE",
                "reason_code": "token_usage_not_attributed_to_model_effort",
                "context_distribution_only": true
            }
        },
        "data_quality": {
            "status": "PASS",
            "reason_codes": [],
            "sample_sufficient_event_count": 2,
            "sample_insufficient_event_count": 0
        },
        "comparison_readiness": {
            "status": "INSUFFICIENT",
            "reason_codes": ["comparison_baseline_not_included"],
            "minimum_event_count": 2,
            "minimum_observed_root_count": 5
        }
    });
    report["generated_at_utc"] =
        json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    report["collection_health"]["latest_received_at_utc"] = report["generated_at_utc"].clone();
    report["cohorts"]["model_effort_token_efficiency"] = json!({
        "status":"DESCRIPTIVE",
        "reason_code":"completed_turns_not_attributed_to_model_effort",
        "context_distribution_only":false
    });
    report["cohorts"]["model_token_distribution"] = json!([{
        "component":"root", "model_family":"gpt-6-sol", "effort":"medium",
        "input_tokens":100, "cached_input_tokens":80, "output_tokens":10,
        "reasoning_output_tokens":2, "total_tokens":110
    }]);
    report
}

fn run(
    dir: &TempDir,
    packet: &Value,
    catalog: &Value,
    audit: &Value,
    report: Option<&str>,
) -> Value {
    run_with_context(dir, packet, catalog, Some(audit), report)
}

fn run_with_context(
    dir: &TempDir,
    packet: &Value,
    catalog: &Value,
    audit: Option<&Value>,
    report: Option<&str>,
) -> Value {
    let input = dir.path().join("input.json");
    let models = dir.path().join("models.json");
    let private_config = dir.path().join("config.toml");
    fs::write(&input, serde_json::to_vec(packet).unwrap()).unwrap();
    fs::write(&models, serde_json::to_vec(catalog).unwrap()).unwrap();
    fs::write(&private_config, "model='gpt-6-sol'\n").unwrap();
    // Invalid packets or receipts can be rejected before audit stdin is read.
    // Preload stdin so that early exit cannot race a parent-side pipe writer.
    let stdin = if let Some(audit) = audit {
        let mut file = tempfile::tempfile().unwrap();
        serde_json::to_writer(&mut file, audit).unwrap();
        file.rewind().unwrap();
        Stdio::from(file)
    } else {
        Stdio::null()
    };
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_groundline"));
    cmd.args(["efficiency", "route", "--input"])
        .arg(&input)
        .arg("--catalog")
        .arg(&models)
        .arg("--json")
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if audit.is_some() {
        cmd.args(["--audit", "-"]);
    }
    if let Some(report) = report {
        let path = dir.path().join("report.json");
        fs::write(&path, report).unwrap();
        cmd.arg("--report").arg(path);
    }
    if dir.path().join("deliveries").exists() {
        cmd.arg("--deliveries").arg(dir.path().join("deliveries"));
    }
    let output = cmd.output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.success(), value["status"] != "FAIL");
    for private in ["PRIVATE_SENTINEL", dir.path().to_str().unwrap()] {
        assert!(!String::from_utf8_lossy(&output.stdout).contains(private));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(private));
    }
    assert_eq!(
        fs::read_to_string(private_config).unwrap(),
        "model='gpt-6-sol'\n"
    );
    assert_eq!(value["mutation_performed"], false);
    assert_eq!(value["network_performed"], false);
    value
}

fn delivery_receipt(index: usize, cohort: &Value, model: &str, effort: &str, tokens: u64) -> Value {
    let unit = format!("{index:064x}");
    json!({
        "kind":"groundline-delivery-receipt", "schema":1,
        "unit_hash":unit, "cohort_sha256":cohort, "phase":"implementation",
        "completed_at_utc":(Utc::now()-Duration::minutes(1)).to_rfc3339(),
        "recommendation":null,
        "requested":{"model":"gpt-6-sol","effort":"medium","evidence_sha256":"b".repeat(64)},
        "effective":{"model":model,"effort":effort,"evidence_sha256":"c".repeat(64)},
        "verification":{"status":"verified","evidence_kind":"runtime_check",
            "evidence_sha256":"d".repeat(64),"rework":false,"authenticity_verified":false},
        "resources":{"complete":true,"wall_duration_ms":1000,"entries":[{
            "owner":"root","unit_hash":unit,"response_hash":format!("{:064x}",index+1000),
            "input_tokens":tokens-10,"cached_input_tokens":20,"output_tokens":10,
            "reasoning_output_tokens":5,"total_tokens":tokens
        }]},"activation_verified":false,
        "observed_selection_matches_requested":model == "gpt-6-sol" && effort == "medium"
    })
}

fn write_delivery(dir: &TempDir, name: &str, receipt: &Value) {
    let directory = dir.path().join("deliveries");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), serde_json::to_vec(receipt).unwrap()).unwrap();
}

fn comparable_deliveries(dir: &TempDir, packet: &Value) {
    for index in 0..20 {
        let (model, effort, tokens) = if index < 10 {
            ("gpt-6-sol", "medium", 100)
        } else {
            ("gpt-6-luna", "low", 70)
        };
        let mut manifest = delivery_receipt(index, &packet["cohort_sha256"], model, effort, tokens);
        manifest["kind"] = json!("groundline-delivery-manifest");
        manifest["schema"] = json!(2);
        for field in [
            "activation_verified",
            "observed_selection_matches_requested",
        ] {
            manifest.as_object_mut().unwrap().remove(field);
        }
        manifest["verification"]
            .as_object_mut()
            .unwrap()
            .remove("authenticity_verified");
        let evidence = dir.path().join("evidence").join(index.to_string());
        fs::create_dir_all(&evidence).unwrap();
        for field in ["requested", "effective", "verification"] {
            // Record observed evidence bytes directly; no duplicate typed
            // operator-observation JSON is needed beside the manifest.
            let bytes = format!("fixture delivery {index}: observed {field} evidence").into_bytes();
            let path = evidence.join(format!("{field}.txt"));
            fs::write(&path, &bytes).unwrap();
            manifest[field]["artifact_path"] = json!(path);
            manifest[field]["evidence_sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
        }
        let input = evidence.join("manifest.json");
        fs::write(&input, serde_json::to_vec(&manifest).unwrap()).unwrap();
        fs::create_dir_all(dir.path().join("deliveries")).unwrap();
        let recorded = Command::new(env!("CARGO_BIN_EXE_groundline"))
            .args(["efficiency", "record-delivery", "--input"])
            .arg(input)
            .arg("--output")
            .arg(dir.path().join("deliveries").join(format!("{index}.json")))
            .arg("--json")
            .output()
            .unwrap();
        assert!(
            recorded.status.success(),
            "{}",
            String::from_utf8_lossy(&recorded.stdout)
        );
    }
}

#[test]
fn sol_61_routes_with_native_catalog_and_preserves_existing_config() {
    let (dir, mut packet, mut catalog, _) = fixture();
    catalog["models"].as_array_mut().unwrap().push(json!({
        "slug":"gpt-6.1-sol","default_reasoning_level":"low",
        "supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"}]
    }));
    packet["catalog_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&catalog).unwrap())
    ));
    packet["current"]["model"] = json!("gpt-6.1-sol");
    let out = run_with_context(&dir, &packet, &catalog, None, None);
    assert_eq!(out["schema"], 2);
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(
        out["reason_codes"],
        json!(["native_task_judgment_required"])
    );
    packet["current"]["explicit"] = json!(true);
    assert_eq!(
        run_with_context(&dir, &packet, &catalog, None, None)["status"],
        "PINNED"
    );
}

#[test]
fn out_of_scope_native_selection_is_inconclusive_and_does_not_emit_private_id() {
    let (dir, mut packet, mut catalog, _) = fixture();
    catalog["models"].as_array_mut().unwrap().push(json!({
        "slug":"PRIVATE_SENTINEL","default_reasoning_level":"medium",
        "supported_reasoning_levels":[{"effort":"medium"}]
    }));
    packet["catalog_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&catalog).unwrap())
    ));
    packet["current"]["model"] = json!("PRIVATE_SENTINEL");
    let out = run_with_context(&dir, &packet, &catalog, None, None);
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(
        out["reason_codes"],
        json!(["current_selection_outside_optimization_scope"])
    );
    assert!(out["suggestion"].is_null());
}

#[test]
fn recorded_deliveries_compare_without_aggregate_context_and_preserve_receipts() {
    let (dir, packet, catalog, _) = fixture();
    comparable_deliveries(&dir, &packet);
    let before = fs::read(dir.path().join("deliveries/10.json")).unwrap();
    let out = run_with_context(&dir, &packet, &catalog, None, None);
    assert_eq!(out["status"], "EMPIRICAL");
    assert_eq!(out["suggestion"]["model"], "gpt-6-luna");
    assert_eq!(out["suggestion"]["candidate_tokens_per_verified"], 70.0);
    assert_eq!(out["native_audit"]["status"], "MISSING");
    assert_eq!(out["clickhouse"]["status"], "MISSING");
    assert_eq!(out["data_readiness"]["aggregate_sources_ready"], false);
    assert_eq!(
        out["data_readiness"]["aggregate_context_limitations"],
        json!(["native_audit_missing", "clickhouse_report_missing"])
    );
    assert_eq!(
        out["evidence_scope"]["aggregate_evidence_role"],
        "context_only"
    );
    assert_eq!(
        out["evidence_scope"]["empirical_selection_requires_complete_direct_outcomes"],
        true
    );
    assert_eq!(out["delivery_records"]["included_outcomes"], 20);
    assert_eq!(
        out["delivery_records"]["selection_mismatch_matched_count"],
        10
    );
    assert_eq!(
        out["delivery_records"]["source_authenticity_verified"],
        false
    );
    assert_eq!(out["native_execution"]["activation_verified"], false);
    assert_eq!(
        out["native_execution"]["application_mode"],
        "next_authorized_lane_with_acceptance_checks"
    );
    assert_eq!(
        fs::read(dir.path().join("deliveries/10.json")).unwrap(),
        before
    );
}

#[test]
fn child_selection_cannot_replace_unobserved_root_execution_or_override_pin() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    let mut receipt = delivery_receipt(20, &packet["cohort_sha256"], "gpt-6-luna", "low", 70);
    receipt["effective"] = Value::Null;
    receipt["observed_selection_matches_requested"] = Value::Null;
    receipt["resources"]["entries"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "owner":"child","unit_hash":"e".repeat(64),"response_hash":"f".repeat(64),
            "effective":{"model":"gpt-6-luna","effort":"low","evidence_sha256":"a".repeat(64)},
            "input_tokens":2,"cached_input_tokens":0,"output_tokens":1,
            "reasoning_output_tokens":0,"total_tokens":3
        }));
    write_delivery(&dir, "unknown.json", &receipt);
    let report = report_fixture().to_string();
    // Incomplete actual work remains an explicit blocker even when no outcomes
    // can be attributed to a model/effort pair.
    let out = run(&dir, &packet, &catalog, &audit, Some(&report));
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert!(out["suggestion"].is_null());
    assert_eq!(
        out["native_execution"]["application_mode"],
        "no_evidence_based_replacement"
    );
    assert_eq!(out["native_execution"]["target"], "none");
    comparable_deliveries(&dir, &packet);
    let out = run(&dir, &packet, &catalog, &audit, Some(&report));
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(out["delivery_records"]["unattributed_matched_count"], 1);
    assert_eq!(
        out["native_execution"]["application_mode"],
        "no_evidence_based_replacement"
    );
    assert_eq!(out["native_execution"]["target"], "none");
    assert!(
        out["candidate_assessment"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["selected"] == false)
    );
    assert!(
        out["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("matched_delivery_execution_unobserved"))
    );
    packet["current"]["explicit"] = json!(true);
    assert_eq!(
        run(&dir, &packet, &catalog, &audit, Some(&report))["status"],
        "PINNED"
    );
}

#[test]
fn receipt_resources_cannot_be_double_counted_or_mixed_with_aggregated_outcomes() {
    let (dir, mut packet, catalog, audit) = fixture();
    let first = delivery_receipt(1, &packet["cohort_sha256"], "gpt-6-sol", "medium", 100);
    let mut second = delivery_receipt(2, &packet["cohort_sha256"], "gpt-6-luna", "low", 70);
    second["resources"]["entries"][0]["response_hash"] =
        first["resources"]["entries"][0]["response_hash"].clone();
    write_delivery(&dir, "one.json", &first);
    write_delivery(&dir, "two.json", &second);
    let out = run(&dir, &packet, &catalog, &audit, None);
    assert_eq!(
        out["error"],
        "routing_deliveries_overlapping_response_ownership"
    );
    direct_outcomes(&mut packet);
    let out = run(&dir, &packet, &catalog, &audit, None);
    assert_eq!(out["error"], "routing_deliveries_outcomes_must_be_empty");
}

#[test]
fn outdated_and_other_cohort_receipts_are_counted_separately_without_filling_samples() {
    let (dir, packet, catalog, audit) = fixture();
    let mut old = delivery_receipt(1, &packet["cohort_sha256"], "gpt-6-sol", "medium", 100);
    old["completed_at_utc"] = json!((Utc::now() - Duration::days(31)).to_rfc3339());
    write_delivery(&dir, "old.json", &old);
    let mut other = delivery_receipt(2, &json!("e".repeat(64)), "gpt-6-luna", "low", 70);
    other["effective"] = Value::Null;
    other["observed_selection_matches_requested"] = Value::Null;
    write_delivery(&dir, "other.json", &other);
    let out = run(&dir, &packet, &catalog, &audit, None);
    assert_eq!(out["delivery_records"]["included_outcomes"], 0);
    assert_eq!(out["delivery_records"]["outside_window_count"], 1);
    assert_eq!(out["delivery_records"]["other_cohort_count"], 1);
    assert_eq!(out["delivery_records"]["unattributed_matched_count"], 0);
}

#[test]
fn duplicate_deliveries_and_unexpected_private_fields_are_rejected() {
    let (dir, packet, catalog, audit) = fixture();
    let mut receipt = delivery_receipt(1, &packet["cohort_sha256"], "gpt-6-sol", "medium", 100);
    write_delivery(&dir, "one.json", &receipt);
    write_delivery(&dir, "two.json", &receipt);
    assert_eq!(
        run(&dir, &packet, &catalog, &audit, None)["error"],
        "routing_deliveries_duplicate_delivery"
    );
    receipt["raw_text"] = json!("PRIVATE_SENTINEL");
    write_delivery(&dir, "one.json", &receipt);
    assert_eq!(
        run(&dir, &packet, &catalog, &audit, None)["error"],
        "routing_deliveries_invalid_receipt"
    );
}

#[test]
fn receipt_comparison_requires_matching_current_phase_and_keeps_partial_failure() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    let mut receipt = delivery_receipt(1, &packet["cohort_sha256"], "gpt-6-sol", "medium", 100);
    receipt["phase"] = json!("visual_acceptance");
    write_delivery(&dir, "one.json", &receipt);
    assert_eq!(
        run(&dir, &packet, &catalog, &audit, None)["error"],
        "routing_deliveries_cohort_phase_mismatch"
    );
    packet["task"].as_object_mut().unwrap().remove("phase");
    assert_eq!(
        run(&dir, &packet, &catalog, &audit, None)["error"],
        "routing_deliveries_current_phase_required"
    );
    packet["task"]["phase"] = json!("visual_acceptance");
    receipt["verification"]["status"] = json!("failed");
    receipt["resources"]["complete"] = json!(false);
    receipt["resources"]["entries"][0]["output_tokens"] = Value::Null;
    write_delivery(&dir, "one.json", &receipt);
    let out = run(
        &dir,
        &packet,
        &catalog,
        &audit,
        Some(&report_fixture().to_string()),
    );
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(
        out["delivery_records"]["resources_incomplete_matched_count"],
        1
    );
    assert_eq!(
        out["delivery_records"]["verification_unknown_matched_count"],
        0
    );
    assert_eq!(out["delivery_records"]["included_outcomes"], 1);
}

#[test]
fn complete_failed_delivery_cannot_be_dropped_to_make_candidate_win() {
    let (dir, packet, catalog, _) = fixture();
    comparable_deliveries(&dir, &packet);
    let mut failed = delivery_receipt(20, &packet["cohort_sha256"], "gpt-6-luna", "low", 70);
    failed["verification"]["status"] = json!("failed");
    failed["verification"]["rework"] = json!(true);
    write_delivery(&dir, "failed.json", &failed);
    let out = run_with_context(&dir, &packet, &catalog, None, None);
    assert_eq!(out["status"], "RETAIN");
    assert_eq!(out["delivery_records"]["included_outcomes"], 21);
    let candidate = &out["candidate_assessment"][0];
    assert_eq!(candidate["units"], 11);
    assert_eq!(candidate["verified"], 10);
    assert_eq!(candidate["rework"], 1);
    assert_eq!(candidate["status"], "REJECTED");
}

#[test]
fn observed_model_with_unknown_verification_blocks_that_pair() {
    let (dir, packet, catalog, _) = fixture();
    comparable_deliveries(&dir, &packet);
    let mut unknown = delivery_receipt(20, &packet["cohort_sha256"], "gpt-6-luna", "low", 70);
    unknown["verification"]["status"] = json!("unknown");
    unknown["verification"]["evidence_kind"] = json!("unobserved");
    write_delivery(&dir, "unknown-result.json", &unknown);
    let out = run_with_context(&dir, &packet, &catalog, None, None);
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(
        out["delivery_records"]["verification_unknown_matched_count"],
        1
    );
    assert_eq!(out["delivery_records"]["unattributed_matched_count"], 0);
    assert_eq!(out["candidate_assessment"][0]["units"], 11);
    assert_eq!(out["candidate_assessment"][0]["unknown_outcome"], true);
    assert!(out["suggestion"].is_null());
}

#[cfg(unix)]
#[test]
fn linked_receipt_is_rejected_without_reading_target() {
    let (dir, packet, catalog, audit) = fixture();
    fs::create_dir(dir.path().join("deliveries")).unwrap();
    let target = dir.path().join("private.json");
    fs::write(&target, "PRIVATE_SENTINEL").unwrap();
    std::os::unix::fs::symlink(target, dir.path().join("deliveries/linked.json")).unwrap();
    assert_eq!(
        run(&dir, &packet, &catalog, &audit, None)["error"],
        "routing_deliveries_invalid_receipt_file"
    );
}

#[test]
fn no_outcomes_leave_native_task_selection_open_without_heuristic_downshift() {
    for quality in ["PASS", "PARTIAL"] {
        let (dir, mut packet, mut catalog, mut audit) = fixture();
        complete_audit(&mut audit);
        catalog["models"].as_array_mut().unwrap().push(json!({
            "slug":"gpt-6-astra",
            "default_reasoning_level":"high",
            "supported_reasoning_levels":[{"effort":"high"}]
        }));
        packet["catalog_sha256"] = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&catalog).unwrap())
        ));
        packet["current"] = json!({"model":"gpt-6-astra","effort":"high","explicit":false});
        packet["quality_status"] = json!(quality);
        let out = run(
            &dir,
            &packet,
            &catalog,
            &audit,
            Some(&report_fixture().to_string()),
        );
        assert_eq!(out["status"], "INCONCLUSIVE", "{out}");
        assert!(out["suggestion"].is_null());
        assert_eq!(
            out["reason_codes"],
            json!(["native_task_judgment_required"])
        );
        assert_eq!(
            out["selection_policy"]["task_shape_used_for_model_ranking"],
            false
        );
        assert_eq!(
            out["native_execution"]["application_mode"],
            "no_evidence_based_replacement"
        );
        assert_eq!(out["native_execution"]["target"], "none");
        assert_eq!(
            out["native_execution"]["scope"],
            "evidence_based_replacement_only"
        );
        assert_eq!(
            out["native_execution"]["ordinary_task_selection_owner"],
            "native_codex"
        );
        assert_eq!(
            out["native_execution"]["baseline_selection"],
            json!({"model":"gpt-6-astra","effort":"high"})
        );
        assert_eq!(out["native_execution"]["configuration_changed"], false);
    }
}

#[test]
fn combined_audit_is_reused_and_missing_clickhouse_is_not_fabricated() {
    let (dir, packet, catalog, audit) = fixture();
    let combined = json!({"kind":"groundline-codex-weekly-review","audit":audit});
    let out = run(&dir, &packet, &catalog, &combined, None);
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert!(out["suggestion"].is_null());
    assert_eq!(out["clickhouse"]["status"], "MISSING");
    assert_eq!(out["native_audit"]["usage_observed_rollouts"], 10);
    // Envelope flags alone cannot establish complete native collection.
    assert_eq!(out["native_audit"]["complete_for_aggregate_context"], false);
    assert_eq!(out["data_readiness"]["aggregate_sources_ready"], false);
    assert!(
        out["native_audit"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("native_count_unknown"))
    );
    assert!(
        out["native_audit"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("native_exclusion_counts_unknown"))
    );
    assert!(
        out["data_readiness"]["aggregate_context_limitations"]
            .as_array()
            .unwrap()
            .contains(&json!("clickhouse_report_missing"))
    );
    assert_eq!(out["native_execution"]["activation_verified"], false);
    assert_eq!(out["native_execution"]["configuration_changed"], false);
    assert_eq!(
        out["reason_codes"],
        json!(["native_task_judgment_required"])
    );
}

#[test]
fn valid_strict_report_and_complete_direct_evidence_enable_empirical_route() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    let report = report_fixture();
    let out = run(&dir, &packet, &catalog, &audit, Some(&report.to_string()));

    assert_eq!(out["status"], "EMPIRICAL");
    assert_eq!(out["suggestion"]["model"], "gpt-6-luna");
    assert_eq!(out["suggestion"]["effort"], "low");
    assert_eq!(out["suggestion"]["candidate_tokens_per_verified"], 70.0);
    assert_eq!(out["native_audit"]["complete_for_aggregate_context"], true);
    assert_eq!(out["native_audit"]["reason_codes"], json!([]));
    assert_eq!(out["clickhouse"]["status"], "AVAILABLE");
    assert_eq!(out["clickhouse"]["reason_codes"], json!([]));
    assert_eq!(out["data_readiness"]["aggregate_sources_ready"], true);
    assert_eq!(
        out["data_readiness"]["aggregate_context_limitations"],
        json!([])
    );
    assert_eq!(out["clickhouse"]["data_quality"]["status"], "PASS");
    assert_eq!(out["clickhouse"]["coverage"]["observed_root_count"], 5);
    assert_eq!(
        out["clickhouse"]["model_token_distribution"][0]["total_tokens"],
        110
    );
    assert_eq!(out["clickhouse"]["used_for_model_ranking"], false);
    assert_eq!(out["automatic_config_write"], false);
    assert_eq!(out["causal_improvement_claimed"], false);
}

#[test]
fn partial_clickhouse_quality_limits_context_not_complete_direct_comparison() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    let mut report = report_fixture();
    // Missing usage is a coverage defect with a matching, schema-valid quality result.
    report["coverage"]["root_usage_missing_event_count"] = json!(1);
    report["data_quality"]["status"] = json!("PARTIAL");
    report["data_quality"]["reason_codes"] = json!(["usage_missing"]);
    report["comparison_readiness"]["reason_codes"] =
        json!(["comparison_baseline_not_included", "data_quality_not_pass"]);
    let out = run(&dir, &packet, &catalog, &audit, Some(&report.to_string()));

    assert_eq!(out["status"], "EMPIRICAL");
    assert_eq!(out["clickhouse"]["status"], "PARTIAL");
    assert_eq!(
        out["clickhouse"]["data_quality"]["reason_codes"],
        json!(["usage_missing"])
    );
    assert_eq!(out["native_audit"]["complete_for_aggregate_context"], true);
    assert!(
        out["clickhouse"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("clickhouse_data_quality_not_pass"))
    );
    assert_eq!(out["data_readiness"]["aggregate_sources_ready"], false);
    assert_eq!(
        out["reason_codes"],
        json!(["observational_comparison_only"])
    );
}

#[test]
fn stale_clickhouse_report_limits_context_not_complete_direct_comparison() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    let mut report = report_fixture();
    report["generated_at_utc"] =
        json!((Utc::now() - Duration::days(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let out = run(&dir, &packet, &catalog, &audit, Some(&report.to_string()));

    assert_eq!(out["status"], "EMPIRICAL");
    assert_eq!(out["clickhouse"]["status"], "PARTIAL");
    assert_eq!(out["clickhouse"]["fresh"], false);
    assert_eq!(out["clickhouse"]["data_quality"]["status"], "PASS");
    assert_eq!(out["native_audit"]["complete_for_aggregate_context"], true);
    assert!(
        out["clickhouse"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("clickhouse_report_stale"))
    );
}

#[test]
fn stale_clickhouse_collection_limits_context_not_complete_direct_comparison() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    let mut report = report_fixture();
    report["collection_health"]["freshness_status"] = json!("STALE");
    report["collection_health"]["latest_received_at_utc"] =
        json!((Utc::now() - Duration::days(3)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    report["collection_health"]["recent_installation_count"] = json!(0);
    report["collection_health"]["current_recent_installation_count"] = json!(0);
    report["collection_health"]["stale_observed_installation_count"] = json!(1);
    report["data_quality"]["status"] = json!("PARTIAL");
    report["data_quality"]["reason_codes"] = json!(["stale_installation_reporting"]);
    report["comparison_readiness"]["reason_codes"] =
        json!(["comparison_baseline_not_included", "data_quality_not_pass"]);
    let out = run(&dir, &packet, &catalog, &audit, Some(&report.to_string()));

    assert_eq!(out["status"], "EMPIRICAL");
    assert_eq!(out["clickhouse"]["status"], "PARTIAL");
    assert_eq!(out["clickhouse"]["fresh"], true);
    assert_eq!(out["clickhouse"]["freshness_status"], "STALE");
    assert_eq!(out["native_audit"]["complete_for_aggregate_context"], true);
    assert!(
        out["clickhouse"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("clickhouse_collection_not_fresh"))
    );
}

#[test]
fn known_native_usage_gap_is_reported_and_blocks_aggregate_readiness() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    audit["root"]["provider_reported_usage"]["rollout_count_without_usage"] = json!(1);
    let report = report_fixture();
    let out = run(&dir, &packet, &catalog, &audit, Some(&report.to_string()));

    assert_eq!(out["native_audit"]["complete_for_aggregate_context"], false);
    assert!(
        out["native_audit"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("native_usage_gap"))
    );
    assert_eq!(out["clickhouse"]["reason_codes"], json!([]));
    assert_eq!(out["data_readiness"]["aggregate_sources_ready"], false);
    assert!(
        out["data_readiness"]["aggregate_context_limitations"]
            .as_array()
            .unwrap()
            .contains(&json!("native_usage_gap"))
    );
    assert_eq!(out["status"], "EMPIRICAL");
}

#[test]
fn native_pass_does_not_hide_unknown_or_incomplete_recommendation_coverage() {
    for coverage in [None, Some(json!(null)), Some(json!(false))] {
        let (dir, mut packet, catalog, mut audit) = fixture();
        complete_audit(&mut audit);
        direct_outcomes(&mut packet);
        let reason = if coverage == Some(json!(false)) {
            "native_recommendation_evidence_incomplete"
        } else {
            "native_recommendation_evidence_unknown"
        };
        if let Some(coverage) = coverage {
            audit["coverage"]["recommendation_evidence_complete"] = coverage;
        } else {
            audit.as_object_mut().unwrap().remove("coverage");
        }
        let out = run(
            &dir,
            &packet,
            &catalog,
            &audit,
            Some(&report_fixture().to_string()),
        );
        assert_eq!(out["native_audit"]["status"], "PASS");
        assert_eq!(out["native_audit"]["complete_for_aggregate_context"], false);
        assert_eq!(out["data_readiness"]["aggregate_sources_ready"], false);
        assert!(
            out["native_audit"]["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!(reason))
        );
        // Direct matched outcome evidence keeps its independent role; incomplete
        // aggregate context must not be promoted to ready or model evidence.
        assert_eq!(out["status"], "EMPIRICAL");
        assert_eq!(out["native_audit"]["used_for_model_ranking"], false);
    }
}

#[test]
fn native_audit_uses_coverage_rollout_count_and_rejects_missing_or_mismatched_counts() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    let report = report_fixture();

    let mut missing = audit.clone();
    missing["root"].as_object_mut().unwrap().remove("coverage");
    let missing_out = run(&dir, &packet, &catalog, &missing, Some(&report.to_string()));
    assert_eq!(missing_out["status"], "EMPIRICAL");
    assert_eq!(
        missing_out["native_audit"]["complete_for_aggregate_context"],
        false
    );
    assert!(
        missing_out["native_audit"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("native_count_unknown"))
    );

    audit["root"]["coverage"]["rollout_count"] = json!(9);
    let mismatch_out = run(&dir, &packet, &catalog, &audit, Some(&report.to_string()));
    assert_eq!(mismatch_out["status"], "EMPIRICAL");
    assert_eq!(
        mismatch_out["native_audit"]["complete_for_aggregate_context"],
        false
    );
    assert!(
        mismatch_out["native_audit"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("native_count_disagreement"))
    );
}

#[test]
fn explicit_choices_are_preserved_with_incomplete_aggregate_evidence() {
    let (dir, mut packet, catalog, audit) = fixture();
    packet["current"]["explicit"] = json!(true);
    let out = run(&dir, &packet, &catalog, &audit, None);
    assert_eq!(out["status"], "PINNED");
    assert!(out["suggestion"].is_null());
    assert_eq!(
        out["native_execution"]["application_mode"],
        "preserve_explicit_selection"
    );
}

#[test]
fn stale_audit_limits_aggregate_context_without_changing_direct_comparison() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    audit["scope"]["generated_at"] = json!((Utc::now() - Duration::days(2)).to_rfc3339());
    let out = run(&dir, &packet, &catalog, &audit, None);
    assert_eq!(out["native_audit"]["fresh"], false);
    assert!(
        out["native_audit"]["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("native_audit_stale"))
    );
    assert_eq!(out["status"], "EMPIRICAL");
}

#[test]
fn direct_packet_quality_cannot_be_repaired_by_optional_context() {
    let (dir, mut packet, catalog, mut audit) = fixture();
    complete_audit(&mut audit);
    direct_outcomes(&mut packet);
    let report = report_fixture().to_string();
    for (quality, reason) in [
        ("PARTIAL", "quality_blocks_matched_outcome_comparison"),
        ("FAIL", "evidence_quality_failed"),
    ] {
        packet["quality_status"] = json!(quality);
        for out in [
            run_with_context(&dir, &packet, &catalog, None, None),
            run(&dir, &packet, &catalog, &audit, Some(&report)),
        ] {
            assert_eq!(out["status"], "INCONCLUSIVE");
            assert!(out["suggestion"].is_null());
            assert_eq!(out["reason_codes"], json!([reason]));
            assert_eq!(
                out["native_execution"]["application_mode"],
                "no_evidence_based_replacement"
            );
        }
    }
}

#[test]
fn supplied_malformed_optional_context_is_not_ignored_for_complete_direct_outcomes() {
    let (dir, mut packet, catalog, _) = fixture();
    direct_outcomes(&mut packet);
    let malformed = json!({"raw_text":"PRIVATE_SENTINEL"});
    let bad_audit = run_with_context(&dir, &packet, &catalog, Some(&malformed), None);
    assert_eq!(bad_audit["status"], "FAIL");
    let bad_report = run_with_context(&dir, &packet, &catalog, None, Some(&malformed.to_string()));
    assert_eq!(bad_report["status"], "FAIL");
}

#[test]
fn private_extra_fields_and_invalid_reports_are_rejected_without_echo() {
    let (dir, mut packet, catalog, audit) = fixture();
    packet["raw_conversation"] = json!("PRIVATE_SENTINEL");
    assert_eq!(run(&dir, &packet, &catalog, &audit, None)["status"], "FAIL");
    packet.as_object_mut().unwrap().remove("raw_conversation");
    assert_eq!(
        run(
            &dir,
            &packet,
            &catalog,
            &audit,
            Some("{\"prompt\":\"PRIVATE_SENTINEL\"}")
        )["status"],
        "FAIL"
    );
    packet["quality_status"] = json!("PRIVATE_SENTINEL");
    assert_eq!(run(&dir, &packet, &catalog, &audit, None)["status"], "FAIL");
}

#[test]
fn changed_catalog_is_not_silently_accepted() {
    let (dir, packet, mut catalog, audit) = fixture();
    catalog["models"][0]["supported_reasoning_levels"] = json!([{"effort":"max"}]);
    catalog["models"][0]["default_reasoning_level"] = json!("max");
    let out = run(&dir, &packet, &catalog, &audit, None);
    assert_eq!(out["status"], "FAIL");
}
