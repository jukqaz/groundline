//! Compare bounded local outcomes with optional audit and ClickHouse context.
//! Native Codex owns classification, authorized execution, and activation checks.
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use groundline_cli::config_catalog::{Catalog, MAX_CATALOG_BYTES};
use groundline_contracts::{ContractError, efficiency, insights::WeeklyReport};
use serde_json::{Value, json};

mod receipts;

fn timestamp_freshness(
    value: Option<&Value>,
    stale_code: &'static str,
    unknown_code: &'static str,
) -> Result<(), &'static str> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Err(unknown_code);
    };
    let Ok(at) = DateTime::parse_from_rfc3339(value) else {
        return Err(unknown_code);
    };
    let age = Utc::now().signed_duration_since(at);
    if age < -Duration::minutes(5) || age > Duration::hours(24) {
        Err(stale_code)
    } else {
        Ok(())
    }
}

fn native_audit_reasons(
    audit: &Value,
    selected: Option<u64>,
    freshness: Result<(), &'static str>,
) -> Vec<&'static str> {
    let root = &audit["root"];
    let scope = &audit["scope"];
    let usage = &root["provider_reported_usage"];
    let mut reasons = Vec::new();

    if audit["status"] != "PASS" || root["status"] != "PASS" {
        reasons.push("native_audit_status_not_pass");
    }
    if audit["collection_complete"] != true || root["collection_complete"] != true {
        reasons.push("native_collection_incomplete");
    }
    match audit
        .pointer("/coverage/recommendation_evidence_complete")
        .and_then(Value::as_bool)
    {
        Some(true) => {}
        Some(false) => reasons.push("native_recommendation_evidence_incomplete"),
        None => reasons.push("native_recommendation_evidence_unknown"),
    }
    if let Err(reason) = freshness {
        reasons.push(reason);
    }

    match selected {
        Some(count) if count < 5 => reasons.push("native_sample_below_minimum"),
        Some(_) => {}
        None => reasons.push("native_sample_count_unknown"),
    }
    if audit.pointer("/scope/sample_sufficient") != Some(&json!(true)) {
        reasons.push("native_sample_not_sufficient");
    }

    let matching_counts = [
        (scope, "completed_root_sample_count"),
        (&root["coverage"], "rollout_count"),
        (usage, "rollout_count_with_usage"),
    ];
    let mut count_unknown = false;
    let mut count_disagrees = false;
    for (value, key) in matching_counts {
        match (selected, value[key].as_u64()) {
            (Some(expected), Some(actual)) if expected != actual => count_disagrees = true,
            (Some(_), None) | (None, _) => count_unknown = true,
            _ => {}
        }
    }
    if count_disagrees {
        reasons.push("native_count_disagreement");
    }
    if count_unknown {
        reasons.push("native_count_unknown");
    }

    match usage["rollout_count_without_usage"].as_u64() {
        Some(0) => {}
        Some(_) => reasons.push("native_usage_gap"),
        None => reasons.push("native_usage_gap_unknown"),
    }

    let exclusions = [
        (root, "collection_issue_count"),
        (root, "ownership_unavailable_rollout_count"),
        (scope, "root_truncated_count"),
        (scope, "unreadable_completed_root_count"),
        (scope, "delegated_truncated_count"),
        (scope, "guardian_truncated_count"),
        (scope, "unreadable_delegated_count"),
        (scope, "unreadable_guardian_count"),
    ];
    let mut exclusions_unknown = false;
    let mut exclusions_present = false;
    for (value, key) in exclusions {
        match value[key].as_u64() {
            Some(0) => {}
            Some(_) => exclusions_present = true,
            None => exclusions_unknown = true,
        }
    }
    if exclusions_unknown {
        reasons.push("native_exclusion_counts_unknown");
    }
    if exclusions_present {
        reasons.push("native_exclusions_present");
    }
    reasons
}

fn clickhouse_reasons(
    report: &WeeklyReport,
    freshness: Result<(), &'static str>,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if let Err(reason) = freshness {
        reasons.push(reason);
    }
    if report.data_quality.status != "PASS" {
        reasons.push("clickhouse_data_quality_not_pass");
    }
    if report.collection_health.freshness_status != "FRESH" {
        reasons.push("clickhouse_collection_not_fresh");
    }
    reasons
}

pub(crate) fn run(
    input: &Path,
    catalog: &Path,
    audit: Option<&Path>,
    report: Option<&Path>,
    deliveries: Option<&Path>,
) -> Result<Value, ContractError> {
    let mut packet = super::load_object(input)?;
    let catalog_bytes = super::load_bounded(catalog, MAX_CATALOG_BYTES)?;
    Catalog::parse(&catalog_bytes)?;
    let catalog = super::parse_object(&catalog_bytes)?;
    let delivery_records = deliveries
        .map(|directory| receipts::read(&mut packet, directory))
        .transpose()?;
    let (audit_summary, audit_complete, audit_reasons) = if let Some(path) = audit {
        let audit_input = super::load_audit(path)?;
        let audit = if audit_input["kind"] == "groundline-codex-weekly-review" {
            &audit_input["audit"]
        } else {
            &audit_input
        };
        // Validate supplied aggregate context without promoting it to direct
        // outcome evidence. Hashes do not authenticate an operator file.
        efficiency::recommend_weekly_optimization(audit)?;
        let freshness = timestamp_freshness(
            audit.pointer("/scope/generated_at"),
            "native_audit_stale",
            "native_audit_timestamp_unknown",
        );
        let fresh = freshness.is_ok();
        let selected = audit
            .pointer("/scope/selected_root_count")
            .and_then(Value::as_u64);
        let root = &audit["root"];
        let reasons = native_audit_reasons(audit, selected, freshness);
        let complete = reasons.is_empty();
        (
            json!({
                "status": audit["status"], "fresh": fresh,
                "completed_root_sample_count": audit.pointer("/scope/completed_root_sample_count"),
                "selected_root_count": audit.pointer("/scope/selected_root_count"),
                "usage_observed_rollouts": root["provider_reported_usage"]["rollout_count_with_usage"],
                "usage_selected_rollouts": audit.pointer("/scope/selected_root_count"),
                "collection_complete": audit["collection_complete"],
                "recommendation_evidence_complete": audit.pointer("/coverage/recommendation_evidence_complete").and_then(Value::as_bool),
                "complete_for_aggregate_context": complete,
                "reason_codes": reasons.clone(),
                "source_authenticity_verified": false,
                "used_for_model_ranking": false,
                "raw_conversations_exported": false
            }),
            complete,
            reasons,
        )
    } else {
        (
            json!({"status":"MISSING", "reason_codes":["native_audit_missing"],
                "complete_for_aggregate_context":false, "used_for_model_ranking":false,
                "source_authenticity_verified":false, "raw_conversations_exported":false}),
            false,
            vec!["native_audit_missing"],
        )
    };
    let (insights, insights_complete, insights_reasons) = if let Some(path) = report {
        let report = WeeklyReport::from_slice(&super::load_bounded(
            path,
            groundline_contracts::insights::MAX_WEEKLY_REPORT_BYTES as u64,
        )?)?;
        let report_freshness = timestamp_freshness(
            Some(&json!(report.generated_at_utc)),
            "clickhouse_report_stale",
            "clickhouse_report_timestamp_unknown",
        );
        let report_fresh = report_freshness.is_ok();
        let reasons = clickhouse_reasons(&report, report_freshness);
        let complete = reasons.is_empty();
        (
            json!({
                "status": if complete { "AVAILABLE" } else { "PARTIAL" },
                "fresh": report_fresh,
                "requested_days": report.requested_days,
                "data_quality": report.data_quality,
                "coverage": report.coverage,
                "freshness_status": report.collection_health.freshness_status,
                "reason_codes": reasons.clone(),
                "quarantined_event_count": report.collection_health.quarantined_event_count,
                "model_effort_context_distribution": report.cohorts.model_effort_context_distribution,
                "model_token_distribution": report.cohorts.model_token_distribution,
                "used_for_model_ranking": false,
                "model_delivery_outcomes_available": false,
                "scope": "owner_aggregate_not_a_unique_task_ledger"
            }),
            complete,
            reasons,
        )
    } else {
        (
            json!({"status":"MISSING", "reason_codes":["clickhouse_report_missing"], "used_for_model_ranking":false,
            "model_delivery_outcomes_available":false}),
            false,
            vec!["clickhouse_report_missing"],
        )
    };
    // Only the direct packet's own quality gates its comparison. Aggregate
    // coverage limits context and generalization, not complete matched outcomes.
    let mut result = groundline_contracts::routing::propose(&packet, &catalog)?;
    if let Some(records) = &delivery_records {
        result["delivery_records"] = records.summary.clone();
        if records.unattributed_matched > 0 && result["status"] != "PINNED" {
            result["status"] = json!("INCONCLUSIVE");
            result["suggestion"] = Value::Null;
            result["reason_codes"]
                .as_array_mut()
                .expect("proposal reason codes")
                .push(json!("matched_delivery_execution_unobserved"));
            for candidate in result["candidate_assessment"].as_array_mut().unwrap() {
                candidate["selected"] = json!(false);
            }
        }
    }
    result["native_audit"] = audit_summary;
    result["clickhouse"] = insights;
    let mut aggregate_limitations = audit_reasons;
    for reason in insights_reasons {
        if !aggregate_limitations.contains(&reason) {
            aggregate_limitations.push(reason);
        }
    }
    result["data_readiness"] = json!({
        "aggregate_sources_ready": audit_complete && insights_complete,
        "aggregate_context_limitations": aggregate_limitations
    });
    result["evidence_scope"] = json!({
        "conversation_classification": "operator_observed_private_packet",
        "cohort_matching": "operator_supplied_fingerprint",
        "digests_independently_verified": false,
        "aggregate_and_outcome_counters_added": false,
        "aggregate_evidence_role": "context_only",
        "empirical_selection_requires_complete_direct_outcomes": true
    });
    // This advice concerns an evidence-based replacement only. Missing empirical
    // support does not suspend ordinary task-scoped judgment by native Codex.
    let (application_mode, target) = match result["status"].as_str() {
        Some("EMPIRICAL") => (
            "next_authorized_lane_with_acceptance_checks",
            "next_authorized_task_or_subagent_lane",
        ),
        Some("PINNED") => ("preserve_explicit_selection", "none"),
        _ => ("no_evidence_based_replacement", "none"),
    };
    result["native_execution"] = json!({
        "scope": "evidence_based_replacement_only",
        "ordinary_task_selection_owner": "native_codex",
        "application_mode": application_mode,
        "target": target,
        "baseline_selection": {
            "model": packet["current"]["model"],
            "effort": packet["current"]["effort"]
        },
        "root_self_switch_supported": false,
        "activation_verified": false,
        "configuration_changed": false,
        "recheck_at_meaningful_phase_change": true
    });
    result["network_performed"] = json!(false);
    result["mutation_performed"] = json!(false);
    Ok(result)
}
