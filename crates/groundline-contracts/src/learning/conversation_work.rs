//! Retrospective, owner-private conversation improvement records. Native Codex
//! interprets the conversation. These contracts bind declarations and evidence
//! bytes; they do not authenticate semantic claims or establish causal effects.
use super::conversation::{
    ConversationScope, ConversationTarget, CorrectionInterpretation, TransferabilityReview,
};
use super::{content_sha256, digest, error, identifier, text};
use crate::ContractError;
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum Input {
    #[serde(rename = "groundline-learning-conversation-review-input")]
    Review(ReviewInput),
    #[serde(rename = "groundline-learning-conversation-snapshot-input")]
    Snapshot(SnapshotInput),
    #[serde(rename = "groundline-learning-conversation-change-input")]
    Change(ChangeInput),
    #[serde(rename = "groundline-learning-conversation-activation-input")]
    Activation(ActivationInput),
    #[serde(rename = "groundline-learning-conversation-observation-input")]
    Observation(ObservationInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewInput {
    pub schema: u8,
    pub pattern_id: String,
    pub feedback_ref_sha256: String,
    pub source_work_sha256: String,
    pub interpretation: CorrectionInterpretation,
    pub observed_scope: ConversationScope,
    pub project_dependent: bool,
    pub criterion_sha256: String,
    pub criterion_version: String,
    pub preceding_assistant_action_sha256: String,
    pub user_correction_sha256: String,
    pub context_evidence_sha256: String,
    pub observed_at_utc: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChangeInput {
    pub schema: u8,
    pub review_sha256: String,
    pub snapshot_sha256: String,
    pub target_binding_sha256: String,
    pub target: ConversationTarget,
    pub verification_evidence_sha256: String,
    pub applied_at_utc: String,
    pub comparison_context_sha256: String,
    pub transferability_review: Option<TransferabilityReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SnapshotInput {
    pub schema: u8,
    pub review_sha256: String,
    pub scope: ConversationScope,
    pub target_id: String,
    pub target_kind: String,
    pub before_sha256: String,
    /// CLI-derived local target identity. A hash does not authenticate scope.
    pub target_binding_sha256: String,
    pub captured_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActivationInput {
    pub schema: u8,
    pub change_sha256: String,
    pub target_after_sha256: String,
    pub activation_kind: String,
    pub comparison_context_sha256: String,
    pub evidence_sha256: String,
    pub activated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationInput {
    pub schema: u8,
    pub change_sha256: String,
    /// Stable native work reference. Repeated reports of the same work cannot
    /// increase the observed opportunity count.
    pub source_work_sha256: String,
    /// Provenance only. Effect comparisons bind the changed target, criterion,
    /// criterion version and comparison context, not this work revision.
    pub source_revision_sha256: String,
    pub target_after_sha256: String,
    pub criterion_sha256: String,
    pub criterion_version: String,
    pub comparison_context_sha256: String,
    pub opportunity: Option<bool>,
    pub recurred: Option<bool>,
    pub evidence_sha256: String,
    pub recurrence_evidence_sha256: Option<String>,
    pub observed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub kind: String,
    pub schema: u8,
    pub recorded_at_utc: String,
    pub input_sha256: String,
    pub input: Input,
}

fn timestamp(value: &str) -> Result<DateTime<FixedOffset>, ContractError> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| error("invalid_conversation_work_timestamp"))?;
    if parsed.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5) {
        return Err(error("invalid_conversation_work_timestamp"));
    }
    Ok(parsed)
}

fn scope(value: &ConversationScope) -> bool {
    match value {
        ConversationScope::Common => true,
        ConversationScope::Project { project_sha256 } => digest(project_sha256),
    }
}

impl Input {
    pub fn record_kind(&self) -> &'static str {
        match self {
            Self::Review(_) => "groundline-learning-conversation-review",
            Self::Snapshot(_) => "groundline-learning-conversation-snapshot",
            Self::Change(_) => "groundline-learning-conversation-change",
            Self::Activation(_) => "groundline-learning-conversation-activation",
            Self::Observation(_) => "groundline-learning-conversation-observation",
        }
    }

    fn event_time(&self) -> &str {
        match self {
            Self::Review(v) => &v.observed_at_utc,
            Self::Snapshot(v) => &v.captured_at_utc,
            Self::Change(v) => &v.applied_at_utc,
            Self::Activation(v) => &v.activated_at_utc,
            Self::Observation(v) => &v.observed_at_utc,
        }
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        let valid = match self {
            Self::Review(v) => {
                v.schema == 1
                    && identifier(&v.pattern_id)
                    && scope(&v.observed_scope)
                    && identifier(&v.criterion_version)
                    && [
                        &v.feedback_ref_sha256,
                        &v.source_work_sha256,
                        &v.criterion_sha256,
                        &v.preceding_assistant_action_sha256,
                        &v.user_correction_sha256,
                        &v.context_evidence_sha256,
                    ]
                    .into_iter()
                    .all(|s| digest(s))
                    && v.summary.as_ref().is_none_or(|s| text(s))
            }
            Self::Change(v) => {
                v.schema == 1
                    && digest(&v.review_sha256)
                    && digest(&v.snapshot_sha256)
                    && digest(&v.target_binding_sha256)
                    && scope(&v.target.scope)
                    && identifier(&v.target.target_id)
                    && ["skill", "agents_block"].contains(&v.target.kind.as_str())
                    && [
                        &v.target.before_sha256,
                        &v.target.after_sha256,
                        &v.verification_evidence_sha256,
                        &v.comparison_context_sha256,
                    ]
                    .into_iter()
                    .all(|s| digest(s))
                    && v.target.before_sha256 != v.target.after_sha256
                    && v.transferability_review
                        .as_ref()
                        .is_none_or(|r| digest(&r.evidence_sha256))
            }
            Self::Snapshot(v) => {
                v.schema == 1
                    && digest(&v.review_sha256)
                    && scope(&v.scope)
                    && identifier(&v.target_id)
                    && ["skill", "agents_block"].contains(&v.target_kind.as_str())
                    && digest(&v.before_sha256)
                    && digest(&v.target_binding_sha256)
            }
            Self::Activation(v) => {
                v.schema == 1
                    && [
                        &v.change_sha256,
                        &v.target_after_sha256,
                        &v.comparison_context_sha256,
                        &v.evidence_sha256,
                    ]
                    .into_iter()
                    .all(|s| digest(s))
                    && ["skill_invoked", "instruction_read"].contains(&v.activation_kind.as_str())
            }
            Self::Observation(v) => {
                v.schema == 1
                    && identifier(&v.criterion_version)
                    && [
                        &v.change_sha256,
                        &v.source_work_sha256,
                        &v.source_revision_sha256,
                        &v.target_after_sha256,
                        &v.criterion_sha256,
                        &v.comparison_context_sha256,
                        &v.evidence_sha256,
                    ]
                    .into_iter()
                    .all(|s| digest(s))
                    && v.recurrence_evidence_sha256
                        .as_ref()
                        .is_none_or(|s| digest(s))
                    && (v.opportunity == Some(true) || v.recurred.is_none())
                    && (v.recurred == Some(true)) == v.recurrence_evidence_sha256.is_some()
            }
        };
        if !valid {
            return Err(error("invalid_conversation_work_input"));
        }
        timestamp(self.event_time())?;
        Ok(())
    }
}

pub fn build_record(input: &Value, recorded_at_utc: &str) -> Result<Value, ContractError> {
    let parsed: Input = serde_json::from_value(input.clone())
        .map_err(|_| error("invalid_conversation_work_input"))?;
    parsed.validate()?;
    let normalized = serde_json::to_value(&parsed).map_err(|_| error("serialization_failed"))?;
    let record = Record {
        kind: parsed.record_kind().into(),
        schema: 1,
        recorded_at_utc: recorded_at_utc.into(),
        input_sha256: content_sha256(&normalized)?,
        input: parsed,
    };
    let value = serde_json::to_value(record).map_err(|_| error("serialization_failed"))?;
    validate_record(&value)?;
    Ok(value)
}

pub fn validate_record(value: &Value) -> Result<(), ContractError> {
    let record: Record = serde_json::from_value(value.clone())
        .map_err(|_| error("invalid_conversation_work_record"))?;
    record.input.validate()?;
    let normalized =
        serde_json::to_value(&record.input).map_err(|_| error("serialization_failed"))?;
    if record.schema != 1
        || record.kind != record.input.record_kind()
        || !digest(&record.input_sha256)
        || record.input_sha256 != content_sha256(&normalized)?
        || timestamp(&record.recorded_at_utc)? < timestamp(record.input.event_time())?
    {
        return Err(error("invalid_conversation_work_record"));
    }
    Ok(())
}

fn records(values: &[Value]) -> Result<BTreeMap<String, Record>, ContractError> {
    let mut records = BTreeMap::new();
    for value in values {
        let kind = value["kind"].as_str().unwrap_or_default();
        if !kind.starts_with("groundline-learning-conversation-") {
            continue;
        }
        validate_record(value)?;
        let record = serde_json::from_value(value.clone())
            .map_err(|_| error("invalid_conversation_work_record"))?;
        records.insert(content_sha256(value)?, record);
    }
    Ok(records)
}

fn review<'a>(
    records: &'a BTreeMap<String, Record>,
    hash: &str,
) -> Result<&'a ReviewInput, ContractError> {
    match records.get(hash).map(|r| &r.input) {
        Some(Input::Review(v)) => Ok(v),
        _ => Err(error("conversation_review_reference_missing")),
    }
}

fn change<'a>(
    records: &'a BTreeMap<String, Record>,
    hash: &str,
) -> Result<&'a ChangeInput, ContractError> {
    match records.get(hash).map(|r| &r.input) {
        Some(Input::Change(v)) => Ok(v),
        _ => Err(error("conversation_change_reference_missing")),
    }
}

pub fn validate_collection(values: &[Value]) -> Result<(), ContractError> {
    let records = records(values)?;
    let mut logical = BTreeMap::new();
    // Re-recording identical input at another timestamp must not create a new
    // logical parent for counting the same subsequent work again.
    let reference_key = |hash: &String| {
        records
            .get(hash)
            .map_or_else(|| hash.clone(), |r| r.input_sha256.clone())
    };
    for record in records.values() {
        let key = match &record.input {
            Input::Review(v) => format!("review:{}:{}", v.feedback_ref_sha256, v.pattern_id),
            Input::Snapshot(v) => format!(
                "snapshot:{}:{}:{}",
                reference_key(&v.review_sha256),
                v.target_binding_sha256,
                v.captured_at_utc
            ),
            Input::Change(v) => format!(
                "change:{}:{}",
                reference_key(&v.review_sha256),
                v.target.target_id
            ),
            Input::Activation(v) => format!(
                "activation:{}:{}",
                reference_key(&v.change_sha256),
                v.evidence_sha256
            ),
            Input::Observation(v) => {
                format!(
                    "observation:{}:{}",
                    reference_key(&v.change_sha256),
                    v.source_work_sha256
                )
            }
        };
        if logical
            .insert(key, &record.input_sha256)
            .is_some_and(|old| old != &record.input_sha256)
        {
            return Err(error("conversation_duplicate_conflict"));
        }
        match &record.input {
            Input::Review(_) => {}
            Input::Snapshot(v) => {
                let origin = review(&records, &v.review_sha256)?;
                if origin.interpretation != CorrectionInterpretation::AssistantError {
                    return Err(error("conversation_error_review_required"));
                }
                if timestamp(&v.captured_at_utc)? < timestamp(&origin.observed_at_utc)? {
                    return Err(error("conversation_snapshot_before_review"));
                }
            }
            Input::Change(v) => {
                let origin = review(&records, &v.review_sha256)?;
                if origin.interpretation != CorrectionInterpretation::AssistantError {
                    return Err(error("conversation_error_review_required"));
                }
                if timestamp(&v.applied_at_utc)? < timestamp(&origin.observed_at_utc)? {
                    return Err(error("conversation_change_before_review"));
                }
                let snapshot = match records.get(&v.snapshot_sha256).map(|r| &r.input) {
                    Some(Input::Snapshot(s)) => s,
                    _ => return Err(error("conversation_snapshot_reference_missing")),
                };
                if snapshot.review_sha256 != v.review_sha256
                    || snapshot.scope != v.target.scope
                    || snapshot.target_id != v.target.target_id
                    || snapshot.target_kind != v.target.kind
                    || snapshot.before_sha256 != v.target.before_sha256
                    || snapshot.target_binding_sha256 != v.target_binding_sha256
                    || timestamp(&snapshot.captured_at_utc)? > timestamp(&v.applied_at_utc)?
                {
                    return Err(error("conversation_snapshot_mismatch"));
                }
                match &v.target.scope {
                    ConversationScope::Common => {
                        if origin.project_dependent
                            || !v
                                .transferability_review
                                .as_ref()
                                .is_some_and(|r| r.project_neutral)
                        {
                            return Err(error("conversation_common_scope_unreviewed"));
                        }
                    }
                    ConversationScope::Project { .. } => {
                        if origin.observed_scope != v.target.scope {
                            return Err(error("conversation_project_scope_mismatch"));
                        }
                    }
                }
            }
            Input::Activation(v) => {
                let applied = change(&records, &v.change_sha256)?;
                let expected = if applied.target.kind == "skill" {
                    "skill_invoked"
                } else {
                    "instruction_read"
                };
                if v.target_after_sha256 != applied.target.after_sha256
                    || v.comparison_context_sha256 != applied.comparison_context_sha256
                    || v.activation_kind != expected
                    || timestamp(&v.activated_at_utc)? < timestamp(&applied.applied_at_utc)?
                {
                    return Err(error("conversation_activation_mismatch"));
                }
            }
            Input::Observation(v) => {
                change(&records, &v.change_sha256)?;
            }
        }
    }
    Ok(())
}

fn comparable(
    observation: &ObservationInput,
    applied: &ChangeInput,
    origin: &ReviewInput,
    activations: &[&ActivationInput],
) -> Result<bool, ContractError> {
    let observed = timestamp(&observation.observed_at_utc)?;
    Ok(
        observation.target_after_sha256 == applied.target.after_sha256
            && observation.criterion_sha256 == origin.criterion_sha256
            && observation.criterion_version == origin.criterion_version
            && observation.comparison_context_sha256 == applied.comparison_context_sha256
            && observation.source_work_sha256 != origin.source_work_sha256
            && observed > timestamp(&applied.applied_at_utc)?
            && activations
                .iter()
                .any(|a| timestamp(&a.activated_at_utc).is_ok_and(|t| observed > t)),
    )
}

/// Safe derived status only. Owner-private summaries, evidence bodies and paths
/// are deliberately absent. A bounded non-recurrence is not causal proof.
pub fn readouts(values: &[Value]) -> Result<Value, ContractError> {
    validate_collection(values)?;
    let records = records(values)?;
    let mut seen = BTreeSet::new();
    let unique: Vec<_> = records
        .iter()
        .filter(|(_, r)| seen.insert(&r.input_sha256))
        .collect();
    let mut chains = Vec::new();
    let mut total_opportunities = 0;
    let mut total_recurrences = 0;
    let mut total_inconclusive = 0;
    let mut total_unassessed = 0;
    for (review_hash, record) in &unique {
        let Input::Review(origin) = &record.input else {
            continue;
        };
        let changes: Vec<_> = unique
            .iter()
            .filter_map(|(hash, r)| match &r.input {
                Input::Change(c)
                    if records
                        .get(&c.review_sha256)
                        .is_some_and(|r| r.input_sha256 == record.input_sha256) =>
                {
                    Some((*hash, c))
                }
                _ => None,
            })
            .collect();
        let pending_snapshots: Vec<_> = unique
            .iter()
            .filter_map(|(hash, r)| match &r.input {
                Input::Snapshot(s)
                    if records
                        .get(&s.review_sha256)
                        .is_some_and(|r| r.input_sha256 == record.input_sha256)
                        && !changes.iter().any(|(_, c)| {
                            records
                                .get(&c.snapshot_sha256)
                                .is_some_and(|r| r.input_sha256 == records[*hash].input_sha256)
                        }) =>
                {
                    Some((*hash, s))
                }
                _ => None,
            })
            .collect();
        if changes.is_empty() && pending_snapshots.is_empty() {
            let next = match origin.interpretation {
                CorrectionInterpretation::AssistantError => "apply_scoped_change",
                CorrectionInterpretation::Unknown => "review_context",
                _ => "no_change",
            };
            chains.push(
                json!({"pattern_id":origin.pattern_id,"review_sha256":review_hash,
                "change_sha256":null,"next_required_action":next,"opportunity_count":0,
                "recurrence_count":0,"recurrence_rate":null}),
            );
        }
        for (snapshot_hash, snapshot) in pending_snapshots {
            let next = if snapshot.scope == ConversationScope::Common && origin.project_dependent {
                "review_scope"
            } else {
                "apply_scoped_change"
            };
            chains.push(
                json!({"pattern_id":origin.pattern_id,"review_sha256":review_hash,
                "snapshot_sha256":snapshot_hash,"change_sha256":null,"next_required_action":next,
                "opportunity_count":0,"recurrence_count":0,"recurrence_rate":null}),
            );
        }
        for (change_hash, applied) in changes {
            let activations: Vec<_> = unique
                .iter()
                .filter_map(|(_, r)| match &r.input {
                    Input::Activation(a)
                        if records.get(&a.change_sha256).is_some_and(|r| {
                            r.input_sha256 == records[change_hash].input_sha256
                        }) =>
                    {
                        Some(a)
                    }
                    _ => None,
                })
                .collect();
            let observations: Vec<_> = unique
                .iter()
                .filter_map(|(_, r)| match &r.input {
                    Input::Observation(o)
                        if records.get(&o.change_sha256).is_some_and(|r| {
                            r.input_sha256 == records[change_hash].input_sha256
                        }) =>
                    {
                        Some(o)
                    }
                    _ => None,
                })
                .collect();
            let mut opportunities = 0;
            let mut recurrences = 0;
            let mut inconclusive = 0;
            let mut unassessed = 0;
            for observation in observations {
                if !comparable(observation, applied, origin, &activations)? {
                    inconclusive += 1;
                } else if observation.opportunity.is_none()
                    || (observation.opportunity == Some(true) && observation.recurred.is_none())
                {
                    unassessed += 1;
                } else if observation.opportunity == Some(true) {
                    opportunities += 1;
                    if observation.recurred == Some(true) {
                        recurrences += 1;
                    }
                }
            }
            let next = if activations.is_empty() {
                "verify_activation"
            } else if recurrences > 0 {
                "review_recurrence"
            } else if inconclusive > 0 {
                "comparison_inconclusive"
            } else if unassessed > 0 {
                "assess_opportunity"
            } else if opportunities > 0 {
                "no_recurrence_observed"
            } else {
                "wait_natural_opportunity"
            };
            let rate = if opportunities > 0 && inconclusive == 0 && unassessed == 0 {
                Some(recurrences as f64 / opportunities as f64)
            } else {
                None
            };
            total_opportunities += opportunities;
            total_recurrences += recurrences;
            total_inconclusive += inconclusive;
            total_unassessed += unassessed;
            chains.push(json!({"pattern_id":origin.pattern_id,"review_sha256":review_hash,
                "change_sha256":change_hash,"next_required_action":next,"activation_observed":!activations.is_empty(),
                "opportunity_count":opportunities,"recurrence_count":recurrences,"recurrence_rate":rate,
                "inconclusive_count":inconclusive,"unassessed_count":unassessed}));
        }
    }
    let count =
        |predicate: fn(&Input) -> bool| unique.iter().filter(|(_, r)| predicate(&r.input)).count();
    let assessment_complete = !chains.is_empty()
        && chains.iter().all(|chain| {
            matches!(
                chain["next_required_action"].as_str(),
                Some("no_change" | "no_recurrence_observed" | "review_recurrence")
            )
        });
    Ok(
        json!({"kind":"groundline-learning-conversation-status","schema":1,
        "review_count":count(|i| matches!(i,Input::Review(_))),
        "snapshot_count":count(|i| matches!(i,Input::Snapshot(_))),
        "change_count":count(|i| matches!(i,Input::Change(_))),
        "activation_count":count(|i| matches!(i,Input::Activation(_))),
        "observation_count":count(|i| matches!(i,Input::Observation(_))),
        "opportunity_count":total_opportunities,"recurrence_count":total_recurrences,
        "inconclusive_count":total_inconclusive,"unassessed_count":total_unassessed,
        "assessment_complete":assessment_complete,
        "recurrence_rate":if total_opportunities > 0 && total_inconclusive == 0 && total_unassessed == 0 && assessment_complete {
            Some(total_recurrences as f64 / total_opportunities as f64) } else { None },
        "causal_effect_established":false,"semantic_authenticity_established":false,
        "activation_authenticity_established":false,"efficiency_gain_established":false,
        "source_revision_matching":"provenance_only","chains":chains}),
    )
}

/// Hashes eligible for archival. All records under a review remain live if any
/// chain still needs action, including captured targets without an application.
/// The CLI additionally preserves its latest completed history window.
pub fn compactable_chain_hashes(
    values: &[Value],
) -> Result<BTreeMap<String, BTreeSet<String>>, ContractError> {
    let status = readouts(values)?;
    let records = records(values)?;
    let mut completed = BTreeMap::<String, bool>::new();
    for chain in status["chains"].as_array().into_iter().flatten() {
        let Some(hash) = chain["review_sha256"].as_str() else {
            continue;
        };
        let Some(record) = records.get(hash) else {
            continue;
        };
        let complete = matches!(
            chain["next_required_action"].as_str(),
            Some("no_change" | "no_recurrence_observed")
        );
        completed
            .entry(record.input_sha256.clone())
            .and_modify(|v| *v &= complete)
            .or_insert(complete);
    }
    let referenced_snapshots: BTreeSet<_> = records
        .values()
        .filter_map(|r| match &r.input {
            Input::Change(c) => records.get(&c.snapshot_sha256).map(|s| &s.input_sha256),
            _ => None,
        })
        .collect();
    for record in records.values() {
        if let Input::Snapshot(s) = &record.input
            && !referenced_snapshots.contains(&record.input_sha256)
        {
            completed.insert(records[&s.review_sha256].input_sha256.clone(), false);
        }
    }
    let root = |record: &Record| -> Result<String, ContractError> {
        let review_hash = match &record.input {
            Input::Review(_) => return Ok(record.input_sha256.clone()),
            Input::Snapshot(s) => &s.review_sha256,
            Input::Change(c) => &c.review_sha256,
            Input::Activation(a) => &change(&records, &a.change_sha256)?.review_sha256,
            Input::Observation(o) => &change(&records, &o.change_sha256)?.review_sha256,
        };
        Ok(records[review_hash].input_sha256.clone())
    };
    let review_hashes: BTreeMap<_, _> = records
        .iter()
        .filter(|(_, r)| matches!(r.input, Input::Review(_)))
        .map(|(hash, r)| (r.input_sha256.clone(), hash.clone()))
        .collect();
    let mut eligible = BTreeMap::<String, BTreeSet<String>>::new();
    for (hash, record) in &records {
        let root = root(record)?;
        if completed.get(&root) == Some(&true) {
            eligible
                .entry(review_hashes[&root].clone())
                .or_default()
                .insert(hash.clone());
        }
    }
    Ok(eligible)
}

pub fn compactable_record_hashes(values: &[Value]) -> Result<BTreeSet<String>, ContractError> {
    Ok(compactable_chain_hashes(values)?
        .into_values()
        .flatten()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(c: char) -> String {
        c.to_string().repeat(64)
    }
    fn review_input() -> Value {
        json!({"kind":"groundline-learning-conversation-review-input","schema":1,
            "pattern_id":"verify-condition","feedback_ref_sha256":h('1'),"source_work_sha256":h('2'),
            "interpretation":"assistant_error","observed_scope":{"kind":"project","project_sha256":h('3')},
            "project_dependent":true,"criterion_sha256":h('4'),"criterion_version":"v1",
            "preceding_assistant_action_sha256":h('5'),"user_correction_sha256":h('6'),
            "context_evidence_sha256":h('7'),"observed_at_utc":"2026-01-01T00:00:00Z",
            "summary":"private derived summary"})
    }
    fn record(input: &Value) -> Value {
        build_record(input, "2026-01-02T00:00:00Z").unwrap()
    }
    fn change_input(review: &Value) -> Value {
        json!({"kind":"groundline-learning-conversation-change-input","schema":1,
            "review_sha256":content_sha256(review).unwrap(),
            "snapshot_sha256":content_sha256(&record(&snapshot_input(review))).unwrap(),
            "target_binding_sha256":h('0'),
            "target":{"scope":{"kind":"project","project_sha256":h('3')},
                "target_id":"project-instructions","kind":"agents_block","before_sha256":h('8'),"after_sha256":h('9')},
            "verification_evidence_sha256":h('a'),"applied_at_utc":"2026-01-01T01:00:00Z",
            "comparison_context_sha256":h('b'),"transferability_review":null})
    }
    fn snapshot_input(review: &Value) -> Value {
        json!({"kind":"groundline-learning-conversation-snapshot-input","schema":1,
            "review_sha256":content_sha256(review).unwrap(),"scope":{"kind":"project","project_sha256":h('3')},
            "target_id":"project-instructions","target_kind":"agents_block","before_sha256":h('8'),
            "target_binding_sha256":h('0'),"captured_at_utc":"2026-01-01T00:30:00Z"})
    }
    fn fixture() -> Vec<Value> {
        let reviewed = record(&review_input());
        let applied = record(&change_input(&reviewed));
        let activation = record(
            &json!({"kind":"groundline-learning-conversation-activation-input","schema":1,
            "change_sha256":content_sha256(&applied).unwrap(),"target_after_sha256":h('9'),
            "activation_kind":"instruction_read","comparison_context_sha256":h('b'),
            "evidence_sha256":h('c'),"activated_at_utc":"2026-01-01T02:00:00Z"}),
        );
        let snapshot = record(&snapshot_input(&reviewed));
        vec![reviewed, applied, activation, snapshot]
    }
    fn observation_input(values: &[Value]) -> Value {
        json!({"kind":"groundline-learning-conversation-observation-input","schema":1,
            "change_sha256":content_sha256(&values[1]).unwrap(),"source_work_sha256":h('d'),
            "source_revision_sha256":h('e'),"target_after_sha256":h('9'),"criterion_sha256":h('4'),
            "criterion_version":"v1","comparison_context_sha256":h('b'),
            "opportunity":true,"recurred":false,"evidence_sha256":h('f'),
            "recurrence_evidence_sha256":null,"observed_at_utc":"2026-01-01T03:00:00Z"})
    }

    #[test]
    fn retrospective_review_has_no_delivery_or_resource_prerequisite() {
        let value = record(&review_input());
        validate_collection(std::slice::from_ref(&value)).unwrap();
        let readout = readouts(&[value]).unwrap();
        assert_eq!(
            readout["chains"][0]["next_required_action"],
            "apply_scoped_change"
        );
        assert_eq!(readout["recurrence_rate"], Value::Null);
        assert!(!readout.to_string().contains("private derived summary"));
        assert!(!readout.to_string().contains("observed_scope"));
    }

    #[test]
    fn interpretation_does_not_promote_normal_followups_into_errors() {
        for (kind, next) in [
            ("iteration", "no_change"),
            ("new_requirement", "no_change"),
            ("direction_change", "no_change"),
            ("unknown", "review_context"),
        ] {
            let mut input = review_input();
            input["interpretation"] = json!(kind);
            let reviewed = record(&input);
            assert_eq!(
                readouts(std::slice::from_ref(&reviewed)).unwrap()["chains"][0]["next_required_action"],
                next
            );
            let applied = record(&change_input(&reviewed));
            assert_eq!(
                validate_collection(&[reviewed, applied]).unwrap_err().0,
                "learning_conversation_error_review_required"
            );
        }
    }

    #[test]
    fn project_changes_cannot_pollute_other_projects_or_unreviewed_common_guidance() {
        let reviewed = record(&review_input());
        for scope in [
            json!({"kind":"project","project_sha256":h('a')}),
            json!({"kind":"common"}),
        ] {
            let mut input = change_input(&reviewed);
            input["target"]["scope"] = scope.clone();
            let mut snapshot = snapshot_input(&reviewed);
            snapshot["scope"] = scope;
            let snapshot = record(&snapshot);
            input["snapshot_sha256"] = json!(content_sha256(&snapshot).unwrap());
            input["transferability_review"] =
                json!({"project_neutral":true,"evidence_sha256":h('a')});
            let expected = if input["target"]["scope"]["kind"] == "common" {
                "learning_conversation_common_scope_unreviewed"
            } else {
                "learning_conversation_project_scope_mismatch"
            };
            assert_eq!(
                validate_collection(&[reviewed.clone(), snapshot, record(&input)])
                    .unwrap_err()
                    .0,
                expected
            );
        }
        let mut neutral = review_input();
        neutral["project_dependent"] = json!(false);
        let reviewed = record(&neutral);
        let mut input = change_input(&reviewed);
        input["target"]["scope"] = json!({"kind":"common"});
        let mut snapshot = snapshot_input(&reviewed);
        snapshot["scope"] = json!({"kind":"common"});
        let snapshot = record(&snapshot);
        input["snapshot_sha256"] = json!(content_sha256(&snapshot).unwrap());
        assert!(
            validate_collection(&[reviewed.clone(), snapshot.clone(), record(&input)]).is_err()
        );
        input["transferability_review"] = json!({"project_neutral":true,"evidence_sha256":h('a')});
        validate_collection(&[reviewed, snapshot, record(&input)]).unwrap();
    }

    #[test]
    fn evidence_rebinding_and_missing_references_are_rejected() {
        let mut reviewed = record(&review_input());
        reviewed["input"]["user_correction_sha256"] = json!(h('a'));
        assert!(validate_record(&reviewed).is_err());
        let reviewed = record(&review_input());
        let applied = record(&change_input(&reviewed));
        assert_eq!(
            validate_collection(&[applied]).unwrap_err().0,
            "learning_conversation_review_reference_missing"
        );
        let mut input = review_input();
        input["unexpected_authority"] = json!(true);
        assert!(build_record(&input, "2026-01-02T00:00:00Z").is_err());
    }

    #[test]
    fn duplicate_feedback_and_work_do_not_increase_counts_and_conflicts_fail() {
        let mut values = fixture();
        let input = observation_input(&values);
        let observation = record(&input);
        values.push(observation.clone());
        values.push(observation);
        assert_eq!(readouts(&values).unwrap()["opportunity_count"], 1);
        // Identical input re-recorded at another time remains one work item.
        values.push(build_record(&input, "2026-01-03T00:00:00Z").unwrap());
        assert_eq!(readouts(&values).unwrap()["opportunity_count"], 1);
        let mut changed = input;
        changed["evidence_sha256"] = json!(h('a'));
        values.push(record(&changed));
        assert_eq!(
            validate_collection(&values).unwrap_err().0,
            "learning_conversation_duplicate_conflict"
        );
        let first = record(&review_input());
        let mut input = review_input();
        input["interpretation"] = json!("iteration");
        assert!(validate_collection(&[first, record(&input)]).is_err());
        // Changing only a parent's wrapper timestamp is not a way around the
        // stable native work duplicate guard.
        let mut values = fixture();
        let parent_alias = build_record(&values[1]["input"], "2026-01-03T00:00:00Z").unwrap();
        let input = observation_input(&values);
        values.push(record(&input));
        let mut alias_input = input;
        alias_input["change_sha256"] = json!(content_sha256(&parent_alias).unwrap());
        values.push(parent_alias);
        values.push(record(&alias_input));
        assert_eq!(
            validate_collection(&values).unwrap_err().0,
            "learning_conversation_duplicate_conflict"
        );
    }

    #[test]
    fn change_is_distinct_from_activation_and_activation_binds_revision_kind_and_context() {
        let values = fixture();
        assert_eq!(
            readouts(&[values[0].clone(), values[1].clone(), values[3].clone()]).unwrap()["chains"]
                [0]["next_required_action"],
            "verify_activation"
        );
        assert_eq!(
            readouts(&values).unwrap()["chains"][0]["next_required_action"],
            "wait_natural_opportunity"
        );
        for (field, value) in [
            ("target_after_sha256", json!(h('a'))),
            ("activation_kind", json!("skill_invoked")),
            ("comparison_context_sha256", json!(h('a'))),
            ("activated_at_utc", json!("2026-01-01T00:30:00Z")),
        ] {
            let mut input = values[2]["input"].clone();
            input[field] = value;
            assert!(
                validate_collection(&[
                    values[0].clone(),
                    values[1].clone(),
                    values[3].clone(),
                    record(&input)
                ])
                .is_err()
            );
        }
    }

    #[test]
    fn timing_conditions_criterion_and_revision_changes_are_inconclusive() {
        for (field, value) in [
            ("observed_at_utc", json!("2026-01-01T01:30:00Z")),
            ("comparison_context_sha256", json!(h('a'))),
            ("criterion_sha256", json!(h('a'))),
            ("criterion_version", json!("v2")),
            ("target_after_sha256", json!(h('a'))),
            ("source_work_sha256", json!(h('2'))),
        ] {
            let mut values = fixture();
            let mut input = observation_input(&values);
            input[field] = value;
            values.push(record(&input));
            let status = readouts(&values).unwrap();
            assert_eq!(
                status["chains"][0]["next_required_action"],
                "comparison_inconclusive"
            );
            assert_eq!(status["opportunity_count"], 0);
            assert_eq!(status["recurrence_rate"], Value::Null);
        }
        let mut values = fixture();
        let input = observation_input(&values);
        values.remove(2);
        values.push(record(&input));
        assert_eq!(
            readouts(&values).unwrap()["chains"][0]["next_required_action"],
            "verify_activation"
        );
        assert_eq!(readouts(&values).unwrap()["opportunity_count"], 0);
    }

    #[test]
    fn no_opportunity_and_unassessed_observations_are_never_successes() {
        for (opportunity, recurred, next) in [
            (json!(false), Value::Null, "wait_natural_opportunity"),
            (Value::Null, Value::Null, "assess_opportunity"),
            (json!(true), Value::Null, "assess_opportunity"),
        ] {
            let mut values = fixture();
            let mut input = observation_input(&values);
            input["opportunity"] = opportunity;
            input["recurred"] = recurred;
            values.push(record(&input));
            let status = readouts(&values).unwrap();
            assert_eq!(status["chains"][0]["next_required_action"], next);
            assert_eq!(status["opportunity_count"], 0);
            assert_eq!(status["recurrence_rate"], Value::Null);
        }
        let mut input = observation_input(&fixture());
        input["opportunity"] = json!(false);
        assert!(build_record(&input, "2026-01-02T00:00:00Z").is_err());
    }

    #[test]
    fn real_comparable_observations_report_bounded_recurrence_without_causal_claims() {
        for (recurred, next, rate) in [
            (false, "no_recurrence_observed", 0.0),
            (true, "review_recurrence", 1.0),
        ] {
            let mut values = fixture();
            let mut input = observation_input(&values);
            input["recurred"] = json!(recurred);
            if recurred {
                input["recurrence_evidence_sha256"] = json!(h('a'));
            }
            values.push(record(&input));
            let status = readouts(&values).unwrap();
            assert_eq!(status["chains"][0]["next_required_action"], next);
            assert_eq!(status["opportunity_count"], 1);
            assert_eq!(status["recurrence_rate"], rate);
            assert_eq!(status["causal_effect_established"], false);
            assert_eq!(status["semantic_authenticity_established"], false);
            assert_eq!(status["efficiency_gain_established"], false);
        }
    }

    #[test]
    fn changes_require_bound_actual_before_capture_with_exact_identity_and_timing() {
        let values = fixture();
        validate_collection(&values).unwrap();
        for (field, value) in [
            ("before_sha256", json!(h('a'))),
            ("scope", json!({"kind":"project","project_sha256":h('a')})),
            ("target_id", json!("other-target")),
            ("target_kind", json!("skill")),
            ("target_binding_sha256", json!(h('a'))),
            ("captured_at_utc", json!("2026-01-01T01:30:00Z")),
        ] {
            let mut snapshot_input = values[3]["input"].clone();
            snapshot_input[field] = value;
            let snapshot = record(&snapshot_input);
            let mut change = values[1]["input"].clone();
            change["snapshot_sha256"] = json!(content_sha256(&snapshot).unwrap());
            assert_eq!(
                validate_collection(&[values[0].clone(), snapshot, record(&change)])
                    .unwrap_err()
                    .0,
                "learning_conversation_snapshot_mismatch"
            );
        }
        let mut other = review_input();
        other["feedback_ref_sha256"] = json!(h('a'));
        other["pattern_id"] = json!("other-pattern");
        let other = record(&other);
        let mut snapshot = values[3]["input"].clone();
        snapshot["review_sha256"] = json!(content_sha256(&other).unwrap());
        let snapshot = record(&snapshot);
        let mut change = values[1]["input"].clone();
        change["snapshot_sha256"] = json!(content_sha256(&snapshot).unwrap());
        assert_eq!(
            validate_collection(&[values[0].clone(), other, snapshot, record(&change)])
                .unwrap_err()
                .0,
            "learning_conversation_snapshot_mismatch"
        );
        let mut early = values[3]["input"].clone();
        early["captured_at_utc"] = json!("2025-12-31T23:00:00Z");
        assert_eq!(
            validate_collection(&[values[0].clone(), record(&early)])
                .unwrap_err()
                .0,
            "learning_conversation_snapshot_before_review"
        );
        let mut fake = values[1]["input"].clone();
        fake["snapshot_sha256"] = json!(h('f'));
        assert_eq!(
            validate_collection(&[values[0].clone(), record(&fake)])
                .unwrap_err()
                .0,
            "learning_conversation_snapshot_reference_missing"
        );
    }

    #[test]
    fn archival_preserves_every_pending_inconclusive_recurrence_and_prepared_target() {
        for interpretation in ["iteration", "new_requirement", "direction_change"] {
            let mut input = review_input();
            input["interpretation"] = json!(interpretation);
            let review = record(&input);
            assert_eq!(
                compactable_record_hashes(std::slice::from_ref(&review)).unwrap(),
                BTreeSet::from([content_sha256(&review).unwrap()])
            );
        }
        let mut unknown = review_input();
        unknown["interpretation"] = json!("unknown");
        assert!(
            compactable_record_hashes(&[record(&unknown)])
                .unwrap()
                .is_empty()
        );
        assert!(compactable_record_hashes(&fixture()).unwrap().is_empty());
        for (opportunity, recurred, context) in [
            (Value::Null, Value::Null, h('b')),
            (json!(false), Value::Null, h('b')),
            (json!(true), json!(true), h('b')),
            (json!(true), json!(false), h('a')),
        ] {
            let mut values = fixture();
            let mut input = observation_input(&values);
            input["opportunity"] = opportunity;
            input["recurred"] = recurred;
            input["comparison_context_sha256"] = json!(context);
            if input["recurred"] == true {
                input["recurrence_evidence_sha256"] = json!(h('a'));
            }
            values.push(record(&input));
            assert!(compactable_record_hashes(&values).unwrap().is_empty());
        }
        let mut values = fixture();
        values.push(record(&observation_input(&values)));
        assert_eq!(
            compactable_record_hashes(&values).unwrap().len(),
            values.len()
        );
        let groups = compactable_chain_hashes(&values).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[&content_sha256(&values[0]).unwrap()].len(),
            values.len()
        );
        let mut pending_snapshot = values[3]["input"].clone();
        pending_snapshot["target_id"] = json!("second-target");
        pending_snapshot["target_binding_sha256"] = json!(h('a'));
        let snapshot = record(&pending_snapshot);
        values.push(snapshot.clone());
        assert!(compactable_record_hashes(&values).unwrap().is_empty());
        let status = readouts(&values).unwrap();
        assert_eq!(status["chains"].as_array().unwrap().len(), 2);
        assert_eq!(status["assessment_complete"], false);
        assert_eq!(status["recurrence_rate"], Value::Null);
        let mut pending_change = values[1]["input"].clone();
        pending_change["snapshot_sha256"] = json!(content_sha256(&snapshot).unwrap());
        pending_change["target_binding_sha256"] = json!(h('a'));
        pending_change["target"]["target_id"] = json!("second-target");
        values.push(record(&pending_change));
        assert!(compactable_record_hashes(&values).unwrap().is_empty());
    }

    #[test]
    fn work_source_revision_is_explicit_provenance_instead_of_effect_matching() {
        let mut values = fixture();
        let mut input = observation_input(&values);
        input["source_revision_sha256"] = json!(h('0'));
        values.push(record(&input));
        let status = readouts(&values).unwrap();
        assert_eq!(status["source_revision_matching"], "provenance_only");
        assert_eq!(
            status["chains"][0]["next_required_action"],
            "no_recurrence_observed"
        );
    }
}
