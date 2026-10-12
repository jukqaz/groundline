//! Immutable cold records and small direct indexes. Active queries are bounded;
//! old unreferenced work remains private and can be retried by exact input SHA.
use super::*;

fn record_path(state: &Path, hash: &str) -> Result<PathBuf, ContractError> {
    if !learning::digest(hash) {
        return Err(error("invalid_archive_reference"));
    }
    Ok(state
        .join("archive")
        .join(&hash[..2])
        .join(format!("{hash}.json")))
}

pub(in crate::learning) fn archived_record(
    state: &Path,
    hash: &str,
) -> Result<Option<Value>, ContractError> {
    let Some(bytes) = optional_private(&record_path(state, hash)?)? else {
        return Ok(None);
    };
    let value = parse(&bytes)?;
    super::super::validate_record(&value)?;
    if learning::content_sha256(&value)? != hash {
        return Err(error("archive_digest_mismatch"));
    }
    Ok(Some(value))
}

fn refs(value: &Value) -> Vec<String> {
    let mut refs = Vec::new();
    let mut add = |v: &Value| {
        if let Some(v) = v.as_str().filter(|v| learning::digest(v)) {
            refs.push(v.to_owned());
        }
    };
    match value["kind"].as_str() {
        Some("groundline-learning-task-start") => {
            add(&value["snapshot_sha256"]);
            add(&value["boundary_sha256"]);
        }
        Some("groundline-learning-task-outcome") => {
            add(&value["task_sha256"]);
            add(&value["boundary_sha256"]);
            add(&value["link_sha256"]);
            add(&value["assessment_sha256"]);
        }
        Some("groundline-learning-finalize-request") => {
            add(&value["input"]["task_sha256"]);
            add(&value["boundary_sha256"]);
            add(&value["assessment_sha256"]);
        }
        Some("groundline-learning-assessment") => {
            add(&value["input"]["task_sha256"]);
        }
        Some("groundline-learning-conversation-snapshot") => {
            add(&value["input"]["review_sha256"]);
        }
        Some("groundline-learning-conversation-change") => {
            add(&value["input"]["review_sha256"]);
            add(&value["input"]["snapshot_sha256"]);
        }
        Some(
            "groundline-learning-conversation-activation"
            | "groundline-learning-conversation-observation",
        ) => {
            add(&value["input"]["change_sha256"]);
        }
        Some("groundline-learning-trial-authorization") => add(&value["proposal_sha256"]),
        Some("groundline-learning-duplicate-attempt") => add(&value["duplicate_of_sha256"]),
        Some("groundline-learning-evaluation-record") => {
            add(&value["proposal_sha256"]);
            for v in value["input"]["baseline_refs"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(
                    value["input"]["followup_refs"]
                        .as_array()
                        .into_iter()
                        .flatten(),
                )
            {
                add(v);
            }
        }
        Some("groundline-learning-decision") => {
            add(&value["proposal_sha256"]);
            add(&value["evaluation_sha256"]);
            add(&value["previous_decision_sha256"]);
            for v in value["evidence_refs"].as_array().into_iter().flatten() {
                add(v);
            }
        }
        Some("groundline-learning-proposal" | "groundline-learning-processing") => {
            for v in value["evidence_refs"].as_array().into_iter().flatten() {
                add(v);
            }
        }
        _ => {}
    }
    refs
}

pub(in crate::learning) fn load_archive_references(
    state: &Path,
    records: &mut Vec<Value>,
    extra: &[String],
) -> Result<(), ContractError> {
    let mut seen: BTreeSet<_> = records
        .iter()
        .map(learning::content_sha256)
        .collect::<Result<_, _>>()?;
    let mut queue: Vec<_> = records
        .iter()
        .flat_map(refs)
        .chain(extra.iter().cloned())
        .collect();
    for record in records
        .iter()
        .filter(|record| record["kind"] == "groundline-learning-conversation-review")
    {
        let root = record["input_sha256"]
            .as_str()
            .ok_or_else(|| error("invalid_conversation_work_record"))?;
        queue.extend(conversation_member_refs(state, root)?);
    }
    let mut total: usize = records
        .iter()
        .map(|v| {
            serde_json::to_vec(v)
                .map(|v| v.len())
                .map_err(|_| error("serialization_failed"))
        })
        .sum::<Result<usize, _>>()?;
    while let Some(hash) = queue.pop() {
        if !seen.insert(hash.clone()) {
            continue;
        }
        let Some(value) = archived_record(state, &hash)? else {
            continue;
        };
        total = total
            .checked_add(
                serde_json::to_vec(&value)
                    .map_err(|_| error("serialization_failed"))?
                    .len(),
            )
            .ok_or_else(|| error("state_limit_exceeded"))?;
        if records.len() >= MAX_ENTRIES - 2 || total > MAX_DIRECTORY_BYTES {
            return Err(error("active_reference_limit_exceeded"));
        }
        queue.extend(refs(&value));
        if value["kind"] == "groundline-learning-conversation-review" {
            let root = value["input_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_conversation_work_record"))?;
            queue.extend(conversation_member_refs(state, root)?);
        }
        records.push(value);
    }
    Ok(())
}

fn input_index_path(state: &Path, input: &str) -> Result<PathBuf, ContractError> {
    if !learning::digest(input) {
        return Err(error("invalid_archive_reference"));
    }
    Ok(state
        .join("archive/inputs")
        .join(&input[..2])
        .join(format!("{input}.json")))
}

pub(super) fn replay(
    state: &Path,
    input: &str,
    kind: &str,
) -> Result<Option<Value>, ContractError> {
    let Some(bytes) = optional_private(&input_index_path(state, input)?)? else {
        return Ok(None);
    };
    let index = parse(&bytes)?;
    if index["kind"] != "groundline-learning-archive-input"
        || index["schema"] != 1
        || index["input_sha256"] != input
    {
        return Err(error("invalid_archive_index"));
    }
    let hash = index["record_sha256"]
        .as_str()
        .ok_or_else(|| error("invalid_archive_index"))?;
    let value = archived_record(state, hash)?
        .or_else(|| {
            read_private(&state.join(format!("{hash}.json")))
                .ok()
                .and_then(|v| parse(&v).ok())
        })
        .ok_or_else(|| error("archive_record_missing"))?;
    if value["kind"] != kind
        || value["input_sha256"] != input
        || learning::content_sha256(&value)? != hash
    {
        return Err(error("invalid_archive_index"));
    }
    Ok(Some(value))
}

/// Existing immutable input indexes retain the completed task's direct
/// closure. This permits exact assessment retry/conflict checks after cooling.
pub(super) fn restore_task_completion(
    state: &Path,
    records: &mut Vec<Value>,
    task_hash: &str,
) -> Result<(), ContractError> {
    let Some(task) = records.iter().find(|v| {
        v["kind"] == "groundline-learning-task-start"
            && learning::content_sha256(v).ok().as_deref() == Some(task_hash)
    }) else {
        return Ok(());
    };
    let input = task["input_sha256"]
        .as_str()
        .ok_or_else(|| error("invalid_task_start"))?;
    let Some(bytes) = optional_private(&input_index_path(state, input)?)? else {
        return Ok(());
    };
    let index = parse(&bytes)?;
    if index["kind"] != "groundline-learning-archive-input"
        || index["schema"] != 1
        || index["input_sha256"] != input
        || index["record_sha256"] != task_hash
    {
        return Err(error("invalid_archive_index"));
    }
    let extra = ["assessment_sha256", "completion_sha256"]
        .into_iter()
        .filter_map(|k| index.get(k))
        .map(|v| {
            v.as_str()
                .filter(|h| learning::digest(h))
                .map(str::to_owned)
                .ok_or_else(|| error("invalid_archive_index"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    load_archive_references(state, records, &extra)?;
    for key in ["assessment_sha256", "completion_sha256"] {
        let Some(hash) = index.get(key).and_then(Value::as_str) else {
            continue;
        };
        let value = records
            .iter()
            .find(|v| learning::content_sha256(v).ok().as_deref() == Some(hash))
            .ok_or_else(|| error("archive_record_missing"))?;
        if (key == "assessment_sha256"
            && (value["kind"] != "groundline-learning-assessment"
                || value["input"]["task_sha256"] != task_hash))
            || (key == "completion_sha256"
                && (value["kind"] != "groundline-learning-task-outcome"
                    || value["task_sha256"] != task_hash))
        {
            return Err(error("invalid_archive_index"));
        }
    }
    super::super::validate_record_collection(records)
}

fn archive_file(source: &Path, destination: &Path, before: &[u8]) -> Result<(), ContractError> {
    let source_parent =
        directory_handle(source.parent().ok_or_else(|| error("invalid_path"))?, true)?;
    let destination_parent = directory_handle(
        destination.parent().ok_or_else(|| error("invalid_path"))?,
        true,
    )?;
    let source_name = source.file_name().ok_or_else(|| error("invalid_path"))?;
    let destination_name = destination
        .file_name()
        .ok_or_else(|| error("invalid_path"))?;
    if read_at(&source_parent, source_name)? != before {
        return Err(error("archive_source_changed"));
    }
    match rustix::fs::renameat_with(
        &source_parent,
        source_name,
        &destination_parent,
        destination_name,
        rustix::fs::RenameFlags::NOREPLACE,
    ) {
        Ok(()) => {}
        Err(rustix::io::Errno::EXIST) => {
            if read_at(&destination_parent, destination_name)? != before {
                return Err(error("archive_conflict"));
            }
            unlinkat(&source_parent, source_name, AtFlags::empty())
                .map_err(|_| error("archive_write_failed"))?;
        }
        Err(_) => return Err(error("archive_write_failed")),
    }
    destination_parent
        .sync_all()
        .and_then(|_| source_parent.sync_all())
        .map_err(|_| error("archive_write_failed"))?;
    if read_private(destination)? != before {
        return Err(error("archive_source_changed"));
    }
    Ok(())
}

pub(super) fn compact(
    state: &mut State,
    profile: &contract::LearningProfile,
) -> Result<usize, ContractError> {
    let conversation_moved = compact_conversations(state)?;
    if state.records.len() < 400 {
        return Ok(conversation_moved);
    }
    let hashes: BTreeMap<_, _> = state
        .records
        .iter()
        .map(|v| Ok((learning::content_sha256(v)?, v)))
        .collect::<Result<_, ContractError>>()?;
    let mut keep = BTreeSet::new();
    // Preserve proposals/analysis ownership, decisions, and applied comparisons.
    // Their referenced closure can never be evicted merely to hide a cost gap.
    for (hash, value) in &hashes {
        if [
            "groundline-learning-proposal",
            "groundline-learning-duplicate-attempt",
            "groundline-learning-decision",
            "groundline-learning-evaluation-record",
            "groundline-learning-trial-authorization",
            "groundline-learning-conversation-review",
            "groundline-learning-conversation-snapshot",
            "groundline-learning-conversation-change",
            "groundline-learning-conversation-activation",
            "groundline-learning-conversation-observation",
        ]
        .contains(&value["kind"].as_str().unwrap_or(""))
        {
            keep.insert(hash.clone());
        }
    }
    let mut completed = Vec::new();
    for (hash, value) in &hashes {
        if value["kind"] != "groundline-learning-task-start" {
            continue;
        }
        let finished = hashes
            .values()
            .any(|v| v["kind"] == "groundline-learning-task-outcome" && v["task_sha256"] == *hash);
        if finished {
            let at = value["snapshot_sha256"]
                .as_str()
                .and_then(|h| hashes.get(h))
                .and_then(|v| v["captured_at_utc"].as_str())
                .unwrap_or("");
            completed.push((at.to_owned(), hash.clone()));
        } else {
            keep.insert(hash.clone());
        }
    }
    completed.sort();
    keep.extend(completed.into_iter().rev().take(32).map(|(_, hash)| hash));
    loop {
        let previous = keep.len();
        let included: Vec<_> = keep.iter().filter_map(|h| hashes.get(h)).copied().collect();
        keep.extend(included.iter().flat_map(|v| refs(v)));
        for (hash, value) in &hashes {
            let related_task = value["task_sha256"]
                .as_str()
                .or_else(|| value["input"]["task_sha256"].as_str());
            if related_task.is_some_and(|h| keep.contains(h))
                || value["kind"] == "groundline-learning-task-outcome"
                    && value["link_sha256"]
                        .as_str()
                        .is_some_and(|h| keep.contains(h))
            {
                keep.insert(hash.clone());
            }
        }
        if previous == keep.len() {
            break;
        }
    }
    let archive = state.path.join("archive");
    ensure_private_directory(&archive)?;
    let inputs = archive.join("inputs");
    ensure_private_directory(&inputs)?;
    let removed: Vec<_> = hashes
        .iter()
        .filter(|(h, _)| !keep.contains(*h))
        .map(|(h, v)| (h.clone(), (*v).clone()))
        .collect();
    // Index before moving: either hot or cold exact record remains available
    // after interruption; State::open loads missing referenced cold records.
    for (hash, value) in &removed {
        if [
            "groundline-learning-task-start",
            "groundline-learning-task-outcome",
            "groundline-learning-assessment",
        ]
        .contains(&value["kind"].as_str().unwrap_or(""))
        {
            let input = value["input_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_task_start"))?;
            let path = input_index_path(&state.path, input)?;
            ensure_private_directory(path.parent().ok_or_else(|| error("invalid_path"))?)?;
            let mut index = json!({"kind":"groundline-learning-archive-input","schema":1,"input_sha256":input,"record_sha256":hash});
            if let Some(previous) = optional_private(&path)? {
                let previous = parse(&previous)?;
                if ["kind", "schema", "input_sha256", "record_sha256"]
                    .into_iter()
                    .any(|k| previous[k] != index[k])
                {
                    return Err(error("invalid_archive_index"));
                }
                for key in ["assessment_sha256", "completion_sha256"] {
                    if let Some(value) = previous.get(key) {
                        index[key] = value.clone();
                    }
                }
            }
            if value["kind"] == "groundline-learning-task-start" {
                for related in hashes.values() {
                    if related["kind"] == "groundline-learning-assessment"
                        && related["input"]["task_sha256"] == *hash
                    {
                        index["assessment_sha256"] = json!(learning::content_sha256(related)?);
                    }
                    if related["kind"] == "groundline-learning-task-outcome"
                        && related["task_sha256"] == *hash
                    {
                        index["completion_sha256"] = json!(learning::content_sha256(related)?);
                    }
                }
            }
            write_draft(&path, &index, state)?;
        }
    }
    let mut moved = 0;
    for (hash, _) in &removed {
        let source = state.path.join(format!("{hash}.json"));
        let Some(bytes) = optional_private(&source)? else {
            continue;
        };
        let destination = record_path(&state.path, hash)?;
        ensure_private_directory(destination.parent().ok_or_else(|| error("invalid_path"))?)?;
        archive_file(&source, &destination, &bytes)?;
        moved += 1;
    }
    state
        .records
        .retain(|v| learning::content_sha256(v).is_ok_and(|h| keep.contains(&h)));
    // Cold receipts are looked up directly only when retained records need them.
    let referenced: BTreeSet<_> = state
        .records
        .iter()
        .filter_map(|v| v["receipt_sha256"].as_str().map(str::to_owned))
        .collect();
    let path = Path::new(&profile.deliveries);
    let directory = directory_handle(path, true)?;
    let cold = path.join("archive");
    ensure_private_directory(&cold)?;
    for (hash, receipt) in read_directory(path, &directory, false)? {
        reserve_receipt(state, profile, &hash, &receipt)?;
        if referenced.contains(&hash) {
            continue;
        }
        let source = path.join(format!("{hash}.json"));
        // Existing caller-named receipts are not renamed by this maintenance.
        let Some(bytes) = optional_private(&source)? else {
            continue;
        };
        let destination = cold.join(&hash[..2]).join(format!("{hash}.json"));
        ensure_private_directory(destination.parent().ok_or_else(|| error("invalid_path"))?)?;
        archive_file(&source, &destination, &bytes)?;
    }
    super::super::validate_record_collection(&state.records)?;
    Ok(moved + conversation_moved)
}

fn conversation_parent_hash<'a>(input: &'a Value, records: &'a [Value]) -> Option<&'a str> {
    input["review_sha256"].as_str().or_else(|| {
        let change = input["change_sha256"].as_str()?;
        records.iter().find(|record| learning::content_sha256(record).ok().as_deref() == Some(change))?
            ["input"]["review_sha256"].as_str()
    })
}

fn conversation_root_input(input: &Value, records: &[Value]) -> Result<String, ContractError> {
    if input["kind"] == "groundline-learning-conversation-review-input" {
        let parsed: learning::conversation_work::Input = serde_json::from_value(input.clone())
            .map_err(|_| error("invalid_conversation_work_input"))?;
        return learning::content_sha256(
            &serde_json::to_value(parsed).map_err(|_| error("serialization_failed"))?,
        );
    }
    let parent = conversation_parent_hash(input, records)
        .ok_or_else(|| error("conversation_review_reference_missing"))?;
    records
        .iter()
        .find(|record| learning::content_sha256(record).ok().as_deref() == Some(parent))
        .and_then(|record| record["input_sha256"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| error("conversation_review_reference_missing"))
}

fn conversation_key(input: &Value, records: &[Value]) -> Result<String, ContractError> {
    let parent_key = |field: &str| -> Result<Value, ContractError> {
        let hash = input[field]
            .as_str()
            .ok_or_else(|| error("invalid_conversation_work_input"))?;
        let record = records
            .iter()
            .find(|record| learning::content_sha256(record).ok().as_deref() == Some(hash))
            .ok_or_else(|| error("conversation_reference_missing"))?;
        Ok(record["input_sha256"].clone())
    };
    let key = match input["kind"].as_str() {
        Some("groundline-learning-conversation-review-input") => {
            json!(["review", input["feedback_ref_sha256"], input["pattern_id"]])
        }
        Some("groundline-learning-conversation-snapshot-input") => json!([
            "snapshot",
            parent_key("review_sha256")?,
            input["target_binding_sha256"],
            input["captured_at_utc"]
        ]),
        Some("groundline-learning-conversation-change-input") => json!([
            "change",
            parent_key("review_sha256")?,
            input["target"]["target_id"]
        ]),
        Some("groundline-learning-conversation-activation-input") => json!([
            "activation",
            parent_key("change_sha256")?,
            input["evidence_sha256"]
        ]),
        Some("groundline-learning-conversation-observation-input") => json!([
            "observation",
            parent_key("change_sha256")?,
            input["source_work_sha256"]
        ]),
        _ => return Err(error("invalid_conversation_work_input")),
    };
    learning::content_sha256(
        &json!({"kind":"groundline-learning-conversation-archive-key","key":key}),
    )
}

fn conversation_members_path(state: &Path, root: &str) -> Result<PathBuf, ContractError> {
    if !learning::digest(root) {
        return Err(error("invalid_archive_reference"));
    }
    Ok(state.join("archive/conversation-chains").join(root))
}

fn conversation_member_refs(state_path: &Path, root: &str) -> Result<Vec<String>, ContractError> {
    let path = conversation_members_path(state_path, root)?;
    if fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let directory = directory_handle(&path, true)?;
    let entries = read_directory(&path, &directory, false)?;
    let mut extra = Vec::new();
    for (_, index) in entries {
        let hash = index["record_sha256"]
            .as_str()
            .filter(|hash| learning::digest(hash))
            .ok_or_else(|| error("invalid_conversation_archive_index"))?;
        if index["kind"] != "groundline-learning-conversation-archive-member"
            || index["schema"] != 1
            || index["root_input_sha256"] != root
        {
            return Err(error("invalid_conversation_archive_index"));
        }
        extra.push(hash.to_owned());
    }
    Ok(extra)
}

fn load_conversation_members(state: &mut State, root: &str) -> Result<(), ContractError> {
    let extra = conversation_member_refs(&state.path, root)?;
    load_archive_references(&state.path, &mut state.records, &extra)?;
    super::super::validate_record_collection(&state.records)
}

/// Restore one checked chain by direct private indexes. This never scans all
/// historical records or treats an archived exact retry as new observed work.
pub(in crate::learning) fn restore_conversation_replay(
    state: &mut State,
    input: &Value,
) -> Result<Option<Value>, ContractError> {
    let extra: Vec<_> = ["review_sha256", "change_sha256", "snapshot_sha256"]
        .into_iter()
        .filter_map(|field| {
            input[field]
                .as_str()
                .filter(|hash| learning::digest(hash))
                .map(str::to_owned)
        })
        .collect();
    load_archive_references(&state.path, &mut state.records, &extra)?;
    let key = conversation_key(input, &state.records)?;
    let path = input_index_path(&state.path, &key)?;
    let Some(bytes) = optional_private(&path)? else {
        if input["kind"] != "groundline-learning-conversation-review-input" {
            let root = conversation_root_input(input, &state.records)?;
            load_conversation_members(state, &root)?;
        }
        return Ok(None);
    };
    let index = parse(&bytes)?;
    let hash = index["record_sha256"]
        .as_str()
        .filter(|hash| learning::digest(hash))
        .ok_or_else(|| error("invalid_conversation_archive_index"))?;
    let root = index["root_input_sha256"]
        .as_str()
        .filter(|hash| learning::digest(hash))
        .ok_or_else(|| error("invalid_conversation_archive_index"))?;
    if index["kind"] != "groundline-learning-conversation-archive-key"
        || index["schema"] != 1
        || index["key_sha256"] != key
    {
        return Err(error("invalid_conversation_archive_index"));
    }
    load_archive_references(&state.path, &mut state.records, &[hash.to_owned()])?;
    load_conversation_members(state, root)?;
    let record = state
        .records
        .iter()
        .find(|record| learning::content_sha256(record).ok().as_deref() == Some(hash))
        .ok_or_else(|| error("archive_record_missing"))?;
    if conversation_key(&record["input"], &state.records)? != key
        || conversation_root_input(&record["input"], &state.records)? != root
        || index["input_sha256"] != record["input_sha256"]
    {
        return Err(error("invalid_conversation_archive_index"));
    }
    let parsed: learning::conversation_work::Input = serde_json::from_value(input.clone())
        .map_err(|_| error("invalid_conversation_work_input"))?;
    let normalized = serde_json::to_value(parsed).map_err(|_| error("serialization_failed"))?;
    if record["input"] != normalized {
        return Err(error("conversation_duplicate_conflict"));
    }
    Ok(Some(record.clone()))
}

pub(in crate::learning) fn compact_conversations(
    state: &mut State,
) -> Result<usize, ContractError> {
    if state.records.len() < 400 {
        return Ok(0);
    }
    let groups = learning::conversation_work::compactable_chain_hashes(&state.records)?;
    let hashes: BTreeMap<_, _> = state
        .records
        .iter()
        .map(|record| Ok((learning::content_sha256(record)?, record)))
        .collect::<Result<_, ContractError>>()?;
    let mut groups: Vec<_> = groups
        .into_iter()
        .map(|(root, members)| {
            let latest = members
                .iter()
                .filter_map(|hash| hashes.get(hash))
                .filter_map(|record| record["recorded_at_utc"].as_str())
                .filter_map(|time| chrono::DateTime::parse_from_rfc3339(time).ok())
                .max();
            (latest, root, members)
        })
        .collect();
    groups.sort();
    let mut remove: BTreeSet<_> = groups
        .into_iter()
        .rev()
        .skip(32)
        .flat_map(|(_, _, members)| members)
        .collect();
    // A retained record's transitive references always win over eligibility.
    loop {
        let protected: BTreeSet<_> = hashes
            .iter()
            .filter(|(hash, _)| !remove.contains(*hash))
            .flat_map(|(_, record)| refs(record))
            .collect();
        let before = remove.len();
        remove.retain(|hash| !protected.contains(hash));
        if remove.len() == before {
            break;
        }
    }
    if remove.is_empty() {
        return Ok(0);
    }
    let archive = state.path.join("archive");
    ensure_private_directory(&archive)?;
    let inputs = archive.join("inputs");
    ensure_private_directory(&inputs)?;
    let chains = archive.join("conversation-chains");
    ensure_private_directory(&chains)?;
    for hash in &remove {
        let record = hashes[hash];
        let root = conversation_root_input(&record["input"], &state.records)?;
        let key = conversation_key(&record["input"], &state.records)?;
        let index_path = input_index_path(&state.path, &key)?;
        ensure_private_directory(index_path.parent().ok_or_else(|| error("invalid_path"))?)?;
        write_draft(
            &index_path,
            &json!({"kind":"groundline-learning-conversation-archive-key","schema":1,
            "key_sha256":key,"root_input_sha256":root,"input_sha256":record["input_sha256"],"record_sha256":hash}),
            state,
        )?;
        let members = conversation_members_path(&state.path, &root)?;
        ensure_private_directory(&members)?;
        write_draft(
            &members.join(format!("{hash}.json")),
            &json!({"kind":"groundline-learning-conversation-archive-member",
            "schema":1,"root_input_sha256":root,"record_sha256":hash}),
            state,
        )?;
    }
    let mut moved = 0;
    for hash in &remove {
        let source = state.path.join(format!("{hash}.json"));
        let Some(bytes) = optional_private(&source)? else {
            continue;
        };
        let destination = record_path(&state.path, hash)?;
        ensure_private_directory(destination.parent().ok_or_else(|| error("invalid_path"))?)?;
        archive_file(&source, &destination, &bytes)?;
        moved += 1;
    }
    state.records.retain(|record| {
        learning::content_sha256(record).is_ok_and(|hash| !remove.contains(&hash))
    });
    super::super::validate_record_collection(&state.records)?;
    Ok(moved)
}

pub(in crate::learning) fn conversation_archive_summary(
    state: &State,
) -> Result<Value, ContractError> {
    let mut referenced_cold = 0;
    for record in &state.records {
        let hash = learning::content_sha256(record)?;
        if archived_record(&state.path, &hash)?.is_some() {
            referenced_cold += 1;
        }
    }
    Ok(
        json!({"coverage_scope":"bounded_active_reference_closure","active_record_count":state.records.len(),
        "referenced_archive_record_count":referenced_cold,"archived_history_count":null,
        "historical_coverage_complete":false,"completed_chain_retention":32,"maintenance_threshold":400}),
    )
}

pub(super) fn include_receipts(
    profile: &contract::LearningProfile,
    records: &[Value],
    receipts: &mut Vec<(String, Value)>,
) -> Result<(), ContractError> {
    let mut known: BTreeSet<_> = receipts.iter().map(|(h, _)| h.clone()).collect();
    for value in records {
        let Some(hash) = value["receipt_sha256"].as_str() else {
            continue;
        };
        if !learning::digest(hash) || !known.insert(hash.to_owned()) {
            continue;
        }
        let path = Path::new(&profile.deliveries)
            .join("archive")
            .join(&hash[..2])
            .join(format!("{hash}.json"));
        let Some(bytes) = optional_private(&path)? else {
            continue;
        };
        if sha256(&bytes) != hash {
            return Err(error("receipt_archive_digest_mismatch"));
        }
        receipts.push((hash.into(), parse(&bytes)?));
        if receipts.len() > MAX_ENTRIES - 2 {
            return Err(error("active_receipt_limit_exceeded"));
        }
    }
    delivery::validate_receipt_collection(
        &receipts.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
    )
}

fn ledger_path(root: &Path, kind: &str, key: &str) -> Result<PathBuf, ContractError> {
    if !learning::digest(key) {
        return Err(error("invalid_resource_index"));
    }
    Ok(root
        .join("ownership")
        .join(kind)
        .join(&key[..2])
        .join(format!("{key}.json")))
}

pub(super) fn reserve_receipt(
    state: &State,
    profile: &contract::LearningProfile,
    hash: &str,
    receipt: &Value,
) -> Result<(), ContractError> {
    delivery::validate_receipt(receipt)?;
    let root = Path::new(&profile.deliveries);
    let mut indices = Vec::new();
    let unit = receipt["unit_hash"]
        .as_str()
        .ok_or_else(|| error("invalid_receipt"))?;
    indices.push(("units",unit.to_owned(),json!({"kind":"groundline-learning-owned-unit","schema":1,"unit_hash":unit,"receipt_sha256":hash})));
    for row in receipt["resources"]["entries"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let response = row["response_hash"]
            .as_str()
            .ok_or_else(|| error("invalid_resources"))?;
        indices.push((
            "responses",
            response.to_owned(),
            json!({"kind":"groundline-learning-owned-response","schema":1,"response_hash":response,
            "unit_hash":unit,"receipt_sha256":hash,"row_sha256":learning::content_sha256(row)?}),
        ));
    }
    for (kind, key, value) in &indices {
        if let Some(bytes) = optional_private(&ledger_path(root, kind, key)?)?
            && parse(&bytes)? != *value
        {
            return Err(error("archived_delivery_or_response_ownership_conflict"));
        }
    }
    ensure_private_directory(&root.join("ownership"))?;
    for (kind, key, value) in indices {
        ensure_private_directory(&root.join("ownership").join(kind))?;
        let path = ledger_path(root, kind, &key)?;
        ensure_private_directory(path.parent().ok_or_else(|| error("invalid_path"))?)?;
        write_draft(&path, &value, state)?;
    }
    Ok(())
}
