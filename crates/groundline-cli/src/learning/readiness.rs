//! Pure readiness counts for the caller's bounded, validated record closure.
//! Capture, correlation and an assessment assertion are never delivery proof.
use std::collections::{BTreeMap, BTreeSet};

use groundline_contracts::{ContractError, delivery, learning};
use serde_json::{Value, json};

use super::{MAX_ENTRIES, error};

const COUNTERS: &[&str] = &[
    "task_start_count",
    "task_outcome_count",
    "assessment_count",
    "pending_assessment_count",
    "assessment_pending_connection_count",
    "unlinked_start_count",
    "native_start_unmatched_count",
    "runtime_missing_start_count",
    "environment_revision_missing_start_count",
    "target_revision_missing_start_count",
    "source_revision_missing_start_count",
    "receipt_missing_outcome_count",
    "cost_unknown_outcome_count",
    "effective_selection_missing_outcome_count",
    "verification_unknown_outcome_count",
    "failed_outcome_count",
    "rework_outcome_count",
    "correction_unknown_outcome_count",
];

struct Group<'a> {
    counts: BTreeMap<&'static str, usize>,
    receipts: BTreeMap<&'a str, &'a Value>,
}

impl Default for Group<'_> {
    fn default() -> Self {
        Self {
            counts: COUNTERS.iter().map(|&key| (key, 0)).collect(),
            receipts: BTreeMap::new(),
        }
    }
}

impl Group<'_> {
    fn count(&mut self, key: &'static str, present: bool) {
        if present {
            *self.counts.entry(key).or_default() += 1;
        }
    }
}

fn category(task: Option<&Value>) -> Option<&str> {
    task.and_then(|task| task["scope"]["task_category"].as_str())
}

/// `records` are the checked State's active/reference closure, not a lifetime
/// scan. Receipt hashes must come from the caller's checked original bytes;
/// reserializing a receipt here cannot recover its original byte identity.
pub(super) fn summarize(
    records: &[Value],
    receipts: &[(String, Value)],
) -> Result<Value, ContractError> {
    if records.len() > MAX_ENTRIES || receipts.len() > MAX_ENTRIES {
        return Err(error("readiness_scope_limit_exceeded"));
    }
    let records: BTreeMap<_, _> = records
        .iter()
        .map(|record| Ok((learning::content_sha256(record)?, record)))
        .collect::<Result<_, ContractError>>()?;
    let mut receipt_by_hash = BTreeMap::new();
    for (hash, receipt) in receipts {
        if !learning::digest(hash) {
            return Err(error("invalid_receipt_digest"));
        }
        if let Some(previous) = receipt_by_hash.insert(hash.as_str(), receipt)
            && previous != receipt
        {
            return Err(error("readiness_receipt_conflict"));
        }
    }
    let receipt_values: Vec<_> = receipt_by_hash.values().map(|v| (*v).clone()).collect();
    delivery::validate_receipt_collection(&receipt_values)?;

    let tasks: BTreeMap<_, _> = records
        .iter()
        .filter(|(_, value)| value["kind"] == "groundline-learning-task-start")
        .map(|(hash, value)| (hash.as_str(), *value))
        .collect();
    let mut outcomes = BTreeMap::new();
    let mut assessments = BTreeMap::<&str, usize>::new();
    let mut referenced_snapshots = BTreeSet::new();
    let mut referenced_links = BTreeSet::new();
    for record in records.values() {
        match record["kind"].as_str() {
            Some("groundline-learning-task-start") => {
                if let Some(hash) = record["snapshot_sha256"].as_str() {
                    referenced_snapshots.insert(hash);
                }
            }
            Some("groundline-learning-task-outcome") => {
                let hash = record["task_sha256"]
                    .as_str()
                    .ok_or_else(|| error("invalid_task_outcome"))?;
                if outcomes.insert(hash, *record).is_some() {
                    return Err(error("task_outcome_conflict"));
                }
                if let Some(hash) = record["link_sha256"].as_str() {
                    referenced_links.insert(hash);
                }
            }
            Some("groundline-learning-assessment") => {
                let hash = record["input"]["task_sha256"]
                    .as_str()
                    .ok_or_else(|| error("invalid_assessment"))?;
                *assessments.entry(hash).or_default() += 1;
            }
            _ => {}
        }
    }

    let mut groups = BTreeMap::<Option<&str>, Group<'_>>::new();
    for (hash, task) in &tasks {
        let group = groups.entry(category(Some(task))).or_default();
        let snapshot = task["snapshot_sha256"]
            .as_str()
            .and_then(|hash| records.get(hash))
            .filter(|snapshot| snapshot["kind"] == "groundline-learning-snapshot");
        group.count("task_start_count", true);
        group.count("unlinked_start_count", !outcomes.contains_key(hash));
        group.count(
            "pending_assessment_count",
            !assessments.contains_key(hash) && !outcomes.contains_key(hash),
        );
        group.count(
            "native_start_unmatched_count",
            task["native"]["boundary_matched"] != true,
        );
        group.count(
            "runtime_missing_start_count",
            task["scope"]["runtime"].is_null(),
        );
        for (field, counter) in [
            (
                "environment_revision",
                "environment_revision_missing_start_count",
            ),
            ("skill", "target_revision_missing_start_count"),
            ("source_revision", "source_revision_missing_start_count"),
        ] {
            group.count(
                counter,
                snapshot.is_none_or(|snapshot| snapshot[field].is_null()),
            );
        }
    }
    for (hash, count) in assessments {
        let group = groups
            .entry(category(tasks.get(hash).copied()))
            .or_default();
        *group.counts.entry("assessment_count").or_default() += count;
        if !outcomes.contains_key(hash) {
            *group
                .counts
                .entry("assessment_pending_connection_count")
                .or_default() += count;
        }
    }
    for (task_hash, outcome) in outcomes {
        let task = tasks.get(task_hash).copied();
        let group = groups.entry(category(task)).or_default();
        group.count("task_outcome_count", true);
        let link = outcome["link_sha256"]
            .as_str()
            .and_then(|hash| records.get(hash))
            .filter(|link| link["kind"] == "groundline-learning-link");
        let receipt = outcome["receipt_sha256"]
            .as_str()
            .and_then(|hash| receipt_by_hash.get_key_value(hash));
        let checked = match (link, receipt) {
            (Some(link), Some((hash, receipt))) => {
                let mut input = (*link).clone();
                input["kind"] = json!("groundline-learning-link-input");
                learning::link_outcome(&input, receipt, hash)?;
                if task.is_some_and(|task| {
                    ["unit_hash", "cohort_sha256", "phase"]
                        .into_iter()
                        .any(|field| task["scope"][field] != receipt[field])
                }) {
                    return Err(error("task_receipt_mismatch"));
                }
                group.receipts.insert(hash, receipt);
                Some(*receipt)
            }
            _ => None,
        };
        group.count("receipt_missing_outcome_count", checked.is_none());
        group.count(
            "cost_unknown_outcome_count",
            checked.is_none_or(|receipt| receipt["resources"]["complete"] != true),
        );
        group.count(
            "effective_selection_missing_outcome_count",
            checked.is_none_or(|receipt| receipt["effective"].is_null()),
        );
        group.count(
            "verification_unknown_outcome_count",
            checked.is_none_or(|receipt| receipt["verification"]["status"] == "unknown"),
        );
        group.count(
            "failed_outcome_count",
            checked.is_some_and(|receipt| receipt["verification"]["status"] == "failed"),
        );
        group.count(
            "rework_outcome_count",
            checked.is_some_and(|receipt| receipt["verification"]["rework"] == true),
        );
        group.count(
            "correction_unknown_outcome_count",
            link.is_none_or(|link| link["correction_kind"] == "unknown"),
        );
    }

    let mut totals: BTreeMap<&str, usize> = COUNTERS.iter().map(|&key| (key, 0)).collect();
    let mut categories = Vec::new();
    for (category, group) in groups {
        for (key, count) in &group.counts {
            *totals.entry(key).or_default() += count;
        }
        let receipts: Vec<_> = group.receipts.values().map(|v| (*v).clone()).collect();
        categories.push(json!({"task_category":category,"counts":group.counts,
            "delivery_observations":delivery::summarize_receipts(&receipts)?["overall"]}));
    }
    let analyses = records
        .values()
        .filter(|record| {
            [
                "groundline-learning-proposal",
                "groundline-learning-duplicate-attempt",
            ]
            .contains(&record["kind"].as_str().unwrap_or(""))
        })
        .map(|record| {
            serde_json::from_value::<Option<learning::AnalysisResources>>(
                record["analysis"].clone(),
            )
            .map_err(|_| error("invalid_analysis_resources"))
        })
        .collect::<Result<Vec<_>, ContractError>>()?;
    let mut analysis_resources = learning::summarize_analysis(&analyses, &receipt_values)?;
    if analyses.is_empty() {
        analysis_resources["complete"] = json!(false);
        analysis_resources["additional_owned_total_tokens"] = Value::Null;
    }
    Ok(
        json!({"kind":"groundline-learning-readiness","schema":1,"status":"OBSERVATIONAL",
        "scope":{"coverage_scope":"bounded_active_reference_closure","record_count":records.len(),
            "receipt_count":receipt_by_hash.len(),"max_entries":MAX_ENTRIES,
            "archive_coverage":"referenced_records_only","historical_quality_coverage_complete":false},
        "category_source":"explicit_task_scope","totals":totals,"by_task_category":categories,
        "standalone_capture_count":records.iter().filter(|(hash,record)| record["kind"]=="groundline-learning-snapshot" && !referenced_snapshots.contains(hash.as_str())).count(),
        "link_only_count":records.iter().filter(|(hash,record)| record["kind"]=="groundline-learning-link" && !referenced_links.contains(hash.as_str())).count(),
            "analysis_resources":analysis_resources,
        "native_activation":"UNVERIFIED","causal_effect_verified":false,"efficiency_improvement_verified":false,
        "mutation_performed":false,"model_calls_performed":false,"raw_content_emitted":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u8) -> String {
        format!("{n:064x}")
    }

    fn fixture(category: &str, n: u8) -> (Vec<Value>, (String, Value)) {
        let snapshot = json!({"kind":"groundline-learning-snapshot","schema":1,
            "captured_at_utc":"2026-10-01T00:00:00Z","unit_hash":h(n),"cohort_sha256":h(90),
            "phase":"implementation","environment_revision":null,"source_revision":null,"skill":null,
            "runtime":null,"environment_provenance_sha256":h(91),"capture_input_sha256":h(92),
            "native_activation":"UNVERIFIED"});
        let task = json!({"kind":"groundline-learning-task-start","schema":1,
            "input_sha256":h(93),"scope":{"unit_hash":h(n),"cohort_sha256":h(90),"phase":"implementation",
                "task_category":category,"criterion":{"sha256":h(94),"version":"v1"},"target_id":"workflow","runtime":null},
            "boundary_sha256":h(95),"snapshot_sha256":learning::content_sha256(&snapshot).unwrap(),
            "native":{"session_hash":null,"turn_hash":null,"artifact_sha256":null,"boundary_matched":false},
            "boundary_environment_matched":false,"criterion_change_kind":"unknown","criterion_change_evidence_sha256":null});
        let receipt = json!({"kind":"groundline-delivery-receipt","schema":1,
            "unit_hash":h(n),"cohort_sha256":h(90),"phase":"implementation","completed_at_utc":"2026-10-01T01:00:00Z",
            "recommendation":{"model":"gpt-6.1-sol","effort":"medium","evidence_sha256":h(96)},
            "requested":null,"effective":null,"verification":{"status":"unknown","evidence_kind":"unobserved",
                "evidence_sha256":h(97),"rework":false,"authenticity_verified":false},
            "resources":{"complete":false,"wall_duration_ms":null,"entries":[]},
            "activation_verified":false,"observed_selection_matches_requested":null});
        let raw_hash = receipt_hash(&receipt);
        let input = json!({"kind":"groundline-learning-link-input","schema":1,"receipt_sha256":raw_hash,
            "unit_hash":h(n),"cohort_sha256":h(90),"phase":"implementation","completed_at_utc":"2026-10-01T01:00:00Z",
            "observed_at_utc":null,"environment_revision":null,"source_revision":null,"skill":null,"runtime":null,
            "correction_kind":"unknown","correction_evidence_sha256":null});
        let link = learning::link_outcome(&input, &receipt, &raw_hash).unwrap();
        let outcome = json!({"kind":"groundline-learning-task-outcome","schema":1,"input_sha256":h(98),
            "task_sha256":learning::content_sha256(&task).unwrap(),"boundary_sha256":h(99),
            "native":{"session_hash":null,"turn_hash":null,"artifact_sha256":null,"boundary_matched":false},
            "receipt_sha256":raw_hash,"link_sha256":learning::content_sha256(&link).unwrap()});
        (vec![snapshot, task, link, outcome], (raw_hash, receipt))
    }

    fn receipt_hash(receipt: &Value) -> String {
        let mut bytes = serde_json::to_vec_pretty(receipt).unwrap();
        bytes.push(b'\n');
        learning::continuous::bytes_sha256(&bytes)
    }

    fn bind_receipt(records: &mut [Value], receipt: &Value) -> String {
        let hash = receipt_hash(receipt);
        records[2]["receipt_sha256"] = json!(hash);
        records[3]["receipt_sha256"] = json!(hash);
        records[3]["link_sha256"] = json!(learning::content_sha256(&records[2]).unwrap());
        hash
    }

    #[test]
    fn capture_and_link_only_are_not_completed_work_or_zero_cost() {
        let (records, _) = fixture("installation_verification", 1);
        let out = summarize(&[records[0].clone(), records[2].clone()], &[]).unwrap();
        assert_eq!(out["standalone_capture_count"], 1);
        assert_eq!(out["link_only_count"], 1);
        assert_eq!(out["totals"]["task_outcome_count"], 0);
        assert!(out["by_task_category"].as_array().unwrap().is_empty());
        assert!(out["analysis_resources"]["additional_owned_total_tokens"].is_null());
        assert_eq!(out["scope"]["historical_quality_coverage_complete"], false);
        assert_eq!(out["native_activation"], "UNVERIFIED");
        assert_eq!(out["efficiency_improvement_verified"], false);
    }

    #[test]
    fn assessment_assertions_remain_pending_without_a_direct_outcome() {
        let (records, _) = fixture("natural_work", 2);
        let start = &records[1];
        let initial = summarize(&records[..2], &[]).unwrap();
        assert_eq!(initial["totals"]["pending_assessment_count"], 1);
        let assessment = json!({"kind":"groundline-learning-assessment","schema":1,"input_sha256":h(50),
            "input":{"task_sha256":learning::content_sha256(start).unwrap(),
                "verification":{"status":"verified"}},"status":"PENDING"});
        let out = summarize(
            &[records[0].clone(), start.clone(), assessment.clone()],
            &[],
        )
        .unwrap();
        assert_eq!(out["totals"]["pending_assessment_count"], 0);
        assert_eq!(out["totals"]["assessment_pending_connection_count"], 1);
        assert_eq!(out["totals"]["unlinked_start_count"], 1);
        assert_eq!(out["totals"]["task_outcome_count"], 0);
        assert!(out["by_task_category"][0]["delivery_observations"]["resources"]["all_owners"]["total_tokens"]["known_sum"].is_null());
        let mut complete = records;
        complete.push(assessment);
        let out = summarize(&complete, &[]).unwrap();
        assert_eq!(out["totals"]["assessment_pending_connection_count"], 0);
        assert_eq!(out["totals"]["receipt_missing_outcome_count"], 1);
        assert_eq!(out["totals"]["cost_unknown_outcome_count"], 1);
    }

    #[test]
    fn category_counts_preserve_fixture_and_natural_unknowns_and_deduplicate() {
        let (mut records, receipt_a) = fixture("fixture_verification", 3);
        let (more, receipt_b) = fixture("natural_work", 4);
        records.extend(more);
        records.push(records[1].clone());
        records.push(records[3].clone());
        let out = summarize(&records, &[receipt_a.clone(), receipt_a, receipt_b]).unwrap();
        assert_eq!(out["totals"]["task_start_count"], 2);
        assert_eq!(out["totals"]["task_outcome_count"], 2);
        assert_eq!(out["totals"]["cost_unknown_outcome_count"], 2);
        assert_eq!(
            out["totals"]["effective_selection_missing_outcome_count"],
            2
        );
        assert_eq!(out["totals"]["verification_unknown_outcome_count"], 2);
        assert_eq!(
            out["by_task_category"][0]["task_category"],
            "fixture_verification"
        );
        assert_eq!(out["by_task_category"][1]["task_category"], "natural_work");
        assert_eq!(
            out["by_task_category"][1]["counts"]["runtime_missing_start_count"],
            1
        );
        assert!(out["by_task_category"][1]["delivery_observations"]["resources"]["all_owners"]["total_tokens"]["known_sum"].is_null());
    }

    #[test]
    fn direct_failed_result_and_observed_cost_are_preserved() {
        let (mut records, (_, mut receipt)) = fixture("installation_verification", 5);
        receipt["effective"] =
            json!({"model":"gpt-6.1-sol","effort":"medium","evidence_sha256":h(51)});
        receipt["verification"]["status"] = json!("failed");
        receipt["verification"]["evidence_kind"] = json!("runtime_check");
        receipt["verification"]["rework"] = json!(true);
        receipt["resources"] = json!({"complete":true,"wall_duration_ms":1000,"entries":[
            {"owner":"root","unit_hash":h(5),"response_hash":h(52),"effective":null,
                "input_tokens":5,"cached_input_tokens":0,"output_tokens":5,"reasoning_output_tokens":0,"total_tokens":10}]});
        let hash = bind_receipt(&mut records, &receipt);
        let out = summarize(&records, &[(hash, receipt.clone())]).unwrap();
        assert_eq!(out["totals"]["failed_outcome_count"], 1);
        assert_eq!(out["totals"]["rework_outcome_count"], 1);
        assert_eq!(out["totals"]["cost_unknown_outcome_count"], 0);
        assert_eq!(out["totals"]["verification_unknown_outcome_count"], 0);
        assert_eq!(
            out["by_task_category"][0]["delivery_observations"]["resources"]["all_owners"]["total_tokens"]
                ["known_sum"],
            10
        );
        assert_eq!(out["efficiency_improvement_verified"], false);
        receipt["resources"]["complete"] = json!(false);
        receipt["resources"]["wall_duration_ms"] = Value::Null;
        let hash = bind_receipt(&mut records, &receipt);
        let partial = summarize(&records, &[(hash, receipt)]).unwrap();
        assert_eq!(partial["totals"]["cost_unknown_outcome_count"], 1);
        assert_eq!(
            partial["by_task_category"][0]["delivery_observations"]["resources"]["all_owners"]["total_tokens"]
                ["known_sum"],
            10
        );
        assert!(partial["by_task_category"][0]["delivery_observations"]["resources"]["complete_delivery_total_tokens"]["known_sum"].is_null());
    }

    #[test]
    fn mismatched_direct_receipt_cannot_fill_observation_gaps() {
        let (mut records, (_, mut receipt)) = fixture("natural_work", 6);
        receipt["cohort_sha256"] = json!(h(53));
        let hash = bind_receipt(&mut records, &receipt);
        assert_eq!(
            summarize(&records, &[(hash, receipt)]).unwrap_err().0,
            "learning_receipt_link_mismatch"
        );
        let (records, receipt) = fixture("natural_work", 6);
        let out = summarize(&records, &[(h(54), receipt.1)]).unwrap();
        assert_eq!(out["totals"]["receipt_missing_outcome_count"], 1);
        assert_eq!(out["totals"]["cost_unknown_outcome_count"], 1);
    }

    #[test]
    fn oversized_scope_is_explicitly_rejected() {
        assert_eq!(
            summarize(&vec![Value::Null; MAX_ENTRIES + 1], &[])
                .unwrap_err()
                .0,
            "learning_readiness_scope_limit_exceeded"
        );
    }
}
