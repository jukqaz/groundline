//! Owner-private learning sidecars. Supplied hashes identify evidence; they do
//! not authenticate its origin or prove native activation or a causal effect.
use crate::{ContractError, delivery};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EVIDENCE_REFS: usize = 64;

fn error(code: &str) -> ContractError {
    ContractError(format!("learning_{code}"))
}

pub fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        && value != "."
        && value != ".."
}

fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
}

pub fn content_sha256(value: &Value) -> Result<String, ContractError> {
    let bytes = serde_json::to_vec(value).map_err(|_| error("serialization_failed"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeObservation {
    pub family: String,
    pub version: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillObservation {
    pub target_id: String,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeLink {
    pub kind: String,
    pub schema: u8,
    pub receipt_sha256: String,
    pub unit_hash: String,
    pub cohort_sha256: String,
    pub phase: String,
    pub completed_at_utc: String,
    /// Explicit historical observation. The CLI never reads the current
    /// environment to fill any of these fields.
    pub observed_at_utc: Option<String>,
    pub environment_revision: Option<String>,
    pub skill: Option<SkillObservation>,
    pub source_revision: Option<String>,
    pub runtime: Option<RuntimeObservation>,
    pub correction_kind: String,
    pub correction_evidence_sha256: Option<String>,
}

impl OutcomeLink {
    pub fn validate(&self) -> Result<(), ContractError> {
        if !["groundline-learning-link-input", "groundline-learning-link"]
            .contains(&self.kind.as_str())
            || self.schema != 1
            || !digest(&self.receipt_sha256)
            || !digest(&self.unit_hash)
            || !digest(&self.cohort_sha256)
            || !delivery::PHASES.contains(&self.phase.as_str())
            || ![
                "acceptance",
                "assistant_error",
                "new_requirement",
                "direction_change",
                "unknown",
            ]
            .contains(&self.correction_kind.as_str())
            || self
                .environment_revision
                .as_ref()
                .is_some_and(|s| !identifier(s))
            || self
                .source_revision
                .as_ref()
                .is_some_and(|s| !identifier(s))
            || self
                .correction_evidence_sha256
                .as_ref()
                .is_some_and(|s| !digest(s))
            || (self.correction_kind != "unknown" && self.correction_evidence_sha256.is_none())
        {
            return Err(error("invalid_link"));
        }
        let completed = DateTime::parse_from_rfc3339(&self.completed_at_utc)
            .map_err(|_| error("invalid_link_timestamp"))?;
        if completed.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5) {
            return Err(error("invalid_link_timestamp"));
        }
        if self.environment_revision.is_some()
            || self.skill.is_some()
            || self.source_revision.is_some()
            || self.runtime.is_some()
        {
            let observed = self
                .observed_at_utc
                .as_deref()
                .ok_or_else(|| error("historical_observation_required"))?;
            let observed = DateTime::parse_from_rfc3339(observed)
                .map_err(|_| error("invalid_link_timestamp"))?;
            if observed > completed {
                return Err(error("observation_after_delivery"));
            }
        } else if let Some(observed) = &self.observed_at_utc {
            let observed = DateTime::parse_from_rfc3339(observed)
                .map_err(|_| error("invalid_link_timestamp"))?;
            if observed > completed {
                return Err(error("observation_after_delivery"));
            }
        }
        if self
            .skill
            .as_ref()
            .is_some_and(|s| !identifier(&s.target_id) || !digest(&s.revision))
        {
            return Err(error("invalid_skill_observation"));
        }
        if self.runtime.as_ref().is_some_and(|r| {
            !["codex_app", "codex_cli"].contains(&r.family.as_str())
                || !identifier(&r.version)
                || !digest(&r.evidence_sha256)
        }) {
            return Err(error("invalid_runtime_observation"));
        }
        Ok(())
    }
}

pub fn link_outcome(
    input: &Value,
    receipt: &Value,
    receipt_sha256: &str,
) -> Result<Value, ContractError> {
    let mut link: OutcomeLink =
        serde_json::from_value(input.clone()).map_err(|_| error("invalid_link"))?;
    if link.kind != "groundline-learning-link-input" {
        return Err(error("invalid_link"));
    }
    link.validate()?;
    delivery::validate_receipt(receipt)?;
    if link.receipt_sha256 != receipt_sha256
        || receipt["unit_hash"] != link.unit_hash
        || receipt["cohort_sha256"] != link.cohort_sha256
        || receipt["phase"] != link.phase
        || receipt["completed_at_utc"] != link.completed_at_utc
    {
        return Err(error("receipt_link_mismatch"));
    }
    link.kind = "groundline-learning-link".into();
    serde_json::to_value(link).map_err(|_| error("serialization_failed"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureInput {
    pub kind: String,
    pub schema: u8,
    pub unit_hash: String,
    pub cohort_sha256: String,
    pub phase: String,
    pub runtime: Option<RuntimeObservation>,
}

impl CaptureInput {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.kind != "groundline-learning-capture-input"
            || self.schema != 1
            || !digest(&self.unit_hash)
            || !digest(&self.cohort_sha256)
            || !delivery::PHASES.contains(&self.phase.as_str())
            || self.runtime.as_ref().is_some_and(|r| {
                !["codex_app", "codex_cli"].contains(&r.family.as_str())
                    || !identifier(&r.version)
                    || !digest(&r.evidence_sha256)
            })
        {
            return Err(error("invalid_capture_input"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub kind: String,
    pub schema: u8,
    pub captured_at_utc: String,
    pub unit_hash: String,
    pub cohort_sha256: String,
    pub phase: String,
    pub environment_revision: Option<String>,
    pub source_revision: Option<String>,
    pub skill: Option<SkillObservation>,
    pub runtime: Option<RuntimeObservation>,
    pub environment_provenance_sha256: String,
    pub capture_input_sha256: String,
    pub native_activation: String,
}

impl Snapshot {
    pub fn validate(&self) -> Result<(), ContractError> {
        CaptureInput {
            kind: "groundline-learning-capture-input".into(),
            schema: self.schema,
            unit_hash: self.unit_hash.clone(),
            cohort_sha256: self.cohort_sha256.clone(),
            phase: self.phase.clone(),
            runtime: self.runtime.clone(),
        }
        .validate()?;
        let captured = DateTime::parse_from_rfc3339(&self.captured_at_utc)
            .map_err(|_| error("invalid_snapshot_timestamp"))?;
        if self.kind != "groundline-learning-snapshot"
            || self.native_activation != "UNVERIFIED"
            || captured.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5)
            || !digest(&self.environment_provenance_sha256)
            || !digest(&self.capture_input_sha256)
            || [&self.environment_revision, &self.source_revision]
                .into_iter()
                .flatten()
                .any(|r| !identifier(r))
            || self
                .skill
                .as_ref()
                .is_some_and(|s| !identifier(&s.target_id) || !digest(&s.revision))
        {
            return Err(error("invalid_snapshot"));
        }
        Ok(())
    }
}

/// The CLI supplies its current clock and a checked environment context. Neither
/// an input timestamp nor an input source revision can retrofit an earlier task.
pub fn capture_snapshot(
    input: &Value,
    context: &Value,
    input_sha256: &str,
    captured_at_utc: &str,
) -> Result<Value, ContractError> {
    let input: CaptureInput =
        serde_json::from_value(input.clone()).map_err(|_| error("invalid_capture_input"))?;
    input.validate()?;
    let target_id = context["target_id"]
        .as_str()
        .filter(|s| identifier(s))
        .ok_or_else(|| error("invalid_environment_context"))?;
    let revision = |key: &str| -> Result<Option<String>, ContractError> {
        match &context[key] {
            Value::Null => Ok(None),
            Value::String(s) if identifier(s) => Ok(Some(s.clone())),
            _ => Err(error("invalid_environment_context")),
        }
    };
    let skill = match &context["skill_revision"] {
        Value::Null => None,
        Value::String(s) if digest(s) => Some(SkillObservation {
            target_id: target_id.into(),
            revision: s.clone(),
        }),
        _ => return Err(error("invalid_environment_context")),
    };
    let snapshot = Snapshot {
        kind: "groundline-learning-snapshot".into(),
        schema: 1,
        captured_at_utc: captured_at_utc.into(),
        unit_hash: input.unit_hash,
        cohort_sha256: input.cohort_sha256,
        phase: input.phase,
        environment_revision: revision("environment_revision")?,
        source_revision: revision("source_revision")?,
        skill,
        runtime: input.runtime,
        environment_provenance_sha256: context["provenance_sha256"]
            .as_str()
            .ok_or_else(|| error("invalid_environment_context"))?
            .into(),
        capture_input_sha256: input_sha256.into(),
        native_activation: "UNVERIFIED".into(),
    };
    snapshot.validate()?;
    serde_json::to_value(snapshot).map_err(|_| error("serialization_failed"))
}

/// Prepare an ordinary link input without inventing acceptance or copying the
/// resource ledger. The exact original receipt remains the resource authority.
pub fn prepare_outcome(
    snapshot: &Value,
    receipt: &Value,
    receipt_sha256: &str,
) -> Result<Value, ContractError> {
    let snapshot: Snapshot =
        serde_json::from_value(snapshot.clone()).map_err(|_| error("invalid_snapshot"))?;
    snapshot.validate()?;
    delivery::validate_receipt(receipt)?;
    if receipt["unit_hash"] != snapshot.unit_hash
        || receipt["cohort_sha256"] != snapshot.cohort_sha256
        || receipt["phase"] != snapshot.phase
    {
        return Err(error("snapshot_receipt_mismatch"));
    }
    let completed = receipt["completed_at_utc"]
        .as_str()
        .ok_or_else(|| error("invalid_link_timestamp"))?;
    if DateTime::parse_from_rfc3339(&snapshot.captured_at_utc)
        .map_err(|_| error("invalid_snapshot_timestamp"))?
        > DateTime::parse_from_rfc3339(completed).map_err(|_| error("invalid_link_timestamp"))?
    {
        return Err(error("snapshot_after_delivery"));
    }
    let input = json!({"kind":"groundline-learning-link-input", "schema":1,
        "receipt_sha256":receipt_sha256,"unit_hash":snapshot.unit_hash,
        "cohort_sha256":snapshot.cohort_sha256,"phase":snapshot.phase,
        "completed_at_utc":completed,"observed_at_utc":snapshot.captured_at_utc,
        "environment_revision":snapshot.environment_revision,"skill":snapshot.skill,
        "source_revision":snapshot.source_revision,"runtime":snapshot.runtime,
        "correction_kind":"unknown","correction_evidence_sha256":null});
    link_outcome(&input, receipt, receipt_sha256)?;
    Ok(input)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisResources {
    pub unit_hash: String,
    pub resources: delivery::Resources,
}

impl AnalysisResources {
    pub fn validate(&self) -> Result<(), ContractError> {
        if !digest(&self.unit_hash) {
            return Err(error("invalid_analysis_resources"));
        }
        // Reuse the current owned-response arithmetic and model observation
        // contract; no root selection is ever inherited by a child response.
        delivery::validate_receipt(&self.as_receipt())
            .map_err(|_| error("invalid_analysis_resources"))
    }

    pub fn as_receipt(&self) -> Value {
        json!({
            "kind":"groundline-delivery-receipt", "schema":1,
            "unit_hash":self.unit_hash, "cohort_sha256":"0".repeat(64),
            "phase":"research", "completed_at_utc":"2026-01-01T00:00:00Z",
            "recommendation":null, "requested":null, "effective":null,
            "verification":{"status":"unknown", "evidence_kind":"unobserved",
                "evidence_sha256":"0".repeat(64), "rework":false, "authenticity_verified":false},
            "resources":self.resources, "activation_verified":false,
            "observed_selection_matches_requested":null
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetChange {
    pub target_id: String,
    pub kind: String,
    pub change: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub intended_trigger: String,
    pub hypothesis: String,
    pub expected_result: String,
    pub falsification: String,
    pub authority_ref: String,
    pub rollback_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObservation {
    pub url: String,
    pub checked_at_utc: String,
    pub content_sha256: String,
    pub model: Option<String>,
    pub runtime: Option<RuntimeObservation>,
    pub affected_target_ids: Vec<String>,
}

impl SourceObservation {
    pub fn validate(&self) -> Result<(), ContractError> {
        // A maintained URL parser handles authority, userinfo, ports, and host
        // normalization; string-prefix checks would accept lookalike domains.
        let url = url::Url::parse(&self.url).map_err(|_| error("invalid_source"))?;
        let official = matches!(
            url.host_str(),
            Some("developers.openai.com" | "learn.chatgpt.com" | "openai.com")
        ) || (url.host_str() == Some("github.com")
            && url.path().starts_with("/openai/"));
        let checked = DateTime::parse_from_rfc3339(&self.checked_at_utc)
            .map_err(|_| error("invalid_source"))?;
        let mut ids = BTreeSet::new();
        if self.url.len() > 2048
            || url.scheme() != "https"
            || !official
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some_and(|p| p != 443)
            || url.query().is_some()
            || url.fragment().is_some()
            || checked.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5)
            || !digest(&self.content_sha256)
            || self
                .model
                .as_ref()
                .is_some_and(|m| crate::model::identity_kind(m) != "public_model_id")
            || self.affected_target_ids.is_empty()
            || self.affected_target_ids.len() > MAX_EVIDENCE_REFS
            || self
                .affected_target_ids
                .iter()
                .any(|id| !identifier(id) || !ids.insert(id))
            || self.runtime.as_ref().is_some_and(|r| {
                !["codex_app", "codex_cli"].contains(&r.family.as_str())
                    || !identifier(&r.version)
                    || !digest(&r.evidence_sha256)
            })
        {
            return Err(error("invalid_source"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub kind: String,
    pub schema: u8,
    pub proposal_id: String,
    /// Sorted sidecar content digests, not private filenames or delivery IDs.
    pub evidence_refs: Vec<String>,
    pub scope_sha256: String,
    pub basis_revision: String,
    pub source_revision: Option<String>,
    pub source: Option<SourceObservation>,
    /// Exact environment plan digest; applying a different plan cannot count.
    pub plan_sha256: String,
    pub status: String,
    pub reason: String,
    pub target: Option<TargetChange>,
    pub analysis: Option<AnalysisResources>,
}

impl Proposal {
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut refs = BTreeSet::new();
        if ![
            "groundline-learning-proposal-input",
            "groundline-learning-proposal",
        ]
        .contains(&self.kind.as_str())
            || self.schema != 1
            || !identifier(&self.proposal_id)
            || self.evidence_refs.is_empty()
            || self.evidence_refs.len() > MAX_EVIDENCE_REFS
            || self
                .evidence_refs
                .iter()
                .any(|s| !digest(s) || !refs.insert(s))
            || !digest(&self.scope_sha256)
            || !identifier(&self.basis_revision)
            || !digest(&self.plan_sha256)
            || self
                .source_revision
                .as_ref()
                .is_some_and(|s| !identifier(s))
            || !["candidate", "no_change", "failed"].contains(&self.status.as_str())
            || !text(&self.reason)
            || (self.status == "candidate") != self.target.is_some()
        {
            return Err(error("invalid_proposal"));
        }
        if let Some(target) = &self.target
            && (!identifier(&target.target_id)
                || !["skill", "agents_block"].contains(&target.kind.as_str())
                || !["modify", "reduce", "remove"].contains(&target.change.as_str())
                || !digest(&target.before_sha256)
                || !digest(&target.after_sha256)
                || target.before_sha256 == target.after_sha256
                || ![
                    &target.intended_trigger,
                    &target.hypothesis,
                    &target.expected_result,
                    &target.falsification,
                ]
                .into_iter()
                .all(|s| text(s))
                || !identifier(&target.authority_ref)
                || !digest(&target.rollback_ref))
        {
            return Err(error("invalid_target_change"));
        }
        if let Some(source) = &self.source {
            source.validate()?;
            if self.source_revision.is_none()
                || self
                    .target
                    .as_ref()
                    .is_some_and(|t| !source.affected_target_ids.contains(&t.target_id))
            {
                return Err(error("source_revision_mismatch"));
            }
        }
        if let Some(analysis) = &self.analysis {
            analysis.validate()?;
        }
        Ok(())
    }

    pub fn dedup_sha256(&self) -> Result<String, ContractError> {
        let mut refs = self.evidence_refs.clone();
        refs.sort();
        content_sha256(
            &json!({"evidence_refs":refs, "scope_sha256":self.scope_sha256,
            "basis_revision":self.basis_revision, "source_revision":self.source_revision,
            "source":self.source.as_ref().map(|s| json!({"content_sha256":s.content_sha256,
                "model":s.model,"runtime":s.runtime.as_ref().map(|r|json!({"family":r.family,"version":r.version})),
                "affected_target_ids":s.affected_target_ids})),
            "target":self.target.as_ref().map(|t| json!({"target_id":t.target_id,
                "kind":t.kind,"before_sha256":t.before_sha256,"after_sha256":t.after_sha256}))}),
        )
    }
}

pub fn prepare_proposal(input: &Value, links: &[Value]) -> Result<Value, ContractError> {
    let mut proposal: Proposal =
        serde_json::from_value(input.clone()).map_err(|_| error("invalid_proposal"))?;
    if proposal.kind != "groundline-learning-proposal-input" {
        return Err(error("invalid_proposal"));
    }
    proposal.validate()?;
    let mut available = BTreeSet::new();
    for value in links {
        let link: OutcomeLink =
            serde_json::from_value(value.clone()).map_err(|_| error("invalid_link"))?;
        link.validate()?;
        if link.kind != "groundline-learning-link" {
            return Err(error("invalid_link"));
        }
        available.insert(content_sha256(value)?);
    }
    if proposal
        .evidence_refs
        .iter()
        .any(|s| !available.contains(s))
    {
        return Err(error("evidence_link_missing"));
    }
    proposal.kind = "groundline-learning-proposal".into();
    proposal.evidence_refs.sort();
    serde_json::to_value(proposal).map_err(|_| error("serialization_failed"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationInput {
    pub kind: String,
    pub schema: u8,
    pub proposal_id: String,
    /// Historical comparison revision, independent of the current plan's CAS basis.
    pub baseline_revision: String,
    pub proposal_revision: String,
    pub baseline_refs: Vec<String>,
    pub followup_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub kind: String,
    pub schema: u8,
    pub proposal_id: String,
    pub proposal_sha256: String,
    pub decision: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub evaluation_sha256: Option<String>,
    pub previous_decision_sha256: Option<String>,
    pub recorded_at_utc: Option<String>,
}

impl Decision {
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut refs = BTreeSet::new();
        if ![
            "groundline-learning-decision-input",
            "groundline-learning-decision",
        ]
        .contains(&self.kind.as_str())
            || self.schema != 1
            || !identifier(&self.proposal_id)
            || !digest(&self.proposal_sha256)
            || !["adopt", "hold", "reject"].contains(&self.decision.as_str())
            || !text(&self.reason)
            || self.evidence_refs.is_empty()
            || self.evidence_refs.len() > MAX_EVIDENCE_REFS
            || self
                .evidence_refs
                .iter()
                .any(|r| !digest(r) || !refs.insert(r))
            || [&self.evaluation_sha256, &self.previous_decision_sha256]
                .into_iter()
                .flatten()
                .any(|r| !digest(r))
            || (self.decision == "adopt" && self.evaluation_sha256.is_none())
            || (self.kind == "groundline-learning-decision") != self.recorded_at_utc.is_some()
        {
            return Err(error("invalid_decision"));
        }
        if let Some(timestamp) = &self.recorded_at_utc {
            let recorded = DateTime::parse_from_rfc3339(timestamp)
                .map_err(|_| error("invalid_decision_timestamp"))?;
            if recorded.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5) {
                return Err(error("invalid_decision_timestamp"));
            }
        }
        Ok(())
    }
}

pub fn validate_decision_references(
    decision: &Decision,
    records: &[Value],
) -> Result<(), ContractError> {
    decision.validate()?;
    let hashes: BTreeMap<String, &Value> = records
        .iter()
        .map(|v| Ok((content_sha256(v)?, v)))
        .collect::<Result<_, ContractError>>()?;
    let candidate = hashes
        .get(&decision.proposal_sha256)
        .filter(|v| {
            v["kind"] == "groundline-learning-proposal"
                && v["status"] == "candidate"
                && v["proposal_id"] == decision.proposal_id
        })
        .ok_or_else(|| error("decision_candidate_missing"))?;
    let mut allowed = BTreeSet::from([decision.proposal_sha256.clone()]);
    for reference in candidate["evidence_refs"].as_array().into_iter().flatten() {
        allowed.insert(
            reference
                .as_str()
                .ok_or_else(|| error("invalid_proposal"))?
                .into(),
        );
    }
    let mut evaluations = BTreeSet::new();
    for (hash, record) in &hashes {
        if record["kind"] == "groundline-learning-evaluation-record"
            && record["proposal_sha256"] == decision.proposal_sha256
            && record["input"]["proposal_id"] == decision.proposal_id
        {
            allowed.insert(hash.clone());
            evaluations.insert(hash.clone());
            for reference in record["input"]["baseline_refs"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(
                    record["input"]["followup_refs"]
                        .as_array()
                        .into_iter()
                        .flatten(),
                )
            {
                allowed.insert(
                    reference
                        .as_str()
                        .ok_or_else(|| error("invalid_evaluation"))?
                        .into(),
                );
            }
        }
    }
    if decision.evidence_refs.iter().any(|r| !allowed.contains(r))
        || decision
            .evaluation_sha256
            .as_ref()
            .is_some_and(|r| !evaluations.contains(r))
    {
        return Err(error("decision_evidence_unlinked"));
    }
    if let Some(previous) = &decision.previous_decision_sha256 {
        let previous = hashes
            .get(previous)
            .ok_or_else(|| error("decision_history_conflict"))?;
        if previous["kind"] != "groundline-learning-decision"
            || previous["proposal_sha256"] != decision.proposal_sha256
        {
            return Err(error("decision_history_conflict"));
        }
    }
    Ok(())
}

/// A single explicit decision chain per candidate; forks and disconnected
/// histories fail closed instead of choosing an arbitrary file ordering.
pub fn decision_head(
    records: &[Value],
    proposal_sha256: &str,
) -> Result<Option<String>, ContractError> {
    let mut children = BTreeMap::new();
    for record in records {
        if record["kind"] != "groundline-learning-decision"
            || record["proposal_sha256"] != proposal_sha256
        {
            continue;
        }
        let decision: Decision =
            serde_json::from_value(record.clone()).map_err(|_| error("invalid_decision"))?;
        decision.validate()?;
        if children
            .insert(decision.previous_decision_sha256, content_sha256(record)?)
            .is_some()
        {
            return Err(error("decision_history_conflict"));
        }
    }
    let mut head = None;
    let mut seen = BTreeSet::new();
    while let Some(next) = children.get(&head) {
        if !seen.insert(next.clone()) {
            return Err(error("decision_history_conflict"));
        }
        head = Some(next.clone());
    }
    if seen.len() != children.len() {
        return Err(error("decision_history_conflict"));
    }
    Ok(head)
}

pub fn prepare_decision(
    input: &Value,
    records: &[Value],
    now: &str,
) -> Result<Value, ContractError> {
    let mut decision: Decision =
        serde_json::from_value(input.clone()).map_err(|_| error("invalid_decision"))?;
    if decision.kind != "groundline-learning-decision-input" {
        return Err(error("invalid_decision"));
    }
    decision.evidence_refs.sort();
    validate_decision_references(&decision, records)?;
    let normalized = serde_json::to_value(&decision).map_err(|_| error("serialization_failed"))?;
    for record in records {
        if record["kind"] == "groundline-learning-decision" {
            let mut prior: Decision =
                serde_json::from_value(record.clone()).map_err(|_| error("invalid_decision"))?;
            prior.kind = "groundline-learning-decision-input".into();
            prior.recorded_at_utc = None;
            prior.evidence_refs.sort();
            if serde_json::to_value(prior).map_err(|_| error("serialization_failed"))? == normalized
            {
                return Ok(record.clone());
            }
        }
    }
    if decision_head(records, &decision.proposal_sha256)? != decision.previous_decision_sha256 {
        return Err(error("decision_history_conflict"));
    }
    decision.kind = "groundline-learning-decision".into();
    decision.recorded_at_utc = Some(now.into());
    decision.validate()?;
    serde_json::to_value(decision).map_err(|_| error("serialization_failed"))
}

fn operation_targets_proposal(proposal: &Proposal, operation: &Value) -> bool {
    operation["kind"] == "groundline-environment-operation"
        && operation["schema"] == 1
        && operation["native_activation"] == "UNVERIFIED"
        && operation["proposal_id"] == proposal.proposal_id
        && operation["basis_revision"] == proposal.basis_revision
        && operation["plan_sha256"] == proposal.plan_sha256
        && proposal
            .source_revision
            .as_ref()
            .is_none_or(|r| operation["source_revision"] == *r)
        && proposal.target.as_ref().is_some_and(|target| {
            let matching: Vec<&Value> = operation["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|entry| entry["target_id"] == target.target_id)
                .collect();
            matching.len() == 1
                && operation["authority_ref"] == target.authority_ref
                && if operation["action"] == "rollback" {
                    matching[0]["before_sha256"] == target.after_sha256
                        && matching[0]["after_sha256"] == target.before_sha256
                } else {
                    matching[0]["before_sha256"] == target.before_sha256
                        && matching[0]["after_sha256"] == target.after_sha256
                }
        })
}

pub fn operation_applies_to_proposal(proposal: &Proposal, operation: &Value) -> bool {
    operation_targets_proposal(proposal, operation)
        && operation["action"] == "apply"
        && operation["status"] == "APPLIED"
        && operation["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|entry| {
                proposal
                    .target
                    .as_ref()
                    .is_some_and(|target| entry["target_id"] == target.target_id)
                    && entry["status"] == "APPLIED"
            })
}

/// Structured observations and explicit decisions stay separate. The caller
/// validates private operation files; an evaluation alone cannot infer application.
pub fn candidate_readouts(
    records: &[Value],
    operations: &[(String, Value)],
) -> Result<Vec<Value>, ContractError> {
    let mut candidates = Vec::new();
    for candidate in records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-proposal" && v["status"] == "candidate")
    {
        let proposal: Proposal =
            serde_json::from_value(candidate.clone()).map_err(|_| error("invalid_proposal"))?;
        proposal.validate()?;
        let hash = content_sha256(candidate)?;
        let observed: Vec<_> = operations
            .iter()
            .filter(|(_, op)| op["proposal_id"] == proposal.proposal_id)
            .collect();
        let matching: Vec<_> = observed
            .iter()
            .filter(|(_, op)| {
                operation_targets_proposal(&proposal, op)
                    && (op["action"] != "rollback"
                        || observed.iter().any(|(_, original)| {
                            original["operation_id"].as_str().is_some()
                                && original["operation_id"] == op["original_operation_id"]
                                && original["action"] == "apply"
                                && operation_targets_proposal(&proposal, original)
                        }))
            })
            .collect();
        let rollback = matching.iter().any(|(_, op)| op["action"] == "rollback");
        let application = if rollback {
            if matching
                .iter()
                .any(|(_, op)| op["action"] == "rollback" && op["status"] == "ROLLED_BACK")
            {
                "ROLLED_BACK"
            } else {
                "ROLLBACK_UNRESOLVED"
            }
        } else if matching
            .iter()
            .any(|(_, op)| operation_applies_to_proposal(&proposal, op))
        {
            "APPLIED"
        } else if matching.iter().any(|(_, op)| op["status"] == "PARTIAL") {
            "PARTIAL"
        } else if matching.iter().any(|(_, op)| op["status"] == "PREPARED") {
            "PREPARED"
        } else {
            "UNOBSERVED"
        };
        let evaluations: Vec<_> = records.iter().filter(|v| v["kind"] == "groundline-learning-evaluation-record" && v["proposal_sha256"] == hash)
            .map(|v| Ok(json!({"evaluation_sha256":content_sha256(v)?,"status":v["result"]["status"],
                "comparison_reasons":v["result"]["comparison_reasons"],
                "analysis_attempt_resources_complete":v["result"]["analysis_attempt_resources"]["complete"]})))
            .collect::<Result<_, ContractError>>()?;
        let pending = application == "APPLIED"
            && (evaluations.is_empty()
                || evaluations.iter().all(|v| v["status"] == "INCONCLUSIVE"));
        let head = decision_head(records, &hash)?;
        let decision = head.as_ref().map(|head| {
            let record = records.iter().find(|v| content_sha256(v).ok().as_ref() == Some(head)).ok_or_else(|| error("decision_history_conflict"))?;
            Ok::<_, ContractError>(json!({"decision_sha256":head,"decision":record["decision"],
                "reason_sha256":content_sha256(&record["reason"])?,"evidence_refs":record["evidence_refs"],
                "evaluation_sha256":record["evaluation_sha256"],"recorded_at_utc":record["recorded_at_utc"]}))
        }).transpose()?;
        let mut analyses = vec![proposal.analysis.clone()];
        for record in records {
            let failed = record["kind"] == "groundline-learning-proposal"
                && record["status"] == "failed"
                && record["evidence_refs"] == candidate["evidence_refs"]
                && record["scope_sha256"] == proposal.scope_sha256
                && record["basis_revision"] == proposal.basis_revision
                && record["source_revision"] == candidate["source_revision"];
            let duplicate = record["kind"] == "groundline-learning-duplicate-attempt"
                && record["duplicate_of_sha256"] == hash;
            if failed || duplicate {
                analyses.push(
                    serde_json::from_value::<Option<AnalysisResources>>(record["analysis"].clone())
                        .map_err(|_| error("invalid_analysis_resources"))?,
                );
            }
        }
        candidates.push(json!({"proposal_id":proposal.proposal_id,"proposal_sha256":hash,"plan_sha256":proposal.plan_sha256,
            "application":{"state":application,"operation_sha256s":matching.iter().map(|(hash,_)| hash).collect::<Vec<_>>(),
                "unmatched_operation_count":observed.len()-matching.len()},
            "evaluation_pending":pending,"evaluations":evaluations,"declared_decision":decision,
            "analysis_attempt_resources":summarize_analysis(&analyses,&[])?,
            "native_activation":"UNVERIFIED","causal_effect_verified":false,"efficiency_improvement_verified":false}));
    }
    candidates.sort_by(|a, b| a["proposal_id"].as_str().cmp(&b["proposal_id"].as_str()));
    Ok(candidates)
}

fn unknown_analysis() -> Value {
    json!({"complete":false, "owned_total_tokens":null, "wall_duration_ms":null,
        "effective_selection_missing":true})
}

/// Account related successful, failed, and duplicate analysis attempts by owned
/// response, excluding identical responses already included in a delivery.
pub fn summarize_analysis(
    analyses: &[Option<AnalysisResources>],
    receipts: &[Value],
) -> Result<Value, ContractError> {
    delivery::validate_receipt_collection(receipts)?;
    let delivery_rows: std::collections::BTreeMap<&str, &Value> = receipts
        .iter()
        .flat_map(|v| v["resources"]["entries"].as_array().into_iter().flatten())
        .filter_map(|row| row["response_hash"].as_str().map(|hash| (hash, row)))
        .collect();
    let mut rows = std::collections::BTreeMap::<String, Value>::new();
    let mut unobserved = 0_usize;
    let mut incomplete = 0_usize;
    for analysis in analyses {
        let Some(analysis) = analysis else {
            unobserved += 1;
            continue;
        };
        analysis.validate()?;
        incomplete += usize::from(!analysis.resources.complete);
        for row in &analysis.resources.entries {
            let value = serde_json::to_value(row).map_err(|_| error("serialization_failed"))?;
            if delivery_rows
                .get(row.response_hash.as_str())
                .is_some_and(|existing| **existing != value)
                || rows
                    .get(&row.response_hash)
                    .is_some_and(|existing| *existing != value)
            {
                return Err(error("analysis_response_ownership_overlap"));
            }
            rows.insert(row.response_hash.clone(), value);
        }
    }
    let mut additional = 0_u64;
    let mut shared = 0_usize;
    let mut missing = 0_usize;
    let mut measured = 0_usize;
    let mut pairs = std::collections::BTreeMap::<(String, Option<(String, String)>), usize>::new();
    for (hash, row) in &rows {
        let effective = row["effective"].as_object().map(|o| {
            (
                o["model"].as_str().unwrap_or("").to_owned(),
                o["effort"].as_str().unwrap_or("").to_owned(),
            )
        });
        *pairs
            .entry((row["owner"].as_str().unwrap_or("").into(), effective))
            .or_default() += 1;
        if delivery_rows.contains_key(hash.as_str()) {
            shared += 1;
        } else if let Some(total) = row["total_tokens"].as_u64() {
            additional = additional
                .checked_add(total)
                .ok_or_else(|| error("analysis_token_overflow"))?;
            measured += 1;
        } else {
            missing += 1;
        }
    }
    let complete = unobserved == 0 && incomplete == 0 && missing == 0;
    Ok(json!({"attempt_count":analyses.len(),"complete":complete,
        "unobserved_analysis_count":unobserved,"incomplete_analysis_count":incomplete,
        "owned_response_count":rows.len(),"already_counted_delivery_response_count":shared,
        "additional_response_count":rows.len()-shared,
        "additional_owned_total_tokens":complete.then_some(additional),
        "additional_known_total_tokens":(measured>0).then_some(additional),
        "additional_missing_total_count":missing,
        "by_owner_and_effective_selection":pairs.into_iter().map(|((owner, pair), count)|
            json!({"owner":owner,"effective":pair.map(|(model, effort)|json!({"model":model,"effort":effort})),"response_count":count})).collect::<Vec<_>>()
    }))
}

fn group_receipts(
    refs: &[String],
    links: &[Value],
    receipts: &[(String, Value)],
) -> Result<(Vec<OutcomeLink>, Vec<Value>), ContractError> {
    let mut selected_links = Vec::new();
    let mut selected_receipts = Vec::new();
    let mut units = BTreeSet::new();
    for reference in refs {
        let value = links
            .iter()
            .find(|v| content_sha256(v).ok().as_ref() == Some(reference))
            .ok_or_else(|| error("evaluation_link_missing"))?;
        let link: OutcomeLink =
            serde_json::from_value(value.clone()).map_err(|_| error("invalid_link"))?;
        link.validate()?;
        let receipt = receipts
            .iter()
            .find(|(hash, _)| hash == &link.receipt_sha256)
            .ok_or_else(|| error("evaluation_receipt_missing"))?;
        let mut input = value.clone();
        input["kind"] = json!("groundline-learning-link-input");
        link_outcome(&input, &receipt.1, &receipt.0)?;
        if !units.insert(link.unit_hash.clone()) {
            return Err(error("duplicate_evaluation_delivery"));
        }
        selected_receipts.push(receipt.1.clone());
        selected_links.push(link);
    }
    Ok((selected_links, selected_receipts))
}

/// Descriptive before/after evaluation only. Missing attribution or incompatible
/// execution conditions stay INCONCLUSIVE; supplied JSON cannot prove activation.
pub fn evaluate(
    input: &Value,
    proposal_value: &Value,
    links: &[Value],
    receipts: &[(String, Value)],
    operation: &Value,
) -> Result<Value, ContractError> {
    let input: EvaluationInput =
        serde_json::from_value(input.clone()).map_err(|_| error("invalid_evaluation"))?;
    let proposal: Proposal =
        serde_json::from_value(proposal_value.clone()).map_err(|_| error("invalid_proposal"))?;
    proposal.validate()?;
    let mut refs = BTreeSet::new();
    if input.kind != "groundline-learning-evaluation-input"
        || input.schema != 1
        || !identifier(&input.proposal_id)
        || !identifier(&input.baseline_revision)
        || !digest(&input.proposal_revision)
        || input.baseline_refs.is_empty()
        || input.followup_refs.is_empty()
        || input.baseline_refs.len() + input.followup_refs.len() > MAX_EVIDENCE_REFS
        || input
            .baseline_refs
            .iter()
            .chain(&input.followup_refs)
            .any(|s| !digest(s) || !refs.insert(s))
    {
        return Err(error("invalid_evaluation"));
    }
    let (baseline_links, baseline) = group_receipts(&input.baseline_refs, links, receipts)?;
    let (followup_links, followup) = group_receipts(&input.followup_refs, links, receipts)?;
    let all: Vec<Value> = baseline.iter().chain(&followup).cloned().collect();
    delivery::validate_receipt_collection(&all)?;
    let before = delivery::summarize_receipts(&baseline)?;
    let after = delivery::summarize_receipts(&followup)?;
    let mut reasons = BTreeSet::new();
    if proposal.kind != "groundline-learning-proposal"
        || proposal.status != "candidate"
        || input.proposal_id != proposal.proposal_id
        || input.proposal_revision != proposal.plan_sha256
        || baseline_links
            .iter()
            .any(|l| l.environment_revision.as_deref() != Some(&input.baseline_revision))
        || followup_links
            .iter()
            .any(|l| l.environment_revision.as_deref() != Some(&input.proposal_revision))
    {
        reasons.insert("revision_mismatch");
    }
    if let Some(target) = &proposal.target {
        for (group, revision) in [
            (&baseline_links, &target.before_sha256),
            (&followup_links, &target.after_sha256),
        ] {
            if group.iter().any(|link| {
                link.skill.as_ref().is_none_or(|skill| {
                    skill.target_id != target.target_id || &skill.revision != revision
                })
            }) {
                reasons.insert("target_revision_missing_or_mismatched");
            }
        }
    }
    let baseline_source = baseline_links[0].source_revision.as_ref();
    let followup_source = followup_links[0].source_revision.as_ref();
    if baseline_source.is_none() || followup_source.is_none() {
        reasons.insert("source_observation_missing");
    } else if baseline_links
        .iter()
        .any(|l| l.source_revision.as_ref() != baseline_source)
        || followup_links
            .iter()
            .any(|l| l.source_revision.as_ref() != followup_source)
        || (baseline_source != followup_source
            && followup_source != proposal.source_revision.as_ref())
    {
        reasons.insert("source_transition_not_declared");
    }
    if proposal.source_revision.is_some() && followup_source != proposal.source_revision.as_ref() {
        reasons.insert("candidate_source_revision_mismatch");
    }
    // The CLI first applies the environment layer's structural operation
    // validator. This function checks semantic binding, never authentication.
    if !operation_applies_to_proposal(&proposal, operation) {
        reasons.insert("operation_not_applied_to_proposal");
    }
    let expected_link = &baseline_links[0];
    let expected_receipt = &baseline[0];
    if expected_link.runtime.is_none() || expected_receipt["effective"].is_null() {
        reasons.insert("execution_observation_missing");
    }
    for (link, receipt) in baseline_links
        .iter()
        .zip(&baseline)
        .chain(followup_links.iter().zip(&followup))
    {
        if ["new_requirement", "direction_change"].contains(&link.correction_kind.as_str()) {
            reasons.insert("requirements_or_direction_changed");
        }
        if link.cohort_sha256 != expected_link.cohort_sha256 || link.phase != expected_link.phase {
            reasons.insert("cohort_or_phase_mismatch");
        }
        let same_runtime = link
            .runtime
            .as_ref()
            .zip(expected_link.runtime.as_ref())
            .is_some_and(|(a, b)| a.family == b.family && a.version == b.version);
        if !same_runtime
            || receipt["effective"].is_null()
            || receipt["effective"]["model"] != expected_receipt["effective"]["model"]
            || receipt["effective"]["effort"] != expected_receipt["effective"]["effort"]
        {
            reasons.insert("execution_observation_mismatch");
        }
        if receipt["verification"]["status"] == "unknown" {
            reasons.insert("outcome_unknown");
        }
        if receipt["resources"]["complete"] != true {
            reasons.insert("delivery_resources_incomplete");
        }
        let root = receipt["resources"]["entries"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e["owner"] == "root")
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if root.is_empty()
            || root.iter().any(|entry| {
                entry["effective"].is_null()
                    || entry["effective"]["model"] != receipt["effective"]["model"]
                    || entry["effective"]["effort"] != receipt["effective"]["effort"]
            })
        {
            reasons.insert("root_response_selection_missing_or_mismatched");
        }
    }
    let analysis = if let Some(analysis) = &proposal.analysis {
        let receipt = analysis.as_receipt();
        let response_rows: std::collections::BTreeMap<&str, &Value> = all
            .iter()
            .flat_map(|v| v["resources"]["entries"].as_array().into_iter().flatten())
            .filter_map(|row| row["response_hash"].as_str().map(|hash| (hash, row)))
            .collect();
        let mut already_counted = 0_usize;
        let mut additional = 0_u64;
        let mut missing = 0_usize;
        for row in &analysis.resources.entries {
            if let Some(existing) = response_rows.get(row.response_hash.as_str()) {
                let observed =
                    serde_json::to_value(row).map_err(|_| error("serialization_failed"))?;
                if **existing != observed {
                    return Err(error("analysis_response_ownership_overlap"));
                }
                already_counted += 1;
            } else if let Some(tokens) = row.total_tokens {
                additional = additional
                    .checked_add(tokens)
                    .ok_or_else(|| error("analysis_token_overflow"))?;
            } else {
                missing += 1;
            }
        }
        if !analysis.resources.complete {
            reasons.insert("analysis_resources_incomplete");
        }
        let summary = delivery::summarize_receipts(&[receipt])?;
        json!({"complete":analysis.resources.complete,
            "summary":summary["overall"]["resources"], "effective_selection_missing":
                analysis.resources.entries.iter().any(|e| e.effective.is_none()),
            "already_counted_delivery_response_count":already_counted,
            "additional_owned_total_tokens":(analysis.resources.complete && missing == 0).then_some(additional),
            "additional_response_count":analysis.resources.entries.len() - already_counted,
            "additional_missing_total_count":missing})
    } else {
        reasons.insert("analysis_resources_unknown");
        unknown_analysis()
    };
    let before_overall = &before["overall"];
    let after_overall = &after["overall"];
    let before_count = baseline.len() as u64;
    let after_count = followup.len() as u64;
    let before_assistant_errors = baseline_links
        .iter()
        .filter(|l| l.correction_kind == "assistant_error")
        .count() as u64;
    let after_assistant_errors = followup_links
        .iter()
        .filter(|l| l.correction_kind == "assistant_error")
        .count() as u64;
    // Exact ratios prevent unequal sample sizes from looking like a gain.
    let worsened = ["failed_count", "rework_count"].iter().any(|field| {
        after_overall[*field].as_u64().unwrap_or(0) * before_count
            > before_overall[*field].as_u64().unwrap_or(0) * after_count
    }) || after_assistant_errors * before_count
        > before_assistant_errors * after_count;
    let comparison_missing = reasons.iter().any(|r| {
        ![
            "delivery_resources_incomplete",
            "analysis_resources_incomplete",
            "analysis_resources_unknown",
        ]
        .contains(r)
    });
    let status = if !comparison_missing && worsened {
        "REGRESSION_OBSERVED"
    } else if reasons.is_empty()
        && after_overall["failed_count"] == 0
        && after_overall["rework_count"] == 0
        && after_assistant_errors == 0
    {
        "NO_REGRESSION_OBSERVED"
    } else {
        "INCONCLUSIVE"
    };
    let corrections = |links: &[OutcomeLink]| {
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for link in links {
            *counts.entry(link.correction_kind.clone()).or_default() += 1;
        }
        counts
    };
    Ok(json!({
        "kind":"groundline-learning-evaluation", "schema":1,
        "proposal_id":proposal.proposal_id, "status":status,
        "comparison_reasons":reasons, "baseline":before, "followup":after,
        "correction_kinds":{"baseline":corrections(&baseline_links),"followup":corrections(&followup_links)},
        "analysis_resources":analysis, "native_activation":"UNVERIFIED",
        "causal_effect_verified":false, "efficiency_improvement_verified":false,
        "raw_content_emitted":false, "private_paths_emitted":false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u64) -> String {
        format!("{n:064x}")
    }
    fn receipt(n: u64) -> Value {
        let selection =
            json!({"model":"gpt-6.1-sol", "effort":"high", "evidence_sha256":h(n + 100)});
        json!({"kind":"groundline-delivery-receipt", "schema":1,
            "unit_hash":h(n), "cohort_sha256":h(90), "phase":"implementation",
            "completed_at_utc":"2026-10-01T12:00:00Z", "recommendation":null,
            "requested":selection, "effective":selection,
            "verification":{"status":"verified", "evidence_kind":"runtime_check",
                "evidence_sha256":h(n + 200), "rework":false, "authenticity_verified":false},
            "resources":{"complete":true, "wall_duration_ms":100,
                "entries":[{"owner":"root", "unit_hash":h(n), "response_hash":h(n + 300),
                    "effective":selection, "input_tokens":7, "cached_input_tokens":0,
                    "output_tokens":3, "reasoning_output_tokens":1, "total_tokens":10}]},
            "activation_verified":false, "observed_selection_matches_requested":true})
    }
    fn link(n: u64, revision: &str) -> Value {
        json!({"kind":"groundline-learning-link-input", "schema":1,
            "receipt_sha256":h(n + 400), "unit_hash":h(n), "cohort_sha256":h(90),
            "phase":"implementation", "completed_at_utc":"2026-10-01T12:00:00Z",
            "observed_at_utc":"2026-10-01T11:59:00Z", "environment_revision":revision,
            "skill":{"target_id":"existing-skill","revision":if revision==h(80) {h(82)} else {h(83)}}, "source_revision":h(60),
            "runtime":{"family":"codex_app", "version":"1.2.3", "evidence_sha256":h(60)},
            "correction_kind":"unknown", "correction_evidence_sha256":null})
    }
    fn proposal(refs: Vec<String>) -> Value {
        json!({"kind":"groundline-learning-proposal-input", "schema":1, "proposal_id":"skill-trigger-1",
            "evidence_refs":refs, "scope_sha256":h(70), "basis_revision":h(80),
            "source_revision":null, "plan_sha256":h(81), "status":"candidate", "reason":"Observed trigger friction",
            "target":{"target_id":"existing-skill", "kind":"skill", "change":"reduce",
                "before_sha256":h(82), "after_sha256":h(83), "intended_trigger":"Specific repeated task",
                "hypothesis":"Shorter trigger may help discovery", "expected_result":"Correct task completion",
                "falsification":"More rework", "authority_ref":h(84), "rollback_ref":h(82)}, "analysis":null})
    }
    fn evaluation() -> (Value, Value, Vec<Value>, Vec<(String, Value)>, Value) {
        let links: Vec<Value> = [(1, h(80)), (2, h(81))]
            .iter()
            .map(|(n, revision)| {
                link_outcome(&link(*n, revision), &receipt(*n), &h(*n + 400)).unwrap()
            })
            .collect();
        let p =
            prepare_proposal(&proposal(vec![content_sha256(&links[0]).unwrap()]), &links).unwrap();
        let input = json!({"kind":"groundline-learning-evaluation-input", "schema":1,
            "proposal_id":"skill-trigger-1", "baseline_revision":h(80), "proposal_revision":h(81),
            "baseline_refs":[content_sha256(&links[0]).unwrap()], "followup_refs":[content_sha256(&links[1]).unwrap()]});
        let op = json!({"kind":"groundline-environment-operation", "schema":1,
            "proposal_id":"skill-trigger-1", "basis_revision":h(80), "plan_sha256":h(81),
            "status":"APPLIED", "native_activation":"UNVERIFIED", "action":"apply", "authority_ref":h(84),
            "entries":[{"target_id":"existing-skill", "before_sha256":h(82), "after_sha256":h(83), "status":"APPLIED"}]});
        (
            input,
            p,
            links,
            vec![(h(401), receipt(1)), (h(402), receipt(2))],
            op,
        )
    }

    #[test]
    fn links_exact_receipt_identity_and_rejects_current_revision_retrofit() {
        let input = link(1, &h(80));
        assert!(link_outcome(&input, &receipt(1), &h(999)).is_err());
        let mut input = input;
        input["observed_at_utc"] = json!("2026-10-01T12:01:00Z");
        assert_eq!(
            link_outcome(&input, &receipt(1), &h(401)).unwrap_err().0,
            "learning_observation_after_delivery"
        );
        input["observed_at_utc"] = Value::Null;
        assert!(link_outcome(&input, &receipt(1), &h(401)).is_err());
    }

    #[test]
    fn corrections_require_explicit_evidence_and_unknown_is_preserved() {
        let mut input = link(1, &h(80));
        for correction in [
            "acceptance",
            "assistant_error",
            "new_requirement",
            "direction_change",
        ] {
            input["correction_kind"] = json!(correction);
            assert!(link_outcome(&input, &receipt(1), &h(401)).is_err());
            input["correction_evidence_sha256"] = json!(h(30));
            assert_eq!(
                link_outcome(&input, &receipt(1), &h(401)).unwrap()["correction_kind"],
                correction
            );
            input["correction_evidence_sha256"] = Value::Null;
        }
    }

    #[test]
    fn proposal_dedup_uses_evidence_scope_and_revisions_not_label() {
        let p: Proposal = serde_json::from_value(proposal(vec![h(1), h(2)])).unwrap();
        p.validate().unwrap();
        let mut other = p.clone();
        other.proposal_id = "another-label".into();
        other.plan_sha256 = h(999);
        other.evidence_refs.reverse();
        assert_eq!(p.dedup_sha256().unwrap(), other.dedup_sha256().unwrap());
        other.source_revision = Some(h(3));
        assert_ne!(p.dedup_sha256().unwrap(), other.dedup_sha256().unwrap());
    }

    #[test]
    fn official_source_is_typed_bounded_and_never_a_domain_prefix_match() {
        let mut p: Proposal = serde_json::from_value(proposal(vec![h(1)])).unwrap();
        p.basis_revision = "baseline-2026-10-06".into();
        p.source_revision = Some("official-guidance-1".into());
        p.source = Some(SourceObservation {
            url: "https://developers.openai.com/blog/eval-skills".into(),
            checked_at_utc: "2026-10-01T10:00:00Z".into(),
            content_sha256: h(10),
            model: Some("gpt-6.1-sol".into()),
            runtime: None,
            affected_target_ids: vec!["existing-skill".into()],
        });
        p.validate().unwrap();
        let key = p.dedup_sha256().unwrap();
        p.source.as_mut().unwrap().checked_at_utc = "2026-10-02T10:00:00Z".into();
        assert_eq!(key, p.dedup_sha256().unwrap());
        p.source.as_mut().unwrap().content_sha256 = h(11);
        assert_ne!(key, p.dedup_sha256().unwrap());
        for url in [
            "https://developers.openai.com.evil.example/blog/eval-skills",
            "https://openai.com@evil.example/",
            "http://openai.com/",
            "https://github.com/not-openai/codex",
        ] {
            p.source.as_mut().unwrap().url = url.into();
            assert_eq!(p.validate().unwrap_err().0, "learning_invalid_source");
        }
    }

    #[test]
    fn official_source_accepts_future_public_identity_without_expanding_routing() {
        let mut source = SourceObservation {
            url: "https://developers.openai.com/".into(),
            checked_at_utc: "2026-10-01T10:00:00Z".into(),
            content_sha256: h(10),
            model: Some("gpt-7-sol".into()),
            runtime: None,
            affected_target_ids: vec!["existing-skill".into()],
        };
        source.validate().unwrap();
        assert!(!crate::model::optimization_model("gpt-7-sol"));
        for model in [
            "custom-company-model",
            "sol",
            "unknown",
            &format!("private-{}", h(1)),
        ] {
            source.model = Some(model.into());
            assert_eq!(source.validate().unwrap_err().0, "learning_invalid_source");
        }
    }

    #[test]
    fn snapshot_prepare_preserves_historical_context_and_exact_receipt_scope() {
        let input = json!({"kind":"groundline-learning-capture-input", "schema":1,
            "unit_hash":h(1), "cohort_sha256":h(90), "phase":"implementation", "runtime":null});
        let context = json!({"environment_revision":null,"source_revision":"official-1",
            "target_id":"existing-skill","skill_revision":h(82),"provenance_sha256":h(50)});
        let snapshot = capture_snapshot(&input, &context, &h(51), "2026-10-01T11:00:00Z").unwrap();
        let before = snapshot.clone();
        let prepared = prepare_outcome(&snapshot, &receipt(1), &h(401)).unwrap();
        assert_eq!(snapshot, before);
        assert_eq!(prepared["environment_revision"], Value::Null);
        assert_eq!(prepared["source_revision"], "official-1");
        assert_eq!(prepared["skill"]["revision"], h(82));
        assert_eq!(prepared["correction_kind"], "unknown");
        assert!(prepared.get("resources").is_none());
        assert_eq!(
            prepare_outcome(&snapshot, &receipt(2), &h(402))
                .unwrap_err()
                .0,
            "learning_snapshot_receipt_mismatch"
        );
        let mut changed = snapshot.clone();
        changed["captured_at_utc"] = json!("2026-10-01T12:01:00Z");
        assert_eq!(
            prepare_outcome(&changed, &receipt(1), &h(401))
                .unwrap_err()
                .0,
            "learning_snapshot_after_delivery"
        );
        let mut invalid_input = input.clone();
        invalid_input["source_revision"] = json!("invented-source");
        assert_eq!(
            capture_snapshot(&invalid_input, &context, &h(51), "2026-10-01T11:00:00Z")
                .unwrap_err()
                .0,
            "learning_invalid_capture_input"
        );
        invalid_input = input;
        invalid_input["captured_at_utc"] = json!("2026-10-01T11:00:00Z");
        assert!(
            capture_snapshot(&invalid_input, &context, &h(51), "2026-10-01T11:00:00Z").is_err()
        );
    }

    #[test]
    fn candidate_readout_requires_actual_operation_and_keeps_unknown_evaluation_costs() {
        let (input, p, links, receipts, mut operation) = evaluation();
        let proposal_sha = content_sha256(&p).unwrap();
        operation["operation_id"] = json!("op-skill-trigger-1");
        let result = evaluate(&input, &p, &links, &receipts, &operation).unwrap();
        let record = json!({"kind":"groundline-learning-evaluation-record","schema":1,
            "input":input,"proposal_sha256":proposal_sha,"operation_sha256":h(998),"result":result});
        let mut records = links;
        records.push(p);
        records.push(record);
        let readout = &candidate_readouts(&records, &[]).unwrap()[0];
        assert_eq!(readout["application"]["state"], "UNOBSERVED");
        assert_eq!(readout["evaluations"][0]["status"], "INCONCLUSIVE");
        assert_eq!(readout["analysis_attempt_resources"]["complete"], false);
        assert!(readout["analysis_attempt_resources"]["additional_owned_total_tokens"].is_null());
        operation["status"] = json!("PREPARED");
        let operations = vec![(h(998), operation.clone())];
        assert_eq!(
            candidate_readouts(&records, &operations).unwrap()[0]["application"]["state"],
            "PREPARED"
        );
        operation["status"] = json!("APPLIED");
        let mut operations = vec![(h(998), operation.clone())];
        let readout = &candidate_readouts(&records, &operations).unwrap()[0];
        assert_eq!(readout["application"]["state"], "APPLIED");
        assert_eq!(readout["evaluation_pending"], true);
        let mut rollback = operation;
        rollback["action"] = json!("rollback");
        rollback["operation_id"] = json!("rollback-1");
        rollback["original_operation_id"] = json!("op-skill-trigger-1");
        rollback["status"] = json!("ROLLED_BACK");
        rollback["entries"][0]["before_sha256"] = json!(h(83));
        rollback["entries"][0]["after_sha256"] = json!(h(82));
        operations.push((h(999), rollback));
        assert_eq!(
            candidate_readouts(&records, &operations).unwrap()[0]["application"]["state"],
            "ROLLED_BACK"
        );
        operations.remove(0);
        assert_eq!(
            candidate_readouts(&records, &operations).unwrap()[0]["application"]["state"],
            "UNOBSERVED"
        );
    }

    #[test]
    fn decisions_require_linked_evaluations_and_preserve_single_explicit_history() {
        let (input, p, links, receipts, operation) = evaluation();
        let proposal_sha = content_sha256(&p).unwrap();
        let result = evaluate(&input, &p, &links, &receipts, &operation).unwrap();
        let evaluation = json!({"kind":"groundline-learning-evaluation-record","schema":1,
            "input":input,"proposal_sha256":proposal_sha,"operation_sha256":h(998),"result":result});
        let evaluation_sha = content_sha256(&evaluation).unwrap();
        let mut records = links;
        records.push(p);
        let mut input = json!({"kind":"groundline-learning-decision-input","schema":1,
            "proposal_id":"skill-trigger-1","proposal_sha256":proposal_sha,"decision":"adopt",
            "reason":"Explicit bounded trial","evidence_refs":[proposal_sha],
            "evaluation_sha256":evaluation_sha,"previous_decision_sha256":null});
        assert_eq!(
            prepare_decision(&input, &records, "2026-10-01T12:10:00Z")
                .unwrap_err()
                .0,
            "learning_decision_evidence_unlinked"
        );
        input["decision"] = json!("hold");
        input["evaluation_sha256"] = Value::Null;
        let hold = prepare_decision(&input, &records, "2026-10-01T12:10:00Z").unwrap();
        let hold_sha = content_sha256(&hold).unwrap();
        records.push(hold.clone());
        assert_eq!(
            prepare_decision(&input, &records, "2026-10-01T12:11:00Z").unwrap(),
            hold
        );
        let hold_input = input.clone();
        input["decision"] = json!("reject");
        assert_eq!(
            prepare_decision(&input, &records, "2026-10-01T12:11:00Z")
                .unwrap_err()
                .0,
            "learning_decision_history_conflict"
        );
        records.push(evaluation);
        input["decision"] = json!("adopt");
        input["previous_decision_sha256"] = json!(hold_sha);
        input["evaluation_sha256"] = json!(evaluation_sha);
        input["evidence_refs"] = json!([proposal_sha, evaluation_sha]);
        let adopt = prepare_decision(&input, &records, "2026-10-01T12:12:00Z").unwrap();
        let adopt_sha = content_sha256(&adopt).unwrap();
        records.push(adopt);
        assert_eq!(
            decision_head(&records, &proposal_sha).unwrap(),
            Some(adopt_sha.clone())
        );
        assert_eq!(
            prepare_decision(&hold_input, &records, "2026-10-01T12:13:00Z").unwrap(),
            hold
        );
        let readout = &candidate_readouts(&records, &[]).unwrap()[0];
        assert_eq!(readout["declared_decision"]["decision"], "adopt");
        assert_eq!(readout["declared_decision"]["decision_sha256"], adopt_sha);
        assert_eq!(readout["evaluations"][0]["status"], "INCONCLUSIVE");
        assert_eq!(readout["application"]["state"], "UNOBSERVED");
        assert_eq!(readout["efficiency_improvement_verified"], false);
    }

    #[test]
    fn evaluation_counts_analysis_once_and_keeps_unknown_child_selection() {
        let (input, mut p, links, receipts, op) = evaluation();
        p["analysis"] = json!({"unit_hash":h(1),"resources":receipts[0].1["resources"]});
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "NO_REGRESSION_OBSERVED");
        assert_eq!(
            out["analysis_resources"]["already_counted_delivery_response_count"],
            1
        );
        assert_eq!(
            out["analysis_resources"]["additional_owned_total_tokens"],
            0
        );
        let mut analysis = receipt(9);
        let mut child = analysis["resources"]["entries"][0].clone();
        child["owner"] = json!("child");
        child["unit_hash"] = json!(h(10));
        child["response_hash"] = json!(h(500));
        child["effective"] = Value::Null;
        analysis["resources"]["entries"]
            .as_array_mut()
            .unwrap()
            .push(child);
        p["analysis"] = json!({"unit_hash":h(9),"resources":analysis["resources"]});
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(
            out["analysis_resources"]["additional_owned_total_tokens"],
            20
        );
        assert_eq!(
            out["analysis_resources"]["effective_selection_missing"],
            true
        );
        assert_eq!(
            p["analysis"]["resources"]["entries"][1]["effective"],
            Value::Null
        );
        p["analysis"]["resources"]["entries"][0]["response_hash"] =
            receipts[0].1["resources"]["entries"][0]["response_hash"].clone();
        assert_eq!(
            evaluate(&input, &p, &links, &receipts, &op).unwrap_err().0,
            "learning_analysis_response_ownership_overlap"
        );
    }

    #[test]
    fn missing_resources_requirements_and_partial_operations_stay_inconclusive() {
        let (mut input, mut p, mut links, mut receipts, mut op) = evaluation();
        p["analysis"] = json!({"unit_hash":h(9),"resources":receipt(9)["resources"]});
        op["entries"].as_array_mut().unwrap().push(json!({"target_id":"related-reference", "before_sha256":h(88),"after_sha256":h(88),"status":"UNCHANGED"}));
        assert_eq!(
            evaluate(&input, &p, &links, &receipts, &op).unwrap()["status"],
            "NO_REGRESSION_OBSERVED"
        );
        receipts[1].1["resources"]["complete"] = json!(false);
        receipts[1].1["resources"]["entries"][0]["total_tokens"] = Value::Null;
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert_eq!(out["followup"]["overall"]["resources_incomplete_count"], 1);
        links[1]["correction_kind"] = json!("new_requirement");
        links[1]["correction_evidence_sha256"] = json!(h(999));
        input["followup_refs"] = json!([content_sha256(&links[1]).unwrap()]);
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["correction_kinds"]["followup"]["new_requirement"], 1);
        assert!(
            out["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("requirements_or_direction_changed"))
        );
        op["status"] = json!("PARTIAL");
        assert_eq!(
            evaluate(&input, &p, &links, &receipts, &op).unwrap()["status"],
            "INCONCLUSIVE"
        );
    }

    #[test]
    fn previous_applied_revision_can_be_compared_under_a_new_planning_basis() {
        let (input, mut proposal, links, receipts, mut operation) = evaluation();
        let historical_links = links.clone();
        proposal["basis_revision"] = json!("desired-commonbasis-2");
        operation["basis_revision"] = proposal["basis_revision"].clone();
        proposal["analysis"] = json!({"unit_hash":h(9),"resources":receipt(9)["resources"]});
        assert_ne!(input["baseline_revision"], proposal["basis_revision"]);
        let result = evaluate(&input, &proposal, &links, &receipts, &operation).unwrap();
        assert_eq!(result["status"], "NO_REGRESSION_OBSERVED");
        assert_eq!(links, historical_links);
        assert_eq!(result["native_activation"], "UNVERIFIED");
        operation["basis_revision"] = input["baseline_revision"].clone();
        let result = evaluate(&input, &proposal, &links, &receipts, &operation).unwrap();
        assert_eq!(result["status"], "INCONCLUSIVE");
        assert!(
            result["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("operation_not_applied_to_proposal"))
        );
    }

    #[test]
    fn declared_candidate_source_must_match_followup_even_when_history_is_unchanged() {
        let (input, mut proposal, links, receipts, mut operation) = evaluation();
        let historical_links = links.clone();
        proposal["source_revision"] = json!("new-guidance");
        operation["source_revision"] = proposal["source_revision"].clone();
        proposal["analysis"] = json!({"unit_hash":h(9),"resources":receipt(9)["resources"]});
        assert_eq!(links[0]["source_revision"], links[1]["source_revision"]);
        assert_ne!(links[1]["source_revision"], proposal["source_revision"]);
        let result = evaluate(&input, &proposal, &links, &receipts, &operation).unwrap();
        assert_eq!(result["status"], "INCONCLUSIVE");
        assert!(
            result["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("candidate_source_revision_mismatch"))
        );
        assert_eq!(links, historical_links);
    }

    #[test]
    fn evaluation_requires_exact_historical_target_and_declared_source_transition() {
        let (mut input, mut p, mut links, receipts, mut op) = evaluation();
        p["analysis"] = json!({"unit_hash":h(9),"resources":receipt(9)["resources"]});
        let original = links[1].clone();
        for skill in [
            Value::Null,
            json!({"target_id":"other-skill","revision":h(83)}),
            json!({"target_id":"existing-skill","revision":h(999)}),
        ] {
            links[1]["skill"] = skill;
            input["followup_refs"] = json!([content_sha256(&links[1]).unwrap()]);
            let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
            assert_eq!(out["status"], "INCONCLUSIVE");
            assert!(
                out["comparison_reasons"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("target_revision_missing_or_mismatched"))
            );
        }
        links[1] = original;
        links[1]["source_revision"] = json!("new-source");
        input["followup_refs"] = json!([content_sha256(&links[1]).unwrap()]);
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert!(
            out["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("source_transition_not_declared"))
        );
        p["source_revision"] = json!("new-source");
        op["source_revision"] = json!("new-source");
        assert_eq!(
            evaluate(&input, &p, &links, &receipts, &op).unwrap()["status"],
            "NO_REGRESSION_OBSERVED"
        );
        links[1]["source_revision"] = Value::Null;
        input["followup_refs"] = json!([content_sha256(&links[1]).unwrap()]);
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert!(
            out["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("source_observation_missing"))
        );
    }

    #[test]
    fn assistant_error_rates_affect_quality_even_without_delivery_rework() {
        let (mut input, mut proposal, mut links, receipts, operation) = evaluation();
        proposal["analysis"] = json!({"unit_hash":h(9),"resources":receipt(9)["resources"]});
        links[1]["correction_kind"] = json!("assistant_error");
        links[1]["correction_evidence_sha256"] = json!(h(1000));
        input["followup_refs"] = json!([content_sha256(&links[1]).unwrap()]);
        let result = evaluate(&input, &proposal, &links, &receipts, &operation).unwrap();
        assert_eq!(result["status"], "REGRESSION_OBSERVED");
        assert_eq!(result["followup"]["overall"]["rework_count"], 0);
        assert_eq!(result["correction_kinds"]["followup"]["assistant_error"], 1);
        links[0]["correction_kind"] = json!("assistant_error");
        links[0]["correction_evidence_sha256"] = json!(h(1001));
        input["baseline_refs"] = json!([content_sha256(&links[0]).unwrap()]);
        proposal["evidence_refs"] = input["baseline_refs"].clone();
        let result = evaluate(&input, &proposal, &links, &receipts, &operation).unwrap();
        assert_eq!(result["status"], "INCONCLUSIVE");
        assert_eq!(result["correction_kinds"]["baseline"]["assistant_error"], 1);
        assert_eq!(result["correction_kinds"]["followup"]["assistant_error"], 1);
    }

    #[test]
    fn direct_evaluation_preserves_unknown_analysis_and_failed_rework() {
        let (input, p, links, mut receipts, op) = evaluation();
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert!(out["analysis_resources"]["owned_total_tokens"].is_null());
        receipts[1].1["verification"]["status"] = json!("failed");
        receipts[1].1["verification"]["rework"] = json!(true);
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "REGRESSION_OBSERVED");
        assert_eq!(out["followup"]["overall"]["failed_count"], 1);
        assert_eq!(out["followup"]["overall"]["rework_count"], 1);
        assert_eq!(out["native_activation"], "UNVERIFIED");
        assert_eq!(out["causal_effect_verified"], false);
    }

    #[test]
    fn evaluation_rejects_cross_revision_runtime_cohort_or_activation_claims() {
        let (mut input, p, mut links, receipts, mut op) = evaluation();
        input["baseline_revision"] = json!(h(999));
        assert_eq!(
            evaluate(&input, &p, &links, &receipts, &op).unwrap()["status"],
            "INCONCLUSIVE"
        );
        input["baseline_revision"] = json!(h(80));
        links[1]["runtime"]["version"] = json!("2.0.0");
        input["followup_refs"] = json!([content_sha256(&links[1]).unwrap()]);
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert!(
            out["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("execution_observation_mismatch"))
        );
        op["native_activation"] = json!("VERIFIED");
        let out = evaluate(&input, &p, &links, &receipts, &op).unwrap();
        assert!(
            out["comparison_reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("operation_not_applied_to_proposal"))
        );
    }
}
