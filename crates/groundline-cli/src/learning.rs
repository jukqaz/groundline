//! Bounded, write-once owner-private learning records. Native Codex supplies the
//! candidate; this module neither calls a model nor reads current native state.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use clap::Subcommand;
use groundline_contracts::{ContractError, delivery, learning};
use groundline_runtime::local_file::{open_private_directory, private_for_current_user};
use rustix::fs::{AtFlags, Mode, OFlags, linkat, mkdirat, open, openat, unlinkat};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ENTRIES: usize = 1000;
const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Link an existing receipt to explicit observations from that delivery.
    LinkOutcome {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        receipt: PathBuf,
        #[arg(long)]
        state: PathBuf,
    },
    /// Validate and save a small candidate supplied by native Codex.
    Propose {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        state: PathBuf,
    },
    /// Compare direct delivery evidence under explicit environment revisions.
    Evaluate {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        deliveries: PathBuf,
        #[arg(long)]
        operation: PathBuf,
        #[arg(long)]
        state: PathBuf,
    },
    /// Summarize private records without exposing their text or paths.
    Status {
        #[arg(long)]
        state: PathBuf,
    },
}

fn error(code: &str) -> ContractError {
    ContractError(format!("learning_{code}"))
}

fn absolute(path: &Path) -> Result<PathBuf, ContractError> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(error("invalid_path"));
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|p| p.join(path))
            .map_err(|_| error("invalid_path"))
    }
}

/// Bind every existing directory component from the filesystem root. Following
/// the final component alone is insufficient for an owner-private evidence path.
fn directory_handle(path: &Path, private: bool) -> Result<File, ContractError> {
    let path = absolute(path)?;
    let mut directory = File::from(
        open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| error("directory_unavailable"))?,
    );
    for component in path.components() {
        if let Component::Normal(name) = component {
            directory = File::from(
                openat(
                    &directory,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|_| error("invalid_directory"))?,
            );
        }
    }
    if private && !private_for_current_user(&directory) {
        return Err(error("private_directory_required"));
    }
    Ok(directory)
}

fn read_handle(mut file: File) -> Result<Vec<u8>, ContractError> {
    let metadata = file.metadata().map_err(|_| error("invalid_input_file"))?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || !private_for_current_user(&file)
        || metadata.len() == 0
        || metadata.len() > MAX_FILE_BYTES
    {
        return Err(error("private_regular_input_required"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("input_unavailable"))?;
    let after = file.metadata().map_err(|_| error("invalid_input_file"))?;
    if bytes.is_empty()
        || bytes.len() as u64 > MAX_FILE_BYTES
        || after.nlink() != 1
        || metadata.len() != after.len()
        || metadata.mtime() != after.mtime()
        || metadata.mtime_nsec() != after.mtime_nsec()
        || metadata.ctime() != after.ctime()
        || metadata.ctime_nsec() != after.ctime_nsec()
        || bytes.len() as u64 != after.len()
        || !private_for_current_user(&file)
    {
        return Err(error("input_changed"));
    }
    Ok(bytes)
}

fn read_at(directory: &File, name: &std::ffi::OsStr) -> Result<Vec<u8>, ContractError> {
    let file = File::from(
        openat(
            directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("invalid_input_file"))?,
    );
    read_handle(file)
}

fn read_private(path: &Path) -> Result<Vec<u8>, ContractError> {
    let path = absolute(path)?;
    let parent = path.parent().ok_or_else(|| error("invalid_path"))?;
    let parent = directory_handle(parent, true)?;
    let name = path.file_name().ok_or_else(|| error("invalid_path"))?;
    read_at(&parent, name)
}

fn parse(bytes: &[u8]) -> Result<Value, ContractError> {
    serde_json::from_slice(bytes).map_err(|_| error("invalid_json"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn binding_matches(path: &Path, directory: &File) -> bool {
    directory_handle(path, true)
        .and_then(|current| {
            let current = current.metadata().map_err(|_| error("invalid_directory"))?;
            let held = directory
                .metadata()
                .map_err(|_| error("invalid_directory"))?;
            Ok(current.dev() == held.dev() && current.ino() == held.ino())
        })
        .unwrap_or(false)
}

struct LockGuard(File);

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Duplicated or inherited descriptors can outlive this transaction.
        // Explicit unlock ends its lock lifetime independently of their close.
        let _ = self.0.unlock();
    }
}

struct State {
    path: PathBuf,
    directory: File,
    _lock: Option<LockGuard>,
    records: Vec<Value>,
}

impl State {
    fn open(path: &Path, create: bool) -> Result<Self, ContractError> {
        let path = absolute(path)?;
        if create
            && fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            let parent =
                directory_handle(path.parent().ok_or_else(|| error("invalid_path"))?, false)?;
            if parent
                .metadata()
                .map_err(|_| error("invalid_directory"))?
                .uid()
                != rustix::process::geteuid().as_raw()
            {
                return Err(error("private_directory_required"));
            }
            mkdirat(
                &parent,
                path.file_name().ok_or_else(|| error("invalid_path"))?,
                Mode::from_raw_mode(0o700),
            )
            .map_err(|_| error("state_directory_unavailable"))?;
            parent
                .sync_all()
                .map_err(|_| error("state_directory_unavailable"))?;
        }
        // Both shared helper and component-bound handle checks are deliberate:
        // the former enforces the existing owner-private contract.
        open_private_directory(&path).map_err(|_| error("private_directory_required"))?;
        let directory = directory_handle(&path, true)?;
        let lock_flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        let lock = match openat(&directory, ".learning.lock", lock_flags, Mode::empty()) {
            Ok(fd) => Some(File::from(fd)),
            Err(rustix::io::Errno::NOENT) if create => Some(File::from(
                openat(
                    &directory,
                    ".learning.lock",
                    lock_flags | OFlags::CREATE | OFlags::EXCL,
                    Mode::from_raw_mode(0o600),
                )
                .map_err(|_| error("state_busy"))?,
            )),
            Err(rustix::io::Errno::NOENT) => None,
            Err(_) => return Err(error("invalid_state_lock")),
        };
        if let Some(lock) = &lock {
            let metadata = lock.metadata().map_err(|_| error("invalid_state_lock"))?;
            if !metadata.is_file() || metadata.nlink() != 1 || !private_for_current_user(lock) {
                return Err(error("invalid_state_lock"));
            }
            lock.try_lock().map_err(|_| error("state_busy"))?;
        }
        let lock = lock.map(LockGuard);
        let records = read_directory(&path, &directory, true)?
            .into_iter()
            .map(|(_, value)| {
                validate_record(&value)?;
                Ok(value)
            })
            .collect::<Result<Vec<_>, ContractError>>()?;
        validate_record_collection(&records)?;
        Ok(Self {
            path,
            directory,
            _lock: lock,
            records,
        })
    }

    fn save(&self, value: &Value) -> Result<bool, ContractError> {
        validate_record(value)?;
        let digest = learning::content_sha256(value)?;
        let name = format!("{digest}.json");
        let mut bytes =
            serde_json::to_vec_pretty(value).map_err(|_| error("serialization_failed"))?;
        bytes.push(b'\n');
        let directory_bytes = self.records.iter().try_fold(bytes.len(), |size, record| {
            let bytes =
                serde_json::to_vec_pretty(record).map_err(|_| error("serialization_failed"))?;
            size.checked_add(bytes.len() + 1)
                .ok_or_else(|| error("state_limit_exceeded"))
        })?;
        if bytes.len() as u64 > MAX_FILE_BYTES
            || self.records.len() + 2 > MAX_ENTRIES
            || directory_bytes > MAX_DIRECTORY_BYTES
        {
            return Err(error("state_limit_exceeded"));
        }
        if self
            .records
            .iter()
            .any(|v| learning::content_sha256(v).ok().as_ref() == Some(&digest))
        {
            return Ok(false);
        }
        let mut updated = self.records.clone();
        updated.push(value.clone());
        validate_record_collection(&updated)?;
        if !binding_matches(&self.path, &self.directory) {
            return Err(error("state_binding_changed"));
        }
        let temporary = format!(".learning-{}.tmp", uuid::Uuid::new_v4());
        let mut file = File::from(
            openat(
                &self.directory,
                temporary.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| error("output_unavailable"))?,
        );
        let mut published = false;
        let result = (|| {
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| error("output_write_failed"))?;
            if !private_for_current_user(&file)
                || file
                    .metadata()
                    .map_err(|_| error("output_write_failed"))?
                    .nlink()
                    != 1
            {
                return Err(error("output_write_failed"));
            }
            // Atomic no-clobber publication. Readers hold the same lock while
            // the temporary hardlink exists; persisted inputs require nlink=1.
            linkat(
                &self.directory,
                temporary.as_str(),
                &self.directory,
                name.as_str(),
                AtFlags::empty(),
            )
            .map_err(|_| error("output_exists_or_unavailable"))?;
            published = true;
            unlinkat(&self.directory, temporary.as_str(), AtFlags::empty())
                .map_err(|_| error("output_write_failed"))?;
            self.directory
                .sync_all()
                .map_err(|_| error("output_write_failed"))?;
            if !binding_matches(&self.path, &self.directory) {
                return Err(error("state_binding_changed"));
            }
            Ok(true)
        })();
        if result.is_err()
            && published
            && let Ok(fd) = openat(
                &self.directory,
                name.as_str(),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
        {
            let current = File::from(fd).metadata();
            if let (Ok(current), Ok(created)) = (current, file.metadata())
                && current.dev() == created.dev()
                && current.ino() == created.ino()
            {
                let _ = unlinkat(&self.directory, name.as_str(), AtFlags::empty());
            }
        }
        let _ = unlinkat(&self.directory, temporary.as_str(), AtFlags::empty());
        result
    }
}

fn read_directory(
    path: &Path,
    directory: &File,
    state_names: bool,
) -> Result<Vec<(String, Value)>, ContractError> {
    if !binding_matches(path, directory) {
        return Err(error("directory_binding_changed"));
    }
    let mut names = Vec::new();
    for (index, entry) in fs::read_dir(path)
        .map_err(|_| error("directory_unavailable"))?
        .enumerate()
    {
        if index >= MAX_ENTRIES {
            return Err(error("too_many_entries"));
        }
        let entry = entry.map_err(|_| error("directory_unavailable"))?;
        if entry.path().extension().is_some_and(|e| e == "json") {
            names.push(entry.file_name());
        }
    }
    names.sort();
    let mut total = 0_usize;
    let mut rows = Vec::new();
    for name in names {
        let bytes = read_at(directory, &name)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| error("directory_too_large"))?;
        if total > MAX_DIRECTORY_BYTES {
            return Err(error("directory_too_large"));
        }
        let value = parse(&bytes)?;
        if state_names
            && name != std::ffi::OsStr::new(&format!("{}.json", learning::content_sha256(&value)?))
        {
            return Err(error("state_digest_mismatch"));
        }
        rows.push((sha256(&bytes), value));
    }
    if !binding_matches(path, directory) {
        return Err(error("directory_binding_changed"));
    }
    Ok(rows)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationRecord {
    kind: String,
    schema: u8,
    input: learning::EvaluationInput,
    proposal_sha256: String,
    operation_sha256: String,
    result: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DuplicateAttempt {
    kind: String,
    schema: u8,
    duplicate_of_sha256: String,
    analysis: Option<learning::AnalysisResources>,
}

fn validate_record(value: &Value) -> Result<(), ContractError> {
    match value["kind"].as_str() {
        Some("groundline-learning-link") => {
            let link: learning::OutcomeLink =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_link"))?;
            link.validate()
        }
        Some("groundline-learning-proposal") => {
            let proposal: learning::Proposal =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_proposal"))?;
            proposal.validate()
        }
        Some("groundline-learning-evaluation-record") => {
            let record: EvaluationRecord = serde_json::from_value(value.clone())
                .map_err(|_| error("invalid_evaluation_record"))?;
            if record.schema != 1
                || !learning::digest(&record.proposal_sha256)
                || !learning::digest(&record.operation_sha256)
                || record.input.kind != "groundline-learning-evaluation-input"
                || record.input.schema != 1
                || record.result["kind"] != "groundline-learning-evaluation"
                || record.result["schema"] != 1
                || record.result["proposal_id"] != record.input.proposal_id
                || ![
                    "INCONCLUSIVE",
                    "REGRESSION_OBSERVED",
                    "NO_REGRESSION_OBSERVED",
                ]
                .contains(&record.result["status"].as_str().unwrap_or(""))
                || record.result["native_activation"] != "UNVERIFIED"
                || record.result["causal_effect_verified"] != false
                || record.result["efficiency_improvement_verified"] != false
            {
                return Err(error("invalid_evaluation_record"));
            }
            Ok(())
        }
        Some("groundline-learning-duplicate-attempt") => {
            let record: DuplicateAttempt = serde_json::from_value(value.clone())
                .map_err(|_| error("invalid_duplicate_attempt"))?;
            if record.schema != 1 || !learning::digest(&record.duplicate_of_sha256) {
                return Err(error("invalid_duplicate_attempt"));
            }
            if let Some(analysis) = record.analysis {
                analysis.validate()?;
            }
            Ok(())
        }
        _ => Err(error("unsupported_state")),
    }
}

fn validate_record_collection(records: &[Value]) -> Result<(), ContractError> {
    let analyses = records
        .iter()
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
    learning::summarize_analysis(&analyses, &[])?;
    let mut successful = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let links: Vec<Value> = records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-link")
        .cloned()
        .collect();
    let hashes: BTreeMap<String, &Value> = records
        .iter()
        .map(|v| Ok((learning::content_sha256(v)?, v)))
        .collect::<Result<_, ContractError>>()?;
    for record in records {
        if record["kind"] == "groundline-learning-proposal" {
            let mut input = record.clone();
            input["kind"] = json!("groundline-learning-proposal-input");
            learning::prepare_proposal(&input, &links)?;
        }
        if record["kind"] == "groundline-learning-proposal" && record["status"] != "failed" {
            let p: learning::Proposal =
                serde_json::from_value(record.clone()).map_err(|_| error("invalid_proposal"))?;
            if !successful.insert(p.dedup_sha256()?) || !ids.insert(p.proposal_id) {
                return Err(error("duplicate_successful_proposal"));
            }
        }
        if record["kind"] == "groundline-learning-duplicate-attempt" {
            let reference = record["duplicate_of_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_duplicate_attempt"))?;
            let candidate = hashes
                .get(reference)
                .ok_or_else(|| error("duplicate_source_missing"))?;
            if candidate["kind"] != "groundline-learning-proposal"
                || candidate["status"] == "failed"
            {
                return Err(error("duplicate_source_missing"));
            }
        }
        if record["kind"] == "groundline-learning-evaluation-record" {
            let reference = record["proposal_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_evaluation_record"))?;
            let candidate = hashes
                .get(reference)
                .ok_or_else(|| error("evaluation_source_missing"))?;
            if candidate["kind"] != "groundline-learning-proposal"
                || candidate["status"] != "candidate"
                || candidate["proposal_id"] != record["input"]["proposal_id"]
            {
                return Err(error("evaluation_source_missing"));
            }
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
                if reference
                    .as_str()
                    .and_then(|s| hashes.get(s))
                    .is_none_or(|v| v["kind"] != "groundline-learning-link")
                {
                    return Err(error("evaluation_link_missing"));
                }
            }
        }
    }
    Ok(())
}

fn readout(status: &str, mutation: bool) -> Value {
    json!({"kind":"groundline-learning-result", "schema":1, "status":status,
        "native_activation":"UNVERIFIED", "network_performed":false,
        "mutation_performed":mutation, "raw_content_emitted":false, "private_paths_emitted":false})
}

fn link(input: &Path, receipt: &Path, state: &Path) -> Result<Value, ContractError> {
    let input = parse(&read_private(input)?)?;
    let receipt_bytes = read_private(receipt)?;
    let receipt_sha256 = sha256(&receipt_bytes);
    let value = learning::link_outcome(&input, &parse(&receipt_bytes)?, &receipt_sha256)?;
    let state = State::open(state, true)?;
    let changed = state.save(&value)?;
    let mut out = readout(if changed { "LINKED" } else { "ALREADY_LINKED" }, changed);
    out["link_sha256"] = json!(learning::content_sha256(&value)?);
    out["historical_revision_observed"] = json!(!value["environment_revision"].is_null());
    out["correction_kind"] = value["correction_kind"].clone();
    Ok(out)
}

fn propose(input: &Path, state: &Path) -> Result<Value, ContractError> {
    let input = parse(&read_private(input)?)?;
    let state = State::open(state, true)?;
    let links: Vec<Value> = state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-link")
        .cloned()
        .collect();
    let value = learning::prepare_proposal(&input, &links)?;
    let proposal: learning::Proposal =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_proposal"))?;
    let key = proposal.dedup_sha256()?;
    for stored in &state.records {
        if proposal.status == "failed" {
            break;
        }
        if stored["kind"] != "groundline-learning-proposal" || stored["status"] == "failed" {
            continue;
        }
        let stored_value = stored;
        let stored: learning::Proposal =
            serde_json::from_value(stored.clone()).map_err(|_| error("invalid_proposal"))?;
        if stored.dedup_sha256()? == key {
            let cost_recorded = if value["analysis"] != stored_value["analysis"] {
                let attempt = serde_json::to_value(DuplicateAttempt {
                    kind: "groundline-learning-duplicate-attempt".into(),
                    schema: 1,
                    duplicate_of_sha256: learning::content_sha256(stored_value)?,
                    analysis: proposal.analysis.clone(),
                })
                .map_err(|_| error("serialization_failed"))?;
                state.save(&attempt)?
            } else {
                false
            };
            let mut out = readout("DUPLICATE_SUPPRESSED", cost_recorded);
            out["source_evidence_processed"] = json!(true);
            out["existing_decision"] = json!(stored.status);
            out["analysis_resources_complete"] = json!(
                stored
                    .analysis
                    .as_ref()
                    .is_some_and(|a| a.resources.complete)
            );
            out["duplicate_analysis_cost_recorded"] = json!(cost_recorded);
            return Ok(out);
        }
        if stored.proposal_id == proposal.proposal_id {
            return Err(error("proposal_id_conflict"));
        }
    }
    let changed = state.save(&value)?;
    let mut out = readout(
        match proposal.status.as_str() {
            "candidate" => "PROPOSED",
            "no_change" => "NO_CHANGE",
            _ => "ANALYSIS_FAILED",
        },
        changed,
    );
    out["source_evidence_processed"] = json!(proposal.status != "failed");
    out["proposal_sha256"] = json!(learning::content_sha256(&value)?);
    out["dedup_sha256"] = json!(key);
    out["analysis_resources_complete"] = json!(
        proposal
            .analysis
            .as_ref()
            .is_some_and(|a| a.resources.complete)
    );
    out["analysis_owned_total_tokens"] = json!(
        proposal
            .analysis
            .as_ref()
            .filter(|a| a.resources.complete)
            .map(|a| a
                .resources
                .entries
                .iter()
                .map(|r| r.total_tokens.unwrap_or(0))
                .sum::<u64>())
    );
    Ok(out)
}

fn evaluate(
    input: &Path,
    deliveries: &Path,
    operation: &Path,
    state: &Path,
) -> Result<Value, ContractError> {
    let input = parse(&read_private(input)?)?;
    let operation_bytes = read_private(operation)?;
    let operation = parse(&operation_bytes)?;
    crate::environment::validate_operation(&operation)?;
    let directory = directory_handle(deliveries, true)?;
    let receipts = read_directory(deliveries, &directory, false)?;
    let values: Vec<Value> = receipts.iter().map(|(_, v)| v.clone()).collect();
    delivery::validate_receipt_collection(&values)?;
    let state = State::open(state, false)?;
    let links: Vec<Value> = state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-link")
        .cloned()
        .collect();
    let proposal = state
        .records
        .iter()
        .find(|v| {
            v["kind"] == "groundline-learning-proposal"
                && v["status"] == "candidate"
                && v["proposal_id"] == input["proposal_id"]
        })
        .ok_or_else(|| error("candidate_missing"))?;
    let mut result = learning::evaluate(&input, proposal, &links, &receipts, &operation)?;
    let current: learning::Proposal =
        serde_json::from_value(proposal.clone()).map_err(|_| error("invalid_proposal"))?;
    let mut evidence_refs = current.evidence_refs.clone();
    evidence_refs.sort();
    let proposal_sha256 = learning::content_sha256(proposal)?;
    let mut analyses = vec![current.analysis.clone()];
    for record in &state.records {
        if record["kind"] == "groundline-learning-proposal" && record["status"] == "failed" {
            let failed: learning::Proposal =
                serde_json::from_value(record.clone()).map_err(|_| error("invalid_proposal"))?;
            let mut failed_refs = failed.evidence_refs.clone();
            failed_refs.sort();
            if failed_refs == evidence_refs
                && failed.scope_sha256 == current.scope_sha256
                && failed.basis_revision == current.basis_revision
                && failed.source_revision == current.source_revision
            {
                analyses.push(failed.analysis);
            }
        }
        if record["kind"] == "groundline-learning-duplicate-attempt"
            && record["duplicate_of_sha256"] == proposal_sha256
        {
            let attempt: DuplicateAttempt = serde_json::from_value(record.clone())
                .map_err(|_| error("invalid_duplicate_attempt"))?;
            analyses.push(attempt.analysis);
        }
    }
    let selected_receipts: Vec<Value> = input["baseline_refs"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(input["followup_refs"].as_array().into_iter().flatten())
        .filter_map(|reference| {
            links
                .iter()
                .find(|link| learning::content_sha256(link).ok().as_deref() == reference.as_str())
        })
        .filter_map(|link| {
            receipts
                .iter()
                .find(|(hash, _)| link["receipt_sha256"] == *hash)
        })
        .map(|(_, receipt)| receipt.clone())
        .collect();
    let costs = learning::summarize_analysis(&analyses, &selected_receipts)?;
    if costs["complete"] != true {
        if result["status"] == "NO_REGRESSION_OBSERVED" {
            result["status"] = json!("INCONCLUSIVE");
        }
        let reasons = result["comparison_reasons"]
            .as_array_mut()
            .ok_or_else(|| error("invalid_evaluation"))?;
        if !reasons.contains(&json!("analysis_attempt_resources_incomplete")) {
            reasons.push(json!("analysis_attempt_resources_incomplete"));
            reasons.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        }
    }
    result["analysis_attempt_resources"] = costs;
    let value = serde_json::to_value(EvaluationRecord {
        kind: "groundline-learning-evaluation-record".into(),
        schema: 1,
        input: serde_json::from_value(input).map_err(|_| error("invalid_evaluation"))?,
        proposal_sha256,
        operation_sha256: sha256(&operation_bytes),
        result: result.clone(),
    })
    .map_err(|_| error("serialization_failed"))?;
    let changed = state.save(&value)?;
    result["network_performed"] = json!(false);
    result["mutation_performed"] = json!(changed);
    result["evaluation_sha256"] = json!(learning::content_sha256(&value)?);
    Ok(result)
}

fn status(path: &Path) -> Result<Value, ContractError> {
    let state = State::open(path, false)?;
    let mut counts = BTreeMap::<String, usize>::new();
    let mut processed = BTreeSet::new();
    let mut analyses = BTreeMap::<String, Value>::new();
    for value in &state.records {
        let category = if value["kind"] == "groundline-learning-link" {
            "linked".to_owned()
        } else if value["kind"] == "groundline-learning-proposal" {
            value["status"].as_str().unwrap_or("unknown").to_owned()
        } else if value["kind"] == "groundline-learning-duplicate-attempt" {
            "duplicate_analysis".to_owned()
        } else {
            value["result"]["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned()
        };
        *counts.entry(category).or_default() += 1;
        if value["kind"] == "groundline-learning-proposal" {
            let p: learning::Proposal =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_proposal"))?;
            if p.status != "failed" {
                processed.extend(p.evidence_refs);
            }
            if let Some(analysis) = p.analysis {
                for row in analysis.resources.entries {
                    let current =
                        serde_json::to_value(&row).map_err(|_| error("serialization_failed"))?;
                    if let Some(existing) = analyses.insert(row.response_hash, current.clone())
                        && existing != current
                    {
                        return Err(error("analysis_response_ownership_conflict"));
                    }
                }
            }
        }
        if value["kind"] == "groundline-learning-duplicate-attempt" {
            let attempt: DuplicateAttempt = serde_json::from_value(value.clone())
                .map_err(|_| error("invalid_duplicate_attempt"))?;
            if let Some(analysis) = attempt.analysis {
                for row in analysis.resources.entries {
                    let current =
                        serde_json::to_value(&row).map_err(|_| error("serialization_failed"))?;
                    if let Some(existing) = analyses.insert(row.response_hash, current.clone())
                        && existing != current
                    {
                        return Err(error("analysis_response_ownership_conflict"));
                    }
                }
            }
        }
    }
    let total = analyses.values().try_fold(0_u64, |sum, v| {
        sum.checked_add(v["total_tokens"].as_u64().unwrap_or(0))
            .ok_or_else(|| error("analysis_token_overflow"))
    })?;
    let missing = analyses
        .values()
        .filter(|v| v["total_tokens"].is_null())
        .count();
    let mut out = readout("OBSERVATIONAL", false);
    out["records"] = json!(counts);
    out["processed_evidence_count"] = json!(processed.len());
    out["analysis_resources"] = json!({"owned_response_count":analyses.len(),
        "total_tokens":{"known_sum":(!analyses.is_empty() && missing < analyses.len()).then_some(total), "missing_count":missing},
        "unobserved_analysis_count":state.records.iter().filter(|v| ["groundline-learning-proposal", "groundline-learning-duplicate-attempt"].contains(&v["kind"].as_str().unwrap_or("")) && v["analysis"].is_null()).count()});
    Ok(out)
}

pub(crate) fn run(command: Command) -> Result<Value, ContractError> {
    match command {
        Command::LinkOutcome {
            input,
            receipt,
            state,
        } => link(&input, &receipt, &state),
        Command::Propose { input, state } => propose(&input, &state),
        Command::Evaluate {
            input,
            deliveries,
            operation,
            state,
        } => evaluate(&input, &deliveries, &operation, &state),
        Command::Status { state } => status(&state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn tempdir() -> Result<tempfile::TempDir, std::io::Error> {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)?;
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))?;
        Ok(directory)
    }
    fn h(n: u64) -> String {
        format!("{n:064x}")
    }
    fn private_json(path: &Path, value: &Value) -> String {
        let bytes = serde_json::to_vec_pretty(value).unwrap();
        fs::write(path, &bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        sha256(&bytes)
    }
    fn receipt() -> Value {
        json!({"kind":"groundline-delivery-receipt", "schema":1,
            "unit_hash":h(1), "cohort_sha256":h(2), "phase":"implementation",
            "completed_at_utc":"2026-10-01T12:00:00Z", "recommendation":null, "requested":null, "effective":null,
            "verification":{"status":"unknown", "evidence_kind":"unobserved", "evidence_sha256":h(3), "rework":false, "authenticity_verified":false},
            "resources":{"complete":false, "wall_duration_ms":null, "entries":[]},
            "activation_verified":false, "observed_selection_matches_requested":null})
    }
    fn link_input(digest: &str) -> Value {
        json!({"kind":"groundline-learning-link-input", "schema":1, "receipt_sha256":digest,
            "unit_hash":h(1), "cohort_sha256":h(2), "phase":"implementation", "completed_at_utc":"2026-10-01T12:00:00Z",
            "observed_at_utc":null, "environment_revision":null, "skill":null, "source_revision":null,
            "runtime":null, "correction_kind":"unknown", "correction_evidence_sha256":null})
    }
    fn proposal(reference: &str, decision: &str) -> Value {
        json!({"kind":"groundline-learning-proposal-input", "schema":1,
            "proposal_id":"review-1", "evidence_refs":[reference], "scope_sha256":h(4),
            "basis_revision":h(5), "source_revision":null, "plan_sha256":h(6),
            "status":decision, "reason":"Bounded native review", "target":null, "analysis":null})
    }

    #[test]
    fn private_roundtrip_deduplicates_no_change_and_retains_unknown() {
        let root = tempdir().unwrap();
        let receipt_path = root.path().join("receipt.json");
        let digest = private_json(&receipt_path, &receipt());
        let input = root.path().join("input.json");
        private_json(&input, &link_input(&digest));
        let state = root.path().join("learning");
        let linked = link(&input, &receipt_path, &state).unwrap();
        assert_eq!(linked["status"], "LINKED");
        assert_eq!(
            link(&input, &receipt_path, &state).unwrap()["status"],
            "ALREADY_LINKED"
        );
        private_json(
            &input,
            &proposal(linked["link_sha256"].as_str().unwrap(), "no_change"),
        );
        assert_eq!(propose(&input, &state).unwrap()["status"], "NO_CHANGE");
        assert_eq!(
            propose(&input, &state).unwrap()["status"],
            "DUPLICATE_SUPPRESSED"
        );
        let out = status(&state).unwrap();
        assert_eq!(out["records"]["no_change"], 1);
        assert_eq!(out["processed_evidence_count"], 1);
        assert!(out["analysis_resources"]["total_tokens"]["known_sum"].is_null());
        assert_eq!(
            fs::metadata(&state).unwrap().permissions().mode() & 0o777,
            0o700
        );
        for entry in fs::read_dir(&state).unwrap() {
            assert_eq!(
                entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn failed_analysis_does_not_process_source_or_block_successful_retry() {
        let root = tempdir().unwrap();
        let receipt_path = root.path().join("receipt.json");
        let digest = private_json(&receipt_path, &receipt());
        let input = root.path().join("input.json");
        private_json(&input, &link_input(&digest));
        let state = root.path().join("learning");
        let linked = link(&input, &receipt_path, &state).unwrap();
        let reference = linked["link_sha256"].as_str().unwrap();
        private_json(&input, &proposal(reference, "failed"));
        assert_eq!(
            propose(&input, &state).unwrap()["source_evidence_processed"],
            false
        );
        assert_eq!(status(&state).unwrap()["processed_evidence_count"], 0);
        private_json(&input, &proposal(reference, "no_change"));
        assert_eq!(propose(&input, &state).unwrap()["status"], "NO_CHANGE");
        let out = status(&state).unwrap();
        assert_eq!(out["records"]["failed"], 1);
        assert_eq!(out["records"]["no_change"], 1);
        let mut failed = proposal(reference, "failed");
        failed["reason"] = json!("A later failed attempt is still a failure");
        private_json(&input, &failed);
        assert_eq!(
            propose(&input, &state).unwrap()["status"],
            "ANALYSIS_FAILED"
        );
        assert_eq!(status(&state).unwrap()["records"]["failed"], 2);
    }

    #[test]
    fn duplicate_new_analysis_cost_is_owned_once_without_a_second_candidate() {
        let root = tempdir().unwrap();
        let receipt_path = root.path().join("receipt.json");
        let digest = private_json(&receipt_path, &receipt());
        let input = root.path().join("input.json");
        private_json(&input, &link_input(&digest));
        let state = root.path().join("learning");
        let linked = link(&input, &receipt_path, &state).unwrap();
        let mut p = proposal(linked["link_sha256"].as_str().unwrap(), "no_change");
        private_json(&input, &p);
        propose(&input, &state).unwrap();
        p["proposal_id"] = json!("another-name");
        p["plan_sha256"] = json!(h(999));
        p["analysis"] = json!({"unit_hash":h(20), "resources":{"complete":false,
            "wall_duration_ms":null, "entries":[{"owner":"root","unit_hash":h(20),
                "response_hash":h(21),"effective":null,"input_tokens":7,"cached_input_tokens":null,
                "output_tokens":3,"reasoning_output_tokens":null,"total_tokens":10}]}});
        private_json(&input, &p);
        let out = propose(&input, &state).unwrap();
        assert_eq!(out["status"], "DUPLICATE_SUPPRESSED");
        assert_eq!(out["duplicate_analysis_cost_recorded"], true);
        assert_eq!(
            propose(&input, &state).unwrap()["duplicate_analysis_cost_recorded"],
            false
        );
        let out = status(&state).unwrap();
        assert_eq!(out["records"]["no_change"], 1);
        assert_eq!(out["records"]["duplicate_analysis"], 1);
        assert_eq!(out["analysis_resources"]["owned_response_count"], 1);
        assert_eq!(out["analysis_resources"]["total_tokens"]["known_sum"], 10);
    }

    #[test]
    fn conflicting_analysis_responses_are_rejected_before_publication_without_poisoning_state() {
        let root = tempdir().unwrap();
        let receipt_path = root.path().join("receipt.json");
        let digest = private_json(&receipt_path, &receipt());
        let input = root.path().join("input.json");
        private_json(&input, &link_input(&digest));
        let state = root.path().join("learning");
        let linked = link(&input, &receipt_path, &state).unwrap();
        let mut first = proposal(linked["link_sha256"].as_str().unwrap(), "no_change");
        first["analysis"] = json!({"unit_hash":h(20),"resources":{"complete":false,
            "wall_duration_ms":null,"entries":[{"owner":"root","unit_hash":h(20),
                "response_hash":h(21),"effective":null,"input_tokens":7,"cached_input_tokens":null,
                "output_tokens":3,"reasoning_output_tokens":null,"total_tokens":10}]}});
        private_json(&input, &first);
        assert_eq!(propose(&input, &state).unwrap()["status"], "NO_CHANGE");
        let mut reused = first.clone();
        reused["proposal_id"] = json!("second-context");
        reused["scope_sha256"] = json!(h(22));
        private_json(&input, &reused);
        assert_eq!(propose(&input, &state).unwrap()["status"], "NO_CHANGE");
        let mut duplicate = first.clone();
        duplicate["proposal_id"] = json!("duplicate-context");
        duplicate["analysis"]["resources"]["wall_duration_ms"] = json!(50);
        private_json(&input, &duplicate);
        assert_eq!(
            propose(&input, &state).unwrap()["duplicate_analysis_cost_recorded"],
            true
        );
        let before = status(&state).unwrap();
        assert_eq!(before["analysis_resources"]["owned_response_count"], 1);
        assert_eq!(
            before["analysis_resources"]["total_tokens"]["known_sum"],
            10
        );
        let snapshot = || {
            let mut files: Vec<_> = fs::read_dir(&state)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (entry.file_name(), fs::read(entry.path()).unwrap())
                })
                .collect();
            files.sort();
            files
        };
        let files_before = snapshot();
        for mut conflicting in [reused, duplicate] {
            if conflicting["scope_sha256"] != first["scope_sha256"] {
                conflicting["proposal_id"] = json!("third-context");
                conflicting["scope_sha256"] = json!(h(23));
            }
            conflicting["analysis"]["resources"]["entries"][0]["input_tokens"] = json!(17);
            conflicting["analysis"]["resources"]["entries"][0]["total_tokens"] = json!(20);
            private_json(&input, &conflicting);
            assert_eq!(
                propose(&input, &state).unwrap_err().0,
                "learning_analysis_response_ownership_overlap"
            );
            assert_eq!(status(&state).unwrap(), before);
            assert_eq!(snapshot(), files_before);
        }
    }

    #[test]
    fn rejects_shared_symlink_hardlink_and_ancestor_symlink_inputs() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let file = root.path().join("input.json");
        private_json(&file, &json!({"safe":true}));
        assert!(read_private(&file).is_ok());
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_private(&file).is_err());
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.path().join("alias.json");
        symlink(&file, &link).unwrap();
        assert!(read_private(&link).is_err());
        fs::remove_file(&link).unwrap();
        fs::hard_link(&file, &link).unwrap();
        assert!(read_private(&file).is_err());
        fs::remove_file(&link).unwrap();
        let directory_alias = root.path().join("alias");
        symlink(root.path(), &directory_alias).unwrap();
        assert!(read_private(&directory_alias.join("input.json")).is_err());
    }

    #[test]
    fn state_guard_releases_lock_while_a_duplicated_descriptor_remains_open() {
        let root = tempdir().unwrap();
        let path = root.path().join("learning");
        let first = State::open(&path, true).unwrap();
        let duplicated = first._lock.as_ref().unwrap().0.try_clone().unwrap();
        assert!(
            matches!(State::open(&path, false), Err(ContractError(code)) if code == "learning_state_busy")
        );
        drop(first);
        let second = State::open(&path, false).unwrap();
        assert!(duplicated.metadata().is_ok());
        assert!(
            matches!(State::open(&path, false), Err(ContractError(code)) if code == "learning_state_busy")
        );
        drop(second);
        drop(duplicated);
    }

    #[test]
    fn no_clobber_preserves_existing_files_and_rejects_retired_state() {
        let root = tempdir().unwrap();
        let path = root.path().join("learning");
        let state = State::open(&path, true).unwrap();
        let mut value = link_input(&h(99));
        value["kind"] = json!("groundline-learning-link");
        let target = path.join(format!(
            "{}.json",
            learning::content_sha256(&value).unwrap()
        ));
        private_json(&target, &json!({"user":"preserve"}));
        assert!(state.save(&value).is_err());
        assert_eq!(
            parse(&fs::read(&target).unwrap()).unwrap(),
            json!({"user":"preserve"})
        );
        drop(state);
        assert!(State::open(&path, false).is_err());
    }
}
