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
        }
        Some("groundline-learning-finalize-request") => {
            add(&value["input"]["task_sha256"]);
            add(&value["boundary_sha256"]);
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
    if state.records.len() < 400 {
        return Ok(0);
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
        ]
        .contains(&value["kind"].as_str().unwrap_or(""))
        {
            let input = value["input_sha256"]
                .as_str()
                .ok_or_else(|| error("invalid_task_start"))?;
            let path = input_index_path(&state.path, input)?;
            ensure_private_directory(path.parent().ok_or_else(|| error("invalid_path"))?)?;
            write_draft(
                &path,
                &json!({"kind":"groundline-learning-archive-input","schema":1,"input_sha256":input,"record_sha256":hash}),
                state,
            )?;
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
    Ok(moved)
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
