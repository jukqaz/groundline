//! Opt-in, bounded native-boundary consumption. This is an offline state
//! machine, not a model caller, scheduler, or hook-derived quality oracle.
use super::*;
use groundline_contracts::learning::continuous as contract;

mod native_artifact;
mod storage;
#[path = "work.rs"]
mod work;
pub(super) use storage::{archived_record, load_archive_references};
pub(super) use work::assess;

fn codex_home(value: Option<&Path>) -> Result<PathBuf, ContractError> {
    absolute(
        &value
            .map(Path::to_path_buf)
            .map(Ok)
            .unwrap_or_else(crate::default_codex_home)?,
    )
}

fn profile_path(home: &Path) -> PathBuf {
    home.join("groundline/learning/profile.json")
}

fn ensure_private_directory(path: &Path) -> Result<(), ContractError> {
    if fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        let parent = path.parent().ok_or_else(|| error("invalid_path"))?;
        let parent = directory_handle(parent, false)?;
        if parent
            .metadata()
            .map_err(|_| error("invalid_directory"))?
            .uid()
            != rustix::process::geteuid().as_raw()
        {
            return Err(error("private_directory_required"));
        }
        match mkdirat(
            &parent,
            path.file_name().ok_or_else(|| error("invalid_path"))?,
            Mode::from_raw_mode(0o700),
        ) {
            Ok(()) | Err(rustix::io::Errno::EXIST) => {}
            Err(_) => return Err(error("directory_unavailable")),
        }
        parent
            .sync_all()
            .map_err(|_| error("directory_unavailable"))?;
    }
    directory_handle(path, true)?;
    Ok(())
}

fn profile_root(home: &Path) -> Result<PathBuf, ContractError> {
    directory_handle(home, false)?;
    let groundline = home.join("groundline");
    ensure_private_directory(&groundline)?;
    let root = groundline.join("learning");
    ensure_private_directory(&root)?;
    Ok(root)
}

fn profile(
    home: &Path,
    required: bool,
) -> Result<Option<contract::LearningProfile>, ContractError> {
    let path = profile_path(home);
    if fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return if required {
            Err(error("profile_required"))
        } else {
            Ok(None)
        };
    }
    let value = parse(&read_private(&path)?)?;
    let value: contract::LearningProfile =
        serde_json::from_value(value).map_err(|_| error("invalid_profile"))?;
    value.validate()?;
    Ok(Some(value))
}

fn enabled_profile(home: &Path) -> Result<contract::LearningProfile, ContractError> {
    let profile = profile(home, true)?.ok_or_else(|| error("profile_required"))?;
    if !profile.enabled {
        return Err(error("profile_disabled"));
    }
    Ok(profile)
}

fn executable_pin(path: &Path) -> Result<String, ContractError> {
    let parent = directory_handle(path.parent().ok_or_else(|| error("invalid_path"))?, false)?;
    let mut file = File::from(
        openat(
            &parent,
            path.file_name().ok_or_else(|| error("invalid_path"))?,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("invalid_core_executable"))?,
    );
    let before = file
        .metadata()
        .map_err(|_| error("invalid_core_executable"))?;
    if !before.is_file()
        || before.nlink() != 1
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.mode() & 0o111 == 0
        || before.mode() & 0o022 != 0
        || before.len() > 256 * 1024 * 1024
    {
        return Err(error("invalid_core_executable"));
    }
    let mut digest = Sha256::new();
    let mut remaining = 256 * 1024 * 1024 + 1_u64;
    let mut buffer = [0_u8; 65536];
    while remaining > 0 {
        let count = file
            .read(&mut buffer)
            .map_err(|_| error("invalid_core_executable"))?;
        if count == 0 {
            break;
        }
        remaining = remaining.saturating_sub(count as u64);
        digest.update(&buffer[..count]);
    }
    let after = file
        .metadata()
        .map_err(|_| error("invalid_core_executable"))?;
    if remaining == 0
        || before.len() != after.len()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || after.nlink() != 1
    {
        return Err(error("core_executable_changed"));
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_profile(home: &Path, profile: &contract::LearningProfile) -> Result<bool, ContractError> {
    profile.validate()?;
    let path = profile_path(home);
    let root = profile_root(home)?;
    let directory = directory_handle(&root, true)?;
    let lock = File::from(
        openat(
            &directory,
            ".profile.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| error("invalid_profile_lock"))?,
    );
    let metadata = lock.metadata().map_err(|_| error("invalid_profile_lock"))?;
    if !metadata.is_file() || metadata.nlink() != 1 || !private_for_current_user(&lock) {
        return Err(error("invalid_profile_lock"));
    }
    lock.try_lock().map_err(|_| error("profile_busy"))?;
    let _guard = LockGuard(lock);
    let mut bytes =
        serde_json::to_vec_pretty(profile).map_err(|_| error("serialization_failed"))?;
    bytes.push(b'\n');
    let previous = match openat(
        &directory,
        "profile.json",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(fd) => Some(read_handle(File::from(fd))?),
        Err(rustix::io::Errno::NOENT) => None,
        Err(_) => return Err(error("invalid_profile")),
    };
    if let Some(previous) = &previous {
        let previous_profile: contract::LearningProfile =
            serde_json::from_slice(previous).map_err(|_| error("invalid_profile"))?;
        previous_profile.validate()?;
        if *previous == bytes {
            return Ok(false);
        }
    }
    let temporary = format!(".profile-{}.tmp", uuid::Uuid::new_v4());
    let mut file = File::from(
        openat(
            &directory,
            temporary.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| error("output_unavailable"))?,
    );
    let result = (|| {
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| error("output_write_failed"))?;
        if !binding_matches(&root, &directory) {
            return Err(error("state_binding_changed"));
        }
        if let Some(previous) = &previous {
            if read_at(&directory, std::ffi::OsStr::new("profile.json"))? != *previous {
                return Err(error("profile_changed"));
            }
            rustix::fs::renameat(&directory, temporary.as_str(), &directory, "profile.json")
                .map_err(|_| error("output_write_failed"))?;
        } else {
            linkat(
                &directory,
                temporary.as_str(),
                &directory,
                "profile.json",
                AtFlags::empty(),
            )
            .map_err(|_| error("profile_changed"))?;
            unlinkat(&directory, temporary.as_str(), AtFlags::empty())
                .map_err(|_| error("output_write_failed"))?;
        }
        directory
            .sync_all()
            .map_err(|_| error("output_write_failed"))?;
        if !binding_matches(&root, &directory) || read_private(&path)? != bytes {
            return Err(error("profile_changed"));
        }
        Ok(true)
    })();
    let _ = unlinkat(&directory, temporary.as_str(), AtFlags::empty());
    result
}

pub(super) fn configure(
    environment_state: &Path,
    target_ids: &[String],
    device_id: &str,
    home: Option<&Path>,
    state: Option<&Path>,
    deliveries: Option<&Path>,
) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let root = profile_root(&home)?;
    let environment_state = absolute(environment_state)?;
    // Context is checked before taking a learning-state lock (env → learning).
    for target in target_ids {
        crate::environment::learning_context(&environment_state, target)?;
    }
    let learning_state = state
        .map(absolute)
        .transpose()?
        .unwrap_or_else(|| root.join("records"));
    let deliveries = deliveries
        .map(absolute)
        .transpose()?
        .unwrap_or_else(|| root.join("deliveries"));
    ensure_private_directory(&learning_state)?;
    ensure_private_directory(&deliveries)?;
    ensure_private_directory(&root.join("boundaries"))?;
    let executable = std::env::current_exe().map_err(|_| error("invalid_core_executable"))?;
    let profile = contract::LearningProfile {
        kind: "groundline-learning-profile".into(),
        schema: 1,
        enabled: false,
        environment_state: environment_state
            .to_str()
            .ok_or_else(|| error("invalid_path"))?
            .into(),
        learning_state: learning_state
            .to_str()
            .ok_or_else(|| error("invalid_path"))?
            .into(),
        deliveries: deliveries
            .to_str()
            .ok_or_else(|| error("invalid_path"))?
            .into(),
        target_ids: target_ids.to_vec(),
        device_id: device_id.into(),
        core_executable: executable
            .to_str()
            .ok_or_else(|| error("invalid_path"))?
            .into(),
        core_sha256: executable_pin(&executable)?,
    };
    profile.validate()?;
    State::open(&learning_state, true)?;
    let changed = write_profile(&home, &profile)?;
    let mut out = readout("CONFIGURED", changed);
    out["enabled"] = json!(false);
    out["profile_sha256"] = json!(learning::content_sha256(
        &serde_json::to_value(profile).map_err(|_| error("serialization_failed"))?
    )?);
    Ok(out)
}

pub(super) fn enable(home: Option<&Path>, enabled: bool) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let mut profile = profile(&home, true)?.ok_or_else(|| error("profile_required"))?;
    if enabled {
        if executable_pin(Path::new(&profile.core_executable))? != profile.core_sha256 {
            return Err(error("core_pin_mismatch"));
        }
        for target in &profile.target_ids {
            crate::environment::learning_context(Path::new(&profile.environment_state), target)?;
        }
    }
    profile.enabled = enabled;
    let changed = write_profile(&home, &profile)?;
    let mut out = readout(if enabled { "ENABLED" } else { "DISABLED" }, changed);
    out["enabled"] = json!(enabled);
    Ok(out)
}

fn candidate<'a>(records: &'a [Value], proposal_id: &str) -> Result<&'a Value, ContractError> {
    records
        .iter()
        .find(|v| {
            v["kind"] == "groundline-learning-proposal"
                && v["status"] == "candidate"
                && v["proposal_id"] == proposal_id
        })
        .ok_or_else(|| error("candidate_missing"))
}

pub(crate) fn authorize_application(
    state: &Path,
    proposal_id: &str,
    plan_sha256: &str,
    intent: &str,
) -> Result<Value, ContractError> {
    if !["trial", "adoption"].contains(&intent) || !learning::digest(plan_sha256) {
        return Err(error("application_intent_required"));
    }
    let state = State::open(state, false)?;
    let value = candidate(&state.records, proposal_id)?;
    if value["plan_sha256"] != plan_sha256 {
        return Err(error("application_plan_mismatch"));
    }
    let proposal_sha256 = learning::content_sha256(value)?;
    let head = learning::decision_head(&state.records, &proposal_sha256)?;
    let decision = head.as_ref().and_then(|h| {
        state
            .records
            .iter()
            .find(|v| learning::content_sha256(v).ok().as_ref() == Some(h))
    });
    if decision.is_some_and(|v| ["hold", "reject"].contains(&v["decision"].as_str().unwrap_or("")))
    {
        return Err(error("application_decision_blocked"));
    }
    let authorization = if intent == "trial" {
        state
            .records
            .iter()
            .find(|v| {
                v["kind"] == "groundline-learning-trial-authorization"
                    && v["proposal_sha256"] == proposal_sha256
                    && v["plan_sha256"] == plan_sha256
                    && v["scope_sha256"] == value["scope_sha256"]
                    && v["rollback_sha256"] == value["target"]["rollback_ref"]
                    && v["authority_ref"] == value["target"]["authority_ref"]
            })
            .ok_or_else(|| error("explicit_trial_authorization_required"))?
    } else {
        let decision = decision
            .filter(|v| v["decision"] == "adopt")
            .ok_or_else(|| error("adoption_decision_required"))?;
        let evaluation = state
            .records
            .iter()
            .find(|v| {
                learning::content_sha256(v).ok().as_deref()
                    == decision["evaluation_sha256"].as_str()
                    && v["kind"] == "groundline-learning-evaluation-record"
                    && v["proposal_sha256"] == proposal_sha256
                    && v["input"]["proposal_revision"] == plan_sha256
            })
            .ok_or_else(|| error("adoption_evaluation_required"))?;
        if evaluation["result"]["status"] != "NO_REGRESSION_OBSERVED"
            || evaluation["result"]["analysis_attempt_resources"]["complete"] != true
        {
            return Err(error("adoption_evaluation_inconclusive_or_regressed"));
        }
        evaluation
    };
    Ok(
        json!({"kind":"groundline-learning-application-authorization","schema":1,"intent":intent,
        "proposal_sha256":proposal_sha256,"plan_sha256":plan_sha256,"authorization_sha256":learning::content_sha256(authorization)?,
        "effect_verified":false,"explicit_scope":true,"rollback_preserved":true}),
    )
}

pub(crate) fn default_application_check(
    environment_state: &Path,
    proposal_id: &str,
    plan_sha256: &str,
    learning_state: Option<&Path>,
    intent: Option<&str>,
) -> Result<Value, ContractError> {
    if let Some(state) = learning_state {
        return super::authorize_application(
            state,
            proposal_id,
            plan_sha256,
            intent.ok_or_else(|| error("application_intent_required"))?,
        );
    }
    if intent.is_some() {
        return Err(error("application_state_required"));
    }
    let home = codex_home(None)?;
    if let Some(profile) = profile(&home, false)?
        && profile.enabled
        && absolute(environment_state)? == Path::new(&profile.environment_state)
    {
        let state = State::open(Path::new(&profile.learning_state), false)?;
        if candidate(&state.records, proposal_id).is_ok() {
            return Err(error("application_intent_required"));
        }
    }
    Ok(
        json!({"kind":"groundline-learning-application-authorization","schema":1,"intent":"unrelated_environment_change","effect_verified":false}),
    )
}

pub(crate) fn application_evaluations(
    state: &Path,
    operation: &Value,
    operation_sha256: &str,
) -> Result<Value, ContractError> {
    crate::environment::validate_operation(operation)?;
    if !learning::digest(operation_sha256) {
        return Err(error("invalid_operation"));
    }
    let state = State::open(state, false)?;
    let mut evaluations = Vec::new();
    for record in &state.records {
        if record["kind"] != "groundline-learning-evaluation-record"
            || record["operation_sha256"] != operation_sha256
            || record["input"]["proposal_revision"] != operation["plan_sha256"]
        {
            continue;
        }
        let proposal = candidate(
            &state.records,
            record["input"]["proposal_id"]
                .as_str()
                .ok_or_else(|| error("invalid_evaluation"))?,
        )?;
        let proposal: learning::Proposal =
            serde_json::from_value(proposal.clone()).map_err(|_| error("invalid_proposal"))?;
        if learning::operation_applies_to_proposal(&proposal, operation) {
            evaluations.push(json!({"evaluation_sha256":learning::content_sha256(record)?,"status":record["result"]["status"]}));
        }
    }
    evaluations.sort_by(|a, b| {
        a["evaluation_sha256"]
            .as_str()
            .cmp(&b["evaluation_sha256"].as_str())
    });
    Ok(json!({"evaluations":evaluations,"effect_verified":false}))
}

pub(super) fn authorize_trial(
    input: &Path,
    evidence: &Path,
    state: &Path,
) -> Result<Value, ContractError> {
    let value = parse(&read_private(input)?)?;
    let authorization: contract::TrialAuthorization =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_trial_authorization"))?;
    authorization.validate()?;
    if sha256(&read_private(evidence)?) != authorization.evidence_sha256 {
        return Err(error("trial_evidence_mismatch"));
    }
    let state = State::open(state, false)?;
    let changed = state.save(&value)?;
    let mut out = readout(
        if changed {
            "TRIAL_AUTHORIZED"
        } else {
            "ALREADY_AUTHORIZED"
        },
        changed,
    );
    out["authorization_sha256"] = json!(learning::content_sha256(&value)?);
    Ok(out)
}

pub(super) fn validate_record(value: &Value) -> Result<(), ContractError> {
    match value["kind"].as_str() {
        Some("groundline-learning-boundary") => {
            serde_json::from_value::<contract::Boundary>(value.clone())
                .map_err(|_| error("invalid_boundary"))?
                .validate()
        }
        Some("groundline-learning-trial-authorization") => {
            serde_json::from_value::<contract::TrialAuthorization>(value.clone())
                .map_err(|_| error("invalid_trial_authorization"))?
                .validate()
        }
        Some("groundline-learning-processing") => {
            serde_json::from_value::<contract::ProcessingRecord>(value.clone())
                .map_err(|_| error("invalid_processing_record"))?
                .validate()
        }
        Some("groundline-learning-assessment") => {
            serde_json::from_value::<contract::Assessment>(value.clone())
                .map_err(|_| error("invalid_assessment"))?
                .validate()
        }
        Some("groundline-learning-task-start") => {
            let record: contract::TaskStartRecord =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_task_start"))?;
            record.scope.validate()?;
            record.native.validate()?;
            if record.schema != 1
                || [
                    &record.input_sha256,
                    &record.boundary_sha256,
                    &record.snapshot_sha256,
                ]
                .into_iter()
                .any(|v| !learning::digest(v))
            {
                return Err(error("invalid_task_start"));
            }
            contract::TaskStartInput {
                kind: "groundline-learning-task-start-input".into(),
                schema: 1,
                scope: record.scope,
                boundary_id: "record".into(),
                criterion_change_kind: record.criterion_change_kind,
                criterion_change_evidence_sha256: record.criterion_change_evidence_sha256,
            }
            .validate()
        }
        Some("groundline-learning-task-outcome") => {
            let record: contract::TaskOutcome =
                serde_json::from_value(value.clone()).map_err(|_| error("invalid_task_outcome"))?;
            record.native.validate()?;
            if record.schema != 1
                || [
                    &record.input_sha256,
                    &record.task_sha256,
                    &record.boundary_sha256,
                    &record.receipt_sha256,
                    &record.link_sha256,
                ]
                .into_iter()
                .any(|v| !learning::digest(v))
                || record
                    .assessment_sha256
                    .as_ref()
                    .is_some_and(|v| !learning::digest(v))
                || record
                    .native_response_proof_sha256
                    .as_ref()
                    .is_some_and(|v| !learning::digest(v))
            {
                return Err(error("invalid_task_outcome"));
            }
            Ok(())
        }
        Some("groundline-learning-finalize-request") => {
            let record: contract::FinalizeRequest = serde_json::from_value(value.clone())
                .map_err(|_| error("invalid_finalize_request"))?;
            record.input.validate()?;
            record.native.validate()?;
            if record.schema != 1
                || [
                    &record.input_sha256,
                    &record.boundary_sha256,
                    &record.receipt_sha256,
                ]
                .into_iter()
                .any(|v| !learning::digest(v))
                || record
                    .assessment_sha256
                    .as_ref()
                    .is_some_and(|v| !learning::digest(v))
                || record
                    .native_response_proof_sha256
                    .as_ref()
                    .is_some_and(|v| !learning::digest(v))
            {
                return Err(error("invalid_finalize_request"));
            }
            Ok(())
        }
        _ => Err(error("unsupported_state")),
    }
}

pub(super) fn validate_collection(records: &[Value]) -> Result<(), ContractError> {
    let hashes: BTreeMap<_, _> = records
        .iter()
        .map(|v| Ok((learning::content_sha256(v)?, v)))
        .collect::<Result<_, ContractError>>()?;
    let mut tasks = BTreeMap::new();
    let mut outcomes = BTreeMap::new();
    let mut boundaries = BTreeMap::new();
    let mut requests = BTreeMap::new();
    let mut assessments = BTreeMap::new();
    for value in records {
        match value["kind"].as_str() {
            Some("groundline-learning-assessment") => {
                let task_hash = value["input"]["task_sha256"]
                    .as_str()
                    .ok_or_else(|| error("invalid_assessment"))?;
                if assessments.insert(task_hash, value).is_some() {
                    return Err(error("assessment_conflict"));
                }
                let task = hashes
                    .get(task_hash)
                    .copied()
                    .filter(|v| v["kind"] == "groundline-learning-task-start")
                    .ok_or_else(|| error("task_reference_missing"))?;
                let boundary = reference(
                    &hashes,
                    task,
                    "boundary_sha256",
                    "groundline-learning-boundary",
                )?;
                let snapshot = reference(
                    &hashes,
                    task,
                    "snapshot_sha256",
                    "groundline-learning-snapshot",
                )?;
                let submitted = contract::timestamp(
                    value["submitted_at_utc"]
                        .as_str()
                        .ok_or_else(|| error("invalid_assessment"))?,
                )?;
                if submitted
                    < contract::timestamp(
                        boundary["observed_at_utc"]
                            .as_str()
                            .ok_or_else(|| error("invalid_boundary"))?,
                    )?
                    || submitted
                        < contract::timestamp(
                            snapshot["captured_at_utc"]
                                .as_str()
                                .ok_or_else(|| error("invalid_snapshot"))?,
                        )?
                {
                    return Err(error("assessment_before_task"));
                }
            }
            Some("groundline-learning-boundary") => {
                let id = value["boundary_id"]
                    .as_str()
                    .ok_or_else(|| error("invalid_boundary"))?;
                if boundaries.insert(id, value).is_some() {
                    return Err(error("boundary_conflict"));
                }
            }
            Some("groundline-learning-task-start") => {
                let scope = &value["scope"];
                let key = (
                    scope["unit_hash"].to_string(),
                    scope["phase"].to_string(),
                    scope["criterion"]["version"].to_string(),
                );
                if tasks.insert(key, value).is_some() {
                    return Err(error("task_scope_conflict"));
                }
                let snapshot = reference(
                    &hashes,
                    value,
                    "snapshot_sha256",
                    "groundline-learning-snapshot",
                )?;
                let boundary = reference(
                    &hashes,
                    value,
                    "boundary_sha256",
                    "groundline-learning-boundary",
                )?;
                if ["unit_hash", "cohort_sha256", "phase", "runtime"]
                    .into_iter()
                    .any(|k| snapshot[k] != scope[k])
                    || (!snapshot["skill"].is_null()
                        && snapshot["skill"]["target_id"] != scope["target_id"])
                    || ["session_hash", "turn_hash"]
                        .into_iter()
                        .any(|k| value["native"][k] != boundary[k])
                {
                    return Err(error("task_snapshot_mismatch"));
                }
            }
            Some("groundline-learning-finalize-request") => {
                let task = value["input"]["task_sha256"]
                    .as_str()
                    .ok_or_else(|| error("invalid_finalize_request"))?;
                if requests.insert(task, value).is_some() {
                    return Err(error("task_outcome_conflict"));
                }
                if hashes
                    .get(task)
                    .is_none_or(|v| v["kind"] != "groundline-learning-task-start")
                {
                    return Err(error("task_reference_missing"));
                }
                let boundary = reference(
                    &hashes,
                    value,
                    "boundary_sha256",
                    "groundline-learning-boundary",
                )?;
                if ["session_hash", "turn_hash"]
                    .into_iter()
                    .any(|k| value["native"][k] != boundary[k])
                {
                    return Err(error("native_boundary_mismatch"));
                }
                work::validate_connection(&hashes, value, boundary, task)?;
            }
            Some("groundline-learning-task-outcome") => {
                let task = reference(
                    &hashes,
                    value,
                    "task_sha256",
                    "groundline-learning-task-start",
                )?;
                let boundary = reference(
                    &hashes,
                    value,
                    "boundary_sha256",
                    "groundline-learning-boundary",
                )?;
                let link = reference(&hashes, value, "link_sha256", "groundline-learning-link")?;
                if outcomes
                    .insert(value["task_sha256"].to_string(), value)
                    .is_some()
                {
                    return Err(error("task_outcome_conflict"));
                }
                if link["receipt_sha256"] != value["receipt_sha256"]
                    || ["unit_hash", "cohort_sha256", "phase"]
                        .into_iter()
                        .any(|k| link[k] != task["scope"][k])
                    || contract::timestamp(
                        boundary["observed_at_utc"]
                            .as_str()
                            .ok_or_else(|| error("invalid_boundary"))?,
                    )? > contract::timestamp(
                        link["completed_at_utc"]
                            .as_str()
                            .ok_or_else(|| error("invalid_link_timestamp"))?,
                    )?
                {
                    return Err(error("task_receipt_mismatch"));
                }
                work::validate_connection(
                    &hashes,
                    value,
                    boundary,
                    value["task_sha256"]
                        .as_str()
                        .ok_or_else(|| error("invalid_task_outcome"))?,
                )?;
            }
            Some("groundline-learning-trial-authorization") => {
                let proposal = reference(
                    &hashes,
                    value,
                    "proposal_sha256",
                    "groundline-learning-proposal",
                )?;
                if proposal["status"] != "candidate"
                    || ["plan_sha256", "scope_sha256"]
                        .into_iter()
                        .any(|k| value[k] != proposal[k])
                    || value["rollback_sha256"] != proposal["target"]["rollback_ref"]
                    || value["authority_ref"] != proposal["target"]["authority_ref"]
                {
                    return Err(error("trial_scope_mismatch"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn reference<'a>(
    hashes: &BTreeMap<String, &'a Value>,
    value: &Value,
    key: &str,
    kind: &str,
) -> Result<&'a Value, ContractError> {
    value[key]
        .as_str()
        .and_then(|h| hashes.get(h))
        .copied()
        .filter(|v| v["kind"] == kind)
        .ok_or_else(|| error("task_reference_missing"))
}

fn boundaries_root(home: &Path) -> PathBuf {
    home.join("groundline/learning/boundaries")
}

fn archive_boundary_path(home: &Path, id: &str) -> PathBuf {
    boundaries_root(home)
        .join("archive")
        .join(&sha256(id.as_bytes())[..2])
        .join(format!("{id}.json"))
}

fn queue_lock(home: &Path) -> Result<LockGuard, ContractError> {
    let directory = directory_handle(
        profile_path(home)
            .parent()
            .ok_or_else(|| error("invalid_path"))?,
        true,
    )?;
    let file = File::from(
        openat(
            &directory,
            "boundary.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| error("invalid_boundary_lock"))?,
    );
    let metadata = file
        .metadata()
        .map_err(|_| error("invalid_boundary_lock"))?;
    if !metadata.is_file() || metadata.nlink() != 1 || !private_for_current_user(&file) {
        return Err(error("invalid_boundary_lock"));
    }
    file.try_lock().map_err(|_| error("boundary_busy"))?;
    Ok(LockGuard(file))
}

fn optional_private(path: &Path) -> Result<Option<Vec<u8>>, ContractError> {
    if fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        Ok(None)
    } else {
        read_private(path).map(Some)
    }
}

fn checked_boundary(
    home: &Path,
    profile: &contract::LearningProfile,
    id: &str,
) -> Result<(Value, String), ContractError> {
    checked_boundary_bounded(home, profile, id, None)
}

fn optional_private_bounded(
    path: &Path,
    budget: Option<&mut usize>,
) -> Result<Option<Vec<u8>>, ContractError> {
    let Some(budget) = budget else {
        return optional_private(path);
    };
    if fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(None);
    }
    let directory = directory_handle(path.parent().ok_or_else(|| error("invalid_path"))?, true)?;
    let file = File::from(
        openat(
            &directory,
            path.file_name().ok_or_else(|| error("invalid_path"))?,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("invalid_input_file"))?,
    );
    if file
        .metadata()
        .map_err(|_| error("invalid_input_file"))?
        .len()
        > *budget as u64
    {
        return Err(error("boundary_lookup_budget_exceeded"));
    }
    let bytes = read_handle(file)?;
    *budget = budget
        .checked_sub(bytes.len())
        .ok_or_else(|| error("boundary_lookup_budget_exceeded"))?;
    Ok(Some(bytes))
}

fn checked_boundary_bounded(
    home: &Path,
    profile: &contract::LearningProfile,
    id: &str,
    mut budget: Option<&mut usize>,
) -> Result<(Value, String), ContractError> {
    // IDs are opaque identifiers, never paths. Contract parsing supplies the
    // same grammar as hook records before either path is constructed.
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err(error("invalid_boundary_id"));
    }
    let _lock = queue_lock(home)?;
    let hot = optional_private_bounded(
        &boundaries_root(home).join(format!("{id}.json")),
        budget.as_deref_mut(),
    )?;
    let archived = optional_private_bounded(&archive_boundary_path(home, id), budget)?;
    if hot
        .as_ref()
        .zip(archived.as_ref())
        .is_some_and(|(a, b)| a != b)
    {
        return Err(error("boundary_conflict"));
    }
    let bytes = hot.or(archived).ok_or_else(|| error("boundary_missing"))?;
    let value = parse(&bytes)?;
    let boundary: contract::Boundary =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_boundary"))?;
    boundary.validate()?;
    if boundary.boundary_id != id
        || boundary
            .device_id
            .as_ref()
            .is_some_and(|v| v != &profile.device_id)
    {
        return Err(error("boundary_device_mismatch"));
    }
    let hash = learning::content_sha256(&value)?;
    Ok((value, hash))
}

fn native_reference(
    boundary: &Value,
    artifact: Option<&Path>,
) -> Result<contract::NativeReference, ContractError> {
    let session_hash = boundary["session_hash"].as_str().map(str::to_owned);
    let turn_hash = boundary["turn_hash"].as_str().map(str::to_owned);
    let mut reference = contract::NativeReference {
        session_hash,
        turn_hash,
        artifact_sha256: None,
        boundary_matched: false,
    };
    if let Some(path) = artifact {
        let observation = native_artifact::read(path)?;
        reference.artifact_sha256 = Some(observation.sha256);
        if let (Some(expected_session), Some(expected_turn)) =
            (&reference.session_hash, &reference.turn_hash)
        {
            if observation.session_hash.as_ref() != Some(expected_session)
                || !observation.turn_hashes.contains(expected_turn)
            {
                return Err(error("native_boundary_mismatch"));
            }
            reference.boundary_matched = true;
        }
    }
    reference.validate()?;
    Ok(reference)
}

fn check_evidence(digest: Option<&str>, file: Option<&Path>) -> Result<(), ContractError> {
    match (digest, file) {
        (None, None) => Ok(()),
        (Some(expected), Some(path)) if sha256(&read_private(path)?) == expected => Ok(()),
        _ => Err(error("correction_evidence_required_or_mismatched")),
    }
}

pub(super) fn task_start(
    input: &Path,
    native_artifact: Option<&Path>,
    criterion_evidence: Option<&Path>,
    home: Option<&Path>,
) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let profile = enabled_profile(&home)?;
    let bytes = read_private(input)?;
    let raw = parse(&bytes)?;
    let input: contract::TaskStartInput =
        serde_json::from_value(raw.clone()).map_err(|_| error("invalid_task_start"))?;
    input.validate()?;
    if !profile.target_ids.contains(&input.scope.target_id) {
        return Err(error("target_not_enabled"));
    }
    check_evidence(
        input.criterion_change_evidence_sha256.as_deref(),
        criterion_evidence,
    )?;
    let (boundary, boundary_sha256) = checked_boundary(&home, &profile, &input.boundary_id)?;
    if !["SessionStart", "UserPromptSubmit", "PostCompact"]
        .contains(&boundary["event"].as_str().unwrap_or(""))
    {
        return Err(error("task_start_boundary_required"));
    }
    let native = native_reference(&boundary, native_artifact)?;
    if input
        .scope
        .runtime
        .as_ref()
        .is_some_and(|r| native.artifact_sha256.as_ref() != Some(&r.evidence_sha256))
    {
        return Err(error("native_evidence_mismatch"));
    }
    let input_sha256 = sha256(&bytes);
    if let Some(record) = storage::replay(
        Path::new(&profile.learning_state),
        &input_sha256,
        "groundline-learning-task-start",
    )? {
        let mut out = readout("ALREADY_STARTED", false);
        out["task_sha256"] = json!(learning::content_sha256(&record)?);
        out["archived"] = json!(true);
        return Ok(out);
    }
    {
        let state = State::open(Path::new(&profile.learning_state), false)?;
        if let Some(record) = state.records.iter().find(|v| {
            v["kind"] == "groundline-learning-task-start" && v["input_sha256"] == input_sha256
        }) {
            let mut out = readout("ALREADY_STARTED", false);
            out["task_sha256"] = json!(learning::content_sha256(record)?);
            return Ok(out);
        }
    }
    // The context helper holds its own environment lock; never enter it while
    // holding learning state (environment apply takes env then learning).
    let mut context = crate::environment::learning_context(
        Path::new(&profile.environment_state),
        &input.scope.target_id,
    )?;
    let observed_target = boundary["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|v| v["target_id"] == input.scope.target_id);
    let boundary_environment_matched = observed_target.is_some_and(|v| {
        ["observed", "target_missing"].contains(&v["snapshot_code"].as_str().unwrap_or(""))
            && v["observation_state_sha256"].as_str().is_some()
    });
    if let Some(target) = observed_target
        && target["observation_state_sha256"]
            .as_str()
            .is_some_and(|observed| context["observation_state_sha256"].as_str() != Some(observed))
    {
        return Err(error("boundary_environment_changed"));
    }
    if let Some(target) = observed_target
        && boundary_environment_matched
        && (target["skill_revision"] != context["skill_revision"]
            || target["source_revision"] != context["source_revision"]
            || target["baseline_sha256"] != context["baseline_sha256"])
    {
        return Err(error("boundary_environment_changed"));
    }
    if !boundary_environment_matched {
        // A missing/failed hook snapshot cannot be repaired by attaching today's state
        // to that earlier boundary. Preserve the current clock and unknowns.
        context["environment_revision"] = Value::Null;
        context["source_revision"] = Value::Null;
        context["skill_revision"] = Value::Null;
    }
    let capture_input = json!({"kind":"groundline-learning-capture-input","schema":1,"unit_hash":input.scope.unit_hash,
        "cohort_sha256":input.scope.cohort_sha256,"phase":input.scope.phase,"runtime":input.scope.runtime});
    let mut snapshot = learning::capture_snapshot(
        &capture_input,
        &context,
        &input_sha256,
        context_observed_at(&context)?,
    )?;
    if contract::timestamp(
        boundary["observed_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_boundary"))?,
    )? > contract::timestamp(
        snapshot["captured_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_snapshot_timestamp"))?,
    )? {
        return Err(error("boundary_after_capture"));
    }
    let mut state = State::open(Path::new(&profile.learning_state), false)?;
    for previous in state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-task-start")
    {
        if previous["input_sha256"] == input_sha256 {
            let mut out = readout("ALREADY_STARTED", false);
            out["task_sha256"] = json!(learning::content_sha256(previous)?);
            return Ok(out);
        }
        if previous["scope"]["cohort_sha256"] == input.scope.cohort_sha256
            && previous["scope"]["task_category"] == input.scope.task_category
            && previous["scope"]["phase"] == input.scope.phase
            && previous["scope"]["criterion"]
                != serde_json::to_value(&input.scope.criterion)
                    .map_err(|_| error("serialization_failed"))?
            && (previous["scope"]["criterion"]["version"] == input.scope.criterion.version
                || input.criterion_change_kind == "unknown")
        {
            return Err(error("criterion_change_requires_version_and_correction"));
        }
    }
    if let Some(prior) = state.records.iter().find(|v| {
        v["kind"] == "groundline-learning-snapshot" && v["capture_input_sha256"] == input_sha256
    }) {
        snapshot = prior.clone();
    }
    state.append(&boundary)?;
    state.append(&snapshot)?;
    let record = serde_json::to_value(contract::TaskStartRecord {
        kind: "groundline-learning-task-start".into(),
        schema: 1,
        input_sha256,
        scope: input.scope,
        boundary_sha256,
        snapshot_sha256: learning::content_sha256(&snapshot)?,
        native,
        boundary_environment_matched,
        criterion_change_kind: input.criterion_change_kind,
        criterion_change_evidence_sha256: input.criterion_change_evidence_sha256,
    })
    .map_err(|_| error("serialization_failed"))?;
    state.append(&record)?;
    let mut out = readout("STARTED", true);
    out["task_sha256"] = json!(learning::content_sha256(&record)?);
    out["snapshot_sha256"] = record["snapshot_sha256"].clone();
    out["native_boundary_matched"] = record["native"]["boundary_matched"].clone();
    out["criterion"] = record["scope"]["criterion"].clone();
    Ok(out)
}

pub(super) fn finalize(
    input: &Path,
    receipt: Option<&Path>,
    manifest: Option<&Path>,
    native_artifact: Option<&Path>,
    correction_evidence: Option<&Path>,
    home: Option<&Path>,
) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let profile = enabled_profile(&home)?;
    let bytes = read_private(input)?;
    let input: contract::TaskFinalizeInput =
        serde_json::from_slice(&bytes).map_err(|_| error("invalid_task_finalize"))?;
    input.validate()?;
    check_evidence(
        input.correction_evidence_sha256.as_deref(),
        correction_evidence,
    )?;
    let (boundary, boundary_sha256) = checked_boundary(&home, &profile, &input.boundary_id)?;
    if !["Stop", "SessionEnd"].contains(&boundary["event"].as_str().unwrap_or("")) {
        return Err(error("task_end_boundary_required"));
    }
    let native = native_reference(&boundary, native_artifact)?;
    let receipt_bytes = match (receipt, manifest) {
        (Some(path), None) => read_private(path)?,
        (None, Some(path)) => {
            let before = read_private(path)?;
            let temporary = Path::new(&profile.deliveries)
                .join(format!(".finalize-{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| {
                crate::delivery::record(path, &temporary)?;
                if before != read_private(path)? {
                    return Err(error("manifest_changed"));
                }
                read_private(&temporary)
            })();
            let _ = fs::remove_file(&temporary);
            result?
        }
        _ => return Err(error("receipt_or_manifest_required")),
    };
    let receipt = parse(&receipt_bytes)?;
    delivery::validate_receipt(&receipt)?;
    let receipt_sha256 = sha256(&receipt_bytes);
    if let Some(record) = storage::replay(
        Path::new(&profile.learning_state),
        &sha256(&bytes),
        "groundline-learning-task-outcome",
    )? {
        if record["receipt_sha256"] != receipt_sha256 {
            return Err(error("task_outcome_conflict"));
        }
        let mut out = readout("ALREADY_FINALIZED", false);
        out["outcome_sha256"] = json!(learning::content_sha256(&record)?);
        out["link_sha256"] = record["link_sha256"].clone();
        out["archived"] = json!(true);
        return Ok(out);
    }
    let mut state = State::open(Path::new(&profile.learning_state), false)?;
    let task = state
        .records
        .iter()
        .find(|v| {
            learning::content_sha256(v).ok().as_deref() == Some(&input.task_sha256)
                && v["kind"] == "groundline-learning-task-start"
        })
        .cloned()
        .ok_or_else(|| error("task_missing"))?;
    let snapshot = state
        .records
        .iter()
        .find(|v| learning::content_sha256(v).ok().as_deref() == task["snapshot_sha256"].as_str())
        .ok_or_else(|| error("snapshot_missing"))?;
    learning::prepare_outcome(snapshot, &receipt, &receipt_sha256)?;
    if contract::timestamp(
        boundary["observed_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_boundary"))?,
    )? > contract::timestamp(
        receipt["completed_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_link_timestamp"))?,
    )? || task["native"]["session_hash"]
        .as_str()
        .zip(native.session_hash.as_deref())
        .is_some_and(|(a, b)| a != b)
    {
        return Err(error("task_receipt_mismatch"));
    }
    let existing = load_receipts(&profile)?;
    let mut values: Vec<_> = existing.iter().map(|(_, v)| v.clone()).collect();
    if !existing.iter().any(|(h, _)| h == &receipt_sha256) {
        values.push(receipt.clone());
    }
    delivery::validate_receipt_collection(&values)?;
    let request = serde_json::to_value(contract::FinalizeRequest {
        kind: "groundline-learning-finalize-request".into(),
        schema: 1,
        input_sha256: sha256(&bytes),
        input,
        boundary_sha256,
        native,
        receipt_sha256: receipt_sha256.clone(),
        assessment_sha256: None,
        native_response_proof_sha256: None,
    })
    .map_err(|_| error("serialization_failed"))?;
    // The exact original bytes remain the receipt authority. A failed link is
    // resumable from this immutable request, never marked a successful delivery.
    let already_finalized = state.records.iter().any(|v| {
        v["kind"] == "groundline-learning-task-outcome"
            && v["task_sha256"] == request["input"]["task_sha256"]
    });
    let mut changed = state.append(&boundary)?;
    storage::reserve_receipt(&state, &profile, &receipt_sha256, &receipt)?;
    if !existing.iter().any(|(hash, _)| hash == &receipt_sha256) {
        changed |= write_bytes_draft(
            &Path::new(&profile.deliveries).join(format!("{receipt_sha256}.json")),
            &receipt_bytes,
            &state,
        )?;
    }
    changed |= state.append(&request)?;
    let output = finalize_request(&request, &receipt, &mut state)?;
    drop(state);
    let reconciled = consume(Some(&home))?;
    changed |= !already_finalized || reconciled["mutation_performed"] == true;
    let mut out = readout(
        if already_finalized {
            "ALREADY_FINALIZED"
        } else {
            "FINALIZED"
        },
        changed,
    );
    out["outcome_sha256"] = json!(learning::content_sha256(&output)?);
    out["link_sha256"] = output["link_sha256"].clone();
    out["reconciliation"] = reconciled;
    Ok(out)
}

fn finalize_request(
    request: &Value,
    receipt: &Value,
    state: &mut State,
) -> Result<Value, ContractError> {
    if let Some(existing) = state.records.iter().find(|v| {
        v["kind"] == "groundline-learning-task-outcome"
            && v["task_sha256"] == request["input"]["task_sha256"]
    }) {
        if existing["receipt_sha256"] != request["receipt_sha256"]
            || existing["input_sha256"] != request["input_sha256"]
        {
            return Err(error("task_outcome_conflict"));
        }
        return Ok(existing.clone());
    }
    let task = state
        .records
        .iter()
        .find(|v| {
            learning::content_sha256(v).ok().as_deref() == request["input"]["task_sha256"].as_str()
        })
        .ok_or_else(|| error("task_missing"))?;
    let snapshot = state
        .records
        .iter()
        .find(|v| learning::content_sha256(v).ok().as_deref() == task["snapshot_sha256"].as_str())
        .ok_or_else(|| error("snapshot_missing"))?;
    let receipt_sha256 = request["receipt_sha256"]
        .as_str()
        .ok_or_else(|| error("invalid_task_outcome"))?;
    let mut input = learning::prepare_outcome(snapshot, receipt, receipt_sha256)?;
    input["correction_kind"] = request["input"]["correction_kind"].clone();
    input["correction_evidence_sha256"] = request["input"]["correction_evidence_sha256"].clone();
    let link = learning::link_outcome(&input, receipt, receipt_sha256)?;
    let record = json!({"kind":"groundline-learning-task-outcome","schema":1,"input_sha256":request["input_sha256"],
        "task_sha256":request["input"]["task_sha256"],"boundary_sha256":request["boundary_sha256"],
        "native":request["native"],"receipt_sha256":receipt_sha256,"link_sha256":learning::content_sha256(&link)?});
    let mut record = record;
    for key in ["assessment_sha256", "native_response_proof_sha256"] {
        if let Some(value) = request.get(key) {
            record[key] = value.clone();
        }
    }
    state.append(&link)?;
    state.append(&record)?;
    Ok(record)
}

fn load_receipts(
    profile: &contract::LearningProfile,
) -> Result<Vec<(String, Value)>, ContractError> {
    let path = Path::new(&profile.deliveries);
    let directory = directory_handle(path, true)?;
    let rows = read_directory(path, &directory, false)?;
    delivery::validate_receipt_collection(
        &rows.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
    )?;
    Ok(rows)
}

fn archive_boundaries(
    home: &Path,
    profile: &contract::LearningProfile,
) -> Result<usize, ContractError> {
    let _lock = queue_lock(home)?;
    let root = boundaries_root(home);
    let directory = directory_handle(&root, true)?;
    let archive = root.join("archive");
    ensure_private_directory(&archive)?;
    let rows = read_directory(&root, &directory, false)?;
    let index_state = if rows.is_empty() {
        None
    } else {
        Some(State::open(Path::new(&profile.learning_state), false)?)
    };
    let mut count = 0;
    for (_, value) in rows {
        let boundary: contract::Boundary =
            serde_json::from_value(value.clone()).map_err(|_| error("invalid_boundary"))?;
        boundary.validate()?;
        if boundary
            .device_id
            .as_ref()
            .is_some_and(|v| v != &profile.device_id)
        {
            return Err(error("boundary_device_mismatch"));
        }
        let name = format!("{}.json", boundary.boundary_id);
        let before = read_at(&directory, std::ffi::OsStr::new(&name))?;
        if parse(&before)? != value {
            return Err(error("boundary_changed"));
        }
        let path = archive_boundary_path(home, &boundary.boundary_id);
        ensure_private_directory(path.parent().ok_or_else(|| error("invalid_path"))?)?;
        let destination =
            directory_handle(path.parent().ok_or_else(|| error("invalid_path"))?, true)?;
        // Reject a changed duplicate before publishing its reference index.
        if let Some(previous) = optional_private(&path)?
            && previous != before
        {
            return Err(error("boundary_conflict"));
        }
        index_boundary(
            home,
            &value,
            index_state
                .as_ref()
                .ok_or_else(|| error("state_unavailable"))?,
        )?;
        match rustix::fs::renameat_with(
            &directory,
            name.as_str(),
            &destination,
            name.as_str(),
            rustix::fs::RenameFlags::NOREPLACE,
        ) {
            Ok(()) => {}
            Err(rustix::io::Errno::EXIST) => {
                if read_at(&destination, std::ffi::OsStr::new(&name))? != before {
                    return Err(error("boundary_conflict"));
                }
                unlinkat(&directory, name.as_str(), AtFlags::empty())
                    .map_err(|_| error("archive_write_failed"))?;
            }
            Err(_) => return Err(error("archive_write_failed")),
        }
        destination
            .sync_all()
            .and_then(|_| directory.sync_all())
            .map_err(|_| error("archive_write_failed"))?;
        if read_private(&path)? != before {
            return Err(error("boundary_changed"));
        }
        count += 1;
    }
    Ok(count)
}

fn index_boundary(home: &Path, value: &Value, state: &State) -> Result<(), ContractError> {
    // A bounded per-session index contains opaque references, not transcripts.
    let Some(session) = value["session_hash"].as_str() else {
        return Ok(());
    };
    let indices = boundaries_root(home).join("archive/references");
    ensure_private_directory(&indices)?;
    let session_path = indices.join(session);
    ensure_private_directory(&session_path)?;
    let turn_path = session_path.join(value["turn_hash"].as_str().unwrap_or("unobserved"));
    ensure_private_directory(&turn_path)?;
    let index = json!({"kind":"groundline-learning-boundary-reference","schema":1,
        "boundary_id":value["boundary_id"],"session_hash":value["session_hash"],"turn_hash":value["turn_hash"],
        "event":value["event"],"observed_at_utc":value["observed_at_utc"],"boundary_sha256":learning::content_sha256(value)?});
    // Index shards make an exact native session lookup bounded even after the
    // hot queue has been consumed; no arbitrary globally latest event is used.
    let shard = turn_path.join(
        &sha256(
            value["boundary_id"]
                .as_str()
                .ok_or_else(|| error("invalid_boundary"))?
                .as_bytes(),
        )[..2],
    );
    ensure_private_directory(&shard)?;
    write_draft(
        &shard.join(format!(
            "{}.json",
            value["boundary_id"]
                .as_str()
                .ok_or_else(|| error("invalid_boundary"))?
        )),
        &index,
        state,
    )?;
    Ok(())
}

pub(super) fn boundaries(
    artifact: &Path,
    turn: Option<&str>,
    home: Option<&Path>,
) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let profile = enabled_profile(&home)?;
    let observation = native_artifact::read(artifact)?;
    let session = observation
        .session_hash
        .clone()
        .ok_or_else(|| error("native_session_unobserved"))?;
    let turns = &observation.turn_hashes;
    if turn.is_some_and(|t| !learning::digest(t) || !turns.contains(t)) {
        return Err(error("native_turn_unobserved"));
    }
    let archived_count = archive_boundaries(&home, &profile)?;
    if turn.is_none() && turns.len() != 1 {
        let mut out = readout("BOUNDARY_REFERENCES", archived_count != 0);
        out["boundaries"] = json!([]);
        out["ambiguous"] = json!(true);
        out["known_turn_hashes"] = json!(turns.iter().take(64).collect::<Vec<_>>());
        out["coverage_complete"] = json!(turns.len() <= 64);
        out["archived_boundary_count"] = json!(archived_count);
        out["coverage_scope"] = json!("bounded_matching_native_session");
        out["selection_automatic"] = json!(false);
        out["native_input"] = observation.readout();
        return Ok(out);
    }
    let selected_turn = turn
        .or_else(|| turns.iter().next().map(String::as_str))
        .ok_or_else(|| error("native_turn_unobserved"))?;
    let mut matches = Vec::new();
    let mut incomplete = false;
    // One budget covers every shard/reference and both raw boundary copies.
    // A return-count cap alone does not bound scans of malformed/sparse indexes.
    const LOOKUP_ENTRIES: usize = 512;
    const LOOKUP_BYTES: usize = 4 * 1024 * 1024;
    let mut visited = 0_usize;
    let mut remaining_bytes = LOOKUP_BYTES;
    let root = boundaries_root(&home)
        .join("archive/references")
        .join(&session)
        .join(selected_turn);
    if fs::symlink_metadata(&root).is_ok() {
        directory_handle(&root, true)?;
        'shards: for shard in fs::read_dir(&root).map_err(|_| error("directory_unavailable"))? {
            if visited >= LOOKUP_ENTRIES {
                incomplete = true;
                break;
            }
            visited += 1;
            let path = shard.map_err(|_| error("directory_unavailable"))?.path();
            let handle = directory_handle(&path, true)?;
            for entry in fs::read_dir(&path).map_err(|_| error("directory_unavailable"))? {
                if visited >= LOOKUP_ENTRIES || matches.len() >= 64 {
                    incomplete = true;
                    break 'shards;
                }
                visited += 1;
                let entry = entry.map_err(|_| error("directory_unavailable"))?;
                let name = entry.file_name();
                if !name.as_encoded_bytes().ends_with(b".json") {
                    return Err(error("invalid_boundary_index"));
                }
                if !binding_matches(&path, &handle) {
                    return Err(error("directory_changed"));
                }
                let reference =
                    match optional_private_bounded(&entry.path(), Some(&mut remaining_bytes)) {
                        Ok(Some(bytes)) => parse(&bytes)?,
                        Ok(None) => return Err(error("invalid_boundary_index")),
                        Err(e) if e.0 == "learning_boundary_lookup_budget_exceeded" => {
                            incomplete = true;
                            break 'shards;
                        }
                        Err(e) => return Err(e),
                    };
                if reference["session_hash"] != session {
                    return Err(error("native_boundary_mismatch"));
                }
                if reference["turn_hash"] != selected_turn {
                    return Err(error("native_boundary_mismatch"));
                }
                let id = reference["boundary_id"]
                    .as_str()
                    .ok_or_else(|| error("invalid_boundary"))?;
                let (boundary, hash) =
                    match checked_boundary_bounded(&home, &profile, id, Some(&mut remaining_bytes))
                    {
                        Ok(value) => value,
                        Err(e) if e.0 == "learning_boundary_lookup_budget_exceeded" => {
                            incomplete = true;
                            break 'shards;
                        }
                        Err(e) => return Err(e),
                    };
                if reference["boundary_sha256"] != hash {
                    return Err(error("boundary_conflict"));
                }
                if boundary["session_hash"] != session || boundary["turn_hash"] != selected_turn {
                    return Err(error("native_boundary_mismatch"));
                }
                matches.push(json!({"boundary_id":id,"session_hash":boundary["session_hash"],"turn_hash":boundary["turn_hash"],
                    "event":boundary["event"],"observed_at_utc":boundary["observed_at_utc"],"boundary_sha256":hash,
                    "native_boundary_matched":true}));
            }
        }
    }
    matches.sort_by(|a, b| {
        a["observed_at_utc"]
            .as_str()
            .cmp(&b["observed_at_utc"].as_str())
            .then_with(|| {
                a["boundary_sha256"]
                    .as_str()
                    .cmp(&b["boundary_sha256"].as_str())
            })
    });
    let mut out = readout("BOUNDARY_REFERENCES", archived_count != 0);
    out["archived_boundary_count"] = json!(archived_count);
    out["coverage_scope"] = json!("bounded_matching_native_session");
    out["boundaries"] = json!(matches);
    out["ambiguous"] = json!(turn.is_none() && turns.len() != 1);
    out["coverage_complete"] = json!(!incomplete);
    out["lookup_budget"] = json!({"entry_limit":LOOKUP_ENTRIES,"byte_limit":LOOKUP_BYTES,
        "entries_visited":visited,"bytes_read":LOOKUP_BYTES-remaining_bytes});
    out["selection_automatic"] = json!(false);
    out["native_input"] = observation.readout();
    Ok(out)
}

fn operations(profile: &contract::LearningProfile) -> Result<Vec<(String, Value)>, ContractError> {
    let path = Path::new(&profile.environment_state).join("operations");
    if fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let handle = directory_handle(&path, true)?;
    let values = read_directory(&path, &handle, false)?;
    for (_, value) in &values {
        crate::environment::validate_operation(value)?;
    }
    Ok(values)
}

fn journal(
    state: &mut State,
    input_sha256: &str,
    context: &Value,
    status: &str,
    refs: &[String],
    reasons: &[String],
) -> Result<(String, bool), ContractError> {
    let record = serde_json::to_value(contract::ProcessingRecord {
        kind: "groundline-learning-processing".into(),
        schema: 1,
        input_sha256: input_sha256.into(),
        context_sha256: learning::content_sha256(context)?,
        status: status.into(),
        evidence_refs: refs.to_vec(),
        reasons: reasons.to_vec(),
    })
    .map_err(|_| error("serialization_failed"))?;
    let hash = learning::content_sha256(&record)?;
    let changed = state.append(&record)?;
    Ok((hash, changed))
}

pub(super) fn consume(home: Option<&Path>) -> Result<Value, ContractError> {
    match consume_inner(home) {
        Err(error)
            if [
                "learning_state_busy",
                "learning_boundary_busy",
                "learning_profile_busy",
            ]
            .contains(&error.0.as_str()) =>
        {
            let mut out = readout("PENDING", false);
            out["reason"] = json!(error.0);
            out["retry_safe"] = json!(true);
            Ok(out)
        }
        result => result,
    }
}

fn consume_inner(home: Option<&Path>) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let Some(profile) = profile(&home, false)? else {
        return Ok(readout("UNCONFIGURED", false));
    };
    if !profile.enabled {
        return Ok(readout("DISABLED", false));
    }
    let archived_boundaries = archive_boundaries(&home, &profile)?;
    // Snapshot operations before the learning lock. Only the environment layer
    // writes them; no learning → environment lock inversion is introduced.
    let operations = operations(&profile)?;
    let mut receipts = load_receipts(&profile)?;
    let mut state = State::open(Path::new(&profile.learning_state), false)?;
    storage::include_receipts(&profile, &state.records, &mut receipts)?;
    let archived_records = storage::compact(&mut state, &profile)?;
    let requests: Vec<_> = state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-finalize-request")
        .cloned()
        .collect();
    let mut resumed = 0;
    for request in requests {
        if state.records.iter().any(|v| {
            v["kind"] == "groundline-learning-task-outcome"
                && v["task_sha256"] == request["input"]["task_sha256"]
        }) {
            continue;
        }
        let receipt =
            match work::restore_generated_receipt(&profile, &state, &request, &mut receipts) {
                Ok(Some(receipt)) => Some(receipt),
                Ok(None) => receipts
                    .iter()
                    .find(|(h, _)| request["receipt_sha256"] == *h)
                    .map(|(_, v)| v.clone()),
                Err(_) => continue,
            };
        if let Some(receipt) = receipt {
            if work::check_request_evidence(&state, &request, &receipt).is_err() {
                continue;
            }
            finalize_request(&request, &receipt, &mut state)?;
            resumed += 1;
        }
    }
    let assessments = work::consume_pending(&home, &profile, &mut state, &mut receipts)?;
    let reconciliations = reconcile_candidates(&mut state, &receipts, &operations)?;
    let mut out = readout(
        "CONSUMED",
        archived_boundaries != 0
            || archived_records != 0
            || resumed != 0
            || assessments.iter().any(|v| v["mutation_performed"] == true)
            || reconciliations
                .iter()
                .any(|v| v["mutation_performed"] == true),
    );
    out["archived_boundary_count"] = json!(archived_boundaries);
    out["archived_record_count"] = json!(archived_records);
    out["resumed_finalize_count"] = json!(resumed);
    out["reconciliations"] = json!(reconciliations);
    out["assessments"] = json!(assessments);
    out["readiness"] = readiness::summarize(&state.records, &receipts)?;
    out["quality_inferred_from_boundaries"] = json!(false);
    out["capacity"] = json!({"active_records":state.records.len(),"record_limit":MAX_ENTRIES-2,
        "byte_limit":MAX_DIRECTORY_BYTES,"coverage_scope":"bounded_active_reference_closure","archive_coverage":"uninspected",
        "archival_storage_present":state.path.join("archive").exists(),"historical_quality_coverage_complete":false});
    Ok(out)
}

fn reconcile_candidates(
    state: &mut State,
    receipts: &[(String, Value)],
    operations: &[(String, Value)],
) -> Result<Vec<Value>, ContractError> {
    let candidates: Vec<_> = state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-proposal" && v["status"] == "candidate")
        .cloned()
        .collect();
    let mut readouts = Vec::new();
    for candidate in candidates {
        let proposal: learning::Proposal =
            serde_json::from_value(candidate.clone()).map_err(|_| error("invalid_proposal"))?;
        let hash = learning::content_sha256(&candidate)?;
        let target = proposal
            .target
            .as_ref()
            .ok_or_else(|| error("invalid_proposal"))?;
        let head = learning::decision_head(&state.records, &hash)?;
        let decision = head.as_ref().and_then(|h| {
            state
                .records
                .iter()
                .find(|v| learning::content_sha256(v).ok().as_ref() == Some(h))
        });
        let applied = operations
            .iter()
            .find(|(_, op)| learning::operation_applies_to_proposal(&proposal, op));
        let rolled_back = applied.is_some_and(|(_, op)| {
            operations.iter().any(|(_, other)| {
                other["action"] == "rollback"
                    && other["original_operation_id"] == op["operation_id"]
            })
        });
        let links: Vec<_> = state
            .records
            .iter()
            .filter(|v| v["kind"] == "groundline-learning-link")
            .cloned()
            .collect();
        let find_link = |h: &str| {
            links
                .iter()
                .find(|v| learning::content_sha256(v).ok().as_deref() == Some(h))
        };
        let task_for = |link: &str| {
            state
                .records
                .iter()
                .find(|v| {
                    v["kind"] == "groundline-learning-task-outcome" && v["link_sha256"] == link
                })
                .and_then(|outcome| {
                    state
                        .records
                        .iter()
                        .find(|v| {
                            learning::content_sha256(v).ok().as_deref()
                                == outcome["task_sha256"].as_str()
                        })
                        .map(|task| (task, outcome))
                })
        };
        let baseline = proposal
            .evidence_refs
            .iter()
            .filter_map(|h| find_link(h))
            .find(|v| {
                v["skill"]["target_id"] == target.target_id
                    && v["skill"]["revision"] == target.before_sha256
            });
        let scope_matches = |after: &Value| {
            baseline.is_some_and(|before| {
                if ["cohort_sha256", "phase"]
                    .into_iter()
                    .any(|k| before[k] != after[k])
                    || ["family", "version"]
                        .into_iter()
                        .any(|k| before["runtime"][k] != after["runtime"][k])
                    || proposal
                        .source_revision
                        .as_ref()
                        .is_some_and(|source| after["source_revision"] != *source)
                {
                    return false;
                }
                let receipt_for = |link: &Value| {
                    receipts
                        .iter()
                        .find(|(h, _)| link["receipt_sha256"] == *h)
                        .map(|(_, r)| r)
                };
                if let (Some(a), Some(b)) = (receipt_for(before), receipt_for(after)) {
                    if ["model", "effort"]
                        .into_iter()
                        .any(|k| a["effective"][k] != b["effective"][k])
                    {
                        return false;
                    }
                } else {
                    return false;
                }
                learning::content_sha256(before)
                    .ok()
                    .zip(learning::content_sha256(after).ok())
                    .and_then(|(a, b)| task_for(&a).zip(task_for(&b)))
                    .is_some_and(|((a, _), (b, _))| {
                        a["scope"]["criterion"] == b["scope"]["criterion"]
                            && a["scope"]["task_category"] == b["scope"]["task_category"]
                    })
            })
        };
        let followup = links
            .iter()
            .filter(|v| {
                v["skill"]["target_id"] == target.target_id
                    && v["skill"]["revision"] == target.after_sha256
                    && v["environment_revision"] == proposal.plan_sha256
            })
            .filter(|v| baseline.is_none_or(|b| v["unit_hash"] != b["unit_hash"]))
            .max_by(|a, b| {
                // Prefer matching work conditions, never a favorable outcome.
                // If none match, the latest exact generation stays INCONCLUSIVE.
                scope_matches(a).cmp(&scope_matches(b)).then_with(|| {
                    a["completed_at_utc"]
                        .as_str()
                        .cmp(&b["completed_at_utc"].as_str())
                })
            });
        let analysis_records = state
            .records
            .iter()
            .filter(|v| {
                v["kind"] == "groundline-learning-duplicate-attempt"
                    && v["duplicate_of_sha256"] == hash
                    || v["kind"] == "groundline-learning-proposal"
                        && v["status"] == "failed"
                        && v["evidence_refs"] == candidate["evidence_refs"]
                        && v["scope_sha256"] == candidate["scope_sha256"]
                        && v["basis_revision"] == candidate["basis_revision"]
                        && v["source_revision"] == candidate["source_revision"]
            })
            .map(learning::content_sha256)
            .collect::<Result<Vec<_>, _>>()?;
        let context = json!({"proposal_sha256":hash,"operations":operations.iter().filter(|(_,v)|v["proposal_id"] == proposal.proposal_id).map(|(h,_)|h).collect::<Vec<_>>(),
            "baseline":baseline.map(learning::content_sha256).transpose()?,"followup":followup.map(learning::content_sha256).transpose()?,"decision":head,"analysis_attempts":analysis_records});
        let context_sha = learning::content_sha256(&context)?;
        if let Some(previous) = state.records.iter().find(|v| {
            v["kind"] == "groundline-learning-processing"
                && v["input_sha256"] == hash
                && v["context_sha256"] == context_sha
        }) {
            readouts.push(json!({"proposal_sha256":hash,"status":previous["status"],"processing_sha256":learning::content_sha256(previous)?,
                "mutation_performed":false,"duplicate_suppressed":true,"evidence_refs":previous["evidence_refs"],"reasons":previous["reasons"]}));
            continue;
        }
        let mut reasons = Vec::new();
        let mut status = "PENDING";
        if decision
            .is_some_and(|v| ["hold", "reject"].contains(&v["decision"].as_str().unwrap_or("")))
        {
            status = "BLOCKED";
            reasons.push("candidate_held_or_rejected".into());
        } else if rolled_back {
            status = "BLOCKED";
            reasons.push("candidate_rolled_back".into());
        } else if applied.is_none() {
            reasons.push("applied_operation_missing".into());
        } else if baseline.is_none() || followup.is_none() {
            reasons.push("direct_baseline_or_followup_pending".into());
        }
        let (Some((operation_sha, operation)), Some(before), Some(after)) =
            (applied, baseline, followup)
        else {
            let (journal_hash, changed) = journal(state, &hash, &context, status, &[], &reasons)?;
            readouts.push(json!({"proposal_sha256":hash,"status":status,"reasons":reasons,"processing_sha256":journal_hash,"mutation_performed":changed}));
            continue;
        };
        if status == "BLOCKED" {
            let (journal_hash, changed) = journal(state, &hash, &context, status, &[], &reasons)?;
            readouts.push(json!({"proposal_sha256":hash,"status":status,"reasons":reasons,"processing_sha256":journal_hash,"mutation_performed":changed}));
            continue;
        }
        let before_hash = learning::content_sha256(before)?;
        let after_hash = learning::content_sha256(after)?;
        if let (Some((before_task, before_outcome)), Some((after_task, after_outcome))) =
            (task_for(&before_hash), task_for(&after_hash))
        {
            if before_task["scope"]["criterion"] != after_task["scope"]["criterion"]
                || before_task["scope"]["task_category"] != after_task["scope"]["task_category"]
            {
                reasons.push("completion_criterion_or_category_mismatch".into());
            }
            if before_task["boundary_environment_matched"] != true
                || after_task["boundary_environment_matched"] != true
            {
                reasons.push("boundary_environment_unverified".into());
            }
            if [before_task, before_outcome, after_task, after_outcome]
                .into_iter()
                .any(|v| v["native"]["boundary_matched"] != true)
            {
                reasons.push("native_boundary_unverified".into());
            }
        } else {
            reasons.push("scoped_completion_criterion_unobserved".into());
        }
        let input = json!({"kind":"groundline-learning-evaluation-input","schema":1,"proposal_id":proposal.proposal_id,
            "baseline_revision":before["environment_revision"].as_str().unwrap_or("unobserved-initial-generation"),"proposal_revision":proposal.plan_sha256,
            "baseline_refs":[before_hash],"followup_refs":[after_hash]});
        let result = evaluate_records(&input, receipts, operation, operation_sha, state, &reasons)?;
        // Refresh the in-memory collection after immutable evaluation publication.
        let evaluation_path = state.path.join(format!(
            "{}.json",
            result["evaluation_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_evaluation"))?
        ));
        let evaluation = parse(&read_private(&evaluation_path)?)?;
        if !state.records.contains(&evaluation) {
            state.records.push(evaluation);
        }
        let status = if result["status"] == "INCONCLUSIVE" {
            "INCONCLUSIVE"
        } else {
            "EVALUATED"
        };
        let refs = vec![
            before_hash,
            after_hash,
            result["evaluation_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_evaluation"))?
                .into(),
        ];
        let (journal_hash, changed) = journal(state, &hash, &context, status, &refs, &reasons)?;
        readouts.push(json!({"proposal_sha256":hash,"status":status,"evaluation":result,"processing_sha256":journal_hash,"mutation_performed":changed}));
    }
    Ok(readouts)
}

pub(super) fn patterns(home: Option<&Path>) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let profile = enabled_profile(&home)?;
    let context = crate::environment::learning_context(
        Path::new(&profile.environment_state),
        &profile.target_ids[0],
    )?;
    let planning_revision = context["planning_revision"].clone();
    let mut receipts = load_receipts(&profile)?;
    let state = State::open(Path::new(&profile.learning_state), false)?;
    storage::include_receipts(&profile, &state.records, &mut receipts)?;
    let mut groups = BTreeMap::<String, Value>::new();
    for outcome in state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-task-outcome")
    {
        let Some(task) = state.records.iter().find(|v| {
            learning::content_sha256(v).ok().as_deref() == outcome["task_sha256"].as_str()
        }) else {
            return Err(error("task_missing"));
        };
        let Some(link) = state.records.iter().find(|v| {
            learning::content_sha256(v).ok().as_deref() == outcome["link_sha256"].as_str()
        }) else {
            return Err(error("evaluation_link_missing"));
        };
        let Some((_, receipt)) = receipts
            .iter()
            .find(|(h, _)| outcome["receipt_sha256"] == *h)
        else {
            return Err(error("evaluation_receipt_missing"));
        };
        let scope = json!({"task_category":task["scope"]["task_category"],"criterion":task["scope"]["criterion"],
            "cohort_sha256":task["scope"]["cohort_sha256"],"phase":task["scope"]["phase"],"target":link["skill"],
            "environment_revision":link["environment_revision"],"source_revision":link["source_revision"],
            "runtime":task["scope"]["runtime"].as_object().map(|v|json!({"family":v.get("family"),"version":v.get("version")})),
            "effective":receipt["effective"].as_object().map(|v|json!({"model":v.get("model"),"effort":v.get("effort")}))});
        let hash = learning::content_sha256(&scope)?;
        let group = groups.entry(hash).or_insert_with(||json!({"scope":scope,"sample_count":0,"failed_count":0,"rework_count":0,"assistant_error_count":0,
            "correction_unknown_count":0,"outcome_unknown_count":0,"resource_gap_count":0,"native_boundary_unverified_count":0,"evidence_refs":[]}));
        for (field, increment) in [
            ("sample_count", true),
            (
                "failed_count",
                receipt["verification"]["status"] == "failed",
            ),
            ("rework_count", receipt["verification"]["rework"] == true),
            (
                "assistant_error_count",
                link["correction_kind"] == "assistant_error",
            ),
            (
                "correction_unknown_count",
                link["correction_kind"] == "unknown",
            ),
            (
                "outcome_unknown_count",
                receipt["verification"]["status"] == "unknown",
            ),
            (
                "resource_gap_count",
                receipt["resources"]["complete"] != true,
            ),
            (
                "native_boundary_unverified_count",
                task["native"]["boundary_matched"] != true
                    || outcome["native"]["boundary_matched"] != true,
            ),
        ] {
            if increment {
                group[field] = json!(group[field].as_u64().unwrap_or(0) + 1);
            }
        }
        group["evidence_refs"]
            .as_array_mut()
            .ok_or_else(|| error("invalid_pattern"))?
            .push(outcome["link_sha256"].clone());
    }
    let mut patterns = Vec::new();
    for (scope_sha256, mut value) in groups {
        value["evidence_refs"]
            .as_array_mut()
            .ok_or_else(|| error("invalid_pattern"))?
            .sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        value["scope_sha256"] = json!(scope_sha256);
        value["analysis_input_sha256"] = json!(learning::content_sha256(
            &json!({"scope":value["scope"],"evidence_refs":value["evidence_refs"],"planning_revision":planning_revision})
        )?);
        value["already_analyzed"] = json!(state.records.iter().any(|v| v["kind"]
            == "groundline-learning-proposal"
            && v["status"] != "failed"
            && v["evidence_refs"] == value["evidence_refs"]
            && v["basis_revision"] == planning_revision
            && v["source_revision"] == value["scope"]["source_revision"]));
        value["analysis_resources_observed"] = json!(
            state
                .records
                .iter()
                .filter(|v| v["kind"] == "groundline-learning-proposal"
                    && v["evidence_refs"] == value["evidence_refs"])
                .all(|v| !v["analysis"].is_null())
                && state
                    .records
                    .iter()
                    .any(|v| v["kind"] == "groundline-learning-proposal"
                        && v["evidence_refs"] == value["evidence_refs"])
        );
        patterns.push(value);
    }
    let mut out = readout("PATTERNS", false);
    out["patterns"] = json!(patterns);
    out["readiness"] = readiness::summarize(&state.records, &receipts)?;
    out["automatic_candidate_generation"] = json!(false);
    out["coverage_scope"] = json!("bounded_active_reference_closure");
    out["historical_quality_coverage_complete"] = json!(false);
    Ok(out)
}
