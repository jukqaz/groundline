//! Offline comparison of private GPT-6 delivery outcomes and a native catalog.
//! Native Codex owns task-scoped model/effort judgment and catalog provenance.
use crate::ContractError;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod comparison;
use comparison::{
    MIN_IMPROVEMENT_PERCENT, Metrics, comparison_reasons, eligibility_reasons, eligible, metrics,
    score,
};

const MIN_UNITS: usize = 10;
const ROUTING_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultra"];

fn error(code: &str) -> ContractError {
    ContractError(format!("routing_{code}"))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn timestamp(value: &str) -> Result<DateTime<Utc>, ContractError> {
    DateTime::parse_from_rfc3339(value)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| error("invalid_timestamp"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    kind: String,
    schema: u8,
    generated_at_utc: String,
    catalog_checked_at_utc: String,
    catalog_sha256: String,
    quality_status: String,
    task: Task,
    cohort_sha256: String,
    current: Selection,
    objective: String,
    outcomes: Vec<Outcome>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Task {
    kind: String,
    phase: Option<String>,
    complexity: String,
    evidence_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    model: String,
    effort: String,
    explicit: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    unit_hash: String,
    evidence_sha256: String,
    evidence_kind: String,
    cohort_sha256: String,
    model: String,
    effort: String,
    outcome: String,
    rework: bool,
    owned_total_tokens: Option<u64>,
    wall_duration_ms: Option<u64>,
    owned_resources_complete: bool,
    completed_at_utc: String,
}
#[derive(Deserialize)]
struct Catalog {
    models: Vec<CatalogModel>,
}
#[derive(Deserialize)]
struct CatalogModel {
    slug: String,
    supported_reasoning_levels: Vec<CatalogEffort>,
}
#[derive(Deserialize)]
struct CatalogEffort {
    effort: String,
}

fn supported(catalog: &Catalog, model: &str, effort: &str) -> bool {
    crate::model::optimization_model(model)
        && ROUTING_EFFORTS.contains(&effort)
        && catalog.models.iter().any(|m| {
            m.slug == model
                && m.supported_reasoning_levels
                    .iter()
                    .any(|e| e.effort == effort)
        })
}

fn validate(
    packet: &Packet,
    catalog: &Catalog,
    catalog_value: &Value,
) -> Result<(DateTime<Utc>, DateTime<Utc>), ContractError> {
    let now = Utc::now();
    let generated = timestamp(&packet.generated_at_utc)?;
    let catalog_checked = timestamp(&packet.catalog_checked_at_utc)?;
    let catalog_bytes = serde_json::to_vec(catalog_value).map_err(|_| error("invalid_catalog"))?;
    let catalog_hash = format!("{:x}", Sha256::digest(&catalog_bytes));
    if packet.kind != "groundline-routing-evidence"
        || packet.schema != 2
        || !["PASS", "PARTIAL", "FAIL"].contains(&packet.quality_status.as_str())
        || ![
            "implementation",
            "research",
            "review",
            "operations",
            "documentation",
        ]
        .contains(&packet.task.kind.as_str())
        || packet
            .task
            .phase
            .as_deref()
            .is_some_and(|phase| !crate::delivery::PHASES.contains(&phase))
        || !["routine", "multi_step", "deep_judgment"].contains(&packet.task.complexity.as_str())
        || !["tokens", "latency", "balanced"].contains(&packet.objective.as_str())
        || !digest(&packet.cohort_sha256)
        || !digest(&packet.task.evidence_sha256)
        || packet.catalog_sha256 != catalog_hash
        || catalog_checked > generated
        || !crate::model::valid_id(&packet.current.model)
        || !ROUTING_EFFORTS.contains(&packet.current.effort.as_str())
        || packet.outcomes.len() > 1000
        || generated > now + Duration::minutes(5)
    {
        return Err(error("invalid_evidence"));
    }
    let mut ids = BTreeSet::new();
    for row in &packet.outcomes {
        if !digest(&row.unit_hash)
            || !ids.insert(&row.unit_hash)
            || !digest(&row.evidence_sha256)
            || !digest(&row.cohort_sha256)
            || !crate::model::valid_id(&row.model)
            || !ROUTING_EFFORTS.contains(&row.effort.as_str())
            || !["verified", "failed", "unknown"].contains(&row.outcome.as_str())
            || !["runtime_check", "user_acceptance", "unobserved"]
                .contains(&row.evidence_kind.as_str())
            || (row.outcome == "unknown") != (row.evidence_kind == "unobserved")
            || timestamp(&row.completed_at_utc)? > generated
            || generated - timestamp(&row.completed_at_utc)? > Duration::days(30)
        {
            return Err(error("invalid_outcome"));
        }
    }
    // Native catalog is an availability source, not an instruction. Reject ambiguous entries.
    let mut model_names = BTreeSet::new();
    if catalog.models.len() > 512 {
        return Err(error("invalid_catalog"));
    }
    for model in &catalog.models {
        if !crate::model::valid_id(&model.slug)
            || !model_names.insert(&model.slug)
            || model.supported_reasoning_levels.len() > 16
            || model
                .supported_reasoning_levels
                .iter()
                .map(|e| &e.effort)
                .collect::<BTreeSet<_>>()
                .len()
                != model.supported_reasoning_levels.len()
        {
            return Err(error("invalid_catalog"));
        }
    }
    Ok((generated, catalog_checked))
}

/// Return a next-lane suggestion. This never writes Codex configuration.
pub fn propose(packet: &Value, catalog: &Value) -> Result<Value, ContractError> {
    if packet["schema"] != 2 {
        return Err(error("unsupported_evidence_schema"));
    }
    let packet: Packet =
        serde_json::from_value(packet.clone()).map_err(|_| error("invalid_evidence"))?;
    let parsed_catalog: Catalog =
        serde_json::from_value(catalog.clone()).map_err(|_| error("invalid_catalog"))?;
    let (generated, catalog_checked) = validate(&packet, &parsed_catalog, catalog)?;
    let base = |status: &str, reasons: Vec<&str>, suggestion: Value| {
        json!({
            "kind":"groundline-routing-proposal","schema":2,"status":status,
            "reason_codes":reasons,"suggestion":suggestion,
            "candidate_assessment":[],
            "selection_policy":{"minimum_units_per_pair":MIN_UNITS,
                "minimum_relative_improvement":f64::from(MIN_IMPROVEMENT_PERCENT) / 100.0,"typical_resources_must_not_regress":true,
                "scope":"matched_direct_outcome_comparison",
                "task_shape_used_for_model_ranking":false,
                "statistical_significance_claimed":false},
            "automatic_config_write":false,"causal_improvement_claimed":false,
            "optimality_claimed":false,"raw_content_emitted":false,
            "evidence_origin":"operator_supplied",
            "catalog_evidence":{
                "source":"operator_supplied_native_catalog",
                "checked_at_utc":packet.catalog_checked_at_utc,
                "sha256":packet.catalog_sha256,
                "fresh":Utc::now() - catalog_checked <= Duration::hours(24),
                "freshness_basis":"operator_timestamp_as_of_evaluation",
                "host_identity":"unobserved","refresh_channel":"unobserved",
                "source_authenticity_verified":false,"account_availability_verified":false
            },
        })
    };
    let mut selection_limits = Vec::new();
    if !crate::model::optimization_model(&packet.current.model) {
        selection_limits.push("current_selection_outside_optimization_scope");
    }
    if Utc::now() - generated > Duration::hours(24) {
        selection_limits.push("routing_evidence_stale");
    }
    if Utc::now() - catalog_checked > Duration::hours(24) {
        selection_limits.push("native_catalog_stale");
    }
    if crate::model::optimization_model(&packet.current.model)
        && !supported(
            &parsed_catalog,
            &packet.current.model,
            &packet.current.effort,
        )
    {
        selection_limits.push("current_selection_not_in_native_catalog");
    }
    if packet.current.explicit {
        let mut reasons = vec!["explicit_selection_preserved"];
        reasons.extend(selection_limits);
        return Ok(base("PINNED", reasons, Value::Null));
    }
    if !selection_limits.is_empty() {
        return Ok(base("INCONCLUSIVE", selection_limits, Value::Null));
    }
    if packet.quality_status == "FAIL" {
        return Ok(base(
            "INCONCLUSIVE",
            vec!["evidence_quality_failed"],
            Value::Null,
        ));
    }
    if packet.outcomes.iter().any(|row| {
        row.cohort_sha256 == packet.cohort_sha256 && !crate::model::optimization_model(&row.model)
    }) {
        return Ok(base(
            "INCONCLUSIVE",
            vec!["matched_outcome_outside_optimization_scope"],
            Value::Null,
        ));
    }
    if packet.outcomes.iter().any(|row| {
        row.cohort_sha256 == packet.cohort_sha256
            && !supported(&parsed_catalog, &row.model, &row.effort)
    }) {
        return Ok(base(
            "INCONCLUSIVE",
            vec!["matched_outcome_selection_not_in_native_catalog"],
            Value::Null,
        ));
    }
    if packet.quality_status != "PASS"
        && packet
            .outcomes
            .iter()
            .any(|row| row.cohort_sha256 == packet.cohort_sha256)
    {
        return Ok(base(
            "INCONCLUSIVE",
            vec!["quality_blocks_matched_outcome_comparison"],
            Value::Null,
        ));
    }
    if packet.quality_status == "PASS" {
        let mut groups: BTreeMap<(&str, &str), Vec<&Outcome>> = BTreeMap::new();
        for row in &packet.outcomes {
            if row.cohort_sha256 == packet.cohort_sha256
                && supported(&parsed_catalog, &row.model, &row.effort)
            {
                groups
                    .entry((&row.model, &row.effort))
                    .or_default()
                    .push(row);
            }
        }
        let matched_rows_exist = !groups.is_empty();
        let baseline = groups
            .get(&(
                packet.current.model.as_str(),
                packet.current.effort.as_str(),
            ))
            .map(|rows| metrics(rows));
        let baseline_ready = baseline.as_ref().is_some_and(eligible);
        let mut best: Option<(&str, &str, Metrics)> = None;
        let mut comparable_candidates = 0;
        let mut assessments = Vec::new();
        for ((model, effort), rows) in groups {
            if model == packet.current.model && effort == packet.current.effort {
                continue;
            }
            let candidate = metrics(&rows);
            let candidate_ready = eligible(&candidate);
            let reasons = if let Some(baseline) = baseline.as_ref() {
                comparison_reasons(&candidate, baseline, &packet.objective)
            } else {
                let mut reasons = eligibility_reasons(&candidate);
                reasons.push("baseline_outcomes_missing");
                reasons
            };
            if baseline_ready && candidate_ready {
                comparable_candidates += 1;
            }
            assessments.push(json!({"model":model,"effort":effort,
                "status":if !baseline_ready || !candidate_ready { "INSUFFICIENT" }
                    else if reasons.is_empty() { "ELIGIBLE" } else { "REJECTED" },
                "reason_codes":reasons,"units":candidate.count,
                "verified":candidate.verified,"rework":candidate.rework,
                "resources_complete":candidate.complete,"unknown_outcome":candidate.unknown,
                "median_tokens":candidate.median_tokens_twice.map(|n| n as f64 / 2.0),
                "median_wall_ms":candidate.median_wall_twice.map(|n| n as f64 / 2.0),
                "selected":false}));
            if reasons.is_empty()
                && best.as_ref().is_none_or(|(_, _, m)| {
                    score(&candidate, &packet.objective) < score(m, &packet.objective)
                })
            {
                best = Some((model, effort, candidate));
            }
        }
        if let (Some((model, effort, candidate)), Some(baseline)) = (best, baseline.as_ref()) {
            for assessment in &mut assessments {
                if assessment["model"] == model && assessment["effort"] == effort {
                    assessment["selected"] = json!(true);
                }
            }
            let mut result = base(
                "EMPIRICAL",
                vec!["observational_comparison_only"],
                json!({
                    "model":model,"effort":effort,"evidence_class":"matched_direct_outcomes",
                    "baseline_units":baseline.count,"candidate_units":candidate.count,
                    "baseline_verified":baseline.verified,"candidate_verified":candidate.verified,
                    "baseline_rework":baseline.rework,"candidate_rework":candidate.rework,
                    "baseline_tokens_per_verified":baseline.tokens as f64 / baseline.verified as f64,
                    "candidate_tokens_per_verified":candidate.tokens as f64 / candidate.verified as f64,
                    "baseline_wall_ms_per_verified":baseline.wall as f64 / baseline.verified as f64,
                    "candidate_wall_ms_per_verified":candidate.wall as f64 / candidate.verified as f64,
                }),
            );
            result["candidate_assessment"] = json!(assessments);
            return Ok(result);
        }
        if matched_rows_exist {
            let mut result = if comparable_candidates > 0 {
                base(
                    "RETAIN",
                    vec!["no_meaningful_safe_improvement_in_matched_direct_outcomes"],
                    json!({"model":packet.current.model,"effort":packet.current.effort,
                        "evidence_class":"protected_outcomes_resources_and_margin"}),
                )
            } else {
                base(
                    "INCONCLUSIVE",
                    vec!["matched_direct_outcomes_insufficient"],
                    Value::Null,
                )
            };
            result["candidate_assessment"] = json!(assessments);
            result["baseline_assessment"] = baseline
                .as_ref()
                .map(|m| {
                    json!({
                        "units":m.count,"verified":m.verified,"reason_codes":eligibility_reasons(m)
                    })
                })
                .unwrap_or_else(|| json!({"units":0,"reason_codes":["baseline_outcomes_missing"]}));
            return Ok(result);
        }
    }
    // No matched observations means no empirical replacement advice. Ordinary
    // task selection remains native judgment; task labels do not rank model pairs.
    Ok(base(
        "INCONCLUSIVE",
        vec!["native_task_judgment_required"],
        Value::Null,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u64) -> String {
        format!("{n:064x}")
    }
    fn catalog() -> Value {
        json!({"models":[
            {"slug":"gpt-6.1-sol","supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"},{"effort":"ultra"}]},
            {"slug":"gpt-6-astra","supported_reasoning_levels":[{"effort":"high"},{"effort":"max"},{"effort":"ultra"}]},
            {"slug":"gpt-6-sol","supported_reasoning_levels":[{"effort":"medium"},{"effort":"max"}]},
            {"slug":"gpt-6-luna","supported_reasoning_levels":[{"effort":"low"},{"effort":"high"}]}
        ]})
    }
    fn packet(catalog: &Value) -> Value {
        let now = Utc::now().to_rfc3339();
        json!({
            "kind":"groundline-routing-evidence","schema":2,
            "generated_at_utc":now,"catalog_checked_at_utc":now,
            "catalog_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(catalog).unwrap())),
            "quality_status":"PASS",
            "task":{"kind":"implementation","complexity":"multi_step","evidence_sha256":h(1)},
            "cohort_sha256":h(2),"current":{"model":"gpt-6-sol","effort":"medium","explicit":false},
            "objective":"tokens","outcomes":[]
        })
    }
    fn row(n: u64, model: &str, effort: &str, tokens: Option<u64>) -> Value {
        json!({"unit_hash":h(n),"evidence_sha256":h(1000+n),"evidence_kind":"runtime_check",
            "cohort_sha256":h(2),"model":model,"effort":effort,"outcome":"verified",
            "rework":false,"owned_total_tokens":tokens,"wall_duration_ms":1000,
            "owned_resources_complete":true,"completed_at_utc":(Utc::now()-Duration::seconds(1)).to_rfc3339()})
    }
    fn twenty(packet: &mut Value) {
        packet["outcomes"] = json!(
            (0..10)
                .map(|i| row(i + 10, "gpt-6-sol", "medium", Some(100)))
                .chain((0..10).map(|i| row(i + 30, "gpt-6-luna", "high", Some(70))))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn direct_matched_outcomes_can_suggest_next_lane_without_causal_claim() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        let result = propose(&p, &c).unwrap();
        assert_eq!(result["status"], "EMPIRICAL");
        assert_eq!(result["suggestion"]["model"], "gpt-6-luna");
        assert_eq!(result["suggestion"]["candidate_tokens_per_verified"], 70.0);
        assert_eq!(result["automatic_config_write"], false);
        assert_eq!(result["causal_improvement_claimed"], false);
    }
    #[test]
    fn failures_count_and_protected_outcomes_block_cheaper_candidate() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        p["outcomes"][10]["outcome"] = json!("failed");
        assert_eq!(propose(&p, &c).unwrap()["status"], "RETAIN");
        p["outcomes"][10]["outcome"] = json!("verified");
        p["outcomes"][10]["rework"] = json!(true);
        assert_eq!(propose(&p, &c).unwrap()["status"], "RETAIN");
    }
    #[test]
    fn token_savings_cannot_hide_unapproved_latency_regression() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        for row in p["outcomes"].as_array_mut().unwrap().iter_mut().skip(10) {
            row["wall_duration_ms"] = json!(86_400_000);
        }
        assert_eq!(propose(&p, &c).unwrap()["status"], "RETAIN");
        p["objective"] = json!("latency");
        for row in p["outcomes"].as_array_mut().unwrap().iter_mut().skip(10) {
            row["wall_duration_ms"] = json!(900);
            row["owned_total_tokens"] = json!(101);
        }
        assert_eq!(propose(&p, &c).unwrap()["status"], "RETAIN");
    }
    #[test]
    fn unknown_missing_resource_partial_quality_and_small_sample_are_not_empirical() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        p["outcomes"][10]["owned_total_tokens"] = Value::Null;
        assert_eq!(propose(&p, &c).unwrap()["status"], "INCONCLUSIVE");
        p["outcomes"][10]["owned_total_tokens"] = json!(70);
        p["outcomes"][10]["outcome"] = json!("unknown");
        p["outcomes"][10]["evidence_kind"] = json!("unobserved");
        assert_eq!(propose(&p, &c).unwrap()["status"], "INCONCLUSIVE");
        p["outcomes"][10]["outcome"] = json!("verified");
        p["outcomes"][10]["evidence_kind"] = json!("runtime_check");
        p["quality_status"] = json!("PARTIAL");
        assert_eq!(propose(&p, &c).unwrap()["status"], "INCONCLUSIVE");
        p["quality_status"] = json!("PASS");
        p["outcomes"].as_array_mut().unwrap().pop();
        let out = propose(&p, &c).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert_eq!(
            out["candidate_assessment"][0]["reason_codes"],
            json!(["sample_below_minimum"])
        );
    }
    #[test]
    fn pins_and_unavailable_catalog_never_guess_a_fallback() {
        let c = catalog();
        let mut p = packet(&c);
        p["current"]["explicit"] = json!(true);
        assert_eq!(propose(&p, &c).unwrap()["status"], "PINNED");
        p["current"]["explicit"] = json!(false);
        p["current"]["effort"] = json!("ultra");
        assert_eq!(propose(&p, &c).unwrap()["status"], "INCONCLUSIVE");
        p["current"]["model"] = json!("gpt-5.6-sol");
        let result = propose(&p, &c).unwrap();
        assert_eq!(result["status"], "INCONCLUSIVE");
        assert!(
            result["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("current_selection_outside_optimization_scope"))
        );
    }
    #[test]
    fn sol_61_is_distinct_and_requires_current_native_support() {
        let c = catalog();
        let mut p = packet(&c);
        p["current"]["model"] = json!("gpt-6.1-sol");
        assert_eq!(
            propose(&p, &c).unwrap()["reason_codes"],
            json!(["native_task_judgment_required"])
        );
        p["current"]["explicit"] = json!(true);
        assert_eq!(propose(&p, &c).unwrap()["status"], "PINNED");
        p["current"]["effort"] = json!("max");
        let pinned = propose(&p, &c).unwrap();
        assert_eq!(pinned["status"], "PINNED");
        assert!(
            pinned["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("current_selection_not_in_native_catalog"))
        );
        assert!(pinned["suggestion"].is_null());
        p["current"]["explicit"] = json!(false);
        p["current"]["effort"] = json!("medium");
        p["outcomes"] = json!(
            (0..10)
                .map(|i| row(i + 10, "gpt-6.1-sol", "medium", Some(100)))
                .chain((0..10).map(|i| row(i + 30, "gpt-6-sol", "medium", Some(70))))
                .collect::<Vec<_>>()
        );
        let result = propose(&p, &c).unwrap();
        assert_eq!(result["status"], "EMPIRICAL");
        assert_eq!(result["suggestion"]["model"], "gpt-6-sol");
        assert_eq!(result["automatic_config_write"], false);
    }
    #[test]
    fn valid_out_of_scope_ids_and_stale_catalog_are_not_malformed_packets() {
        let c = catalog();
        let mut p = packet(&c);
        for model in [
            "gpt-5.6-sol",
            "gpt-6.2-sol",
            "gpt-7-sol",
            "gpt-6.1-sol-unconfirmed-snapshot",
        ] {
            p["current"]["model"] = json!(model);
            let result = propose(&p, &c).unwrap();
            assert_eq!(result["status"], "INCONCLUSIVE");
            assert_eq!(
                result["reason_codes"][0],
                "current_selection_outside_optimization_scope"
            );
            assert!(result["suggestion"].is_null());
        }
        p["current"]["model"] = json!("gpt-6.1-sol");
        p["catalog_checked_at_utc"] = json!((Utc::now() - Duration::days(2)).to_rfc3339());
        let result = propose(&p, &c).unwrap();
        assert_eq!(result["status"], "INCONCLUSIVE");
        assert_eq!(result["reason_codes"], json!(["native_catalog_stale"]));
        p["current"]["model"] = json!("invalid\nmodel");
        assert!(propose(&p, &c).is_err());
    }
    #[test]
    fn task_shapes_without_direct_outcomes_require_native_judgment() {
        let c = catalog();
        let mut p = packet(&c);
        for kind in [
            "implementation",
            "research",
            "review",
            "operations",
            "documentation",
        ] {
            for complexity in ["routine", "multi_step", "deep_judgment"] {
                for quality in ["PASS", "PARTIAL"] {
                    p["task"]["kind"] = json!(kind);
                    p["task"]["complexity"] = json!(complexity);
                    p["quality_status"] = json!(quality);
                    let result = propose(&p, &c).unwrap();
                    assert_eq!(result["status"], "INCONCLUSIVE");
                    assert_eq!(
                        result["reason_codes"],
                        json!(["native_task_judgment_required"])
                    );
                    assert!(result["suggestion"].is_null());
                    assert_eq!(
                        result["selection_policy"]["task_shape_used_for_model_ranking"],
                        false
                    );
                }
            }
        }
    }
    #[test]
    fn observed_sol_and_luna_max_are_not_replaced_by_task_shape_defaults() {
        let mut c = catalog();
        c["models"][3]["supported_reasoning_levels"]
            .as_array_mut()
            .unwrap()
            .push(json!({"effort":"max"}));
        for model in ["gpt-6-sol", "gpt-6-luna"] {
            let mut p = packet(&c);
            p["task"]["complexity"] = json!("routine");
            twenty(&mut p);
            for row in p["outcomes"].as_array_mut().unwrap().iter_mut().skip(10) {
                row["model"] = json!(model);
                row["effort"] = json!("max");
            }
            let result = propose(&p, &c).unwrap();
            assert_eq!(result["status"], "EMPIRICAL");
            assert_eq!(result["suggestion"]["model"], model);
            assert_eq!(result["suggestion"]["effort"], "max");
        }
    }
    #[test]
    fn supported_ultra_uses_the_same_outcome_guards_without_feature_policy() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        for row in p["outcomes"].as_array_mut().unwrap().iter_mut().skip(10) {
            row["model"] = json!("gpt-6-astra");
            row["effort"] = json!("ultra");
        }
        let result = propose(&p, &c).unwrap();
        assert_eq!(result["status"], "EMPIRICAL");
        assert_eq!(result["schema"], 2);
        assert!(result.get("feature_assessment").is_none());
        assert!(result.get("feature_suggestions").is_none());
        assert!(result.get("verification_needed").is_none());
        p["outcomes"][10]["rework"] = json!(true);
        assert_eq!(propose(&p, &c).unwrap()["status"], "RETAIN");
    }
    #[test]
    fn private_fields_duplicate_units_and_stale_catalog_are_rejected() {
        let c = catalog();
        let mut p = packet(&c);
        p["prompt"] = json!("private");
        assert!(propose(&p, &c).is_err());
        p.as_object_mut().unwrap().remove("prompt");
        p["task"]["private_path"] = json!("/private");
        assert!(propose(&p, &c).is_err());
        p["task"].as_object_mut().unwrap().remove("private_path");
        p["outcomes"] = json!([
            row(1, "gpt-6-sol", "medium", Some(1)),
            row(1, "gpt-6-sol", "medium", Some(1))
        ]);
        assert!(propose(&p, &c).is_err());
        p["outcomes"] = json!([]);
        p["catalog_checked_at_utc"] = json!((Utc::now() - Duration::days(2)).to_rfc3339());
        assert_eq!(
            propose(&p, &c).unwrap()["reason_codes"],
            json!(["native_catalog_stale"])
        );
    }
    #[test]
    fn arbitrary_effort_and_retired_policy_fields_are_rejected() {
        let c = catalog();
        let mut p = packet(&c);
        p["current"]["effort"] = json!("none");
        assert!(propose(&p, &c).is_err());
        p["current"]["effort"] = json!("medium");
        p["outcomes"] = json!([row(1, "gpt-6-sol", "legacy_effort", Some(1))]);
        assert!(propose(&p, &c).is_err());
        p["outcomes"] = json!([]);
        p["features"] = json!([]);
        assert!(propose(&p, &c).is_err());
        p.as_object_mut().unwrap().remove("features");
        p["task"]["independent_lanes"] = json!(true);
        assert!(propose(&p, &c).is_err());
        p["task"]
            .as_object_mut()
            .unwrap()
            .remove("independent_lanes");
        p["schema"] = json!(1);
        assert_eq!(
            propose(&p, &c).unwrap_err().0,
            "routing_unsupported_evidence_schema"
        );
    }
    #[test]
    fn unsupported_historical_gpt6_effort_is_not_silently_excluded() {
        let c = catalog();
        let mut p = packet(&c);
        p["outcomes"] = json!([row(1, "gpt-6-luna", "max", Some(5))]);
        let result = propose(&p, &c).unwrap();
        assert_eq!(result["status"], "INCONCLUSIVE");
        assert!(result["suggestion"].is_null());
    }
    #[test]
    fn failed_evidence_cannot_bootstrap_a_replacement() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        p["quality_status"] = json!("FAIL");
        let out = propose(&p, &c).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert!(out["suggestion"].is_null());
    }
    #[test]
    fn small_gains_and_typical_regressions_explain_why_a_candidate_is_rejected() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        for row in p["outcomes"].as_array_mut().unwrap().iter_mut().skip(10) {
            row["owned_total_tokens"] = json!(99);
        }
        let out = propose(&p, &c).unwrap();
        assert_eq!(out["status"], "RETAIN");
        assert_eq!(
            out["candidate_assessment"][0]["reason_codes"],
            json!(["improvement_below_margin"])
        );
        assert_eq!(
            out["selection_policy"]["statistical_significance_claimed"],
            false
        );
        for row in p["outcomes"].as_array_mut().unwrap().iter_mut().skip(10) {
            row["owned_total_tokens"] = json!(101);
        }
        p["outcomes"][10]["owned_total_tokens"] = json!(1);
        let out = propose(&p, &c).unwrap();
        assert_eq!(out["status"], "RETAIN");
        assert_eq!(out["candidate_assessment"][0]["median_tokens"], 101.0);
        assert_eq!(
            out["candidate_assessment"][0]["reason_codes"],
            json!(["typical_resource_regression"])
        );
    }
    #[test]
    fn missing_resources_are_inconclusive_not_evidence_against_a_candidate() {
        let c = catalog();
        let mut p = packet(&c);
        twenty(&mut p);
        p["outcomes"][10]["owned_total_tokens"] = Value::Null;
        let out = propose(&p, &c).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert_eq!(out["candidate_assessment"][0]["status"], "INSUFFICIENT");
        assert_eq!(
            out["candidate_assessment"][0]["reason_codes"],
            json!(["missing_owned_resources"])
        );
        assert!(out["candidate_assessment"][0]["median_tokens"].is_null());
        assert!(out["suggestion"].is_null());
    }
}
