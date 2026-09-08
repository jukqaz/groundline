//! Offline personal guidance trials. No inference, scheduling, or native config writes.
use chrono::{DateTime, Duration, Utc};
use clap::Subcommand;
use groundline_contracts::{ContractError, insights::WeeklyReport};
use groundline_runtime::local_file::{
    atomic_write_private, open_bounded_regular_file, open_or_create_private_lock,
    open_private_directory, private_for_current_user,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_STATE_FILES: usize = 128;
const MIN_UNITS: usize = 10;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect fresh model evidence, Insights and native audit; optionally trial one rule.
    Review {
        #[arg(long)]
        report: PathBuf,
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        model_evidence: PathBuf,
        #[arg(long)]
        catalog: PathBuf,
        #[arg(long)]
        outcomes: Option<PathBuf>,
        #[arg(long)]
        state_dir: Option<PathBuf>,
        /// Change only the generated personal-guidance.md in the explicit private directory.
        #[arg(long, requires = "state_dir")]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Compare disjoint, comparable outcomes and retain or restore the trial guidance.
    Evaluate {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        outcomes: PathBuf,
        #[arg(long)]
        model_evidence: PathBuf,
        #[arg(long)]
        catalog: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Restore only unchanged GroundLine-generated guidance, including interrupted trials.
    Rollback {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Rule {
    ApprovalContinuity,
    DiagnoseBeforeRetry,
}
impl Rule {
    fn instruction(self) -> &'static str {
        match self {
            Self::ApprovalContinuity => {
                "Preserve the active task, completed work, and existing scoped approval across follow-ups. Continue authorized work; ask again only when a material choice or new authority is required."
            }
            Self::DiagnoseBeforeRetry => {
                "After a failed operation, identify the cause or a changed condition before retrying. Treat environment failures separately from code failures. Keep transient retries and verification bounded; reuse passing evidence until a relevant change or unresolved risk requires another check."
            }
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Source {
    applies_to_model: String,
    url: String,
    sha256: String,
    checked_at_utc: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelEvidence {
    kind: String,
    schema: u8,
    checked_at_utc: String,
    runtime_version: String,
    runtime_family: String,
    selected_model: String,
    selected_effort: String,
    latest_reference_model: String,
    catalog_sha256: String,
    official_sources: Vec<Source>,
    behavior_focus: Vec<Rule>,
}
#[derive(Deserialize)]
struct Catalog {
    models: Vec<Model>,
}
#[derive(Deserialize)]
struct Model {
    slug: String,
    supported_reasoning_levels: Vec<Effort>,
}
#[derive(Deserialize)]
struct Effort {
    effort: String,
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
    activation_verified: bool,
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
fn label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn time(s: &str) -> Result<DateTime<Utc>, ContractError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| error("invalid_timestamp"))
}
fn fresh(s: &str, now: DateTime<Utc>, age: Duration) -> Result<(), ContractError> {
    let t = time(s)?;
    if t > now + Duration::minutes(5) || now - t > age {
        return Err(error("stale_evidence"));
    }
    Ok(())
}
fn bytes(path: &Path) -> Result<Vec<u8>, ContractError> {
    let mut file =
        open_bounded_regular_file(path, 1, MAX_BYTES).map_err(|_| error("invalid_input_file"))?;
    let mut out = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut out)
        .map_err(|_| error("input_unavailable"))?;
    if out.len() as u64 > MAX_BYTES {
        return Err(error("input_too_large"));
    }
    Ok(out)
}
fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ContractError> {
    serde_json::from_slice(&bytes(path)?).map_err(|_| error("invalid_input"))
}
fn output(operation: &str) -> Value {
    json!({"kind":"groundline-personal-improvement","schema":1,"operation":operation,"status":"PASS",
        "network_performed":false,"mutation_performed":false,"raw_content_emitted":false,
        "private_paths_emitted":false,"model_changed":false,"permissions_changed":false,
        "evidence_origin":"operator_supplied","causal_improvement_claimed":false})
}
fn model_context(
    e: &ModelEvidence,
    catalog_bytes: &[u8],
    now: DateTime<Utc>,
) -> Result<String, ContractError> {
    if e.kind != "groundline-model-evidence"
        || e.schema != 1
        || !["codex_app", "codex_cli"].contains(&e.runtime_family.as_str())
        || ![
            &e.runtime_version,
            &e.selected_model,
            &e.selected_effort,
            &e.latest_reference_model,
        ]
        .iter()
        .all(|s| label(s))
        || e.catalog_sha256 != hash(catalog_bytes)
        || e.official_sources.is_empty()
        || e.official_sources.len() > 4
        || e.behavior_focus.len() > 2
        || e.behavior_focus.iter().collect::<BTreeSet<_>>().len() != e.behavior_focus.len()
    {
        return Err(error("invalid_model_evidence"));
    }
    fresh(&e.checked_at_utc, now, Duration::hours(24))?;
    let catalog: Catalog =
        serde_json::from_slice(catalog_bytes).map_err(|_| error("invalid_catalog"))?;
    if catalog.models.is_empty()
        || catalog.models.len() > 512
        || catalog
            .models
            .iter()
            .map(|m| &m.slug)
            .collect::<BTreeSet<_>>()
            .len()
            != catalog.models.len()
        || !catalog.models.iter().any(|m| {
            m.slug == e.selected_model
                && m.supported_reasoning_levels
                    .iter()
                    .any(|r| r.effort == e.selected_effort)
        })
    {
        return Err(error("model_effort_not_available"));
    }
    let mut sources = BTreeSet::new();
    for source in &e.official_sources {
        let url = url::Url::parse(&source.url).map_err(|_| error("unofficial_source"))?;
        if url.scheme() != "https"
            || ![
                "developers.openai.com",
                "platform.openai.com",
                "learn.chatgpt.com",
            ]
            .contains(&url.host_str().unwrap_or(""))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.port().is_some()
            || !label(&source.applies_to_model)
            || !valid_hash(&source.sha256)
            || !sources.insert(&source.url)
        {
            return Err(error("unofficial_source"));
        }
        fresh(&source.checked_at_utc, now, Duration::days(7))?;
    }
    if ![&e.selected_model, &e.latest_reference_model]
        .iter()
        .all(|model| {
            e.official_sources
                .iter()
                .any(|source| &source.applies_to_model == *model)
        })
    {
        return Err(error("model_guidance_missing"));
    }
    // Fetch dates and unselected models do not change the selected execution cohort.
    Ok(hash(
        serde_json::to_string(
            &json!({"runtime":e.runtime_version,"family":e.runtime_family,
        "model":e.selected_model,"effort":e.selected_effort}),
        )
        .unwrap()
        .as_bytes(),
    ))
}
fn validate_sample(s: &Sample, context: &str, now: DateTime<Utc>) -> Result<(), ContractError> {
    let start = time(&s.period_start_utc)?;
    let end = time(&s.period_end_utc)?;
    if s.kind != "groundline-outcome-sample"
        || s.schema != 1
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
    }
    Ok(())
}
fn sufficient(s: &Sample) -> bool {
    s.units.len() >= MIN_UNITS && s.units.iter().all(|u| u.outcome != Outcome::Unknown)
}
fn metrics(s: &Sample) -> Value {
    let n = s.units.len() as f64;
    let rate = |v: u64| {
        if n == 0.0 {
            Value::Null
        } else {
            json!(v as f64 / n)
        }
    };
    let sum = |f: fn(&Unit) -> u64| s.units.iter().map(f).sum::<u64>();
    json!({"unit_count":s.units.len(),"verified_rate":rate(sum(|u| u64::from(u.outcome==Outcome::Verified))),
        "unknown_count":sum(|u| u64::from(u.outcome==Outcome::Unknown)),"rework_rate":rate(sum(|u| u64::from(u.rework))),
        "redundant_approvals_per_unit":rate(sum(|u| u64::from(u.redundant_approval_count))),
        "continuation_prompts_per_unit":rate(sum(|u| u64::from(u.continuation_prompt_count))),
        "repeated_calls_per_unit":rate(sum(|u| u64::from(u.repeated_call_count))),
        "tool_calls_per_unit":rate(sum(|u| u64::from(u.tool_call_count))),
        "tokens_per_unit":if s.units.iter().all(|u| u.total_tokens.is_some()) {rate(sum(|u| u.total_tokens.unwrap_or(0)))} else {Value::Null},
        "wall_duration_ms_per_unit":rate(sum(|u| (time(&u.completed_at_utc).unwrap()-time(&u.started_at_utc).unwrap()).num_milliseconds() as u64)),
        "wall_duration_is_model_latency":false})
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
fn select_rule(e: &ModelEvidence, s: Option<&Sample>, audit: &Value) -> Option<Rule> {
    let mut candidates = Vec::new();
    if let Some(s) = s {
        if s.units
            .iter()
            .any(|u| u.redundant_approval_count > 0 || u.continuation_prompt_count > 0)
        {
            candidates.push(Rule::ApprovalContinuity);
        }
        if s.units.iter().any(|u| u.repeated_call_count > 0) {
            candidates.push(Rule::DiagnoseBeforeRetry);
        }
    }
    let calls = audit
        .get("root")
        .and_then(|root| root.pointer("/tools/call_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let repeated = audit
        .get("root")
        .and_then(|root| root.pointer("/tools/calls_in_exact_repeated_groups"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if calls > 0 && repeated as f64 / calls as f64 >= 0.10 {
        candidates.push(Rule::DiagnoseBeforeRetry);
    }
    candidates
        .into_iter()
        .find(|r| e.behavior_focus.contains(r))
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
    for entry in fs::read_dir(root)
        .map_err(|_| error("state_unavailable"))?
        .take(MAX_STATE_FILES + 1)
    {
        entry.map_err(|_| error("state_unavailable"))?;
        count += 1;
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
    let all = [Rule::ApprovalContinuity, Rule::DiagnoseBeforeRetry];
    for bits in 0..4 {
        let rules: Vec<_> = all
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
    let t: Trial = serde_json::from_slice(data).map_err(|_| error("invalid_trial"))?;
    if t.kind != "groundline-personal-trial"
        || t.schema != 1
        || !["prepared", "pending", "retained", "rolled_back"].contains(&t.status.as_str())
        || t.before_rules.len() > 1
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
        || t.after_rules.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(error("invalid_trial"));
    }
    Ok(t)
}
fn check_baseline_reuse(
    trial: &Trial,
    rule: Rule,
    ids: &BTreeSet<&str>,
) -> Result<(), ContractError> {
    if trial.rule == rule
        && trial
            .baseline
            .units
            .iter()
            .any(|u| ids.contains(u.unit_hash.as_str()))
    {
        return Err(error("candidate_already_reviewed"));
    }
    Ok(())
}
fn check_archived_baselines(
    root: &Path,
    rule: Rule,
    ids: &BTreeSet<&str>,
) -> Result<(), ContractError> {
    for (index, entry) in fs::read_dir(root)
        .map_err(|_| error("state_unavailable"))?
        .take(MAX_STATE_FILES + 1)
        .enumerate()
    {
        if index == MAX_STATE_FILES {
            return Err(error("state_archive_limit"));
        }
        let entry = entry.map_err(|_| error("state_unavailable"))?;
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("trial-") {
            continue;
        }
        let data = private_bytes(&entry.path())?;
        if name != format!("trial-{}.json", hash(&data)).as_str() {
            return Err(error("archive_mismatch"));
        }
        let trial = parse_trial(&data)?;
        if !matches!(trial.status.as_str(), "retained" | "rolled_back") {
            return Err(error("invalid_trial"));
        }
        check_baseline_reuse(&trial, rule, ids)?;
    }
    Ok(())
}
fn archive(root: &Path, name: &str, data: &[u8]) -> Result<(), ContractError> {
    let path = root.join(format!("{name}-{}.json", hash(data)));
    match fs::symlink_metadata(&path) {
        Ok(_) => {
            if private_bytes(&path)? != data {
                return Err(error("archive_mismatch"));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            ensure_capacity(root, std::slice::from_ref(&path))?;
            atomic_write_private(&path, data).map_err(|_| error("archive_failed"))?;
        }
        Err(_) => return Err(error("archive_failed")),
    }
    Ok(())
}
fn apply(
    root: &Path,
    rule: Rule,
    s: &Sample,
    context: &str,
    now: DateTime<Utc>,
) -> Result<Value, ContractError> {
    let root = state_root(root)?;
    let _lock = lock(&root)?;
    let ids: BTreeSet<_> = s.units.iter().map(|u| u.unit_hash.as_str()).collect();
    let previous = match fs::symlink_metadata(root.join("trial.json")) {
        Ok(_) => {
            let data = private_bytes(&root.join("trial.json"))?;
            let old = parse_trial(&data)?;
            if matches!(old.status.as_str(), "prepared" | "pending") {
                return Err(error("trial_already_active"));
            }
            check_baseline_reuse(&old, rule, &ids)?;
            Some(data)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(error("state_unavailable")),
    };
    check_archived_baselines(&root, rule, &ids)?;
    let before = current_rules(&root)?;
    if before.contains(&rule) {
        return Err(error("candidate_already_applied"));
    }
    if s.guidance_sha256 != hash(render(&before).as_bytes()) {
        return Err(error("baseline_guidance_mismatch"));
    }
    let mut additions = vec![root.join("trial.json"), root.join("personal-guidance.md")];
    if let Some(data) = &previous {
        additions.push(root.join(format!("trial-{}.json", hash(data))));
    }
    ensure_capacity(&root, &additions)?;
    if let Some(data) = previous {
        archive(&root, "trial", &data)?;
    }
    let mut after = before.clone();
    after.push(rule);
    after.sort();
    let mut trial = Trial {
        kind: "groundline-personal-trial".into(),
        schema: 1,
        status: "prepared".into(),
        applied_at_utc: now.to_rfc3339(),
        rule,
        before_rules: before.clone(),
        after_rules: after.clone(),
        baseline: s.clone(),
        model_context_sha256: context.into(),
    };
    save_trial(&root, &trial)?;
    if current_rules(&root)? != before {
        return Err(error("user_edited_guidance_preserved"));
    }
    atomic_write_private(
        &root.join("personal-guidance.md"),
        render(&after).as_bytes(),
    )
    .map_err(|_| error("guidance_write_failed"))?;
    trial.status = "pending".into();
    save_trial(&root, &trial)?;
    Ok(
        json!({"state":"pending","guidance_sha256":hash(render(&after).as_bytes()),"native_activation":"UNVERIFIED"}),
    )
}
struct ReviewInputs<'a> {
    report: &'a Path,
    audit: &'a Path,
    outcomes: Option<&'a Path>,
    state_dir: Option<&'a Path>,
    apply: bool,
}
fn review(
    input: ReviewInputs<'_>,
    e: &ModelEvidence,
    catalog: &[u8],
) -> Result<Value, ContractError> {
    let now = Utc::now();
    let context = model_context(e, catalog, now)?;
    let report = WeeklyReport::from_slice(&bytes(input.report)?)
        .map_err(|_| error("invalid_insights_report"))?;
    let audit: Value = read(input.audit)?;
    groundline_contracts::efficiency::recommend_weekly_optimization(&audit)?;
    let sample: Option<Sample> = input.outcomes.map(read).transpose()?;
    if let Some(s) = &sample {
        validate_sample(s, &context, now)?;
    }
    let rule = select_rule(e, sample.as_ref(), &audit).or_else(|| {
        let workflow = &report.weekly_metrics.workflow;
        (workflow.repeated_call_rate.is_some_and(|rate| rate >= 0.10)
            && e.behavior_focus.contains(&Rule::DiagnoseBeforeRetry))
        .then_some(Rule::DiagnoseBeforeRetry)
    });
    let mut reasons = Vec::new();
    if report.data_quality.status != "PASS" || report.collection_health.freshness_status != "FRESH"
    {
        reasons.push("insights_quality_or_freshness");
    }
    fresh(&report.generated_at_utc, now, Duration::hours(24))?;
    if audit["status"] != "PASS" {
        reasons.push("native_audit_not_complete");
    }
    fresh(
        audit
            .pointer("/scope/generated_at")
            .and_then(Value::as_str)
            .ok_or_else(|| error("audit_timestamp_required"))?,
        now,
        Duration::hours(24),
    )?;
    if sample.as_ref().is_none_or(|s| !sufficient(s)) {
        reasons.push("verified_outcomes_required");
    }
    if let Some(s) = &sample {
        fresh(&s.period_end_utc, now, Duration::days(7))?;
        if s.guidance_sha256 != hash(b"") && !s.activation_verified {
            reasons.push("baseline_activation_unverified");
        }
        if rule.is_some_and(|r| {
            !s.units.iter().any(|u| match r {
                Rule::ApprovalContinuity => {
                    u.redundant_approval_count > 0 || u.continuation_prompt_count > 0
                }
                Rule::DiagnoseBeforeRetry => u.repeated_call_count > 0,
            })
        }) {
            reasons.push("candidate_signal_missing_in_outcomes");
        }
    }
    if rule.is_none() {
        reasons.push("no_supported_candidate");
    }
    let mut out = output("review");
    out["status"] = json!(if reasons.is_empty() {
        "READY"
    } else {
        "OBSERVE"
    });
    out["reason_codes"] = json!(reasons);
    out["model_context_sha256"] = json!(context);
    out["model_evidence_fresh"] = json!(true);
    out["latest_model_independently_verified"] = json!(false);
    out["insights"] = json!({"requested_days":report.requested_days,"event_count":report.coverage.event_count,
        "root_observations":report.coverage.observed_root_count,"unique_task_count_available":false,
        "quality":report.data_quality.status,"workflow":report.weekly_metrics.workflow,
        "tokens":report.weekly_metrics.tokens,"verification":report.weekly_metrics.verification,"model_effort_tokens_available":false});
    out["outcomes"] = sample.as_ref().map(metrics).unwrap_or(Value::Null);
    out["candidate"] = rule
        .map(|r| json!({"rule":r,"instruction":r.instruction()}))
        .unwrap_or(Value::Null);
    out["automatic_application_eligible"] = json!(reasons.is_empty());
    if input.apply && reasons.is_empty() {
        out["trial"] = apply(
            input
                .state_dir
                .ok_or_else(|| error("state_directory_required"))?,
            rule.unwrap(),
            sample.as_ref().unwrap(),
            &context,
            now,
        )?;
        out["mutation_performed"] = json!(true);
    }
    Ok(out)
}
fn restore(root: &Path, t: &mut Trial) -> Result<(), ContractError> {
    let current = current_rules(root)?;
    if current != t.after_rules && !(t.status == "prepared" && current == t.before_rules) {
        return Err(error("user_edited_guidance_preserved"));
    }
    atomic_write_private(
        &root.join("personal-guidance.md"),
        render(&t.before_rules).as_bytes(),
    )
    .map_err(|_| error("guidance_write_failed"))?;
    t.status = "rolled_back".into();
    save_trial(root, t)
}
fn evaluate(
    root: &Path,
    s: Sample,
    e: &ModelEvidence,
    catalog: &[u8],
) -> Result<Value, ContractError> {
    let now = Utc::now();
    let context = model_context(e, catalog, now)?;
    validate_sample(&s, &context, now)?;
    fresh(&s.period_end_utc, now, Duration::days(7))?;
    let root = state_root(root)?;
    let _lock = lock(&root)?;
    let mut t = load_trial(&root)?;
    if t.status != "pending" {
        return Err(error("pending_trial_required"));
    }
    if current_rules(&root)? != t.after_rules {
        return Err(error("user_edited_guidance_preserved"));
    }
    let mut out = output("evaluate");
    let mut reasons = Vec::new();
    if context != t.model_context_sha256
        || s.task_kind != t.baseline.task_kind
        || s.scope_size != t.baseline.scope_size
        || s.comparison_context_sha256 != t.baseline.comparison_context_sha256
    {
        reasons.push("cohort_mismatch");
    }
    if time(&s.period_start_utc)? < time(&t.applied_at_utc)?
        || time(&s.period_start_utc)? < time(&t.baseline.period_end_utc)?
    {
        reasons.push("overlapping_or_pretrial_period");
    }
    let ids: BTreeSet<_> = t.baseline.units.iter().map(|u| &u.unit_hash).collect();
    if s.units.iter().any(|u| ids.contains(&u.unit_hash)) {
        reasons.push("reused_outcome_units");
    }
    if s.guidance_sha256 != hash(render(&t.after_rules).as_bytes()) || !s.activation_verified {
        reasons.push("native_activation_unverified");
    }
    if !sufficient(&s) || !sufficient(&t.baseline) {
        reasons.push("verified_outcomes_required");
    }
    let before = metrics(&t.baseline);
    let after = metrics(&s);
    out["baseline"] = before.clone();
    out["candidate"] = after.clone();
    if !reasons.is_empty() {
        out["status"] = json!("INCONCLUSIVE");
        out["reason_codes"] = json!(reasons);
        return Ok(out);
    }
    let v = |m: &Value, k: &str| m[k].as_f64().unwrap();
    let quality_regression = v(&after, "verified_rate") < v(&before, "verified_rate")
        || v(&after, "rework_rate") > v(&before, "rework_rate");
    let intervention_regression = v(&after, "redundant_approvals_per_unit")
        > v(&before, "redundant_approvals_per_unit")
        || v(&after, "continuation_prompts_per_unit") > v(&before, "continuation_prompts_per_unit");
    let resource_regression = v(&after, "wall_duration_ms_per_unit")
        > v(&before, "wall_duration_ms_per_unit")
        || after["tokens_per_unit"]
            .as_f64()
            .zip(before["tokens_per_unit"].as_f64())
            .is_some_and(|(a, b)| a > b);
    let primary = match t.rule {
        Rule::ApprovalContinuity => "redundant_approvals_per_unit",
        Rule::DiagnoseBeforeRetry => "repeated_calls_per_unit",
    };
    let improves = v(&after, primary) < v(&before, primary)
        || (t.rule == Rule::ApprovalContinuity
            && v(&after, "continuation_prompts_per_unit")
                < v(&before, "continuation_prompts_per_unit"));
    archive(
        &root,
        "evaluation",
        &serde_json::to_vec(&json!({"sample":s,"baseline":before,"candidate":after})).unwrap(),
    )?;
    if quality_regression || intervention_regression || resource_regression || !improves {
        restore(&root, &mut t)?;
        out["status"] = json!("ROLLED_BACK");
        out["reason_codes"] = json!([if quality_regression {
            "quality_regression"
        } else if intervention_regression {
            "user_intervention_regression"
        } else if resource_regression {
            "resource_regression"
        } else {
            "no_observed_benefit"
        }]);
    } else {
        t.status = "retained".into();
        save_trial(&root, &t)?;
        out["status"] = json!("RETAINED");
        out["reason_codes"] = json!(["observed_association_not_causation"]);
    }
    out["mutation_performed"] = json!(true);
    Ok(out)
}
pub fn run(command: Command) -> Result<Value, ContractError> {
    match command {
        Command::Review {
            report,
            audit,
            model_evidence,
            catalog,
            outcomes,
            state_dir,
            apply,
            ..
        } => review(
            ReviewInputs {
                report: &report,
                audit: &audit,
                outcomes: outcomes.as_deref(),
                state_dir: state_dir.as_deref(),
                apply,
            },
            &read(&model_evidence)?,
            &bytes(&catalog)?,
        ),
        Command::Evaluate {
            state_dir,
            outcomes,
            model_evidence,
            catalog,
            ..
        } => evaluate(
            &state_dir,
            read(&outcomes)?,
            &read(&model_evidence)?,
            &bytes(&catalog)?,
        ),
        Command::Rollback { state_dir, .. } => {
            let root = state_root(&state_dir)?;
            let _lock = lock(&root)?;
            let mut t = load_trial(&root)?;
            if t.status == "rolled_back" {
                return Err(error("trial_already_rolled_back"));
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
