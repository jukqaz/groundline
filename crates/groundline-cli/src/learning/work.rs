//! A thin bridge from an explicit direct assessment to existing delivery and
//! task outcome records. Native paths are ephemeral and no model is called.
use super::*;
use groundline_runtime::learning_response::{self, NativeRef, TaskObservation};
use std::time::{Duration, Instant};

const DISCOVERY_ENTRIES: usize = 2048;
const DISCOVERY_CANDIDATES: usize = 64;
const DISCOVERY_BYTES: u64 = 64 * 1024 * 1024;
const DISCOVERY_BUDGET: Duration = Duration::from_secs(10);
const END_ENTRIES: usize = 1024;
const END_BYTES: usize = 4 * 1024 * 1024;

fn evidence_path(state: &State, hash: &str) -> Result<PathBuf, ContractError> {
    if !learning::digest(hash) {
        return Err(error("invalid_assessment_evidence"));
    }
    Ok(state.path.join("evidence").join(format!("{hash}.artifact")))
}

fn save_evidence(state: &State, hash: &str, bytes: &[u8]) -> Result<bool, ContractError> {
    if sha256(bytes) != hash || bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(error("assessment_evidence_mismatch"));
    }
    ensure_private_directory(&state.path.join("evidence"))?;
    write_bytes_draft(&evidence_path(state, hash)?, bytes, state)
}

fn checked_evidence(state: &State, hash: &str) -> Result<PathBuf, ContractError> {
    let path = evidence_path(state, hash)?;
    if sha256(&read_private(&path)?) != hash {
        return Err(error("assessment_evidence_mismatch"));
    }
    Ok(path)
}

fn supplied_evidence(
    hash: Option<&str>,
    path: Option<&Path>,
) -> Result<Option<Vec<u8>>, ContractError> {
    match (hash, path) {
        (None, None) => Ok(None),
        (Some(expected), Some(path)) => {
            let bytes = read_private(path)?;
            if sha256(&bytes) != expected {
                return Err(error("assessment_evidence_mismatch"));
            }
            Ok(Some(bytes))
        }
        _ => Err(error("assessment_evidence_required")),
    }
}

pub(in crate::learning) fn assess(
    path: &Path,
    verification_evidence: Option<&Path>,
    correction_evidence: Option<&Path>,
    home: Option<&Path>,
) -> Result<Value, ContractError> {
    let home = codex_home(home)?;
    let profile = enabled_profile(&home)?;
    let bytes = read_private(path)?;
    let input: contract::AssessmentInput =
        serde_json::from_slice(&bytes).map_err(|_| error("invalid_assessment"))?;
    input.validate()?;
    let input_sha256 = sha256(&bytes);
    let verification = supplied_evidence(
        input.verification.evidence_sha256.as_deref(),
        verification_evidence,
    )?;
    let correction = supplied_evidence(
        input.correction_evidence_sha256.as_deref(),
        correction_evidence,
    )?;
    let mut state = State::open(Path::new(&profile.learning_state), false)?;
    storage::load_archive_references(
        &state.path,
        &mut state.records,
        std::slice::from_ref(&input.task_sha256),
    )?;
    storage::restore_task_completion(&state.path, &mut state.records, &input.task_sha256)?;
    if !state.records.iter().any(|v| {
        v["kind"] == "groundline-learning-task-start"
            && learning::content_sha256(v).ok().as_deref() == Some(&input.task_sha256)
    }) {
        return Err(error("task_missing"));
    }
    let archived = storage::replay(&state.path, &input_sha256, "groundline-learning-assessment")?;
    let existing = state
        .records
        .iter()
        .find(|v| {
            v["kind"] == "groundline-learning-assessment"
                && v["input"]["task_sha256"] == input.task_sha256
        })
        .cloned()
        .or(archived);
    if let Some(existing) = &existing {
        if existing["input_sha256"] != input_sha256 {
            return Err(error("assessment_conflict"));
        }
        // An idempotent retry still checks original evidence, including any
        // preserved private copy. Missing or altered proof is never replaced.
        checked_evidence(&state, &input_sha256)?;
        if let Some(hash) = &input.verification.evidence_sha256 {
            checked_evidence(&state, hash)?;
        }
        if let Some(hash) = &input.correction_evidence_sha256 {
            checked_evidence(&state, hash)?;
        }
        let mut out = readout("ALREADY_ASSESSED", false);
        out["assessment_sha256"] = json!(learning::content_sha256(existing)?);
        out["submitted_at_utc"] = existing["submitted_at_utc"].clone();
        out["completion_pending"] = json!(
            !state
                .records
                .iter()
                .any(|v| v["kind"] == "groundline-learning-task-outcome"
                    && v["task_sha256"] == input.task_sha256)
        );
        out["retry_safe"] = json!(true);
        return Ok(out);
    }
    if state.records.iter().any(|v| {
        v["kind"] == "groundline-learning-task-outcome" && v["task_sha256"] == input.task_sha256
    }) {
        return Err(error("task_already_finalized"));
    }
    let record = serde_json::to_value(contract::Assessment {
        kind: "groundline-learning-assessment".into(),
        schema: 1,
        input_sha256: input_sha256.clone(),
        input,
        submitted_at_utc: chrono::Utc::now().to_rfc3339(),
        status: "PENDING".into(),
    })
    .map_err(|_| error("serialization_failed"))?;
    // Validate chronology and ownership before publishing evidence or records.
    let mut updated = state.records.clone();
    updated.push(record.clone());
    super::super::validate_record_collection(&updated)?;
    save_evidence(&state, &input_sha256, &bytes)?;
    if let Some(bytes) = verification {
        save_evidence(
            &state,
            record["input"]["verification"]["evidence_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_assessment"))?,
            &bytes,
        )?;
    }
    if let Some(bytes) = correction {
        save_evidence(
            &state,
            record["input"]["correction_evidence_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_assessment"))?,
            &bytes,
        )?;
    }
    state.append(&record)?;
    let mut out = readout("PENDING", true);
    out["assessment_sha256"] = json!(learning::content_sha256(&record)?);
    out["submitted_at_utc"] = record["submitted_at_utc"].clone();
    out["completion_pending"] = json!(true);
    out["reason"] = json!("actual_native_end_required");
    out["retry_safe"] = json!(true);
    Ok(out)
}

/// Both automatic finalization stages retain the exact assessment, task/start
/// closure and a real end boundary. Manual finalization has no assessment ref.
pub(super) fn validate_connection(
    hashes: &BTreeMap<String, &Value>,
    value: &Value,
    boundary: &Value,
    task_sha256: &str,
) -> Result<(), ContractError> {
    let Some(hash) = value.get("assessment_sha256") else {
        return Ok(());
    };
    let assessment = hash
        .as_str()
        .and_then(|h| hashes.get(h))
        .copied()
        .filter(|v| v["kind"] == "groundline-learning-assessment")
        .ok_or_else(|| error("assessment_reference_missing"))?;
    let task = hashes
        .get(task_sha256)
        .copied()
        .ok_or_else(|| error("task_reference_missing"))?;
    if assessment["input"]["task_sha256"] != task_sha256
        || boundary["session_hash"] != task["native"]["session_hash"]
        || boundary["session_hash"].is_null()
        || boundary["turn_hash"].is_null()
        || value["native"]["session_hash"] != boundary["session_hash"]
        || value["native"]["turn_hash"] != boundary["turn_hash"]
        || value["native"]["boundary_matched"] != true
        || !["Stop", "SessionEnd"].contains(&boundary["event"].as_str().unwrap_or(""))
        || (boundary["turn_hash"] != task["native"]["turn_hash"]
            && !assessment["input"]["owned_root_turn_hashes"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|turn| *turn == boundary["turn_hash"]))
        || contract::timestamp(
            assessment["submitted_at_utc"]
                .as_str()
                .ok_or_else(|| error("invalid_assessment"))?,
        )? > contract::timestamp(
            boundary["observed_at_utc"]
                .as_str()
                .ok_or_else(|| error("invalid_boundary"))?,
        )?
    {
        return Err(error("assessment_end_mismatch"));
    }
    if value["kind"] == "groundline-learning-finalize-request"
        && ["correction_kind", "correction_evidence_sha256"]
            .into_iter()
            .any(|k| value["input"][k] != assessment["input"][k])
    {
        return Err(error("assessment_correction_mismatch"));
    }
    if value["kind"] == "groundline-learning-task-outcome" {
        let link = reference(hashes, value, "link_sha256", "groundline-learning-link")?;
        if contract::timestamp(
            link["completed_at_utc"]
                .as_str()
                .ok_or_else(|| error("invalid_link_timestamp"))?,
        )? < contract::timestamp(
            boundary["observed_at_utc"]
                .as_str()
                .ok_or_else(|| error("invalid_boundary"))?,
        )? || ["correction_kind", "correction_evidence_sha256"]
            .into_iter()
            .any(|k| link[k] != assessment["input"][k])
        {
            return Err(error("assessment_result_mismatch"));
        }
    }
    Ok(())
}

pub(super) fn check_request_evidence(
    state: &State,
    request: &Value,
    receipt: &Value,
) -> Result<(), ContractError> {
    let Some(hash) = request.get("assessment_sha256").and_then(Value::as_str) else {
        return Ok(());
    };
    let assessment = state
        .records
        .iter()
        .find(|v| learning::content_sha256(v).ok().as_deref() == Some(hash))
        .ok_or_else(|| error("assessment_reference_missing"))?;
    checked_evidence(
        state,
        assessment["input_sha256"]
            .as_str()
            .ok_or_else(|| error("invalid_assessment"))?,
    )?;
    let expected = assessment["input"]["verification"]["evidence_sha256"]
        .as_str()
        .or_else(|| assessment["input_sha256"].as_str())
        .ok_or_else(|| error("invalid_assessment"))?;
    checked_evidence(state, expected)?;
    if let Some(hash) = assessment["input"]["correction_evidence_sha256"].as_str() {
        checked_evidence(state, hash)?;
    }
    if let Some(hash) = request["native_response_proof_sha256"].as_str() {
        let proof = parse(&read_private(&checked_evidence(state, hash)?)?)?;
        let boundary = state
            .records
            .iter()
            .find(|v| {
                learning::content_sha256(v).ok().as_deref() == request["boundary_sha256"].as_str()
            })
            .ok_or_else(|| error("boundary_missing"))?;
        let root_closed = proof["proof"]["root_closed_at_utc"]
            .as_str()
            .ok_or_else(|| error("native_response_proof_mismatch"))?;
        let closed_at = contract::timestamp(root_closed)?;
        let boundary_at = contract::timestamp(
            boundary["observed_at_utc"]
                .as_str()
                .ok_or_else(|| error("invalid_boundary"))?,
        )?;
        if proof["kind"] != "groundline-native-response-proof"
            || proof["schema"] != 1
            || proof["proof"]["closure_observed"] != true
            || closed_at
                < contract::timestamp(
                    assessment["submitted_at_utc"]
                        .as_str()
                        .ok_or_else(|| error("invalid_assessment"))?,
                )?
            || contract::timestamp(
                receipt["completed_at_utc"]
                    .as_str()
                    .ok_or_else(|| error("invalid_receipt"))?,
            )? != closed_at.max(boundary_at)
            || proof["proof"]["resources"] != receipt["resources"]
            || proof["proof"]["root_effective"] != receipt["effective"]
            || proof["proof"]["native_artifacts"][0]["source_sha256"]
                != request["native"]["artifact_sha256"]
        {
            return Err(error("native_response_proof_mismatch"));
        }
    }
    if receipt["verification"]["evidence_sha256"] != expected
        || ["status", "evidence_kind", "rework"]
            .into_iter()
            .any(|k| receipt["verification"][k] != assessment["input"]["verification"][k])
    {
        return Err(error("assessment_result_mismatch"));
    }
    Ok(())
}

/// A generated request commits its exact original receipt before any durable
/// ownership or live publication. Retry never rereads a now-growing rollout
/// to regenerate selections, costs, or a different receipt SHA.
pub(super) fn restore_generated_receipt(
    profile: &contract::LearningProfile,
    state: &State,
    request: &Value,
    receipts: &mut Vec<(String, Value)>,
) -> Result<Option<Value>, ContractError> {
    if request["assessment_sha256"].as_str().is_none()
        || request["native_response_proof_sha256"].as_str().is_none()
    {
        return Ok(None);
    }
    let hash = request["receipt_sha256"]
        .as_str()
        .ok_or_else(|| error("invalid_finalize_request"))?;
    let bytes = read_private(&evidence_path(state, hash)?)?;
    if sha256(&bytes) != hash {
        return Err(error("assessment_evidence_mismatch"));
    }
    let receipt = parse(&bytes)?;
    delivery::validate_receipt(&receipt)?;
    check_request_evidence(state, request, &receipt)?;
    if let Some((_, existing)) = receipts.iter().find(|(h, _)| h == hash) {
        if *existing != receipt {
            return Err(error("assessment_original_receipt_mismatch"));
        }
    } else {
        let mut collection: Vec<_> = receipts.iter().map(|(_, v)| v.clone()).collect();
        collection.push(receipt.clone());
        delivery::validate_receipt_collection(&collection)?;
    }
    // These checks also reject a conflicting reservation left by interrupted
    // work; saved evidence never bypasses response or unit ownership.
    storage::reserve_receipt(state, profile, hash, &receipt)?;
    write_bytes_draft(
        &Path::new(&profile.deliveries).join(format!("{hash}.json")),
        &bytes,
        state,
    )?;
    if !receipts.iter().any(|(h, _)| h == hash) {
        receipts.push((hash.into(), receipt.clone()));
    }
    Ok(Some(receipt))
}

struct DiscoveryBudget {
    started: Instant,
    entries: usize,
    candidates: usize,
    bytes: u64,
}

impl DiscoveryBudget {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            entries: 0,
            candidates: 0,
            bytes: 0,
        }
    }
    fn exhausted(&self) -> bool {
        self.started.elapsed() >= DISCOVERY_BUDGET
            || self.candidates >= DISCOVERY_CANDIDATES
            || self.bytes >= DISCOVERY_BYTES
    }
}

fn discover(
    home: &Path,
    expected: &[(String, String)],
    budget: &mut DiscoveryBudget,
) -> Result<Option<Vec<NativeRef>>, ContractError> {
    let mut found = BTreeMap::new();
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for name in ["archived_sessions", "sessions"] {
        let path = home.join(name);
        if fs::symlink_metadata(&path).is_ok() {
            directories.push((path, 0));
        }
    }
    while let Some((directory, depth)) = directories.pop() {
        if budget.exhausted() || budget.entries >= DISCOVERY_ENTRIES {
            break;
        }
        let handle = directory_handle(&directory, false)?;
        if handle
            .metadata()
            .map_err(|_| error("native_discovery_unavailable"))?
            .uid()
            != rustix::process::geteuid().as_raw()
        {
            return Err(error("native_artifact_owner_required"));
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|_| error("native_discovery_unavailable"))? {
            if budget.exhausted() || budget.entries >= DISCOVERY_ENTRIES {
                break;
            }
            budget.entries += 1;
            let entry = entry.map_err(|_| error("native_discovery_unavailable"))?;
            paths.push(entry.path());
        }
        paths.sort();
        for path in paths.into_iter().rev() {
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| error("native_discovery_unavailable"))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() && depth < 5 {
                directories.push((path, depth + 1));
            } else if metadata.is_file()
                && path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| v.ends_with(".jsonl") || v.ends_with(".jsonl.zst"))
            {
                files.push(path);
            }
        }
        // Stack order is oldest-to-newest so its next pop is the newest dir.
        directories.sort_by(|a, b| a.0.cmp(&b.0));
    }
    // The UUID suffix is a scheduling hint only. An old, still-active native
    // session must be tried before 64 unrelated recent files exhaust reading.
    // Every selected source still requires prefix identity and full proof.
    files.sort_by(|a, b| {
        filename_matches(b, expected)
            .cmp(&filename_matches(a, expected))
            .then_with(|| b.cmp(a))
    });
    for path in files {
        if budget.exhausted() {
            break;
        }
        let remaining = DISCOVERY_BYTES.saturating_sub(budget.bytes);
        budget.candidates += 1;
        let identity = match learning_response::identify(&path, remaining) {
            Ok(identity) => identity,
            Err(_) => {
                // A failed unrelated prefix must not prevent discovery of
                // the exact task source. Conservatively charge its cap.
                budget.bytes = budget.bytes.saturating_add(remaining.min(2 * 1024 * 1024));
                continue;
            }
        };
        budget.bytes = budget
            .bytes
            .saturating_add(identity.bytes_read.max(identity.expanded_bytes));
        for (session, turn) in expected {
            // Prefix discovery establishes only session identity. The
            // full independent observation checks each expected turn.
            if identity.session_hash.as_ref() == Some(session) {
                found.insert(
                    (session.clone(), turn.clone()),
                    NativeRef {
                        path: path.clone(),
                        session_hash: session.clone(),
                        turn_hash: turn.clone(),
                    },
                );
            }
        }
        if expected.iter().all(|key| found.contains_key(key)) {
            return Ok(Some(
                expected
                    .iter()
                    .map(|key| found.remove(key).expect("matched identity"))
                    .collect(),
            ));
        }
    }
    Ok(None)
}

fn filename_matches(path: &Path, expected: &[(String, String)]) -> bool {
    let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
        return false;
    };
    let Some(stem) = name
        .strip_suffix(".jsonl.zst")
        .or_else(|| name.strip_suffix(".jsonl"))
    else {
        return false;
    };
    if !stem.starts_with("rollout-") {
        return false;
    }
    let Some(id) = stem.get(stem.len().saturating_sub(36)..) else {
        return false;
    };
    if uuid::Uuid::parse_str(id).is_err() {
        return false;
    }
    let hint = sha256(format!("groundline-hook-session\0{id}").as_bytes());
    expected.iter().any(|(session, _)| *session == hint)
}

fn ending_boundary(
    home: &Path,
    profile: &contract::LearningProfile,
    task: &Value,
    assessment: &Value,
    state: &State,
) -> Result<Option<Value>, ContractError> {
    let Some(session) = task["native"]["session_hash"].as_str() else {
        return Ok(None);
    };
    let Some(turn) = task["native"]["turn_hash"].as_str() else {
        return Ok(None);
    };
    let mut turns = BTreeSet::from([turn.to_owned()]);
    turns.extend(
        assessment["input"]["owned_root_turn_hashes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned),
    );
    let submitted = contract::timestamp(
        assessment["submitted_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_assessment"))?,
    )?;
    let mut candidates = Vec::new();
    let matches = |v: &Value| {
        v["session_hash"].as_str() == Some(session)
            && v["turn_hash"].as_str().is_some_and(|t| turns.contains(t))
            && ["Stop", "SessionEnd"].contains(&v["event"].as_str().unwrap_or(""))
            && v["observed_at_utc"]
                .as_str()
                .and_then(|at| contract::timestamp(at).ok())
                .is_some_and(|at| at >= submitted)
    };
    for v in &state.records {
        if v["kind"] == "groundline-learning-boundary" && matches(v) {
            candidates.push(v.clone());
        }
    }
    let mut entries = 0;
    let mut bytes = END_BYTES;
    for turn in &turns {
        let root = boundaries_root(home)
            .join("archive/references")
            .join(session)
            .join(turn);
        if fs::symlink_metadata(&root).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
            continue;
        }
        directory_handle(&root, true)?;
        for shard in fs::read_dir(root).map_err(|_| error("boundary_lookup_unavailable"))? {
            entries += 1;
            if entries > END_ENTRIES {
                return Err(error("boundary_lookup_budget_exceeded"));
            }
            let shard = shard
                .map_err(|_| error("boundary_lookup_unavailable"))?
                .path();
            directory_handle(&shard, true)?;
            for entry in fs::read_dir(shard).map_err(|_| error("boundary_lookup_unavailable"))? {
                entries += 1;
                if entries > END_ENTRIES {
                    return Err(error("boundary_lookup_budget_exceeded"));
                }
                let path = entry
                    .map_err(|_| error("boundary_lookup_unavailable"))?
                    .path();
                let Some(data) = optional_private_bounded(&path, Some(&mut bytes))? else {
                    continue;
                };
                let index = parse(&data)?;
                if index["kind"] != "groundline-learning-boundary-reference" || index["schema"] != 1
                {
                    return Err(error("invalid_boundary_index"));
                }
                if !matches(&index) {
                    continue;
                }
                let (boundary, hash) = checked_boundary_bounded(
                    home,
                    profile,
                    index["boundary_id"]
                        .as_str()
                        .ok_or_else(|| error("invalid_boundary_index"))?,
                    Some(&mut bytes),
                )?;
                if index["boundary_sha256"] != hash || !matches(&boundary) {
                    return Err(error("boundary_index_mismatch"));
                }
                candidates.push(boundary);
            }
        }
    }
    candidates.sort_by_key(|v| {
        v["observed_at_utc"]
            .as_str()
            .and_then(|at| contract::timestamp(at).ok())
    });
    Ok(candidates.pop())
}

fn pending(assessment: &Value, reason: &str) -> Value {
    json!({"assessment_sha256":learning::content_sha256(assessment).ok(),
        "task_sha256":assessment["input"]["task_sha256"],"status":"PENDING","reason":reason,"retry_safe":true})
}

pub(super) fn consume_pending(
    home: &Path,
    profile: &contract::LearningProfile,
    state: &mut State,
    receipts: &mut Vec<(String, Value)>,
) -> Result<Vec<Value>, ContractError> {
    let assessments: Vec<_> = state
        .records
        .iter()
        .filter(|v| v["kind"] == "groundline-learning-assessment")
        .cloned()
        .collect();
    let mut budget = DiscoveryBudget::new();
    let mut results = Vec::new();
    for assessment in assessments {
        if state.records.iter().any(|v| {
            v["kind"] == "groundline-learning-task-outcome"
                && v["task_sha256"] == assessment["input"]["task_sha256"]
        }) {
            continue;
        }
        match connect(home, profile, &assessment, state, receipts, &mut budget) {
            Ok(Some(outcome)) => results.push(
                json!({"assessment_sha256":learning::content_sha256(&assessment)?,
                "task_sha256":assessment["input"]["task_sha256"],"status":"FINALIZED",
                "outcome_sha256":learning::content_sha256(&outcome)?,
                "native_response_proof_sha256":outcome["native_response_proof_sha256"],"mutation_performed":true}),
            ),
            Ok(None) => results.push(pending(
                &assessment,
                "actual_native_end_or_artifact_required",
            )),
            Err(e) => results.push(pending(&assessment, &e.0)),
        }
    }
    Ok(results)
}

fn connect(
    home: &Path,
    profile: &contract::LearningProfile,
    assessment: &Value,
    state: &mut State,
    receipts: &mut Vec<(String, Value)>,
    budget: &mut DiscoveryBudget,
) -> Result<Option<Value>, ContractError> {
    let input: contract::AssessmentInput = serde_json::from_value(assessment["input"].clone())
        .map_err(|_| error("invalid_assessment"))?;
    if let Some(request) = state.records.iter().find(|v| {
        v["kind"] == "groundline-learning-finalize-request"
            && v["input"]["task_sha256"] == input.task_sha256
    }) {
        if let Some((_, receipt)) = receipts
            .iter()
            .find(|(h, _)| request["receipt_sha256"] == *h)
        {
            check_request_evidence(state, request, receipt)?;
        }
        return Err(error("assessment_original_receipt_required"));
    }
    let original = checked_evidence(
        state,
        assessment["input_sha256"]
            .as_str()
            .ok_or_else(|| error("invalid_assessment"))?,
    )?;
    let verification_hash = input
        .verification
        .evidence_sha256
        .as_deref()
        .or_else(|| assessment["input_sha256"].as_str())
        .ok_or_else(|| error("invalid_assessment"))?;
    let verification = checked_evidence(state, verification_hash)?;
    if let Some(hash) = &input.correction_evidence_sha256 {
        checked_evidence(state, hash)?;
    }
    let original_input: contract::AssessmentInput =
        serde_json::from_slice(&read_private(&original)?)
            .map_err(|_| error("invalid_assessment"))?;
    if serde_json::to_value(original_input).map_err(|_| error("serialization_failed"))?
        != assessment["input"]
    {
        return Err(error("assessment_evidence_mismatch"));
    }
    let task = state
        .records
        .iter()
        .find(|v| {
            v["kind"] == "groundline-learning-task-start"
                && learning::content_sha256(v).ok().as_deref() == Some(&input.task_sha256)
        })
        .cloned()
        .ok_or_else(|| error("task_missing"))?;
    let Some(boundary) = ending_boundary(home, profile, &task, assessment, state)? else {
        return Ok(None);
    };
    let Some(session) = task["native"]["session_hash"].as_str() else {
        return Ok(None);
    };
    let Some(turn) = task["native"]["turn_hash"].as_str() else {
        return Ok(None);
    };
    let mut expected = vec![(session.to_owned(), turn.to_owned())];
    expected.extend(
        input
            .children
            .iter()
            .map(|child| (child.session_hash.clone(), child.turn_hash.clone())),
    );
    let Some(mut native) = discover(home, &expected, budget)? else {
        return Ok(None);
    };
    let root = native.remove(0);
    let start = state
        .records
        .iter()
        .find(|v| learning::content_sha256(v).ok().as_deref() == task["boundary_sha256"].as_str())
        .ok_or_else(|| error("boundary_missing"))?;
    let observation_cutoff_utc = chrono::Utc::now().to_rfc3339();
    let boundary_at = contract::timestamp(
        boundary["observed_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_boundary"))?,
    )?;
    if boundary_at > contract::timestamp(&observation_cutoff_utc)? {
        return Err(error("boundary_after_observation"));
    }
    let proof = learning_response::observe(&TaskObservation {
        unit_hash: task["scope"]["unit_hash"]
            .as_str()
            .ok_or_else(|| error("invalid_task_start"))?
            .into(),
        root,
        children: native,
        start_utc: start["observed_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_boundary"))?
            .into(),
        // App may dispatch Stop before persisting task_complete. The current
        // clock bounds a read of only explicitly owned turns; it is never the
        // completion timestamp. A retry can observe the actual later closure.
        end_utc: observation_cutoff_utc.clone(),
        owned_root_turn_hashes: input.owned_root_turn_hashes.clone(),
    })?;
    if !proof.closure_observed
        || proof
            .root_closed_at_utc
            .as_deref()
            .and_then(|at| contract::timestamp(at).ok())
            .is_none_or(|at| {
                contract::timestamp(assessment["submitted_at_utc"].as_str().unwrap_or(""))
                    .is_ok_and(|submitted| at < submitted)
            })
    {
        return Ok(None);
    }
    let closed_at = proof
        .root_closed_at_utc
        .as_deref()
        .ok_or_else(|| error("native_closure_missing"))?;
    let completed_at_utc = if contract::timestamp(closed_at)? > boundary_at {
        closed_at
    } else {
        boundary["observed_at_utc"]
            .as_str()
            .ok_or_else(|| error("invalid_boundary"))?
    };
    let mut proof_bytes = serde_json::to_vec_pretty(
        &json!({"kind":"groundline-native-response-proof","schema":1,
            "observation_cutoff_utc":observation_cutoff_utc,"proof":&proof}),
    )
    .map_err(|_| error("serialization_failed"))?;
    proof_bytes.push(b'\n');
    let proof_sha256 = sha256(&proof_bytes);
    save_evidence(state, &proof_sha256, &proof_bytes)?;
    let mut evidence = BTreeMap::new();
    for entry in &proof.selection_evidence {
        let mut bytes = serde_json::to_vec_pretty(&entry.artifact)
            .map_err(|_| error("serialization_failed"))?;
        bytes.push(b'\n');
        save_evidence(state, &entry.sha256, &bytes)?;
        evidence.insert(
            entry.sha256.clone(),
            checked_evidence(state, &entry.sha256)?,
        );
    }
    let mut resources =
        serde_json::to_value(&proof.resources).map_err(|_| error("serialization_failed"))?;
    for entry in resources["entries"]
        .as_array_mut()
        .ok_or_else(|| error("invalid_resources"))?
    {
        add_selection_path(&mut entry["effective"], &evidence)?;
    }
    let mut effective =
        serde_json::to_value(&proof.root_effective).map_err(|_| error("serialization_failed"))?;
    add_selection_path(&mut effective, &evidence)?;
    let manifest = json!({"kind":"groundline-delivery-manifest","schema":2,
        "unit_hash":task["scope"]["unit_hash"],"cohort_sha256":task["scope"]["cohort_sha256"],"phase":task["scope"]["phase"],
        "completed_at_utc":completed_at_utc,"recommendation":null,"requested":null,"effective":effective,
        "verification":{"status":input.verification.status,"evidence_kind":input.verification.evidence_kind,
            "evidence_sha256":verification_hash,"artifact_path":verification,"rework":input.verification.rework},
        "resources":resources});
    let temporary_manifest = state
        .path
        .join("evidence")
        .join(format!(".assessment-manifest-{}.tmp", uuid::Uuid::new_v4()));
    let temporary_receipt = state
        .path
        .join("evidence")
        .join(format!(".assessment-receipt-{}.tmp", uuid::Uuid::new_v4()));
    let receipt_bytes = (|| {
        write_draft(&temporary_manifest, &manifest, state)?;
        crate::delivery::record(&temporary_manifest, &temporary_receipt)?;
        read_private(&temporary_receipt)
    })();
    let _ = fs::remove_file(&temporary_manifest);
    let _ = fs::remove_file(&temporary_receipt);
    let receipt_bytes = receipt_bytes?;
    let receipt = parse(&receipt_bytes)?;
    delivery::validate_receipt(&receipt)?;
    let receipt_hash = sha256(&receipt_bytes);
    let mut collection: Vec<_> = receipts.iter().map(|(_, v)| v.clone()).collect();
    collection.push(receipt.clone());
    delivery::validate_receipt_collection(&collection)?;
    let final_input = contract::TaskFinalizeInput {
        kind: "groundline-learning-task-finalize-input".into(),
        schema: 1,
        task_sha256: input.task_sha256,
        boundary_id: boundary["boundary_id"]
            .as_str()
            .ok_or_else(|| error("invalid_boundary"))?
            .into(),
        correction_kind: input.correction_kind,
        correction_evidence_sha256: input.correction_evidence_sha256,
    };
    let mut final_bytes =
        serde_json::to_vec_pretty(&final_input).map_err(|_| error("serialization_failed"))?;
    final_bytes.push(b'\n');
    let native_sources =
        serde_json::to_value(&proof.native_artifacts).map_err(|_| error("serialization_failed"))?;
    let native_sha = native_sources
        .as_array()
        .and_then(|v| v.first())
        .and_then(|v| v["source_sha256"].as_str())
        .ok_or_else(|| error("native_artifact_digest_missing"))?;
    let request = serde_json::to_value(contract::FinalizeRequest {
        kind: "groundline-learning-finalize-request".into(),
        schema: 1,
        input_sha256: sha256(&final_bytes),
        input: final_input,
        boundary_sha256: learning::content_sha256(&boundary)?,
        native: contract::NativeReference {
            session_hash: boundary["session_hash"].as_str().map(str::to_owned),
            turn_hash: boundary["turn_hash"].as_str().map(str::to_owned),
            artifact_sha256: Some(native_sha.into()),
            boundary_matched: true,
        },
        receipt_sha256: receipt_hash.clone(),
        assessment_sha256: Some(learning::content_sha256(assessment)?),
        native_response_proof_sha256: Some(proof_sha256),
    })
    .map_err(|_| error("serialization_failed"))?;
    let snapshot = state
        .records
        .iter()
        .find(|v| learning::content_sha256(v).ok().as_deref() == task["snapshot_sha256"].as_str())
        .ok_or_else(|| error("snapshot_missing"))?;
    learning::prepare_outcome(snapshot, &receipt, &receipt_hash)?;
    // Receipt and its journal are durable before publishing ownership. A
    // failed publication can then reuse the exact original after native append.
    save_evidence(state, &receipt_hash, &receipt_bytes)?;
    state.append(&boundary)?;
    state.append(&request)?;
    storage::reserve_receipt(state, profile, &receipt_hash, &receipt)?;
    write_bytes_draft(
        &Path::new(&profile.deliveries).join(format!("{receipt_hash}.json")),
        &receipt_bytes,
        state,
    )?;
    let outcome = finalize_request(&request, &receipt, state)?;
    receipts.push((receipt_hash, receipt));
    Ok(Some(outcome))
}

fn add_selection_path(
    selection: &mut Value,
    evidence: &BTreeMap<String, PathBuf>,
) -> Result<(), ContractError> {
    if selection.is_null() {
        return Ok(());
    }
    let hash = selection["evidence_sha256"]
        .as_str()
        .ok_or_else(|| error("selection_artifact_missing"))?;
    let path = evidence
        .get(hash)
        .ok_or_else(|| error("selection_artifact_missing"))?;
    selection["artifact_path"] = json!(path);
    Ok(())
}
