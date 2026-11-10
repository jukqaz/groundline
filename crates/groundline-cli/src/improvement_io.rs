//! Existing bounded private-file and no-clobber storage primitives.
use groundline_contracts::{ContractError, learning};
use groundline_runtime::local_file::{open_private_directory, private_for_current_user};
use rustix::fs::{AtFlags, Mode, OFlags, linkat, mkdirat, open, openat, unlinkat};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ENTRIES: usize = 1000;
const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;
fn error(code: &str) -> ContractError {
    ContractError(format!("improvement_{code}"))
}
pub(super) fn absolute(path: &Path) -> Result<PathBuf, ContractError> {
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
pub(super) fn directory_handle(path: &Path, private: bool) -> Result<File, ContractError> {
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

pub(super) fn read_private(path: &Path) -> Result<Vec<u8>, ContractError> {
    let path = absolute(path)?;
    let parent = path.parent().ok_or_else(|| error("invalid_path"))?;
    let parent = directory_handle(parent, true)?;
    let name = path.file_name().ok_or_else(|| error("invalid_path"))?;
    read_at(&parent, name)
}

pub(super) fn parse(bytes: &[u8]) -> Result<Value, ContractError> {
    serde_json::from_slice(bytes).map_err(|_| error("invalid_json"))
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn read_conversation_target(path: &Path) -> Result<Vec<u8>, ContractError> {
    let path = absolute(path)?;
    let parent_path = path.parent().ok_or_else(|| error("invalid_path"))?;
    let parent = directory_handle(parent_path, false)?;
    let parent_metadata = parent.metadata().map_err(|_| error("invalid_directory"))?;
    let uid = rustix::process::geteuid().as_raw();
    if parent_metadata.uid() != uid || parent_metadata.mode() & 0o022 != 0 {
        return Err(error("conversation_target_owner_required"));
    }
    let name = path.file_name().ok_or_else(|| error("invalid_path"))?;
    let mut file = File::from(
        openat(
            &parent,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("invalid_conversation_target"))?,
    );
    let metadata = file
        .metadata()
        .map_err(|_| error("invalid_conversation_target"))?;
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.nlink() != 1
        || metadata.mode() & 0o022 != 0
        || metadata.len() == 0
        || metadata.len() > MAX_FILE_BYTES
    {
        return Err(error("invalid_conversation_target"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("conversation_target_unavailable"))?;
    let after = file
        .metadata()
        .map_err(|_| error("invalid_conversation_target"))?;
    let current_parent = directory_handle(parent_path, false)?
        .metadata()
        .map_err(|_| error("invalid_directory"))?;
    let current = File::from(
        openat(
            &parent,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| error("conversation_target_changed"))?,
    )
    .metadata()
    .map_err(|_| error("conversation_target_changed"))?;
    if metadata.dev() != current.dev()
        || metadata.ino() != current.ino()
        || parent_metadata.dev() != current_parent.dev()
        || parent_metadata.ino() != current_parent.ino()
        || metadata.len() != after.len()
        || metadata.mtime() != after.mtime()
        || metadata.mtime_nsec() != after.mtime_nsec()
        || metadata.ctime() != after.ctime()
        || metadata.ctime_nsec() != after.ctime_nsec()
        || metadata.uid() != after.uid()
        || metadata.mode() != after.mode()
        || after.nlink() != 1
        || bytes.len() as u64 != after.len()
        || bytes.len() as u64 > MAX_FILE_BYTES
    {
        return Err(error("conversation_target_changed"));
    }
    Ok(bytes)
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

pub(super) struct State {
    path: PathBuf,
    directory: File,
    _lock: Option<LockGuard>,
    pub(super) records: Vec<Value>,
}

impl State {
    pub(super) fn open(path: &Path, create: bool) -> Result<Self, ContractError> {
        Self::open_locked(path, create, OFlags::RDWR)
    }

    pub(super) fn open_for_status(path: &Path) -> Result<Self, ContractError> {
        // Status keeps the exclusive lock without requiring write access to its file.
        Self::open_locked(path, false, OFlags::RDONLY)
    }

    fn open_locked(path: &Path, create: bool, access: OFlags) -> Result<Self, ContractError> {
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
        let lock_flags = access | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
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
            Err(rustix::io::Errno::NOENT) if access == OFlags::RDONLY => None,
            Err(rustix::io::Errno::NOENT) => return Err(error("state_lock_required")),
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

    pub(super) fn save(&self, value: &Value) -> Result<bool, ContractError> {
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
        if !entry
            .file_type()
            .map_err(|_| error("directory_unavailable"))?
            .is_file()
            || (entry.file_name() != ".learning.lock"
                && entry.path().extension().is_none_or(|e| e != "json"))
        {
            return Err(error("unsupported_state_layout"));
        }
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

fn historical(value: &Value) -> bool {
    value["kind"]
        .as_str()
        .is_some_and(|k| k.starts_with("groundline-learning-conversation-"))
}
fn validate_record(value: &Value) -> Result<(), ContractError> {
    if historical(value) {
        learning::conversation_work::validate_record(value)
    } else {
        let record: groundline_contracts::improvement::Record =
            serde_json::from_value(value.clone()).map_err(|_| error("unsupported_record"))?;
        record.validate()
    }
}
fn validate_record_collection(values: &[Value]) -> Result<(), ContractError> {
    let history: Vec<_> = values.iter().filter(|v| historical(v)).cloned().collect();
    learning::conversation_work::validate_collection(&history)?;
    groundline_contracts::improvement::validate_collection(
        &groundline_contracts::improvement::records(values)?,
    )
}
