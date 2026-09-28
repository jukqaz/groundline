//! Private, operator-supplied delivery receipts. Hashes identify checked local
//! artifacts; they do not authenticate the origin of the observations.
use crate::ContractError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MODELS: &[&str] = &["gpt-6-astra", "gpt-6-sol", "gpt-6-luna"];
const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultra"];
const MAX_ENTRIES: usize = 1000;
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
    MODELS.contains(&model) && EFFORTS.contains(&effort)
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
        if self.resources.entries.len() > MAX_ENTRIES
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
}
