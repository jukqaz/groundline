//! Codex's on-disk representations share one bounded, read-only reader.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::local_file::{open_bounded_regular_file, owned_by_current_user};

const MAX_ROLLOUT_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_AUDIT_BYTES: u64 = 512 * 1024 * 1024;

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

fn read_decoded(reader: impl Read, limit: u64, total: &mut u64) -> io::Result<String> {
    let mut bytes = Vec::new();
    let result = reader.take(limit + 1).read_to_end(&mut bytes);
    // Failed/corrupt inputs also consume the invocation's work budget.
    *total = total
        .saturating_add(bytes.len() as u64)
        .min(MAX_AUDIT_BYTES);
    result?;
    if bytes.is_empty() || bytes.len() as u64 > limit {
        return Err(rejected());
    }
    String::from_utf8(bytes).map_err(|_| rejected())
}

pub(crate) fn read_rollout(
    path: &Path,
    allowed_roots: &[PathBuf],
    total: &mut u64,
) -> io::Result<String> {
    retry_missing_representation(|| read_once(path, allowed_roots, total))
}

fn retry_missing_representation(
    mut read: impl FnMut() -> io::Result<String>,
) -> io::Result<String> {
    match read() {
        // Codex may swap plain/compressed representations while we open one.
        Err(error) if error.kind() == io::ErrorKind::NotFound => read(),
        result => result,
    }
}

fn read_once(path: &Path, allowed_roots: &[PathBuf], total: &mut u64) -> io::Result<String> {
    let limit = MAX_AUDIT_BYTES
        .saturating_sub(*total)
        .min(MAX_ROLLOUT_BYTES);
    if limit == 0 {
        return Err(rejected());
    }
    let (path, compressed) = representation(path)?;
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(rejected());
    }
    let canonical = path.canonicalize()?;
    if !allowed_roots.iter().any(|root| canonical.starts_with(root)) {
        return Err(rejected());
    }
    let file = open_bounded_regular_file(&canonical, 1, MAX_ROLLOUT_BYTES)?;
    if !owned_by_current_user(&file) {
        return Err(rejected());
    }
    let stored_bytes = file.metadata()?.len();
    let before_read = *total;
    if stored_bytes > MAX_AUDIT_BYTES.saturating_sub(before_read) {
        *total = MAX_AUDIT_BYTES;
        return Err(rejected());
    }
    // Bound input reads as well as decoded output if an active file grows.
    let input = file.take(MAX_ROLLOUT_BYTES + 1);
    let result = (|| {
        if compressed {
            let mut decoder = zstd::stream::read::Decoder::new(input)?;
            decoder.window_log_max(26)?;
            read_decoded(decoder, limit, total)
        } else {
            read_decoded(input, limit, total)
        }
    })();
    // Charge at least the stored size, even if a malformed stream emits nothing.
    *total = (*total)
        .max(before_read + stored_bytes)
        .min(MAX_AUDIT_BYTES);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_missing_representation_gets_one_retry() {
        let mut calls = 0;
        assert_eq!(
            retry_missing_representation(|| {
                calls += 1;
                if calls == 1 {
                    Err(io::ErrorKind::NotFound.into())
                } else {
                    Ok("recovered".into())
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
                    Err(kind.into())
                })
                .is_err()
            );
            assert_eq!(calls, expected);
        }
    }

    #[test]
    fn resolves_both_representations_and_prefers_live_plain_file() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().canonicalize().unwrap();
        let plain = root.join("rollout.jsonl");
        let compressed = plain.with_extension("jsonl.zst");
        let data = "{\"type\":\"session_meta\"}\n";
        std::fs::write(&compressed, zstd::encode_all(data.as_bytes(), 1).unwrap()).unwrap();
        let mut total = 0;
        for path in [&plain, &compressed] {
            assert_eq!(
                read_rollout(path, std::slice::from_ref(&root), &mut total).unwrap(),
                data
            );
        }
        let charged = (data.len() as u64).max(std::fs::metadata(&compressed).unwrap().len());
        assert_eq!(total, 2 * charged);
        assert!(!plain.exists(), "auditing must not materialize a file");
        std::fs::write(&plain, "{\"new\":true}\n").unwrap();
        assert_eq!(
            read_rollout(&compressed, &[root], &mut total).unwrap(),
            "{\"new\":true}\n"
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
        assert!(read_rollout(&path, std::slice::from_ref(&root), &mut 0).is_err());
        let compressed = zstd::encode_all(&b"123456789"[..], 1).unwrap();
        std::fs::write(&path, &compressed).unwrap();
        let mut total = MAX_AUDIT_BYTES - 8;
        assert!(read_rollout(&path, std::slice::from_ref(&root), &mut total).is_err());
        assert_eq!(total, MAX_AUDIT_BYTES);
        assert!(read_rollout(&path, &[root], &mut total).is_err());
        assert!(read_decoded(&b"12345"[..], 4, &mut 0).is_err());
        assert_eq!(read_decoded(&b"1234"[..], 4, &mut 0).unwrap(), "1234");
        assert!(read_decoded(&[255][..], 4, &mut 0).is_err());
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
        assert!(read_rollout(&path, &[root], &mut 0).is_err());
    }
}
