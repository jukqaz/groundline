//! Inspect and safely restore existing personal guidance; no new trials or policy engine.
use chrono::{DateTime, Duration, Utc};
use clap::Subcommand;
use groundline_contracts::ContractError;
use groundline_runtime::local_file::{
    atomic_write_private, open_bounded_regular_file, open_or_create_private_lock,
    open_private_directory, private_for_current_user,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_STATE_FILES: usize = 128;
// Interrupted atomic replacements do not consume durable history slots. Keep
// them private and bounded, and preserve them for explicit owner inspection.
const MAX_INTERRUPTED_WRITES: usize = 8;
const MAX_DIRECTORY_ENTRIES: usize = MAX_STATE_FILES + MAX_INTERRUPTED_WRITES;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect an existing private trial without creating files or changing state.
    Status {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Restore only unchanged generated guidance, including interrupted restores.
    Rollback {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

// These fixed instruction bytes identify existing generated guidance. They are
// recovery data, not active recommendations; edits would strand existing trials.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Rule {
    ApprovalContinuity,
    DiagnoseBeforeRetry,
    EvidenceReuse,
    JustInTimeContext,
    BoundedParallelReads,
}
impl Rule {
    const ALL: [Self; 5] = [
        Self::ApprovalContinuity,
        Self::DiagnoseBeforeRetry,
        Self::EvidenceReuse,
        Self::JustInTimeContext,
        Self::BoundedParallelReads,
    ];

    fn instruction(self) -> &'static str {
        match self {
            Self::ApprovalContinuity => {
                "Preserve the active task, completed work, and existing scoped approval across follow-ups. Continue authorized work; ask again only when a material choice or new authority is required."
            }
            Self::DiagnoseBeforeRetry => {
                "After a failed operation, identify the cause or a changed condition before retrying. Treat environment failures separately from code failures. Keep transient retries and verification bounded; reuse passing evidence until a relevant change or unresolved risk requires another check."
            }
            Self::EvidenceReuse => {
                "Reuse a directly verified result only while its relevant inputs, code, environment, scope, and risk remain unchanged. Check that evidence still covers the requested outcome; rerun the smallest affected check after a relevant change or unresolved risk. Do not substitute old evidence for a required fresh live check."
            }
            Self::JustInTimeContext => {
                "Load the smallest relevant context needed for the active task, using targeted discovery and conditional references. Read every selected mandatory instruction fully, preserve decision-critical evidence, and expand context when uncertainty remains. Do not preload unrelated references or truncate required evidence to reduce tokens."
            }
            Self::BoundedParallelReads => {
                "Use an available native parallel mechanism for bounded, independent read-only operations when their results do not depend on each other. Preserve per-operation status and evidence, respect rate limits and permissions, and serialize dependent or state-changing work. This guidance does not authorize subagents or change native settings."
            }
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum OpportunityKind {
    EvidenceReuse,
    JustInTimeContext,
    BoundedParallelReads,
}

/// Operator-observed eligible opportunities, never inferred from effort, time,
/// or tool counts. The digest refers to private, strategy-specific evidence.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OpportunityEvidence {
    kind: OpportunityKind,
    evidence_sha256: String,
    eligible_count: u16,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ActivationEvidence {
    instruction_load_sha256: String,
    behavior_check_sha256: String,
    guidance_sha256: String,
    observed_at_utc: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Verified,
    Failed,
    Unknown,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Evidence {
    RuntimeCheck,
    UserAcceptance,
    Unobserved,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    unit_hash: String,
    started_at_utc: String,
    completed_at_utc: String,
    outcome: Outcome,
    evidence: Evidence,
    rework: bool,
    redundant_approval_count: u16,
    continuation_prompt_count: u16,
    repeated_call_count: u32,
    tool_call_count: u32,
    total_tokens: Option<u64>,
    optimization_opportunities: Option<Vec<OpportunityEvidence>>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    kind: String,
    schema: u8,
    period_start_utc: String,
    period_end_utc: String,
    model_context_sha256: String,
    guidance_sha256: String,
    comparison_context_sha256: String,
    task_kind: String,
    scope_size: String,
    activation_evidence: Option<ActivationEvidence>,
    units: Vec<Unit>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Trial {
    kind: String,
    schema: u8,
    status: String,
    applied_at_utc: String,
    rule: Rule,
    before_rules: Vec<Rule>,
    after_rules: Vec<Rule>,
    baseline: Sample,
    model_context_sha256: String,
}
fn error(code: &str) -> ContractError {
    ContractError(format!("personal_{code}"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn time(s: &str) -> Result<DateTime<Utc>, ContractError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| error("invalid_timestamp"))
}
fn output(operation: &str) -> Value {
    json!({"kind":"groundline-personal-recovery","schema":1,"operation":operation,
        "network_performed":false,"mutation_performed":false,"raw_content_emitted":false,
        "private_paths_emitted":false,"model_changed":false,"permissions_changed":false,
        "native_activation":"UNVERIFIED"})
}
fn validate_sample(s: &Sample, context: &str, now: DateTime<Utc>) -> Result<(), ContractError> {
    let start = time(&s.period_start_utc)?;
    let end = time(&s.period_end_utc)?;
    if s.kind != "groundline-outcome-sample"
        || s.schema != 2
        || s.model_context_sha256 != context
        || !valid_hash(&s.guidance_sha256)
        || !valid_hash(&s.comparison_context_sha256)
        || !["implementation", "review", "deployment", "documentation"]
            .contains(&s.task_kind.as_str())
        || !["small", "medium", "large"].contains(&s.scope_size.as_str())
        || start >= end
        || end > now
        || end - start > Duration::days(90)
        || s.units.len() > 1000
    {
        return Err(error("invalid_outcome_sample"));
    }
    if let Some(activation) = &s.activation_evidence
        && (!valid_hash(&activation.instruction_load_sha256)
            || !valid_hash(&activation.behavior_check_sha256)
            || activation.guidance_sha256 != s.guidance_sha256
            || time(&activation.observed_at_utc)? >= start)
    {
        return Err(error("invalid_activation_evidence"));
    }
    let mut ids = BTreeSet::new();
    for u in &s.units {
        let started = time(&u.started_at_utc)?;
        let completed = time(&u.completed_at_utc)?;
        if !valid_hash(&u.unit_hash)
            || !ids.insert(&u.unit_hash)
            || started < start
            || completed > end
            || started >= completed
            || u.repeated_call_count > u.tool_call_count
            || u.total_tokens.is_some_and(|n| n > 1_000_000_000_000)
            || (u.outcome == Outcome::Unknown) != (u.evidence == Evidence::Unobserved)
        {
            return Err(error("invalid_or_duplicate_outcome"));
        }
        if let Some(opportunities) = &u.optimization_opportunities
            && (opportunities.len() > 3
                || opportunities
                    .iter()
                    .map(|item| item.kind)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != opportunities.len()
                || opportunities.iter().any(|item| {
                    !valid_hash(&item.evidence_sha256)
                        || item.eligible_count == 0
                        || item.eligible_count > 10_000
                }))
        {
            return Err(error("invalid_opportunity_evidence"));
        }
    }
    Ok(())
}
fn render(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return String::new();
    }
    let mut out="# GroundLine Personal Guidance\n\nApply within the user's current request and existing scoped authority. Higher-priority instructions and explicit user choices take precedence.\n".to_owned();
    for rule in rules {
        out.push_str("\n- ");
        out.push_str(rule.instruction());
        out.push('\n');
    }
    out
}
fn state_root(path: &Path) -> Result<PathBuf, ContractError> {
    let meta = fs::symlink_metadata(path).map_err(|_| error("state_directory_required"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(error("private_state_required"));
    }
    let root = path
        .canonicalize()
        .map_err(|_| error("state_directory_required"))?;
    if root.ancestors().any(|p| p.join(".git").exists()) {
        return Err(error("private_state_outside_git_required"));
    }
    open_private_directory(path).map_err(|_| error("private_state_required"))?;
    ensure_capacity(&root, &[])?;
    Ok(root)
}
fn ensure_capacity(root: &Path, additions: &[PathBuf]) -> Result<(), ContractError> {
    let mut count = 0;
    let mut interrupted = 0;
    for entry in fs::read_dir(root)
        .map_err(|_| error("state_unavailable"))?
        .take(MAX_DIRECTORY_ENTRIES + 1)
    {
        let entry = entry.map_err(|_| error("state_unavailable"))?;
        if is_interrupted_write(&entry.path())? {
            interrupted += 1;
        } else {
            count += 1;
        }
        if interrupted > MAX_INTERRUPTED_WRITES {
            return Err(error("interrupted_write_limit"));
        }
        if count > MAX_STATE_FILES {
            return Err(error("state_archive_limit"));
        }
    }
    for path in additions.iter().collect::<BTreeSet<_>>() {
        match fs::symlink_metadata(path) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => count += 1,
            Err(_) => return Err(error("state_unavailable")),
        }
    }
    if count > MAX_STATE_FILES {
        return Err(error("state_archive_limit"));
    }
    Ok(())
}
fn is_interrupted_write(path: &Path) -> Result<bool, ContractError> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(false);
    };
    let Some(name) = name
        .strip_prefix('.')
        .and_then(|name| name.strip_suffix(".tmp"))
    else {
        return Ok(false);
    };
    let mut parts = name.rsplitn(3, '.');
    let (Some(sequence), Some(pid), Some(target)) = (parts.next(), parts.next(), parts.next())
    else {
        return Ok(false);
    };
    let archive = ["trial-", "evaluation-"].iter().any(|prefix| {
        target
            .strip_prefix(prefix)
            .and_then(|name| name.strip_suffix(".json"))
            .is_some_and(valid_hash)
    });
    if !["trial.json", "personal-guidance.md"].contains(&target) && !archive
        || sequence
            .parse::<u64>()
            .ok()
            .is_none_or(|n| n.to_string() != sequence)
        || pid
            .parse::<u32>()
            .ok()
            .is_none_or(|n| n == 0 || n.to_string() != pid)
    {
        return Ok(false);
    }
    // Never follow links, relax permissions, delete, or interpret a leftover as
    // committed state. A partially written file is still an interrupted write.
    private_bytes(path)?;
    Ok(true)
}
fn lock(root: &Path) -> Result<File, ContractError> {
    ensure_capacity(root, &[root.join(".lock")])?;
    let file =
        open_or_create_private_lock(&root.join(".lock")).map_err(|_| error("invalid_lock"))?;
    file.try_lock().map_err(|_| error("state_busy"))?;
    Ok(file)
}
fn private_bytes(path: &Path) -> Result<Vec<u8>, ContractError> {
    let file =
        open_bounded_regular_file(path, 0, MAX_BYTES).map_err(|_| error("invalid_state_file"))?;
    if !private_for_current_user(&file) {
        return Err(error("private_state_required"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if file
            .metadata()
            .map_err(|_| error("invalid_state_file"))?
            .nlink()
            != 1
        {
            return Err(error("linked_state_file"));
        }
    }
    let mut out = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut out)
        .map_err(|_| error("state_unavailable"))?;
    if out.len() as u64 > MAX_BYTES {
        return Err(error("state_too_large"));
    }
    Ok(out)
}
fn current_rules(root: &Path) -> Result<Vec<Rule>, ContractError> {
    let path = root.join("personal-guidance.md");
    let content = match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return Err(error("state_unavailable")),
        Ok(_) => private_bytes(&path)?,
    };
    for bits in 0..(1 << Rule::ALL.len()) {
        let rules: Vec<_> = Rule::ALL
            .iter()
            .enumerate()
            .filter(|(i, _)| bits & (1 << i) != 0)
            .map(|(_, r)| *r)
            .collect();
        if render(&rules).as_bytes() == content {
            return Ok(rules);
        }
    }
    Err(error("user_edited_guidance_preserved"))
}
#[cfg(test)]
fn save_trial(root: &Path, t: &Trial) -> Result<(), ContractError> {
    atomic_write_private(
        &root.join("trial.json"),
        &serde_json::to_vec_pretty(t).map_err(|_| error("serialization_failed"))?,
    )
    .map_err(|_| error("state_write_failed"))
}
fn load_trial(root: &Path) -> Result<Trial, ContractError> {
    parse_trial(&private_bytes(&root.join("trial.json"))?)
}
fn parse_trial(data: &[u8]) -> Result<Trial, ContractError> {
    // Inspect only the version before parsing the current contract. Do not
    // reinterpret or migrate retired state; its matching CLI must restore it.
    #[derive(Deserialize)]
    struct TrialVersion {
        schema: u64,
    }
    let version: TrialVersion = serde_json::from_slice(data).map_err(|_| error("invalid_trial"))?;
    if version.schema != 2 {
        return Err(error(
            "unsupported_trial_schema_preserved_restore_with_matching_cli",
        ));
    }
    let t: Trial = serde_json::from_slice(data).map_err(|_| error("invalid_trial"))?;
    if t.kind != "groundline-personal-trial"
        || t.schema != 2
        || ![
            "prepared",
            "pending",
            "restoring",
            "retained",
            "rolled_back",
        ]
        .contains(&t.status.as_str())
        || t.before_rules.len() >= Rule::ALL.len()
        || t.after_rules.len() != t.before_rules.len() + 1
        || t.before_rules.contains(&t.rule)
        || !t.after_rules.contains(&t.rule)
        || t.after_rules.iter().collect::<BTreeSet<_>>().len() != t.after_rules.len()
        || !t.before_rules.iter().all(|r| t.after_rules.contains(r))
        || t.baseline.guidance_sha256 != hash(render(&t.before_rules).as_bytes())
    {
        return Err(error("invalid_trial"));
    }
    validate_sample(&t.baseline, &t.model_context_sha256, Utc::now())?;
    if time(&t.applied_at_utc)? < time(&t.baseline.period_end_utc)?
        || time(&t.applied_at_utc)? > Utc::now()
        || t.before_rules.windows(2).any(|pair| pair[0] >= pair[1])
        || t.after_rules.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(error("invalid_trial"));
    }
    Ok(t)
}
fn restore(root: &Path, t: &mut Trial) -> Result<(), ContractError> {
    restore_with_writer(root, t, |path, contents| {
        atomic_write_private(path, contents).map_err(|_| error("restore_write_failed"))
    })
}
fn restore_with_writer(
    root: &Path,
    t: &mut Trial,
    mut write: impl FnMut(&Path, &[u8]) -> Result<(), ContractError>,
) -> Result<(), ContractError> {
    let current = current_rules(root)?;
    if current != t.after_rules
        && !(matches!(t.status.as_str(), "prepared" | "restoring") && current == t.before_rules)
    {
        return Err(error("user_edited_guidance_preserved"));
    }
    if t.status != "restoring" {
        t.status = "restoring".into();
        write(
            &root.join("trial.json"),
            &serde_json::to_vec_pretty(t).map_err(|_| error("serialization_failed"))?,
        )?;
    }
    // Check again after persisting intent; owner edits must still win.
    let current = current_rules(root)?;
    if current != t.after_rules && current != t.before_rules {
        return Err(error("user_edited_guidance_preserved"));
    }
    write(
        &root.join("personal-guidance.md"),
        render(&t.before_rules).as_bytes(),
    )?;
    t.status = "rolled_back".into();
    write(
        &root.join("trial.json"),
        &serde_json::to_vec_pretty(t).map_err(|_| error("serialization_failed"))?,
    )
}
fn status(root: &Path) -> Result<Value, ContractError> {
    let root = state_root(root)?;
    let original = private_bytes(&root.join("trial.json"))?;
    let trial = parse_trial(&original)?;
    let (guidance_state, rollback_available) = match current_rules(&root) {
        Ok(current) if current == trial.before_rules => (
            "before_trial",
            matches!(
                trial.status.as_str(),
                "prepared" | "restoring" | "rolled_back"
            ),
        ),
        Ok(current) if current == trial.after_rules => {
            ("trial_guidance", trial.status != "rolled_back")
        }
        Ok(_) => ("other_generated_guidance", false),
        Err(err) if err.0 == "personal_user_edited_guidance_preserved" => ("user_edited", false),
        Err(err) => return Err(err),
    };
    // Read-only status does not create a lock. Detect a journal change and never
    // present a mixed observation as a stable snapshot; rollback rechecks locked.
    if private_bytes(&root.join("trial.json"))? != original {
        return Err(error("state_changed_during_read"));
    }
    let mut out = output("status");
    out["status"] = json!(if rollback_available {
        "AVAILABLE"
    } else {
        "BLOCKED"
    });
    out["trial_state"] = json!(trial.status);
    out["rule"] = json!(trial.rule);
    out["guidance_state"] = json!(guidance_state);
    out["rollback_available"] = json!(rollback_available);
    out["rollback_needed"] = json!(trial.status != "rolled_back");
    out["snapshot_atomic"] = json!(false);
    Ok(out)
}

pub fn run(command: Command) -> Result<Value, ContractError> {
    match command {
        Command::Status { state_dir, .. } => status(&state_dir),
        Command::Rollback { state_dir, .. } => {
            let root = state_root(&state_dir)?;
            let _lock = lock(&root)?;
            let mut t = load_trial(&root)?;
            if t.status == "rolled_back" {
                if current_rules(&root)? != t.before_rules {
                    return Err(error("user_edited_guidance_preserved"));
                }
                let mut out = output("rollback");
                out["status"] = json!("ROLLED_BACK");
                return Ok(out);
            }
            restore(&root, &mut t)?;
            let mut out = output("rollback");
            out["status"] = json!("ROLLED_BACK");
            out["mutation_performed"] = json!(true);
            Ok(out)
        }
    }
}
#[cfg(test)]
mod tests;
