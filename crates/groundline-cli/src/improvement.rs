//! One small change, native editing, and a later contextual observation.
use crate::improvement_io::{self as io, State};
use chrono::Utc;
use clap::Subcommand;
use groundline_contracts::{ContractError, improvement as contract, learning};
use serde_json::{Value, json};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Capture one existing guidance target after contextual review; never edits it.
    Capture {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        target: PathBuf,
        #[arg(long)]
        owner_project_root: Option<PathBuf>,
        #[arg(long)]
        state: PathBuf,
    },
    /// Record the actual target after native Codex applies and verifies its diff.
    Record {
        #[arg(long)]
        capture: String,
        #[arg(long)]
        target: PathBuf,
        #[arg(long)]
        owner_project_root: Option<PathBuf>,
        #[arg(long)]
        state: PathBuf,
    },
    /// Record one independent natural follow-up; no model/tier comparison required.
    Observe {
        #[arg(long)]
        change: String,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        state: PathBuf,
    },
    /// Read current changes and preserved conversation history without writing state.
    Status {
        #[arg(long)]
        state: PathBuf,
    },
}

fn error(code: &str) -> ContractError {
    ContractError(format!("improvement_{code}"))
}

fn load<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, ContractError> {
    serde_json::from_slice(&io::read_private(path)?).map_err(|_| error("invalid_input"))
}

fn binding(
    brief: &contract::Brief,
    target: &Path,
    root: Option<&Path>,
) -> Result<String, ContractError> {
    let target: PathBuf = io::absolute(target)?.components().collect();
    let root = root
        .map(io::absolute)
        .transpose()?
        .map(|p| p.components().collect::<PathBuf>());
    if brief.target_kind == "agents_block" {
        if !target
            .file_name()
            .is_some_and(|v| v == "AGENTS.md" || v == "AGENTS.override.md")
        {
            return Err(error("invalid_target_kind"));
        }
    } else if target.extension().is_none_or(|v| v != "md") {
        return Err(error("invalid_target_kind"));
    }
    match (brief.scope.as_str(), root.as_deref()) {
        ("project", Some(root)) => {
            let meta = io::directory_handle(root, false)?
                .metadata()
                .map_err(|_| error("invalid_root"))?;
            if !target.starts_with(root)
                || target == root
                || meta.uid() != rustix::process::geteuid().as_raw()
                || meta.mode() & 0o022 != 0
            {
                return Err(error("project_root_mismatch"));
            }
        }
        ("common", None) => {}
        _ => return Err(error("project_root_required")),
    }
    learning::content_sha256(
        &json!({"target":target,"root":root,"scope":brief.scope,"target_kind":brief.target_kind}),
    )
}

fn receipt(
    status: &str,
    record: Option<&contract::Record>,
    changed: bool,
) -> Result<Value, ContractError> {
    Ok(
        json!({"kind":"groundline-improvement-result","schema":1,"status":status,
        "record_sha256":record.map(contract::Record::sha256).transpose()?,
        "mutation_performed":changed,"target_mutation_performed":false,"network_performed":false,
        "native_execution_authenticated":false,"causal_effect_verified":false,
        "efficiency_improvement_verified":false,"raw_content_emitted":false,"private_paths_emitted":false}),
    )
}

fn lookup<'a>(
    records: &'a [contract::Record],
    hash: &str,
) -> Result<&'a contract::Record, ContractError> {
    if !learning::digest(hash) {
        return Err(error("invalid_reference"));
    }
    records
        .iter()
        .find(|v| v.sha256().is_ok_and(|h| h == hash))
        .ok_or_else(|| error("record_required"))
}

pub(crate) fn status(path: &Path) -> Result<Value, ContractError> {
    let state = State::open_for_status(path)?;
    let current = contract::records(&state.records)?;
    let historical: Vec<_> = state
        .records
        .iter()
        .filter(|v| {
            v["kind"]
                .as_str()
                .is_some_and(|k| k.starts_with("groundline-learning-conversation-"))
        })
        .cloned()
        .collect();
    let mut result = contract::readout(&current)?;
    result["historical_record_count"] = json!(historical.len());
    result["conversation_work"] = learning::conversation_work::readouts(&historical)?;
    result["mutation_performed"] = json!(false);
    result["network_performed"] = json!(false);
    Ok(result)
}

pub(crate) fn run(command: Command) -> Result<Value, ContractError> {
    match command {
        Command::Status { state } => status(&state),
        Command::Capture {
            input,
            target,
            owner_project_root,
            state,
        } => {
            let brief: contract::Brief = load(&input)?;
            brief.validate()?;
            if !brief.actionable() {
                return receipt("NO_CHANGE", None, false);
            }
            let target_binding = binding(&brief, &target, owner_project_root.as_deref())?;
            let before_sha256 = io::sha256(&io::read_conversation_target(&target)?);
            let state = State::open(&state, true)?;
            for record in contract::records(&state.records)? {
                if let contract::Record::Capture {
                    brief: previous,
                    target_binding_sha256,
                    before_sha256: previous_before,
                    ..
                } = &record
                    && previous.source_work_sha256 == brief.source_work_sha256
                    && previous.pattern_id == brief.pattern_id
                    && *target_binding_sha256 == target_binding
                {
                    if previous == &brief && previous_before == &before_sha256 {
                        return receipt("ALREADY_RECORDED", Some(&record), false);
                    }
                    return Err(error("capture_conflict"));
                }
            }
            let record = contract::Record::Capture {
                schema: 1,
                brief,
                target_binding_sha256: target_binding,
                before_sha256,
                captured_at_utc: Utc::now().to_rfc3339(),
            };
            if let contract::Record::Capture { before_sha256, .. } = &record
                && io::sha256(&io::read_conversation_target(&target)?) != *before_sha256
            {
                return Err(error("target_changed"));
            }
            let changed = state.save(&record.value()?)?;
            receipt("CAPTURED", Some(&record), changed)
        }
        Command::Record {
            capture,
            target,
            owner_project_root,
            state,
        } => {
            let state = State::open(&state, false)?;
            let records = contract::records(&state.records)?;
            let contract::Record::Capture {
                brief,
                target_binding_sha256,
                before_sha256,
                ..
            } = lookup(&records, &capture)?
            else {
                return Err(error("capture_required"));
            };
            if binding(brief, &target, owner_project_root.as_deref())? != *target_binding_sha256 {
                return Err(error("target_binding_mismatch"));
            }
            let after_sha256 = io::sha256(&io::read_conversation_target(&target)?);
            for record in &records {
                if let contract::Record::Change {
                    capture_sha256,
                    after_sha256: previous,
                    ..
                } = record
                    && *capture_sha256 == capture
                {
                    if *previous == after_sha256 {
                        return receipt("ALREADY_RECORDED", Some(record), false);
                    }
                    return Err(error("change_conflict"));
                }
            }
            if *before_sha256 == after_sha256 {
                return receipt("NO_CHANGE", None, false);
            }
            let record = contract::Record::Change {
                schema: 1,
                capture_sha256: capture,
                after_sha256,
                changed_at_utc: Utc::now().to_rfc3339(),
            };
            if let contract::Record::Change { after_sha256, .. } = &record
                && io::sha256(&io::read_conversation_target(&target)?) != *after_sha256
            {
                return Err(error("target_changed"));
            }
            let changed = state.save(&record.value()?)?;
            receipt("RECORDED", Some(&record), changed)
        }
        Command::Observe {
            change,
            input,
            state,
        } => {
            let followup: contract::Followup = load(&input)?;
            followup.validate()?;
            if chrono::DateTime::parse_from_rfc3339(&followup.observed_at_utc)
                .map_err(|_| error("invalid_time"))?
                > Utc::now()
            {
                return Err(error("future_observation"));
            }
            let state = State::open(&state, false)?;
            let records = contract::records(&state.records)?;
            if !matches!(lookup(&records, &change)?, contract::Record::Change { .. }) {
                return Err(error("change_required"));
            }
            for record in &records {
                if let contract::Record::Observation {
                    change_sha256,
                    followup: previous,
                    ..
                } = record
                    && *change_sha256 == change
                    && previous.source_work_sha256 == followup.source_work_sha256
                {
                    if previous == &followup {
                        return receipt("ALREADY_RECORDED", Some(record), false);
                    }
                    return Err(error("followup_conflict"));
                }
            }
            let record = contract::Record::Observation {
                schema: 1,
                change_sha256: change,
                followup,
            };
            let changed = state.save(&record.value()?)?;
            receipt("RECORDED", Some(&record), changed)
        }
    }
}
