//! Codex's on-disk representations share one bounded, read-only reader.

use std::fs::{File, Metadata};
use std::io::{self, BufRead, BufReader, Read};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use groundline_contracts::{ContractError, rollout::SourceDigest};
use rustix::fs::{Mode, OFlags, open, openat};

use crate::local_file::{open_bounded_regular_file, owned_by_current_user};

pub(crate) const MAX_AUDIT_BYTES: u64 = 512 * 1024 * 1024;
// Input I/O and retained audit memory are independent budgets. Long-lived
// native logs contain large context-storage records that need no audit body.
pub(crate) const MAX_AUDIT_SCAN_BYTES: u64 = 8 * 1024 * 1024 * 1024;
// Native compaction/context records can contain multi-megabyte unused blobs.
// Their raw read is bounded separately from the 4 MiB retained projection.
const MAX_AUDIT_RECORD_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) const NATIVE_SOURCE_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const NATIVE_RECORD_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const NATIVE_RECORDS: u64 = 1_000_000;
pub(crate) const NATIVE_READ_BUDGET: Duration = Duration::from_secs(10);

/// Held source bindings allow the task reader to recheck *all* sources after
/// reading children, rather than only checking the root before child I/O.
pub(crate) struct NativeSnapshot {
    pub(crate) source_sha256: String,
    pub(crate) bytes_read: u64,
    pub(crate) expanded_bytes: u64,
    pub(crate) records_scanned: u64,
    path: PathBuf,
    directory: File,
    file: File,
    before: Metadata,
    parent_before: Metadata,
}

fn native_error(code: &str) -> ContractError {
    ContractError(format!("learning_response_{code}"))
}

fn native_directory(path: &Path) -> Result<File, ContractError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(native_error("invalid_path"));
    }
    let mut directory = File::from(
        open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| native_error("source_unavailable"))?,
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
                .map_err(|_| native_error("source_unavailable"))?,
            );
        }
    }
    Ok(directory)
}

fn same_native_file(before: &Metadata, after: &Metadata) -> bool {
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

impl NativeSnapshot {
    pub(crate) fn recheck(&self) -> Result<(), ContractError> {
        let after = self
            .file
            .metadata()
            .map_err(|_| native_error("source_changed"))?;
        let current = native_directory(
            self.path
                .parent()
                .ok_or_else(|| native_error("invalid_path"))?,
        )
        .map_err(|_| native_error("source_changed"))?;
        let held = self
            .directory
            .metadata()
            .map_err(|_| native_error("source_changed"))?;
        let bound = current
            .metadata()
            .map_err(|_| native_error("source_changed"))?;
        if !same_native_file(&self.before, &after)
            || held.dev() != bound.dev()
            || held.ino() != bound.ino()
            || bound.dev() != self.parent_before.dev()
            || bound.ino() != self.parent_before.ino()
            || bound.uid() != self.parent_before.uid()
            || bound.gid() != self.parent_before.gid()
            || bound.mode() != self.parent_before.mode()
        {
            return Err(native_error("source_changed"));
        }
        let rebound = File::from(
            openat(
                &current,
                self.path
                    .file_name()
                    .ok_or_else(|| native_error("invalid_path"))?,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|_| native_error("source_changed"))?,
        );
        if !same_native_file(
            &self.before,
            &rebound
                .metadata()
                .map_err(|_| native_error("source_changed"))?,
        ) {
            return Err(native_error("source_changed"));
        }
        Ok(())
    }
}

struct NativeHashingReader<R> {
    reader: R,
    digest: SourceDigest,
    bytes: u64,
}

impl<R: Read> Read for NativeHashingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(buffer)?;
        self.digest.update(&buffer[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
}

pub(crate) fn read_owned_native(
    path: &Path,
    maximum_bytes: u64,
    started: Instant,
    mut accept: impl FnMut(serde_json::Value) -> Result<(), ContractError>,
) -> Result<NativeSnapshot, ContractError> {
    read_native(path, maximum_bytes, started, false, |record| {
        accept(record)?;
        Ok(false)
    })
}

pub(crate) fn read_native_metadata(
    path: &Path,
    maximum_bytes: u64,
) -> Result<(Option<serde_json::Value>, NativeSnapshot), ContractError> {
    let mut metadata = None;
    let snapshot = read_native(
        path,
        maximum_bytes.min(2 * 1024 * 1024),
        Instant::now(),
        true,
        |record| {
            if record["type"] == "session_meta" {
                metadata = Some(record);
                Ok(true)
            } else {
                Ok(false)
            }
        },
    )?;
    Ok((metadata, snapshot))
}

fn read_native(
    path: &Path,
    maximum_bytes: u64,
    started: Instant,
    prefix: bool,
    mut accept: impl FnMut(serde_json::Value) -> Result<bool, ContractError>,
) -> Result<NativeSnapshot, ContractError> {
    if !path.is_absolute()
        || path.as_os_str().len() > 4096
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(native_error("invalid_path"));
    }
    let compressed = path.to_string_lossy().ends_with(".jsonl.zst");
    if !compressed && path.extension().is_none_or(|v| v != "jsonl") {
        return Err(native_error("invalid_path"));
    }
    let maximum_bytes = maximum_bytes.min(NATIVE_SOURCE_BYTES);
    if maximum_bytes == 0 {
        return Err(native_error("source_byte_limit_exceeded"));
    }
    let directory = native_directory(path.parent().ok_or_else(|| native_error("invalid_path"))?)?;
    let parent_before = directory
        .metadata()
        .map_err(|_| native_error("source_unavailable"))?;
    let owner = rustix::process::geteuid().as_raw();
    if parent_before.uid() != owner || parent_before.mode() & 0o022 != 0 {
        return Err(native_error("source_owner_required"));
    }
    let mut file = File::from(
        openat(
            &directory,
            path.file_name()
                .ok_or_else(|| native_error("invalid_path"))?,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| native_error("source_unavailable"))?,
    );
    let before = file
        .metadata()
        .map_err(|_| native_error("source_unavailable"))?;
    if !before.is_file()
        || before.nlink() != 1
        || before.uid() != owner
        || before.mode() & 0o022 != 0
        || before.len() == 0
    {
        return Err(native_error("source_owner_required"));
    }
    if !prefix && before.len() > maximum_bytes {
        return Err(native_error("source_byte_limit_exceeded"));
    }
    let mut input = NativeHashingReader {
        reader: Read::by_ref(&mut file).take(if prefix {
            before.len().min(maximum_bytes)
        } else {
            before.len()
        }),
        digest: SourceDigest::default(),
        bytes: 0,
    };
    let decoded: Box<dyn Read + '_> = if compressed {
        let mut decoder = zstd::stream::read::Decoder::new(&mut input)
            .map_err(|_| native_error("invalid_source"))?;
        decoder
            .window_log_max(26)
            .map_err(|_| native_error("invalid_source"))?;
        Box::new(decoder)
    } else {
        Box::new(&mut input)
    };
    let mut reader = BufReader::with_capacity(64 * 1024, decoded.take(maximum_bytes + 1));
    let mut expanded_bytes = 0_u64;
    let mut records_scanned = 0_u64;
    let mut line = Vec::new();
    loop {
        if started.elapsed() > NATIVE_READ_BUDGET {
            return Err(native_error("processing_budget_exceeded"));
        }
        line.clear();
        let count = Read::by_ref(&mut reader)
            .take(NATIVE_RECORD_BYTES + 1)
            .read_until(b'\n', &mut line)
            .map_err(|_| native_error("invalid_source"))?;
        if count == 0 {
            break;
        }
        expanded_bytes += count as u64;
        records_scanned += 1;
        if expanded_bytes > maximum_bytes {
            return Err(native_error("source_byte_limit_exceeded"));
        }
        if count as u64 > NATIVE_RECORD_BYTES {
            return Err(native_error("source_record_limit_exceeded"));
        }
        if records_scanned > NATIVE_RECORDS || (prefix && records_scanned > 1024) {
            return Err(native_error("source_record_count_exceeded"));
        }
        if line.last() != Some(&b'\n') {
            return Err(native_error("incomplete_source_record"));
        }
        let text = std::str::from_utf8(&line).map_err(|_| native_error("invalid_source"))?;
        if text.trim().is_empty() {
            continue;
        }
        if let Some(record) = groundline_contracts::rollout::Record::parse(text)
            .map_err(|_| native_error("invalid_source"))?
            && accept(
                record
                    .learning_projection()
                    .map_err(|_| native_error("invalid_source_projection"))?,
            )?
        {
            break;
        }
    }
    drop(reader);
    let bytes_read = input.bytes;
    let source_sha256 = input.digest.finish();
    if !prefix && bytes_read != before.len() {
        return Err(native_error("source_changed"));
    }
    let snapshot = NativeSnapshot {
        source_sha256,
        bytes_read,
        expanded_bytes,
        records_scanned,
        path: path.to_path_buf(),
        directory,
        file,
        before,
        parent_before,
    };
    snapshot.recheck()?;
    if started.elapsed() > NATIVE_READ_BUDGET {
        return Err(native_error("processing_budget_exceeded"));
    }
    Ok(snapshot)
}

pub(crate) fn read_audit_rollout(
    path: &Path,
    allowed_roots: &[PathBuf],
    scanned: &mut u64,
    retained: &mut u64,
    mut accept_metadata: impl FnMut(&serde_json::Value) -> bool,
) -> io::Result<Option<String>> {
    retry_missing_representation(|| {
        let (path, compressed) = representation(path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(rejected());
        }
        let canonical = path.canonicalize()?;
        if !allowed_roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(rejected());
        }
        // File size is not the read budget: inspect metadata before excluding
        // other runtimes, then stream within the shared invocation I/O limit.
        let file = open_bounded_regular_file(&canonical, 1, u64::MAX)?;
        if !owned_by_current_user(&file) {
            return Err(rejected());
        }
        let stored_bytes = file.metadata()?.len();
        let before_scan = *scanned;
        let limit = MAX_AUDIT_SCAN_BYTES.saturating_sub(*scanned);
        if limit == 0 {
            return Err(rejected());
        }
        let input = file.take(limit + 1);
        let reader: Box<dyn Read> = if compressed {
            let mut decoder = zstd::stream::read::Decoder::new(input)?;
            decoder.window_log_max(26)?;
            Box::new(decoder)
        } else {
            Box::new(input)
        };
        let result = project_audit(reader, limit, scanned, retained, &mut accept_metadata);
        if result.is_err() {
            // A malformed compressed stream may consume input without producing
            // a complete line. Failed inputs still consume the invocation budget.
            *scanned = (*scanned)
                .max(before_scan.saturating_add(stored_bytes))
                .min(MAX_AUDIT_SCAN_BYTES);
        }
        result
    })
}

fn project_audit(
    reader: impl Read,
    limit: u64,
    scanned: &mut u64,
    retained: &mut u64,
    accept_metadata: &mut impl FnMut(&serde_json::Value) -> bool,
) -> io::Result<Option<String>> {
    let mut reader = BufReader::new(reader.take(limit + 1));
    let mut output = String::new();
    let mut consumed = 0_u64;
    let mut metadata_found = false;
    let mut records = 0_u64;
    loop {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take(MAX_AUDIT_RECORD_BYTES + 1)
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            break;
        }
        consumed = consumed.saturating_add(count as u64);
        *scanned = scanned
            .saturating_add(count as u64)
            .min(MAX_AUDIT_SCAN_BYTES);
        if consumed > limit || count as u64 > MAX_AUDIT_RECORD_BYTES {
            return Err(rejected());
        }
        records += 1;
        if !metadata_found && records > 1024 {
            metadata_found = true;
            if !accept_metadata(&serde_json::Value::Null) {
                return Ok(None);
            }
        }
        let line = std::str::from_utf8(&line).map_err(|_| rejected())?;
        let record = groundline_contracts::rollout::Record::parse(line).map_err(|_| rejected())?;
        let projected = if let Some(record) = record {
            let projected = record.audit_projection().map_err(|_| rejected())?;
            if !metadata_found && record.string("type").as_deref() == Some("session_meta") {
                let metadata = groundline_contracts::rollout::Record::parse(&projected)
                    .map_err(|_| rejected())?
                    .ok_or_else(rejected)?
                    .value("payload")
                    .map_err(|_| rejected())?
                    .ok_or_else(rejected)?;
                metadata_found = true;
                if !accept_metadata(&metadata) {
                    return Ok(None);
                }
            }
            projected
        } else {
            "null".to_owned()
        };
        let bytes = projected.len() as u64 + 1;
        if bytes > MAX_AUDIT_BYTES.saturating_sub(*retained) {
            *retained = MAX_AUDIT_BYTES;
            return Err(rejected());
        }
        *retained += bytes;
        output.push_str(&projected);
        output.push('\n');
    }
    if consumed == 0 {
        return Err(rejected());
    }
    if !metadata_found && !accept_metadata(&serde_json::Value::Null) {
        return Ok(None);
    }
    Ok(Some(output))
}

fn rejected() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "rollout_input_unavailable")
}

pub(crate) fn logical_path(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(rejected());
    }
    if path
        .extension()
        .is_some_and(|extension| extension == "jsonl")
    {
        Ok(path.to_path_buf())
    } else if path.to_string_lossy().ends_with(".jsonl.zst") {
        Ok(path.with_extension(""))
    } else {
        Err(rejected())
    }
}

fn representation(path: &Path) -> io::Result<(PathBuf, bool)> {
    let plain = logical_path(path)?;
    // Codex can materialize a compressed file for append. Prefer that live
    // representation even when SQLite still names its compressed sibling.
    match std::fs::symlink_metadata(&plain) {
        Ok(_) => Ok((plain, false)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok((plain.with_extension("jsonl.zst"), true))
        }
        Err(error) => Err(error),
    }
}

fn retry_missing_representation<T>(mut read: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    match read() {
        // Codex may swap plain/compressed representations while we open one.
        Err(error) if error.kind() == io::ErrorKind::NotFound => read(),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_stream_drops_unused_bodies_but_validates_every_input_record() {
        let metadata = "{\"type\":\"session_meta\",\"payload\":{\"originator\":\"codex_app\"}}\n";
        let data = format!(
            "{metadata}{{\"type\":\"world_state\",\"timestamp\":\"2026-09-01T00:00:00Z\",\"ordinal\":1,\"payload\":{{\"body\":\"{}\"}}}}\n",
            "x".repeat(100_000)
        );
        let mut scanned = 0;
        let mut retained = 0;
        let projected = project_audit(
            data.as_bytes(),
            data.len() as u64,
            &mut scanned,
            &mut retained,
            &mut |_| true,
        )
        .unwrap()
        .unwrap();
        assert_eq!(scanned, data.len() as u64);
        assert_eq!(retained, projected.len() as u64);
        assert!(retained < 300);
        assert!(projected.contains("world_state"));
        assert!(!projected.contains("body"));
        let malformed = format!("{metadata}{{\"type\":\"world_state\",\"payload\":[1,]}}\n");
        assert!(project_audit(malformed.as_bytes(), 1000, &mut 0, &mut 0, &mut |_| true).is_err());
        assert!(project_audit(data.as_bytes(), 100, &mut 0, &mut 0, &mut |_| true).is_err());
        assert!(
            project_audit(
                metadata.as_bytes(),
                1000,
                &mut 0,
                &mut (MAX_AUDIT_BYTES - 1),
                &mut |_| true
            )
            .is_err()
        );
        let mut scanned = 0;
        assert!(
            project_audit(
                data.as_bytes(),
                data.len() as u64,
                &mut scanned,
                &mut 0,
                &mut |_| false
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            scanned,
            metadata.len() as u64,
            "other runtimes consume only metadata I/O"
        );
    }

    #[test]
    fn large_other_runtime_file_is_filtered_before_reading_its_body() {
        use std::io::Write;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        let metadata = b"{\"type\":\"session_meta\",\"payload\":{\"originator\":\"codex_app\"}}\n";
        file.write_all(metadata).unwrap();
        file.set_len(MAX_AUDIT_SCAN_BYTES + 1).unwrap();
        let mut scanned = 0;
        let result = read_audit_rollout(
            &path,
            &[directory.path().canonicalize().unwrap()],
            &mut scanned,
            &mut 0,
            |_| false,
        )
        .unwrap();
        assert!(result.is_none());
        assert_eq!(scanned, metadata.len() as u64);
    }

    #[test]
    fn only_missing_representation_gets_one_retry() {
        let mut calls = 0;
        assert_eq!(
            retry_missing_representation(|| {
                calls += 1;
                if calls == 1 {
                    Err(io::ErrorKind::NotFound.into())
                } else {
                    Ok("recovered".to_owned())
                }
            })
            .unwrap(),
            "recovered"
        );
        assert_eq!(calls, 2);
        for (kind, expected) in [
            (io::ErrorKind::NotFound, 2),
            (io::ErrorKind::PermissionDenied, 1),
            (io::ErrorKind::InvalidData, 1),
        ] {
            let mut calls = 0;
            assert!(
                retry_missing_representation(|| {
                    calls += 1;
                    Err::<String, _>(kind.into())
                })
                .is_err()
            );
            assert_eq!(calls, expected);
        }
    }

    #[test]
    fn oversized_unused_native_records_work_in_plain_and_compressed_history() {
        let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        let raw = format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"owner\",\"originator\":\"codex_app\"}}}}\n{{\"type\":\"compacted\",\"timestamp\":\"2026-09-08T00:00:00Z\",\"payload\":{{\"guardian_history\":[\"{}\"]}}}}\n",
            "x".repeat(5 * 1024 * 1024)
        );
        for compressed in [false, true] {
            let path = root.path().join(if compressed {
                "compressed.jsonl.zst"
            } else {
                "plain.jsonl"
            });
            let bytes = if compressed {
                zstd::stream::encode_all(raw.as_bytes(), 1).unwrap()
            } else {
                raw.as_bytes().to_vec()
            };
            std::fs::write(&path, bytes).unwrap();
            let mut scanned = 0;
            let mut retained = 0;
            let projected = read_audit_rollout(
                &path,
                &[root.path().to_owned()],
                &mut scanned,
                &mut retained,
                |meta| meta["originator"] == "codex_app",
            )
            .unwrap()
            .unwrap();
            assert_eq!(scanned, raw.len() as u64);
            assert!(retained < 300);
            assert!(projected.contains("compacted"));
            let broken = raw.replace("]}}\n", ",]}}\n");
            let bytes = if compressed {
                zstd::stream::encode_all(broken.as_bytes(), 1).unwrap()
            } else {
                broken.into_bytes()
            };
            std::fs::write(&path, bytes).unwrap();
            assert!(
                read_audit_rollout(&path, &[root.path().to_owned()], &mut 0, &mut 0, |_| true)
                    .is_err()
            );
        }
        let input = std::io::repeat(b'x').take(MAX_AUDIT_RECORD_BYTES + 1);
        let mut scanned = 0;
        assert!(
            project_audit(
                input,
                MAX_AUDIT_RECORD_BYTES + 1,
                &mut scanned,
                &mut 0,
                &mut |_| true
            )
            .is_err()
        );
        assert_eq!(scanned, MAX_AUDIT_RECORD_BYTES + 1);
    }

    #[test]
    fn resolves_both_representations_and_prefers_live_plain_file() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().canonicalize().unwrap();
        let plain = root.join("rollout.jsonl");
        let compressed = plain.with_extension("jsonl.zst");
        let data = "{\"payload\":{},\"type\":\"session_meta\"}\n";
        std::fs::write(&compressed, zstd::encode_all(data.as_bytes(), 1).unwrap()).unwrap();
        let mut total = 0;
        for path in [&plain, &compressed] {
            assert_eq!(
                read_audit_rollout(
                    path,
                    std::slice::from_ref(&root),
                    &mut total,
                    &mut 0,
                    |_| true
                )
                .unwrap()
                .unwrap(),
                data
            );
        }
        assert_eq!(total, 2 * data.len() as u64);
        assert!(!plain.exists(), "auditing must not materialize a file");
        std::fs::write(&plain, "{\"type\":\"new\"}\n").unwrap();
        assert_eq!(
            read_audit_rollout(&compressed, &[root], &mut total, &mut 0, |_| true)
                .unwrap()
                .unwrap(),
            "{\"type\":\"new\"}\n"
        );
        assert_eq!(
            logical_path(&plain).unwrap(),
            logical_path(&compressed).unwrap()
        );
    }

    #[test]
    fn rejects_corrupt_compressed_input_and_bounds_decoded_bytes() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().canonicalize().unwrap();
        let path = root.join("rollout.jsonl.zst");
        std::fs::write(&path, b"not-zstd").unwrap();
        assert!(
            read_audit_rollout(&path, std::slice::from_ref(&root), &mut 0, &mut 0, |_| true)
                .is_err()
        );
        let compressed = zstd::encode_all(&b"123456789"[..], 1).unwrap();
        std::fs::write(&path, &compressed).unwrap();
        let mut total = MAX_AUDIT_SCAN_BYTES - 8;
        assert!(
            read_audit_rollout(
                &path,
                std::slice::from_ref(&root),
                &mut total,
                &mut 0,
                |_| true
            )
            .is_err()
        );
        assert_eq!(total, MAX_AUDIT_SCAN_BYTES);
        assert!(read_audit_rollout(&path, &[root], &mut total, &mut 0, |_| true).is_err());
        assert!(project_audit(&[255][..], 4, &mut 0, &mut 0, &mut |_| true).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn never_uses_compressed_sibling_to_bypass_a_rejected_plain_file() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().canonicalize().unwrap();
        let path = root.join("rollout.jsonl");
        std::fs::write(
            path.with_extension("jsonl.zst"),
            zstd::encode_all(&b"{}\n"[..], 1).unwrap(),
        )
        .unwrap();
        std::os::unix::fs::symlink(root.join("missing"), &path).unwrap();
        assert!(read_audit_rollout(&path, &[root], &mut 0, &mut 0, |_| true).is_err());
    }
}
