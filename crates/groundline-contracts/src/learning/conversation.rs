//! Bounded conversation declarations for ordinary learning proposals. Native
//! Codex reviews the original assistant action, authored user correction, and
//! owning file. Digests prevent local rebinding; they do not authenticate those
//! observations, establish universal applicability, or authorize a mutation.
use super::{OutcomeLink, Proposal, content_sha256, digest, error, identifier};
use crate::ContractError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConversationScope {
    Common,
    Project { project_sha256: String },
}

impl ConversationScope {
    fn validate(&self) -> Result<(), ContractError> {
        if matches!(self, Self::Project { project_sha256 } if !digest(project_sha256)) {
            return Err(error("invalid_conversation_scope"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionInterpretation {
    AssistantError,
    NewRequirement,
    DirectionChange,
    Iteration,
    Unknown,
}

impl CorrectionInterpretation {
    fn matches_link(&self, link: &OutcomeLink) -> bool {
        let expected = match self {
            Self::AssistantError => "assistant_error",
            Self::NewRequirement => "new_requirement",
            Self::DirectionChange => "direction_change",
            // Iteration is a reviewed conversation interpretation, not a
            // manufactured acceptance/error classification in an outcome link.
            Self::Iteration | Self::Unknown => "unknown",
        };
        link.correction_kind == expected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConversationTarget {
    /// Declared destination. The actual path owner must be checked by Codex;
    /// a target ID and hash are not an authenticated filesystem scope.
    pub scope: ConversationScope,
    pub target_id: String,
    pub kind: String,
    pub before_sha256: String,
    pub after_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransferabilityReview {
    pub project_neutral: bool,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConversationBasis {
    pub pattern_id: String,
    pub observed_scope: ConversationScope,
    pub target: ConversationTarget,
    pub evidence_link_sha256: String,
    pub preceding_assistant_action_sha256: String,
    pub user_correction_sha256: String,
    pub interpretation: CorrectionInterpretation,
    pub project_dependent: bool,
    pub transferability_review: Option<TransferabilityReview>,
}

impl ConversationBasis {
    /// Bind the pattern and declared origin/destination, including the exact
    /// target revisions, instead of accepting an opaque caller-chosen scope.
    pub fn scope_sha256(&self) -> Result<String, ContractError> {
        content_sha256(&json!({"pattern_id":self.pattern_id,
            "observed_scope":self.observed_scope,"target":self.target}))
    }

    pub fn validate_proposal(&self, proposal: &Proposal) -> Result<(), ContractError> {
        self.observed_scope.validate()?;
        self.target.scope.validate()?;
        if !identifier(&self.pattern_id)
            || !identifier(&self.target.target_id)
            || !["skill", "agents_block"].contains(&self.target.kind.as_str())
            || [
                &self.target.before_sha256,
                &self.target.after_sha256,
                &self.evidence_link_sha256,
                &self.preceding_assistant_action_sha256,
                &self.user_correction_sha256,
            ]
            .into_iter()
            .any(|s| !digest(s))
            || self.target.before_sha256 == self.target.after_sha256
            || !proposal.evidence_refs.contains(&self.evidence_link_sha256)
            || proposal.scope_sha256 != self.scope_sha256()?
            || self.transferability_review.as_ref().is_some_and(|r| {
                !digest(&r.evidence_sha256) || !proposal.evidence_refs.contains(&r.evidence_sha256)
            })
        {
            return Err(error("invalid_conversation_basis"));
        }
        match &self.target.scope {
            ConversationScope::Common => {
                if self.project_dependent
                    || !self
                        .transferability_review
                        .as_ref()
                        .is_some_and(|r| r.project_neutral)
                {
                    return Err(error("conversation_common_scope_unreviewed"));
                }
            }
            ConversationScope::Project { .. } => {
                if self.observed_scope != self.target.scope {
                    return Err(error("conversation_project_scope_mismatch"));
                }
            }
        }
        if proposal.status == "candidate"
            && self.interpretation != CorrectionInterpretation::AssistantError
        {
            return Err(error("conversation_error_basis_required"));
        }
        if proposal.target.as_ref().is_some_and(|target| {
            target.target_id != self.target.target_id
                || target.kind != self.target.kind
                || target.before_sha256 != self.target.before_sha256
                || target.after_sha256 != self.target.after_sha256
        }) {
            return Err(error("conversation_target_mismatch"));
        }
        Ok(())
    }

    pub fn validate_origin(&self, links: &[Value]) -> Result<(), ContractError> {
        let value = links
            .iter()
            .find(|v| content_sha256(v).ok().as_ref() == Some(&self.evidence_link_sha256))
            .ok_or_else(|| error("conversation_origin_link_missing"))?;
        let link: OutcomeLink =
            serde_json::from_value(value.clone()).map_err(|_| error("invalid_link"))?;
        link.validate()?;
        if link.kind != "groundline-learning-link"
            || !self.interpretation.matches_link(&link)
            || (self.interpretation != CorrectionInterpretation::Unknown
                && self.interpretation != CorrectionInterpretation::Iteration
                && link.correction_evidence_sha256.as_ref() != Some(&self.user_correction_sha256))
        {
            return Err(error("conversation_origin_evidence_mismatch"));
        }
        if let Some(review) = &self.transferability_review {
            let value = links
                .iter()
                .find(|v| content_sha256(v).ok().as_ref() == Some(&review.evidence_sha256))
                .ok_or_else(|| error("conversation_transferability_link_missing"))?;
            let link: OutcomeLink =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_link"))?;
            link.validate()?;
            if link.kind != "groundline-learning-link" {
                return Err(error("conversation_transferability_link_missing"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConversationObservation {
    pub link_sha256: String,
    pub pattern_id: String,
    pub scope_sha256: String,
    pub opportunity: bool,
    /// None is unassessed. No opportunity cannot be recorded as a success.
    pub recurred: Option<bool>,
    /// Exact selected delivery receipt digest, including negative observations.
    pub evidence_sha256: String,
    /// A recurrence must point to the actual own-user correction in the link.
    pub recurrence_evidence_sha256: Option<String>,
}

pub fn validate_observations(
    basis: Option<&ConversationBasis>,
    observations: Option<&[ConversationObservation]>,
    selected_refs: &[&String],
    selected_links: &[&OutcomeLink],
) -> Result<(), ContractError> {
    let Some(observations) = observations else {
        return Ok(());
    };
    let basis = basis.ok_or_else(|| error("conversation_basis_missing"))?;
    if observations.len() > super::MAX_EVIDENCE_REFS {
        return Err(error("invalid_conversation_observation"));
    }
    let scope = basis.scope_sha256()?;
    let links: BTreeMap<&str, &OutcomeLink> = selected_refs
        .iter()
        .zip(selected_links)
        .map(|(reference, link)| (reference.as_str(), *link))
        .collect();
    let mut seen = BTreeSet::new();
    for observation in observations {
        if !digest(&observation.link_sha256)
            || !digest(&observation.evidence_sha256)
            || observation.pattern_id != basis.pattern_id
            || observation.scope_sha256 != scope
            || !seen.insert(&observation.link_sha256)
            || (!observation.opportunity && observation.recurred.is_some())
            || observation
                .recurrence_evidence_sha256
                .as_ref()
                .is_some_and(|s| !digest(s))
            || (observation.recurred == Some(true))
                != observation.recurrence_evidence_sha256.is_some()
        {
            return Err(error("invalid_conversation_observation"));
        }
        let link = links
            .get(observation.link_sha256.as_str())
            .ok_or_else(|| error("conversation_observation_link_missing"))?;
        if observation.evidence_sha256 != link.receipt_sha256 {
            return Err(error("conversation_observation_receipt_mismatch"));
        }
        if observation.recurred == Some(true)
            && (link.correction_kind != "assistant_error"
                || link.correction_evidence_sha256 != observation.recurrence_evidence_sha256)
        {
            return Err(error("conversation_recurrence_evidence_mismatch"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivationKind {
    InstructionRead,
    SkillInvoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConversationApplication {
    pub operation_content_sha256: String,
    pub applied_at_utc: String,
    pub activated_at_utc: String,
    pub target_sha256: String,
    pub activation_kind: ActivationKind,
    /// CLI callers must read the actual private evidence bytes and verify this
    /// digest. The contract still cannot authenticate native activation.
    pub activation_evidence_sha256: String,
}

pub struct ApplicationAssessment {
    pub application_observation: &'static str,
    pub activation_observation: &'static str,
    pub comparison_reasons: BTreeSet<&'static str>,
}

/// Validate local operation/target binding and retain chronology gaps as
/// inconclusive. Applying bytes and reading/invoking them are separate events.
pub fn assess_application(
    proposal: &Proposal,
    application: Option<&ConversationApplication>,
    operation: &Value,
    baseline_links: &[OutcomeLink],
    followup_links: &[OutcomeLink],
) -> Result<ApplicationAssessment, ContractError> {
    let mut assessment = ApplicationAssessment {
        application_observation: "missing",
        activation_observation: "missing",
        comparison_reasons: BTreeSet::new(),
    };
    let Some(application) = application else {
        assessment
            .comparison_reasons
            .insert("conversation_application_missing");
        assessment
            .comparison_reasons
            .insert("conversation_activation_missing");
        return Ok(assessment);
    };
    if proposal.conversation.is_none() {
        return Err(error("conversation_basis_missing"));
    }
    if [
        &application.operation_content_sha256,
        &application.target_sha256,
        &application.activation_evidence_sha256,
    ]
    .into_iter()
    .any(|s| !digest(s))
        || application.operation_content_sha256 != content_sha256(operation)?
        || proposal.target.as_ref().is_none_or(|target| {
            target.after_sha256 != application.target_sha256
                || !matches!(
                    (target.kind.as_str(), &application.activation_kind),
                    ("skill", ActivationKind::SkillInvoked)
                        | ("agents_block", ActivationKind::InstructionRead)
                )
        })
    {
        return Err(error("invalid_conversation_application"));
    }
    let applied = DateTime::parse_from_rfc3339(&application.applied_at_utc)
        .map_err(|_| error("invalid_conversation_application_timestamp"))?;
    let activated = DateTime::parse_from_rfc3339(&application.activated_at_utc)
        .map_err(|_| error("invalid_conversation_application_timestamp"))?;
    if applied > activated
        || applied.with_timezone(&Utc) > Utc::now()
        || activated.with_timezone(&Utc) > Utc::now()
    {
        return Err(error("invalid_conversation_application_timestamp"));
    }
    assessment.application_observation = "observed";
    assessment.activation_observation = "observed";
    if baseline_links.iter().any(|link| {
        DateTime::parse_from_rfc3339(&link.completed_at_utc)
            .is_ok_and(|completed| completed > applied)
    }) {
        assessment
            .comparison_reasons
            .insert("baseline_after_conversation_application");
    }
    for link in followup_links {
        match link
            .observed_at_utc
            .as_deref()
            .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
        {
            None => {
                assessment
                    .comparison_reasons
                    .insert("followup_activation_observation_missing");
            }
            Some(observed) if observed < activated => {
                assessment
                    .comparison_reasons
                    .insert("followup_before_conversation_activation");
            }
            Some(_) => {}
        }
    }
    Ok(assessment)
}

#[derive(Default)]
struct Counts {
    samples: u64,
    numerator: u64,
    denominator: u64,
    no_opportunity: u64,
    unassessed: u64,
    missing: u64,
}

impl Counts {
    fn for_refs(refs: &[String], observations: Option<&[ConversationObservation]>) -> Self {
        let mut out = Self {
            samples: refs.len() as u64,
            ..Self::default()
        };
        for reference in refs {
            let observed = observations
                .into_iter()
                .flatten()
                .find(|o| &o.link_sha256 == reference);
            match observed {
                None => out.missing += 1,
                Some(o) if !o.opportunity => out.no_opportunity += 1,
                Some(o) => match o.recurred {
                    None => out.unassessed += 1,
                    Some(recurred) => {
                        out.denominator += 1;
                        out.numerator += u64::from(recurred);
                    }
                },
            }
        }
        out
    }

    fn value(&self) -> Value {
        json!({"sample_count":self.samples,"numerator":self.numerator,
            "denominator":self.denominator,"no_opportunity_count":self.no_opportunity,
            "unassessed_opportunity_count":self.unassessed,"missing_observation_count":self.missing,
            "unobserved_count":self.no_opportunity+self.unassessed+self.missing,
            "recurrence_rate":(self.denominator>0).then(||self.numerator as f64/self.denominator as f64)})
    }
}

/// Descriptive rates only; explicit opportunity is the denominator. Missing or
/// unassessed rows keep comparison inconclusive. Non-opportunities are reported
/// separately and never improve a rate.
pub fn compare_recurrence(
    basis: &ConversationBasis,
    observations: Option<&[ConversationObservation]>,
    baseline_refs: &[String],
    followup_refs: &[String],
    execution_comparable: bool,
    application: &ApplicationAssessment,
) -> Value {
    let before = Counts::for_refs(baseline_refs, observations);
    let after = Counts::for_refs(followup_refs, observations);
    let mut reasons = application.comparison_reasons.clone();
    if !execution_comparable {
        reasons.insert("execution_comparison_inconclusive");
    }
    if before.denominator == 0 || after.denominator == 0 {
        reasons.insert("assessed_opportunity_missing");
    }
    if before.missing + after.missing > 0 {
        reasons.insert("conversation_observation_missing");
    }
    if before.unassessed + after.unassessed > 0 {
        reasons.insert("conversation_opportunity_unassessed");
    }
    let comparison = if !reasons.is_empty() {
        "inconclusive"
    } else {
        match (after.numerator * before.denominator).cmp(&(before.numerator * after.denominator)) {
            std::cmp::Ordering::Less => "reduced",
            std::cmp::Ordering::Greater => "increased",
            std::cmp::Ordering::Equal => "unchanged",
        }
    };
    json!({"pattern_id":basis.pattern_id,"scope_sha256":basis.scope_sha256().ok(),
        "application_observation":application.application_observation,
        "activation_observation":application.activation_observation,
        "comparison":comparison,"comparison_reasons":reasons,
        "baseline":before.value(),"followup":after.value(),
        "basis":"declared_opportunities_and_linked_corrections",
        "semantic_authenticity_verified":false,"causal_effect_verified":false,
        "efficiency_improvement_verified":false})
}
