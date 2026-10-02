//! Private, operator-supplied delivery receipts. Hashes identify checked local
//! artifacts; they do not authenticate the origin of the observations.
use crate::ContractError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultra"];
pub const MAX_RESOURCE_ENTRIES: usize = 1000;
pub const PHASES: &[&str] = &[
    "implementation",
    "runtime_verification",
    "visual_acceptance",
    "deployment",
    "research",
    "review",
    "documentation",
];

fn error(code: &str) -> ContractError {
    ContractError(format!("delivery_{code}"))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn selection(model: &str, effort: &str) -> bool {
    crate::model::optimization_model(model) && EFFORTS.contains(&effort)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionObservation {
    pub model: String,
    pub effort: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub status: String,
    pub evidence_kind: String,
    pub evidence_sha256: String,
    pub rework: bool,
    pub authenticity_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceEntry {
    pub owner: String,
    pub unit_hash: String,
    pub response_hash: String,
    /// Optional observed pair for this response, never inherited from the root.
    pub effective: Option<SelectionObservation>,
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub reasoning_output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub complete: bool,
    /// Elapsed wall time for the delivery. Parallel child elapsed times are not added.
    pub wall_duration_ms: Option<u64>,
    pub entries: Vec<ResourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub kind: String,
    pub schema: u8,
    pub unit_hash: String,
    pub cohort_sha256: String,
    pub phase: String,
    pub completed_at_utc: String,
    pub recommendation: Option<SelectionObservation>,
    pub requested: Option<SelectionObservation>,
    pub effective: Option<SelectionObservation>,
    pub verification: Verification,
    pub resources: Resources,
    /// A supplied observation alone never authenticates native activation.
    pub activation_verified: bool,
    /// A comparison of two supplied records, not proof of activation.
    pub observed_selection_matches_requested: Option<bool>,
}

impl Receipt {
    fn validate(&self) -> Result<(), ContractError> {
        if self.kind != "groundline-delivery-receipt"
            || self.schema != 1
            || !digest(&self.unit_hash)
            || !digest(&self.cohort_sha256)
            || !PHASES.contains(&self.phase.as_str())
            || DateTime::parse_from_rfc3339(&self.completed_at_utc).is_err()
            || self.activation_verified
            || self.observed_selection_matches_requested
                != self.requested.as_ref().zip(self.effective.as_ref()).map(
                    |(requested, effective)| {
                        requested.model == effective.model && requested.effort == effective.effort
                    },
                )
        {
            return Err(error("invalid_receipt"));
        }
        let completed = DateTime::parse_from_rfc3339(&self.completed_at_utc)
            .map_err(|_| error("invalid_timestamp"))?
            .with_timezone(&Utc);
        if completed > Utc::now() + chrono::Duration::minutes(5) {
            return Err(error("invalid_timestamp"));
        }
        for observed in [&self.recommendation, &self.requested, &self.effective]
            .into_iter()
            .flatten()
        {
            if !selection(&observed.model, &observed.effort) || !digest(&observed.evidence_sha256) {
                return Err(error("invalid_selection"));
            }
        }
        if !["verified", "failed", "unknown"].contains(&self.verification.status.as_str())
            || !["runtime_check", "user_acceptance", "unobserved"]
                .contains(&self.verification.evidence_kind.as_str())
            || (self.verification.status == "unknown")
                != (self.verification.evidence_kind == "unobserved")
            || !digest(&self.verification.evidence_sha256)
            || self.verification.authenticity_verified
        {
            return Err(error("invalid_verification"));
        }
        if self.resources.entries.len() > MAX_RESOURCE_ENTRIES
            || (self.resources.complete
                && (self.resources.entries.is_empty() || self.resources.wall_duration_ms.is_none()))
        {
            return Err(error("invalid_resources"));
        }
        let mut response_hashes = BTreeSet::new();
        let mut owned_sum = 0_u64;
        let mut root_seen = false;
        for row in &self.resources.entries {
            if !["root", "child", "approval", "retry"].contains(&row.owner.as_str())
                || !digest(&row.unit_hash)
                || !digest(&row.response_hash)
                || !response_hashes.insert(&row.response_hash)
                || (row.owner == "root" && row.unit_hash != self.unit_hash)
                || row
                    .cached_input_tokens
                    .zip(row.input_tokens)
                    .is_some_and(|(cached, input)| cached > input)
                || row
                    .reasoning_output_tokens
                    .zip(row.output_tokens)
                    .is_some_and(|(reasoning, output)| reasoning > output)
                || row
                    .input_tokens
                    .zip(row.total_tokens)
                    .is_some_and(|(input, total)| input > total)
                || row
                    .output_tokens
                    .zip(row.total_tokens)
                    .is_some_and(|(output, total)| output > total)
                || row
                    .cached_input_tokens
                    .zip(row.total_tokens)
                    .is_some_and(|(cached, total)| cached > total)
                || row
                    .reasoning_output_tokens
                    .zip(row.total_tokens)
                    .is_some_and(|(reasoning, total)| reasoning > total)
            {
                return Err(error("invalid_resource_entry"));
            }
            if let Some(observed) = &row.effective
                && (!selection(&observed.model, &observed.effort)
                    || !digest(&observed.evidence_sha256))
            {
                return Err(error("invalid_resource_selection"));
            }
            root_seen |= row.owner == "root";
            // Unknown totals must not hide arithmetic already contradicted by
            // known components. Subset counters also provide a lower bound.
            if let (Some(input), Some(output)) = (
                row.input_tokens.or(row.cached_input_tokens),
                row.output_tokens.or(row.reasoning_output_tokens),
            ) {
                let minimum = input
                    .checked_add(output)
                    .ok_or_else(|| error("token_overflow"))?;
                if row.total_tokens.is_some_and(|total| minimum > total) {
                    return Err(error("invalid_token_arithmetic"));
                }
            }
            if let (Some(input), Some(output), Some(total)) =
                (row.input_tokens, row.output_tokens, row.total_tokens)
                && input.checked_add(output) != Some(total)
            {
                return Err(error("invalid_token_arithmetic"));
            }
            if self.resources.complete
                && [
                    row.input_tokens,
                    row.cached_input_tokens,
                    row.output_tokens,
                    row.reasoning_output_tokens,
                    row.total_tokens,
                ]
                .contains(&None)
            {
                return Err(error("incomplete_resources"));
            }
            if let Some(total) = row.total_tokens {
                owned_sum = owned_sum
                    .checked_add(total)
                    .ok_or_else(|| error("token_overflow"))?;
            }
        }
        if self.resources.complete && !root_seen {
            return Err(error("root_resource_missing"));
        }
        let _ = owned_sum;
        Ok(())
    }

    fn owned_total_tokens(&self) -> Result<Option<u64>, ContractError> {
        if !self.resources.complete {
            return Ok(None);
        }
        self.resources
            .entries
            .iter()
            .try_fold(0_u64, |sum, row| {
                sum.checked_add(
                    row.total_tokens
                        .ok_or_else(|| error("incomplete_resources"))?,
                )
                .ok_or_else(|| error("token_overflow"))
            })
            .map(Some)
    }
}

/// Parse and validate a path-free receipt. Unknown fields are rejected so raw
/// content, account identifiers, and local paths cannot be smuggled into it.
pub fn validate_receipt(value: &Value) -> Result<(), ContractError> {
    let receipt: Receipt =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_receipt"))?;
    receipt.validate()
}

/// Project only observed effective selections into routing's schema-1 Outcome.
/// A missing effective selection cannot be attributed to the requested lane.
pub fn outcome_from_receipt(value: &Value) -> Result<Option<Value>, ContractError> {
    let receipt: Receipt =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_receipt"))?;
    receipt.validate()?;
    let Some(effective) = &receipt.effective else {
        return Ok(None);
    };
    let owned_total_tokens = receipt.owned_total_tokens()?;
    let outcome = receipt.verification.status.as_str();
    let evidence_kind = receipt.verification.evidence_kind.as_str();
    Ok(Some(json!({
        "unit_hash":receipt.unit_hash,
        "evidence_sha256":receipt.verification.evidence_sha256,
        "evidence_kind":evidence_kind,
        "cohort_sha256":receipt.cohort_sha256,
        "model":effective.model,
        "effort":effective.effort,
        "outcome":outcome,
        "rework":receipt.verification.rework,
        "owned_total_tokens":owned_total_tokens,
        "wall_duration_ms":receipt.resources.wall_duration_ms,
        "owned_resources_complete":receipt.resources.complete,
        "completed_at_utc":receipt.completed_at_utc,
    })))
}

fn parse_receipts(values: &[Value]) -> Result<Vec<Receipt>, ContractError> {
    let mut units = BTreeSet::new();
    let mut responses = BTreeSet::new();
    values
        .iter()
        .map(|value| {
            let receipt: Receipt =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_receipt"))?;
            receipt.validate()?;
            if !units.insert(receipt.unit_hash.clone()) {
                return Err(error("duplicate_delivery"));
            }
            for entry in &receipt.resources.entries {
                if !responses.insert(entry.response_hash.clone()) {
                    return Err(error("overlapping_response_ownership"));
                }
            }
            Ok(receipt)
        })
        .collect()
}

/// Validate collection ownership before filtering or aggregating any receipts.
pub fn validate_receipt_collection(values: &[Value]) -> Result<(), ContractError> {
    parse_receipts(values).map(|_| ())
}

#[derive(Default)]
struct Measurement {
    sum: u64,
    measured: usize,
    missing: usize,
}

impl Measurement {
    fn add(&mut self, value: Option<u64>) -> Result<(), ContractError> {
        if let Some(value) = value {
            self.sum = self
                .sum
                .checked_add(value)
                .ok_or_else(|| error("measurement_overflow"))?;
            self.measured += 1;
        } else {
            self.missing += 1;
        }
        Ok(())
    }

    fn value(&self) -> Value {
        json!({
            "known_sum": (self.measured > 0).then_some(self.sum),
            "measured_count": self.measured,
            "missing_count": self.missing
        })
    }
}

#[derive(Default)]
struct ResourceSummary {
    entries: usize,
    deliveries: usize,
    input: Measurement,
    cached_input: Measurement,
    output: Measurement,
    reasoning_output: Measurement,
    total: Measurement,
}

impl ResourceSummary {
    fn add(&mut self, row: &ResourceEntry) -> Result<(), ContractError> {
        self.entries += 1;
        self.input.add(row.input_tokens)?;
        self.cached_input.add(row.cached_input_tokens)?;
        self.output.add(row.output_tokens)?;
        self.reasoning_output.add(row.reasoning_output_tokens)?;
        self.total.add(row.total_tokens)
    }

    fn value(&self) -> Value {
        json!({
            "entry_count": self.entries,
            "delivery_count": self.deliveries,
            "input_tokens": self.input.value(),
            "cached_input_tokens": self.cached_input.value(),
            "output_tokens": self.output.value(),
            "reasoning_output_tokens": self.reasoning_output.value(),
            "total_tokens": self.total.value()
        })
    }
}

fn same_selection(
    left: &Option<SelectionObservation>,
    right: &Option<SelectionObservation>,
) -> Option<bool> {
    left.as_ref()
        .zip(right.as_ref())
        .map(|(left, right)| left.model == right.model && left.effort == right.effort)
}

fn pair(value: &Option<SelectionObservation>) -> Option<(String, String)> {
    value
        .as_ref()
        .map(|value| (value.model.clone(), value.effort.clone()))
}

fn pair_value(value: &Option<(String, String)>) -> Value {
    value
        .as_ref()
        .map(|(model, effort)| json!({"model":model,"effort":effort}))
        .unwrap_or(Value::Null)
}

#[derive(Default)]
struct DeliverySummary {
    deliveries: usize,
    verified: usize,
    failed: usize,
    unknown: usize,
    rework: usize,
    effective_missing: usize,
    recommendation_only: usize,
    resources_complete: usize,
    resources_incomplete: usize,
    fully_observed: usize,
    recommendation_requested: [usize; 3],
    recommendation_effective: [usize; 3],
    requested_effective: [usize; 3],
    resources: ResourceSummary,
    owners: BTreeMap<String, ResourceSummary>,
    execution_pairs: BTreeMap<(String, Option<(String, String)>), ResourceSummary>,
    complete_tokens: Measurement,
    wall_duration: Measurement,
}

impl DeliverySummary {
    fn add(&mut self, receipt: &Receipt) -> Result<(), ContractError> {
        self.deliveries += 1;
        match receipt.verification.status.as_str() {
            "verified" => self.verified += 1,
            "failed" => self.failed += 1,
            _ => self.unknown += 1,
        }
        self.rework += usize::from(receipt.verification.rework);
        self.effective_missing += usize::from(receipt.effective.is_none());
        self.recommendation_only += usize::from(
            receipt.recommendation.is_some()
                && receipt.requested.is_none()
                && receipt.effective.is_none(),
        );
        self.resources_complete += usize::from(receipt.resources.complete);
        self.resources_incomplete += usize::from(!receipt.resources.complete);
        self.fully_observed += usize::from(
            receipt.effective.is_some()
                && receipt.verification.status != "unknown"
                && receipt.resources.complete,
        );
        for (counts, relation) in [
            (
                &mut self.recommendation_requested,
                same_selection(&receipt.recommendation, &receipt.requested),
            ),
            (
                &mut self.recommendation_effective,
                same_selection(&receipt.recommendation, &receipt.effective),
            ),
            (
                &mut self.requested_effective,
                same_selection(&receipt.requested, &receipt.effective),
            ),
        ] {
            counts[match relation {
                Some(true) => 0,
                Some(false) => 1,
                None => 2,
            }] += 1;
        }
        self.complete_tokens.add(receipt.owned_total_tokens()?)?;
        self.wall_duration.add(receipt.resources.wall_duration_ms)?;
        self.resources.deliveries += 1;
        let mut owners = BTreeSet::new();
        let mut execution_pairs = BTreeSet::new();
        for row in &receipt.resources.entries {
            self.resources.add(row)?;
            self.owners.entry(row.owner.clone()).or_default().add(row)?;
            owners.insert(row.owner.clone());
            let key = (row.owner.clone(), pair(&row.effective));
            self.execution_pairs
                .entry(key.clone())
                .or_default()
                .add(row)?;
            execution_pairs.insert(key);
        }
        for key in execution_pairs {
            self.execution_pairs
                .get_mut(&key)
                .expect("observed pair")
                .deliveries += 1;
        }
        for owner in owners {
            self.owners
                .get_mut(&owner)
                .expect("observed owner")
                .deliveries += 1;
        }
        Ok(())
    }

    fn value(&self) -> Value {
        let relationship = |counts: &[usize; 3]| json!({"matched_count":counts[0],"mismatched_count":counts[1],"unobserved_count":counts[2]});
        let owners: Vec<Value> = self
            .owners
            .iter()
            .map(|(owner, resources)| json!({"owner":owner,"resources":resources.value()}))
            .collect();
        json!({
            "delivery_count":self.deliveries,
            "verified_count":self.verified,
            "failed_count":self.failed,
            "unknown_count":self.unknown,
            "rework_count":self.rework,
            "effective_selection_missing_count":self.effective_missing,
            "recommendation_only_count":self.recommendation_only,
            "resources_complete_count":self.resources_complete,
            "resources_incomplete_count":self.resources_incomplete,
            "fully_observed_delivery_count":self.fully_observed,
            "selection_relationships":{
                "recommendation_to_requested":relationship(&self.recommendation_requested),
                "recommendation_to_effective":relationship(&self.recommendation_effective),
                "requested_to_effective":relationship(&self.requested_effective)
            },
            "resources":{
                "all_owners":self.resources.value(),
                "by_owner":owners,
                "by_owner_and_effective_selection":self.execution_pairs.iter().map(|((owner, selection), resources)|
                    json!({"owner":owner,"effective":pair_value(selection),"resources":resources.value()})
                ).collect::<Vec<_>>(),
                "complete_delivery_total_tokens":self.complete_tokens.value(),
                "wall_duration_ms":self.wall_duration.value()
            }
        })
    }
}

/// Aggregate every supplied delivery without ranking models or mixing cohorts
/// into a comparison. No hashes, paths, raw evidence, or delivery IDs are emitted.
pub fn summarize_receipts(values: &[Value]) -> Result<Value, ContractError> {
    let receipts = parse_receipts(values)?;
    let mut overall = DeliverySummary::default();
    let mut groups = BTreeMap::<(String, Option<(String, String)>), DeliverySummary>::new();
    type SelectionPath = (
        String,
        Option<(String, String)>,
        Option<(String, String)>,
        Option<(String, String)>,
    );
    let mut paths = BTreeMap::<SelectionPath, DeliverySummary>::new();
    for receipt in &receipts {
        overall.add(receipt)?;
        groups
            .entry((receipt.phase.clone(), pair(&receipt.effective)))
            .or_default()
            .add(receipt)?;
        paths
            .entry((
                receipt.phase.clone(),
                pair(&receipt.recommendation),
                pair(&receipt.requested),
                pair(&receipt.effective),
            ))
            .or_default()
            .add(receipt)?;
    }
    let groups: Vec<Value> = groups
        .iter()
        .map(|((phase, effective), summary)| {
            json!({"phase":phase,"effective":pair_value(effective),"observations":summary.value()})
        })
        .collect();
    let paths: Vec<Value> = paths
        .iter()
        .map(|((phase, recommendation, requested, effective), summary)| {
            json!({"phase":phase,"recommendation":pair_value(recommendation),
                "requested":pair_value(requested),"effective":pair_value(effective),
                "observations":summary.value()})
        })
        .collect();
    let children: Vec<_> = receipts
        .iter()
        .flat_map(|receipt| &receipt.resources.entries)
        .filter(|entry| entry.owner == "child")
        .collect();
    let child_attribution_complete =
        (!children.is_empty()).then(|| children.iter().all(|entry| entry.effective.is_some()));
    Ok(json!({
        "kind":"groundline-delivery-summary","schema":1,
        "status":if receipts.is_empty() {"EMPTY"} else if overall.fully_observed < receipts.len() {"PARTIAL"} else {"AVAILABLE"},
        "scope":"all_retained_receipts_without_comparison_filters",
        "overall":overall.value(),
        "by_phase_and_effective_selection":groups,
        "selection_paths":paths,
        "data_readiness":{
            "retained_receipts_available":!receipts.is_empty(),
            "fully_observed_deliveries_available":overall.fully_observed > 0,
            "all_retained_deliveries_fully_observed":!receipts.is_empty() && overall.fully_observed == receipts.len(),
            "comparison_eligibility_assessed":false
        },
        "evidence_scope":{
            "selection_and_acceptance_source":"operator_observed_local_artifacts",
            "source_authenticity_verified":false,"activation_verified":false,
            "artifact_hashes_rechecked":false,"used_for_model_ranking":false,
            "cohorts_combined_for_comparison":false,"optimization_demonstrated":false,
            "child_model_effort_attribution_available":receipts.iter().any(|receipt|
                receipt.resources.entries.iter().any(|entry| entry.owner == "child" && entry.effective.is_some())),
            "child_model_effort_attribution_complete":child_attribution_complete,
            "resource_unit":"provider_reported_tokens_and_delivery_wall_duration",
            "known_sums_are_partial_when_measurements_missing":true,
            "subset_tokens_added_to_total":false,"parallel_child_wall_times_added":false,
            "monetary_cost":null,"subscription_quota":null
        },
        "network_performed":false,"mutation_performed":false,
        "raw_content_emitted":false,"private_paths_emitted":false,"private_ids_emitted":false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u8) -> String {
        format!("{n:064x}")
    }
    fn receipt() -> Value {
        json!({
            "kind":"groundline-delivery-receipt","schema":1,"unit_hash":h(1),
            "cohort_sha256":h(2),"phase":"implementation",
            "completed_at_utc":Utc::now().to_rfc3339(),
            "recommendation":{"model":"gpt-6-sol","effort":"medium","evidence_sha256":h(3)},
            "requested":{"model":"gpt-6-sol","effort":"medium","evidence_sha256":h(4)},
            "effective":{"model":"gpt-6-sol","effort":"medium","evidence_sha256":h(5)},
            "verification":{"status":"verified","evidence_kind":"runtime_check",
                "evidence_sha256":h(6),"rework":false,"authenticity_verified":false},
            "resources":{"complete":true,"wall_duration_ms":1000,"entries":[
                {"owner":"root","unit_hash":h(1),"response_hash":h(7),
                 "input_tokens":10,"cached_input_tokens":4,"output_tokens":7,
                 "reasoning_output_tokens":3,"total_tokens":17},
                {"owner":"child","unit_hash":h(8),"response_hash":h(9),
                 "input_tokens":5,"cached_input_tokens":0,"output_tokens":3,
                 "reasoning_output_tokens":1,"total_tokens":8}
            ]},"activation_verified":false,"observed_selection_matches_requested":true
        })
    }

    #[test]
    fn sol_61_receipts_preserve_exact_root_and_child_observations() {
        let mut value = receipt();
        for field in ["recommendation", "requested", "effective"] {
            value[field]["model"] = json!("gpt-6.1-sol");
        }
        value["resources"]["entries"][1]["effective"] = json!({
            "model":"gpt-6-sol","effort":"medium","evidence_sha256":h(10)});
        validate_receipt(&value).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(
            outcome_from_receipt(&value).unwrap().unwrap()["model"],
            "gpt-6.1-sol"
        );
        let summary = summarize_receipts(&[value.clone()]).unwrap();
        assert!(summary.to_string().contains("gpt-6.1-sol"));
        value["effective"]["model"] = json!("gpt-6.1-sol-unconfirmed-snapshot");
        assert!(validate_receipt(&value).is_err());
    }
    #[test]
    fn projects_actual_selection_and_owned_tokens_without_summing_child_elapsed_time() {
        let mut value = receipt();
        value["effective"]["model"] = json!("gpt-6-astra");
        value["effective"]["effort"] = json!("high");
        value["observed_selection_matches_requested"] = json!(false);
        let outcome = outcome_from_receipt(&value).unwrap().unwrap();
        assert_eq!(outcome["model"], "gpt-6-astra");
        assert_eq!(outcome["effort"], "high");
        assert_eq!(outcome["owned_total_tokens"], 25);
        assert_eq!(outcome["wall_duration_ms"], 1000);
        assert_eq!(value["activation_verified"], false);
    }

    #[test]
    fn c1_known_retry_usage_is_included_once_in_delivery_totals() {
        let mut value = receipt();
        value["resources"]["entries"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "owner":"retry","unit_hash":h(10),"response_hash":h(11),
                "input_tokens":9,"cached_input_tokens":3,"output_tokens":4,
                "reasoning_output_tokens":2,"total_tokens":13
            }));
        let outcome = outcome_from_receipt(&value).unwrap().unwrap();
        assert_eq!(outcome["owned_total_tokens"], 38);
        assert_eq!(outcome["owned_resources_complete"], true);
        let summary = summarize_receipts(&[value.clone()]).unwrap();
        let resources = &summary["overall"]["resources"];
        assert_eq!(resources["all_owners"]["entry_count"], 3);
        assert_eq!(resources["all_owners"]["total_tokens"]["known_sum"], 38);
        assert_eq!(resources["all_owners"]["total_tokens"]["missing_count"], 0);
        assert_eq!(resources["complete_delivery_total_tokens"]["known_sum"], 38);
        let retry = resources["by_owner"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["owner"] == "retry")
            .unwrap();
        for (field, expected) in [
            ("input_tokens", 9),
            ("cached_input_tokens", 3),
            ("output_tokens", 4),
            ("reasoning_output_tokens", 2),
            ("total_tokens", 13),
        ] {
            assert_eq!(retry["resources"][field]["known_sum"], expected, "{field}");
            assert_eq!(retry["resources"][field]["missing_count"], 0, "{field}");
        }
        let duplicate = value["resources"]["entries"][2].clone();
        value["resources"]["entries"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(outcome_from_receipt(&value).is_err());
        assert!(summarize_receipts(&[value]).is_err());
    }

    #[test]
    fn absent_effective_selection_never_inherits_requested_selection() {
        let mut value = receipt();
        value["effective"] = Value::Null;
        value["observed_selection_matches_requested"] = Value::Null;
        assert!(outcome_from_receipt(&value).unwrap().is_none());
        value["activation_verified"] = json!(true);
        assert!(validate_receipt(&value).is_err());
    }

    #[test]
    fn invalid_model_private_fields_and_duplicate_ownership_are_rejected() {
        let mut value = receipt();
        value["effective"]["model"] = json!("gpt-5.5");
        assert!(validate_receipt(&value).is_err());
        value = receipt();
        value["account_id"] = json!("private");
        assert!(validate_receipt(&value).is_err());
        value = receipt();
        let duplicate = value["resources"]["entries"][0].clone();
        value["resources"]["entries"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(validate_receipt(&value).is_err());
    }

    #[test]
    fn rejects_overflow_and_inconsistent_components() {
        let mut value = receipt();
        value["resources"]["entries"][0]["input_tokens"] = json!(u64::MAX);
        assert!(validate_receipt(&value).is_err());
        value = receipt();
        value["resources"]["complete"] = json!(false);
        value["resources"]["entries"][0]["input_tokens"] = json!(u64::MAX);
        value["resources"]["entries"][0]["total_tokens"] = Value::Null;
        assert!(validate_receipt(&value).is_err());
        value = receipt();
        value["resources"]["complete"] = json!(false);
        value["resources"]["entries"][0]["input_tokens"] = Value::Null;
        value["resources"]["entries"][0]["cached_input_tokens"] = json!(15);
        assert!(validate_receipt(&value).is_err());
        value = receipt();
        value["resources"]["entries"][1]["input_tokens"] = json!(u64::MAX - 2);
        value["resources"]["entries"][1]["total_tokens"] = json!(u64::MAX);
        value["resources"]["entries"][1]["output_tokens"] = json!(2);
        assert!(validate_receipt(&value).is_err());
    }

    #[test]
    fn unknown_and_partial_resources_remain_unknown_for_routing() {
        let mut value = receipt();
        value["resources"]["complete"] = json!(false);
        value["resources"]["entries"][1]["total_tokens"] = Value::Null;
        let outcome = outcome_from_receipt(&value).unwrap().unwrap();
        assert_eq!(outcome["outcome"], "verified");
        assert!(outcome["owned_total_tokens"].is_null());
        assert_eq!(outcome["owned_resources_complete"], false);
        value["verification"]["status"] = json!("unknown");
        value["verification"]["evidence_kind"] = json!("unobserved");
        assert!(validate_receipt(&value).is_ok());
        value["verification"]["evidence_kind"] = json!("runtime_check");
        assert!(validate_receipt(&value).is_err());
    }

    fn separate_receipt(mut value: Value, unit: u8, response: u8) -> Value {
        value["unit_hash"] = json!(h(unit));
        value["resources"]["entries"][0]["unit_hash"] = json!(h(unit));
        value["resources"]["entries"][0]["response_hash"] = json!(h(response));
        value["resources"]["entries"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        value
    }

    #[test]
    fn summary_preserves_failed_unknown_and_unobserved_selection_paths() {
        let success = receipt();
        let mut failed = separate_receipt(receipt(), 10, 11);
        failed["effective"]["model"] = json!("gpt-6-luna");
        failed["effective"]["effort"] = json!("low");
        failed["observed_selection_matches_requested"] = json!(false);
        failed["verification"]["status"] = json!("failed");
        failed["verification"]["rework"] = json!(true);
        // Old records remain in the observational summary, unlike routing's
        // preceding-30-day matched-cohort comparison.
        failed["completed_at_utc"] = json!((Utc::now() - chrono::Duration::days(60)).to_rfc3339());
        failed["cohort_sha256"] = json!(h(12));
        let mut unknown = separate_receipt(receipt(), 13, 14);
        unknown["phase"] = json!("runtime_verification");
        unknown["effective"] = Value::Null;
        unknown["requested"] = Value::Null;
        unknown["observed_selection_matches_requested"] = Value::Null;
        unknown["verification"]["status"] = json!("unknown");
        unknown["verification"]["evidence_kind"] = json!("unobserved");
        unknown["resources"]["complete"] = json!(false);
        unknown["resources"]["entries"] = json!([]);
        unknown["resources"]["wall_duration_ms"] = Value::Null;
        let summary = summarize_receipts(&[success, failed, unknown]).unwrap();
        let overall = &summary["overall"];
        assert_eq!(summary["status"], "PARTIAL");
        assert_eq!(overall["delivery_count"], 3);
        assert_eq!(overall["verified_count"], 1);
        assert_eq!(overall["failed_count"], 1);
        assert_eq!(overall["unknown_count"], 1);
        assert_eq!(overall["rework_count"], 1);
        assert_eq!(overall["recommendation_only_count"], 1);
        assert_eq!(overall["fully_observed_delivery_count"], 2);
        assert_eq!(
            overall["resources"]["all_owners"]["total_tokens"]["known_sum"],
            42
        );
        assert_eq!(
            overall["resources"]["complete_delivery_total_tokens"]["known_sum"],
            42
        );
        assert_eq!(
            overall["resources"]["complete_delivery_total_tokens"]["missing_count"],
            1
        );
        assert_eq!(overall["resources"]["by_owner"][0]["owner"], "child");
        assert_eq!(
            overall["resources"]["by_owner"][0]["resources"]["total_tokens"]["known_sum"],
            8
        );
        assert_eq!(
            overall["selection_relationships"]["requested_to_effective"]["mismatched_count"],
            1
        );
        assert_eq!(
            overall["selection_relationships"]["requested_to_effective"]["unobserved_count"],
            1
        );
        let luna = summary["by_phase_and_effective_selection"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["effective"]["model"] == "gpt-6-luna")
            .unwrap();
        assert_eq!(luna["observations"]["failed_count"], 1);
        let unknown = summary["by_phase_and_effective_selection"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["effective"].is_null())
            .unwrap();
        assert!(
            unknown["observations"]["resources"]["all_owners"]["total_tokens"]["known_sum"]
                .is_null()
        );
        assert_eq!(summary["selection_paths"].as_array().unwrap().len(), 3);
        assert_eq!(
            summary["evidence_scope"]["optimization_demonstrated"],
            false
        );
        let output = summary.to_string();
        assert!(!output.contains(&h(1)));
        assert!(!output.contains("unit_hash"));
        assert!(!output.contains("cohort_sha256"));
    }

    #[test]
    fn summary_partial_measurements_and_all_resource_owners_are_retained() {
        let mut partial = receipt();
        let mut approval = partial["resources"]["entries"][1].clone();
        approval["owner"] = json!("approval");
        approval["response_hash"] = json!(h(10));
        let mut retry = approval.clone();
        retry["owner"] = json!("retry");
        retry["response_hash"] = json!(h(11));
        retry["total_tokens"] = Value::Null;
        partial["resources"]["complete"] = json!(false);
        partial["resources"]["entries"]
            .as_array_mut()
            .unwrap()
            .extend([approval, retry]);
        let summary = summarize_receipts(&[partial]).unwrap();
        let resources = &summary["overall"]["resources"];
        assert_eq!(resources["all_owners"]["entry_count"], 4);
        assert_eq!(resources["all_owners"]["total_tokens"]["known_sum"], 33);
        assert_eq!(resources["all_owners"]["total_tokens"]["missing_count"], 1);
        assert_eq!(
            resources["all_owners"]["cached_input_tokens"]["known_sum"],
            4
        );
        assert!(resources["complete_delivery_total_tokens"]["known_sum"].is_null());
        assert_eq!(resources["by_owner"].as_array().unwrap().len(), 4);
        assert_eq!(summary["overall"]["verified_count"], 1);
        assert_eq!(summary["overall"]["resources_incomplete_count"], 1);
    }

    #[test]
    fn collection_duplicates_overlap_and_cross_delivery_overflow_are_rejected() {
        let first = receipt();
        assert_eq!(
            validate_receipt_collection(&[first.clone(), first.clone()])
                .unwrap_err()
                .0,
            "delivery_duplicate_delivery"
        );
        let overlapping = separate_receipt(receipt(), 10, 7);
        assert_eq!(
            summarize_receipts(&[first, overlapping]).unwrap_err().0,
            "delivery_overlapping_response_ownership"
        );
        let mut large = separate_receipt(receipt(), 1, 7);
        large["resources"]["entries"][0] = json!({
            "owner":"root","unit_hash":h(1),"response_hash":h(7),
            "input_tokens":u64::MAX,"cached_input_tokens":0,
            "output_tokens":0,"reasoning_output_tokens":0,"total_tokens":u64::MAX
        });
        let small = separate_receipt(receipt(), 10, 11);
        assert!(validate_receipt_collection(&[large.clone(), small.clone()]).is_ok());
        assert_eq!(
            summarize_receipts(&[large, small.clone()]).unwrap_err().0,
            "delivery_measurement_overflow"
        );
        let mut wall = separate_receipt(receipt(), 1, 7);
        wall["resources"]["wall_duration_ms"] = json!(u64::MAX);
        assert_eq!(
            summarize_receipts(&[wall, small]).unwrap_err().0,
            "delivery_measurement_overflow"
        );
    }

    #[test]
    fn empty_summary_has_no_measured_zero_or_optimization_claim() {
        let summary = summarize_receipts(&[]).unwrap();
        assert_eq!(summary["status"], "EMPTY");
        assert_eq!(summary["overall"]["delivery_count"], 0);
        assert_eq!(
            summary["data_readiness"]["retained_receipts_available"],
            false
        );
        assert_eq!(
            summary["data_readiness"]["all_retained_deliveries_fully_observed"],
            false
        );
        assert!(
            summary["overall"]["resources"]["all_owners"]["total_tokens"]["known_sum"].is_null()
        );
        assert_eq!(summary["by_phase_and_effective_selection"], json!([]));
        assert_eq!(summary["mutation_performed"], false);
    }
}
