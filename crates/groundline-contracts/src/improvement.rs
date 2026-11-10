//! Small guidance changes and contextual follow-up. These declarations never
//! authenticate native execution or establish causal quality/cost effects.
use crate::ContractError;
use crate::learning::{content_sha256, digest};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn error(code: &str) -> ContractError {
    ContractError(format!("improvement_{code}"))
}

fn time(value: &str) -> Result<DateTime<FixedOffset>, ContractError> {
    DateTime::parse_from_rfc3339(value).map_err(|_| error("invalid_time"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Brief {
    pub pattern_id: String,
    pub source_work_sha256: String,
    pub context_sha256: String,
    pub criterion_sha256: String,
    pub interpretation: String,
    pub scope: String,
    pub target_kind: String,
    pub transferability_sha256: Option<String>,
}

impl Brief {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.pattern_id.is_empty()
            || self.pattern_id.len() > 64
            || !self
                .pattern_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || ![
                &self.source_work_sha256,
                &self.context_sha256,
                &self.criterion_sha256,
            ]
            .into_iter()
            .all(|v| digest(v))
            || ![
                "assistant_error",
                "durable_preference",
                "requirement_change",
                "exploration",
                "unknown",
            ]
            .contains(&self.interpretation.as_str())
            || !["project", "common"].contains(&self.scope.as_str())
            || !["agents_block", "skill"].contains(&self.target_kind.as_str())
            || self
                .transferability_sha256
                .as_ref()
                .is_some_and(|v| !digest(v))
            || (self.scope == "common"
                && self.actionable()
                && self.transferability_sha256.is_none())
        {
            return Err(error("invalid_brief"));
        }
        Ok(())
    }

    pub fn actionable(&self) -> bool {
        ["assistant_error", "durable_preference"].contains(&self.interpretation.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Record {
    #[serde(rename = "groundline-improvement-capture")]
    Capture {
        schema: u8,
        brief: Brief,
        target_binding_sha256: String,
        before_sha256: String,
        captured_at_utc: String,
    },
    #[serde(rename = "groundline-improvement-change")]
    Change {
        schema: u8,
        capture_sha256: String,
        after_sha256: String,
        changed_at_utc: String,
    },
    #[serde(rename = "groundline-improvement-observation")]
    Observation {
        schema: u8,
        change_sha256: String,
        followup: Followup,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Followup {
    pub source_work_sha256: String,
    pub evidence_sha256: String,
    pub target_after_sha256: String,
    pub observed_at_utc: String,
    pub exposure: String,
    pub opportunity: String,
    pub behavior: String,
}

impl Followup {
    pub fn validate(&self) -> Result<(), ContractError> {
        if ![
            &self.source_work_sha256,
            &self.evidence_sha256,
            &self.target_after_sha256,
        ]
        .into_iter()
        .all(|v| digest(v))
            || !["observed", "unknown"].contains(&self.exposure.as_str())
            || !["present", "absent", "unknown"].contains(&self.opportunity.as_str())
            || !["met", "violated", "unknown"].contains(&self.behavior.as_str())
            || (self.opportunity != "present" && self.behavior != "unknown")
        {
            return Err(error("invalid_followup"));
        }
        time(&self.observed_at_utc)?;
        Ok(())
    }
}

impl Record {
    pub fn value(&self) -> Result<Value, ContractError> {
        serde_json::to_value(self).map_err(|_| error("serialization_failed"))
    }

    pub fn sha256(&self) -> Result<String, ContractError> {
        content_sha256(&self.value()?)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        match self {
            Self::Capture {
                schema,
                brief,
                target_binding_sha256,
                before_sha256,
                captured_at_utc,
            } => {
                brief.validate()?;
                if *schema != 1
                    || !brief.actionable()
                    || !digest(target_binding_sha256)
                    || !digest(before_sha256)
                {
                    return Err(error("invalid_capture"));
                }
                time(captured_at_utc)?;
            }
            Self::Change {
                schema,
                capture_sha256,
                after_sha256,
                changed_at_utc,
            } => {
                if *schema != 1 || !digest(capture_sha256) || !digest(after_sha256) {
                    return Err(error("invalid_change"));
                }
                time(changed_at_utc)?;
            }
            Self::Observation {
                schema,
                change_sha256,
                followup,
            } => {
                if *schema != 1 || !digest(change_sha256) {
                    return Err(error("invalid_observation"));
                }
                followup.validate()?;
            }
        }
        Ok(())
    }
}

pub fn records(values: &[Value]) -> Result<Vec<Record>, ContractError> {
    values
        .iter()
        .filter(|v| {
            v["kind"]
                .as_str()
                .is_some_and(|k| k.starts_with("groundline-improvement-"))
        })
        .map(|v| serde_json::from_value(v.clone()).map_err(|_| error("invalid_record")))
        .collect()
}

pub fn validate_collection(records: &[Record]) -> Result<(), ContractError> {
    let mut by_hash = BTreeMap::new();
    for record in records {
        record.validate()?;
        if by_hash.insert(record.sha256()?, record).is_some() {
            return Err(error("duplicate_record"));
        }
    }
    let mut changed_captures = BTreeSet::new();
    let mut captures = BTreeSet::new();
    let mut observations = BTreeSet::new();
    let mut evidence_work = BTreeMap::new();
    for record in records {
        match record {
            Record::Capture {
                brief,
                target_binding_sha256,
                ..
            } => {
                if !captures.insert((
                    &brief.source_work_sha256,
                    &brief.pattern_id,
                    target_binding_sha256,
                )) {
                    return Err(error("capture_conflict"));
                }
            }
            Record::Change {
                capture_sha256,
                after_sha256,
                changed_at_utc,
                ..
            } => {
                let Some(Record::Capture {
                    before_sha256,
                    captured_at_utc,
                    ..
                }) = by_hash.get(capture_sha256).copied()
                else {
                    return Err(error("capture_required"));
                };
                if before_sha256 == after_sha256
                    || time(changed_at_utc)? < time(captured_at_utc)?
                    || !changed_captures.insert(capture_sha256)
                {
                    return Err(error("change_conflict"));
                }
            }
            Record::Observation {
                change_sha256,
                followup,
                ..
            } => {
                let Some(Record::Change {
                    capture_sha256,
                    after_sha256,
                    changed_at_utc,
                    ..
                }) = by_hash.get(change_sha256).copied()
                else {
                    return Err(error("change_required"));
                };
                let Some(Record::Capture { brief, .. }) = by_hash.get(capture_sha256).copied()
                else {
                    return Err(error("capture_required"));
                };
                if followup.target_after_sha256 != *after_sha256
                    || followup.source_work_sha256 == brief.source_work_sha256
                    || time(&followup.observed_at_utc)? <= time(changed_at_utc)?
                    || !observations.insert((change_sha256, &followup.source_work_sha256))
                {
                    return Err(error("followup_conflict"));
                }
                if evidence_work
                    .insert(&followup.evidence_sha256, &followup.source_work_sha256)
                    .is_some_and(|work| work != &followup.source_work_sha256)
                {
                    return Err(error("evidence_work_conflict"));
                }
            }
        }
    }
    Ok(())
}

pub fn readout(records: &[Record]) -> Result<Value, ContractError> {
    validate_collection(records)?;
    let mut changes = Vec::new();
    for record in records {
        let Record::Change {
            capture_sha256,
            after_sha256,
            ..
        } = record
        else {
            continue;
        };
        let Some(Record::Capture { brief, .. }) = records.iter().find(|r| {
            matches!(r, Record::Capture { .. }) && r.sha256().is_ok_and(|h| h == *capture_sha256)
        }) else {
            return Err(error("capture_required"));
        };
        let hash = record.sha256()?;
        let followups: Vec<_> = records
            .iter()
            .filter_map(|v| match v {
                Record::Observation {
                    change_sha256,
                    followup,
                    ..
                } if *change_sha256 == hash => Some(followup),
                _ => None,
            })
            .collect();
        let assessed: Vec<_> = followups
            .iter()
            .filter(|v| {
                v.exposure == "observed" && v.opportunity == "present" && v.behavior != "unknown"
            })
            .collect();
        changes.push(json!({"change_sha256":hash,"target_after_sha256":after_sha256,
            "pattern_id":brief.pattern_id,"criterion_sha256":brief.criterion_sha256,
            "followup_count":followups.len(),"assessed_count":assessed.len(),
            "opportunities":{"present":followups.iter().filter(|v| v.opportunity == "present").count(),
                "absent":followups.iter().filter(|v| v.opportunity == "absent").count(),
                "unknown":followups.iter().filter(|v| v.opportunity == "unknown").count()},
            "exposure_unknown_count":followups.iter().filter(|v| v.exposure == "unknown").count(),
            "criterion_met_count":assessed.iter().filter(|v| v.behavior == "met").count(),
            "recurrence_count":assessed.iter().filter(|v| v.behavior == "violated").count(),
            "unassessed_count":followups.len()-assessed.len(),
            "next_action":if assessed.is_empty() {"wait_natural_opportunity"} else {"review_contextual_result"}}));
    }
    Ok(
        json!({"kind":"groundline-improvement-readout","schema":1,"status":"OBSERVATIONAL",
        "changes":changes,"causal_effect_verified":false,"efficiency_improvement_verified":false,
        "native_execution_authenticated":false,"raw_content_emitted":false,"private_paths_emitted":false}),
    )
}
