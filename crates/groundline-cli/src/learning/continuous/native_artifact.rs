//! Read normal native JSONL without changing its permissions or retaining text.
use super::*;
use std::io::{BufRead, BufReader};
use std::time::{Duration, Instant};

const MAX_SOURCE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LINE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORDS: u64 = 1_000_000;
const MAX_TURNS: usize = 4096;
const READ_BUDGET: Duration = Duration::from_secs(10);

pub(super) struct Observation {
    pub(super) session_hash: Option<String>,
    pub(super) turn_hashes: BTreeSet<String>,
    pub(super) sha256: String,
    pub(super) bytes_read: u64,
    pub(super) records_scanned: u64,
}

impl Observation {
    pub(super) fn readout(&self) -> Value {
        json!({"source_sha256":self.sha256,"bytes_read":self.bytes_read,
            "records_scanned":self.records_scanned,"distinct_turn_count":self.turn_hashes.len(),
            "byte_limit":MAX_SOURCE_BYTES,"line_byte_limit":MAX_LINE_BYTES,
            "record_limit":MAX_RECORDS,"turn_limit":MAX_TURNS,
            "processing_budget_ms":READ_BUDGET.as_millis(),"source_mutation_performed":false,
            "source_authenticity_verified":false})
    }
}

fn same_file(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.len() == after.len()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
        && before.ctime() == after.ctime()
        && before.ctime_nsec() == after.ctime_nsec()
        && before.uid() == after.uid()
        && before.gid() == after.gid()
        && before.mode() == after.mode()
        && after.nlink() == 1
        && after.is_file()
}

fn check_unchanged(
    path: &Path,
    directory: &File,
    file: &File,
    before: &fs::Metadata,
    parent_before: &fs::Metadata,
) -> Result<(), ContractError> {
    let after = file
        .metadata()
        .map_err(|_| error("native_artifact_changed"))?;
    let current_directory =
        directory_handle(path.parent().ok_or_else(|| error("invalid_path"))?, false)
            .map_err(|_| error("native_artifact_changed"))?;
    let held = directory
        .metadata()
        .map_err(|_| error("native_artifact_changed"))?;
    let current = current_directory
        .metadata()
        .map_err(|_| error("native_artifact_changed"))?;
    if !same_file(before, &after)
        || current.dev() != held.dev()
        || current.ino() != held.ino()
        || current.dev() != parent_before.dev()
        || current.ino() != parent_before.ino()
        || current.uid() != parent_before.uid()
        || current.gid() != parent_before.gid()
        || current.mode() != parent_before.mode()
    {
        return Err(error("native_artifact_changed"));
    }
    let rebound = File::from(
        openat(
            &current_directory,
            path.file_name().ok_or_else(|| error("invalid_path"))?,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("native_artifact_changed"))?,
    );
    if !same_file(
        before,
        &rebound
            .metadata()
            .map_err(|_| error("native_artifact_changed"))?,
    ) {
        return Err(error("native_artifact_changed"));
    }
    Ok(())
}

pub(super) fn read(path: &Path) -> Result<Observation, ContractError> {
    let started = Instant::now();
    let path = absolute(path)?;
    let directory = directory_handle(path.parent().ok_or_else(|| error("invalid_path"))?, false)?;
    let parent = directory
        .metadata()
        .map_err(|_| error("invalid_native_artifact"))?;
    if parent.uid() != rustix::process::geteuid().as_raw() || parent.mode() & 0o022 != 0 {
        return Err(error("native_artifact_owner_required"));
    }
    let mut file = File::from(
        openat(
            &directory,
            path.file_name().ok_or_else(|| error("invalid_path"))?,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("invalid_native_artifact"))?,
    );
    let before = file
        .metadata()
        .map_err(|_| error("invalid_native_artifact"))?;
    if !before.is_file()
        || before.nlink() != 1
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.mode() & 0o022 != 0
        || before.len() == 0
    {
        return Err(error("native_artifact_owner_required"));
    }
    if before.len() > MAX_SOURCE_BYTES {
        return Err(error("native_artifact_byte_limit_exceeded"));
    }
    // A fixed initial prefix bounds concurrent append. It is returned only if
    // the source still has the exact initial metadata and path binding below.
    let mut reader =
        BufReader::with_capacity(64 * 1024, Read::by_ref(&mut file).take(before.len()));
    let mut line = Vec::new();
    let mut digest = Sha256::new();
    let mut observed = Observation {
        session_hash: None,
        turn_hashes: BTreeSet::new(),
        sha256: String::new(),
        bytes_read: 0,
        records_scanned: 0,
    };
    loop {
        if started.elapsed() > READ_BUDGET {
            return Err(error("native_artifact_processing_budget_exceeded"));
        }
        line.clear();
        let count = Read::by_ref(&mut reader)
            .take(MAX_LINE_BYTES + 1)
            .read_until(b'\n', &mut line)
            .map_err(|_| error("native_artifact_unavailable"))?;
        if count == 0 {
            break;
        }
        if count as u64 > MAX_LINE_BYTES {
            return Err(error("native_artifact_line_limit_exceeded"));
        }
        observed.bytes_read += count as u64;
        observed.records_scanned += 1;
        if observed.records_scanned > MAX_RECORDS {
            return Err(error("native_artifact_record_limit_exceeded"));
        }
        digest.update(&line);
        let text = std::str::from_utf8(&line).map_err(|_| error("invalid_native_artifact"))?;
        if text.trim().is_empty() {
            continue;
        }
        let Some(record) = groundline_contracts::rollout::Record::parse(text)
            .map_err(|_| error("invalid_native_artifact"))?
        else {
            continue;
        };
        let kind = record.string("type");
        if !matches!(kind.as_deref(), Some("session_meta" | "turn_context")) {
            continue;
        }
        // Borrow the native payload, then inspect only its identity fields.
        // Instruction, compaction and message bodies are never materialized.
        let payload: BTreeMap<String, &serde_json::value::RawValue> =
            serde_json::from_str(text).map_err(|_| error("invalid_native_artifact"))?;
        let Some(payload) = payload.get("payload") else {
            continue;
        };
        let Some(payload) = groundline_contracts::rollout::Record::parse(payload.get())
            .map_err(|_| error("invalid_native_artifact"))?
        else {
            continue;
        };
        if kind.as_deref() == Some("session_meta") {
            if let Some(id) = payload
                .string("id")
                .filter(|v| !v.is_empty() && v.len() <= 256)
            {
                let hash = sha256(format!("groundline-hook-session\0{id}").as_bytes());
                if observed.session_hash.as_ref().is_some_and(|s| s != &hash) {
                    return Err(error("native_session_conflict"));
                }
                observed.session_hash = Some(hash);
            }
        } else if let Some(id) = payload
            .string("turn_id")
            .filter(|v| !v.is_empty() && v.len() <= 256)
        {
            observed
                .turn_hashes
                .insert(sha256(format!("groundline-hook-turn\0{id}").as_bytes()));
            if observed.turn_hashes.len() > MAX_TURNS {
                return Err(error("native_artifact_turn_limit_exceeded"));
            }
        }
    }
    drop(reader);
    if observed.bytes_read != before.len() {
        return Err(error("native_artifact_changed"));
    }
    check_unchanged(&path, &directory, &file, &before, &parent)?;
    if started.elapsed() > READ_BUDGET {
        return Err(error("native_artifact_processing_budget_exceeded"));
    }
    observed.sha256 = format!("{:x}", digest.finalize());
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn native_identity_keeps_the_existing_last_key_policy_and_full_source_digest() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let source = root.join("native.jsonl");
        let bytes = b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"ignored-owner\"},\"payload\":{\"id\":\"owner\",\"instructions\":\"private-body\"}}\n{\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"turn\"}}\n";
        fs::write(&source, bytes).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o644)).unwrap();
        let observed = read(&source).unwrap();
        assert_eq!(observed.sha256, sha256(bytes));
        assert_eq!(observed.records_scanned, 2);
        assert_eq!(
            observed.session_hash,
            Some(sha256(b"groundline-hook-session\0owner"))
        );
        assert_eq!(
            observed.turn_hashes,
            BTreeSet::from([sha256(b"groundline-hook-turn\0turn")])
        );
    }

    #[test]
    fn native_source_recheck_rejects_append_truncate_overwrite_and_path_replacement() {
        for action in 0..4 {
            let temporary = tempfile::tempdir().unwrap();
            let root = temporary.path().canonicalize().unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
            let source = root.join("native.jsonl");
            fs::write(
                &source,
                b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"owner\"}}\n",
            )
            .unwrap();
            fs::set_permissions(&source, fs::Permissions::from_mode(0o644)).unwrap();
            let directory = directory_handle(&root, false).unwrap();
            let parent_before = directory.metadata().unwrap();
            let file = File::open(&source).unwrap();
            let before = file.metadata().unwrap();
            match action {
                0 => fs::OpenOptions::new()
                    .append(true)
                    .open(&source)
                    .unwrap()
                    .write_all(b"{}\n")
                    .unwrap(),
                1 => fs::OpenOptions::new()
                    .write(true)
                    .open(&source)
                    .unwrap()
                    .set_len(1)
                    .unwrap(),
                2 => fs::write(&source, vec![b'x'; before.len() as usize]).unwrap(),
                _ => {
                    fs::rename(&source, root.join("original.jsonl")).unwrap();
                    fs::copy(root.join("original.jsonl"), &source).unwrap();
                }
            }
            assert_eq!(
                check_unchanged(&source, &directory, &file, &before, &parent_before)
                    .unwrap_err()
                    .0,
                "learning_native_artifact_changed"
            );
        }
    }
}
