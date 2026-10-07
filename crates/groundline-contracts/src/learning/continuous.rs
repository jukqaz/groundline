//! Scoped natural-work records. Hook correlation is never an outcome or proof
//! of response ownership; direct receipts remain the resource authority.
use super::{RuntimeObservation, digest, error, identifier};
use crate::{ContractError, delivery};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path};

pub fn bytes_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn absolute_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.chars().any(char::is_control)
        && Path::new(value).is_absolute()
        && !Path::new(value)
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
}

pub fn timestamp(value: &str) -> Result<DateTime<chrono::FixedOffset>, ContractError> {
    let at = DateTime::parse_from_rfc3339(value).map_err(|_| error("invalid_task_timestamp"))?;
    if at.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5) {
        return Err(error("invalid_task_timestamp"));
    }
    Ok(at)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningProfile {
    pub kind: String,
    pub schema: u8,
    pub enabled: bool,
    pub environment_state: String,
    pub learning_state: String,
    pub deliveries: String,
    pub target_ids: Vec<String>,
    pub device_id: String,
    pub core_executable: String,
    pub core_sha256: String,
}

impl LearningProfile {
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut targets = BTreeSet::new();
        if self.kind != "groundline-learning-profile"
            || self.schema != 1
            || [
                &self.environment_state,
                &self.learning_state,
                &self.deliveries,
                &self.core_executable,
            ]
            .into_iter()
            .any(|p| !absolute_path(p))
            || !digest(&self.core_sha256)
            || !identifier(&self.device_id)
            || self.target_ids.is_empty()
            || self.target_ids.len() > super::MAX_EVIDENCE_REFS
            || self
                .target_ids
                .iter()
                .any(|t| !identifier(t) || !targets.insert(t))
            || self.environment_state == self.learning_state
            || self.deliveries == self.learning_state
            || self.deliveries == self.environment_state
        {
            return Err(error("invalid_profile"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryTarget {
    pub target_id: String,
    pub snapshot_code: String,
    pub skill_revision: Option<String>,
    pub environment_revision: Option<String>,
    pub source_revision: Option<String>,
    pub baseline_sha256: Option<String>,
    pub observation_state_sha256: Option<String>,
    pub observed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    pub kind: String,
    pub schema: u8,
    pub boundary_id: String,
    pub event: String,
    pub observed_at_utc: String,
    pub session_hash: Option<String>,
    pub turn_hash: Option<String>,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    pub payload_status: String,
    pub native_evidence_sha256: Option<String>,
    pub missing_fields: Vec<String>,
    pub source_verified: bool,
    pub device_id: Option<String>,
    pub targets: Vec<BoundaryTarget>,
    pub native_activation: String,
}

impl Boundary {
    pub fn validate(&self) -> Result<(), ContractError> {
        timestamp(&self.observed_at_utc)?;
        let mut targets = BTreeSet::new();
        if self.kind != "groundline-learning-boundary"
            || self.schema != 1
            || !identifier(&self.boundary_id)
            || ![
                "SessionStart",
                "UserPromptSubmit",
                "Stop",
                "PostCompact",
                "SessionEnd",
            ]
            .contains(&self.event.as_str())
            || ![
                "complete",
                "missing_fields",
                "invalid",
                "oversize",
                "unavailable",
            ]
            .contains(&self.payload_status.as_str())
            || self
                .native_evidence_sha256
                .as_ref()
                .is_some_and(|v| !digest(v))
            || self.missing_fields.len() > 64
            || self.missing_fields.iter().any(|v| !identifier(v))
            || self.source_verified
            || self.native_activation != "UNVERIFIED"
            || [&self.session_hash, &self.turn_hash]
                .into_iter()
                .flatten()
                .any(|h| !digest(h))
            || self.model.as_ref().is_some_and(|v| !identifier(v))
            || self
                .permission_mode
                .as_ref()
                .is_some_and(|v| !identifier(v))
            || self.device_id.as_ref().is_some_and(|v| !identifier(v))
            || self.targets.len() > super::MAX_EVIDENCE_REFS
        {
            return Err(error("invalid_boundary"));
        }
        for target in &self.targets {
            timestamp(&target.observed_at_utc)?;
            if !identifier(&target.target_id)
                || !identifier(&target.snapshot_code)
                || !targets.insert(&target.target_id)
                || [
                    &target.skill_revision,
                    &target.baseline_sha256,
                    &target.observation_state_sha256,
                ]
                .into_iter()
                .flatten()
                .any(|h| !digest(h))
                || target.environment_revision.is_some()
                || target
                    .source_revision
                    .as_ref()
                    .is_some_and(|v| !identifier(v))
            {
                return Err(error("invalid_boundary_target"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletionCriterion {
    pub sha256: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskScope {
    pub unit_hash: String,
    pub cohort_sha256: String,
    pub phase: String,
    pub task_category: String,
    pub criterion: CompletionCriterion,
    pub target_id: String,
    pub runtime: Option<RuntimeObservation>,
}

impl TaskScope {
    pub fn validate(&self) -> Result<(), ContractError> {
        if !digest(&self.unit_hash)
            || !digest(&self.cohort_sha256)
            || !delivery::PHASES.contains(&self.phase.as_str())
            || !identifier(&self.task_category)
            || !identifier(&self.target_id)
            || !digest(&self.criterion.sha256)
            || !identifier(&self.criterion.version)
        {
            return Err(error("invalid_task_scope"));
        }
        super::CaptureInput {
            kind: "groundline-learning-capture-input".into(),
            schema: 1,
            unit_hash: self.unit_hash.clone(),
            cohort_sha256: self.cohort_sha256.clone(),
            phase: self.phase.clone(),
            runtime: self.runtime.clone(),
        }
        .validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskStartInput {
    pub kind: String,
    pub schema: u8,
    pub scope: TaskScope,
    pub boundary_id: String,
    pub criterion_change_kind: String,
    pub criterion_change_evidence_sha256: Option<String>,
}

impl TaskStartInput {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.scope.validate()?;
        if self.kind != "groundline-learning-task-start-input"
            || self.schema != 1
            || !identifier(&self.boundary_id)
            || !["unknown", "new_requirement", "direction_change"]
                .contains(&self.criterion_change_kind.as_str())
            || self
                .criterion_change_evidence_sha256
                .as_ref()
                .is_some_and(|h| !digest(h))
            || (self.criterion_change_kind != "unknown"
                && self.criterion_change_evidence_sha256.is_none())
        {
            return Err(error("invalid_task_start"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeReference {
    pub session_hash: Option<String>,
    pub turn_hash: Option<String>,
    pub artifact_sha256: Option<String>,
    /// Only independent native artifact matching sets this; hook JSON cannot.
    pub boundary_matched: bool,
}

impl NativeReference {
    pub fn validate(&self) -> Result<(), ContractError> {
        if [&self.session_hash, &self.turn_hash, &self.artifact_sha256]
            .into_iter()
            .flatten()
            .any(|v| !digest(v))
            || (self.boundary_matched
                && [&self.session_hash, &self.turn_hash, &self.artifact_sha256]
                    .into_iter()
                    .any(|v| v.is_none()))
        {
            return Err(error("invalid_native_reference"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskStartRecord {
    pub kind: String,
    pub schema: u8,
    pub input_sha256: String,
    pub scope: TaskScope,
    pub boundary_sha256: String,
    pub snapshot_sha256: String,
    pub native: NativeReference,
    pub boundary_environment_matched: bool,
    pub criterion_change_kind: String,
    pub criterion_change_evidence_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskFinalizeInput {
    pub kind: String,
    pub schema: u8,
    pub task_sha256: String,
    pub boundary_id: String,
    pub correction_kind: String,
    pub correction_evidence_sha256: Option<String>,
}

impl TaskFinalizeInput {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.kind != "groundline-learning-task-finalize-input"
            || self.schema != 1
            || !digest(&self.task_sha256)
            || !identifier(&self.boundary_id)
            || ![
                "unknown",
                "acceptance",
                "assistant_error",
                "new_requirement",
                "direction_change",
            ]
            .contains(&self.correction_kind.as_str())
            || self
                .correction_evidence_sha256
                .as_ref()
                .is_some_and(|h| !digest(h))
            || (self.correction_kind != "unknown" && self.correction_evidence_sha256.is_none())
        {
            return Err(error("invalid_task_finalize"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskOutcome {
    pub kind: String,
    pub schema: u8,
    pub input_sha256: String,
    pub task_sha256: String,
    pub boundary_sha256: String,
    pub native: NativeReference,
    pub receipt_sha256: String,
    pub link_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizeRequest {
    pub kind: String,
    pub schema: u8,
    pub input_sha256: String,
    pub input: TaskFinalizeInput,
    pub boundary_sha256: String,
    pub native: NativeReference,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialAuthorization {
    pub kind: String,
    pub schema: u8,
    pub proposal_sha256: String,
    pub plan_sha256: String,
    pub scope_sha256: String,
    pub rollback_sha256: String,
    pub authority_ref: String,
    pub evidence_sha256: String,
}

impl TrialAuthorization {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.kind != "groundline-learning-trial-authorization"
            || self.schema != 1
            || [
                &self.proposal_sha256,
                &self.plan_sha256,
                &self.scope_sha256,
                &self.rollback_sha256,
                &self.evidence_sha256,
            ]
            .into_iter()
            .any(|h| !digest(h))
            || !identifier(&self.authority_ref)
        {
            return Err(error("invalid_trial_authorization"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingRecord {
    pub kind: String,
    pub schema: u8,
    pub input_sha256: String,
    pub context_sha256: String,
    pub status: String,
    pub evidence_refs: Vec<String>,
    pub reasons: Vec<String>,
}

impl ProcessingRecord {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.kind != "groundline-learning-processing"
            || self.schema != 1
            || !digest(&self.input_sha256)
            || !digest(&self.context_sha256)
            || ![
                "PENDING",
                "INCONCLUSIVE",
                "EVALUATED",
                "BLOCKED",
                "OBSERVED",
            ]
            .contains(&self.status.as_str())
            || self.evidence_refs.len() > super::MAX_EVIDENCE_REFS
            || self.evidence_refs.iter().any(|h| !digest(h))
            || self.reasons.len() > super::MAX_EVIDENCE_REFS
            || self.reasons.iter().any(|r| !identifier(r))
        {
            return Err(error("invalid_processing_record"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn boundary_keeps_missing_native_fields_unknown_and_rejects_asserted_activation() {
        let mut boundary: Boundary = serde_json::from_value(json!({
            "kind":"groundline-learning-boundary","schema":1,"boundary_id":"event-1","event":"Stop",
            "observed_at_utc":"2026-10-01T00:00:00Z","session_hash":null,"turn_hash":null,
            "model":null,"permission_mode":null,"payload_status":"missing_fields","native_evidence_sha256":null,
            "missing_fields":["session_id","turn_id"],"source_verified":false,"device_id":null,
            "targets":[],"native_activation":"UNVERIFIED"
        })).unwrap();
        boundary.validate().unwrap();
        boundary.source_verified = true;
        assert!(boundary.validate().is_err());
        boundary.source_verified = false;
        boundary.native_activation = "VERIFIED".into();
        assert!(boundary.validate().is_err());
        boundary.native_activation = "UNVERIFIED".into();
        boundary.event = "UploadACK".into();
        assert!(boundary.validate().is_err());
        boundary.event = "UserPromptSubmit".into();
        boundary.targets.push(BoundaryTarget {
            target_id: "workflow".into(),
            snapshot_code: "observed".into(),
            skill_revision: None,
            environment_revision: None,
            source_revision: None,
            baseline_sha256: None,
            observation_state_sha256: None,
            observed_at_utc: boundary.observed_at_utc.clone(),
        });
        boundary.validate().unwrap();
        boundary.targets[0].observation_state_sha256 = Some("invalid".into());
        assert!(boundary.validate().is_err());
        boundary.targets[0].observation_state_sha256 = Some(bytes_sha256(b"observed-state"));
        boundary.validate().unwrap();
    }

    #[test]
    fn profile_requires_distinct_absolute_storage_and_a_core_digest_pin() {
        let mut profile = LearningProfile {
            kind: "groundline-learning-profile".into(),
            schema: 1,
            enabled: false,
            environment_state: "/owner/environment".into(),
            learning_state: "/owner/learning/records".into(),
            deliveries: "/owner/learning/deliveries".into(),
            target_ids: vec!["workflow".into()],
            device_id: "device-1".into(),
            core_executable: "/owner/bin/groundline".into(),
            core_sha256: bytes_sha256(b"core"),
        };
        profile.validate().unwrap();
        profile.deliveries = profile.learning_state.clone();
        assert!(profile.validate().is_err());
        profile.deliveries = "/owner/../deliveries".into();
        assert!(profile.validate().is_err());
        profile.deliveries = "/owner/learning/deliveries".into();
        profile.core_sha256.clear();
        assert!(profile.validate().is_err());
    }
}
