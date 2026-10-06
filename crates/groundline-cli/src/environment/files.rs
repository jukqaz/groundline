//! Descriptor-relative access; NOFOLLOW on a leaf alone does not confine parents.
use groundline_contracts::ContractError;
use rustix::fs::{
    AtFlags, Mode, OFlags, RenameFlags, mkdirat, open, openat, renameat, renameat_with, unlinkat,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub(super) const MAX_BYTES: u64 = 1024 * 1024;
const MAX_STATE_BYTES: u64 = 16 * MAX_BYTES;
const MAX_ARTIFACTS: usize = 256;

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FaultPoint {
    StateWrite,
    CandidateSync,
    TargetDirectorySync,
}
#[cfg(test)]
thread_local! {
    static FAULT: std::cell::RefCell<Option<(FaultPoint, String, usize)>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
pub(super) fn inject(point: FaultPoint, name: &str, skip: usize) {
    FAULT.with(|fault| *fault.borrow_mut() = Some((point, name.to_owned(), skip)));
}
#[cfg(test)]
fn fail(point: FaultPoint, name: &str) -> Result<(), ContractError> {
    FAULT.with(|fault| {
        let mut fault = fault.borrow_mut();
        if let Some((wanted, label, skip)) = fault.as_mut()
            && *wanted == point
            && (label.is_empty() || label == name)
        {
            if *skip != 0 {
                *skip -= 1;
            } else {
                *fault = None;
                return Err(error("injected_io_failure"));
            }
        }
        Ok(())
    })
}

pub(super) fn error(code: &str) -> ContractError {
    ContractError(format!("environment_{code}"))
}

fn io<T>(result: std::io::Result<T>) -> Result<T, ContractError> {
    result.map_err(|_| error("file_io"))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {
    pub dev: u64,
    pub ino: u64,
}

pub(super) fn identity(file: &File) -> Result<Identity, ContractError> {
    let m = io(file.metadata())?;
    Ok(Identity {
        dev: m.dev(),
        ino: m.ino(),
    })
}

fn owner(file: &File, directory: bool, private: bool) -> Result<(), ContractError> {
    let m = io(file.metadata())?;
    let mode = m.permissions().mode();
    if m.uid() != rustix::process::geteuid().as_raw()
        || (directory && !m.is_dir())
        || (!directory && (!m.is_file() || m.nlink() != 1))
        || mode & 0o022 != 0
        || (private && mode & 0o077 != 0)
        || (private && mode & 0o777 != if directory { 0o700 } else { 0o600 })
    {
        return Err(error("owner_or_link_contract"));
    }
    Ok(())
}

/// The canonical name is resolved once, then every component is opened without links.
pub(super) fn canonical_dir(path: &Path, private: bool) -> Result<Directory, ContractError> {
    // Path::parent returns an empty path for a bare relative filename. Bind
    // that parent to cwd before opening the same canonical, owner-checked fd.
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    let canonical = io(fs::canonicalize(path))?;
    let file = open_absolute(&canonical)?;
    owner(&file, true, private)?;
    let binding = identity(&file)?;
    Ok(Directory {
        file,
        path: canonical,
        binding,
    })
}

fn open_absolute(path: &Path) -> Result<File, ContractError> {
    if !path.is_absolute() {
        return Err(error("absolute_path_required"));
    }
    let mut current =
        File::from(open("/", dir_flags(), Mode::empty()).map_err(|_| error("root_open"))?);
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                current = File::from(
                    openat(&current, name, dir_flags(), Mode::empty())
                        .map_err(|_| error("directory_link_or_missing"))?,
                );
            }
            _ => return Err(error("invalid_path")),
        }
    }
    Ok(current)
}

fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

pub(super) struct Directory {
    pub file: File,
    pub path: PathBuf,
    pub binding: Identity,
}

/// Ending a writer scope must release its flock even while a forked child or
/// duplicated descriptor still shares the open file description. CLOEXEC only
/// closes that child's descriptor once exec happens, not during the fork gap.
#[must_use]
pub(super) struct WriterLock {
    file: File,
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

impl Directory {
    pub fn validate(&self, private: bool) -> Result<(), ContractError> {
        let current = open_absolute(&self.path)?;
        owner(&current, true, private)?;
        if identity(&current)? != self.binding || identity(&self.file)? != self.binding {
            return Err(error("directory_binding_changed"));
        }
        Ok(())
    }

    pub fn child(&self, name: &str, create: bool) -> Result<Self, ContractError> {
        leaf(name)?;
        self.validate(true)?;
        if create {
            match mkdirat(&self.file, name, Mode::from_raw_mode(0o700)) {
                Ok(()) => io(self.file.sync_all())?,
                Err(rustix::io::Errno::EXIST) => {}
                Err(_) => return Err(error("state_directory_create")),
            }
        }
        let file = File::from(
            openat(&self.file, name, dir_flags(), Mode::empty())
                .map_err(|_| error("state_directory_open"))?,
        );
        owner(&file, true, true)?;
        let binding = identity(&file)?;
        Ok(Self {
            file,
            path: self.path.join(name),
            binding,
        })
    }

    pub fn read(&self, name: &str) -> Result<Option<ReadFile>, ContractError> {
        leaf(name)?;
        self.validate(true)?;
        read_at(&self.file, name.as_ref(), true, MAX_STATE_BYTES)
    }

    /// Immutable artifact names use NOREPLACE, including on macOS.
    pub fn write(&self, name: &str, bytes: &[u8], replace: bool) -> Result<(), ContractError> {
        leaf(name)?;
        self.validate(true)?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(error("state_file_too_large"));
        }
        let previous = self.read(name)?;
        if let Some(previous) = &previous {
            if previous.bytes == bytes {
                return Ok(());
            }
            if !replace {
                return Err(error("artifact_id_conflict"));
            }
        } else {
            let entries = io(fs::read_dir(&self.path))?;
            if entries.take(MAX_ARTIFACTS + 1).count() >= MAX_ARTIFACTS {
                return Err(error("state_directory_limit"));
            }
        }
        #[cfg(test)]
        fail(FaultPoint::StateWrite, name)?;
        let temporary = format!(".{}.tmp", Uuid::new_v4());
        let mut candidate = create_at(&self.file, &temporary, 0o600)?;
        let result = (|| {
            io(candidate.write_all(bytes))?;
            io(candidate.sync_all())?;
            self.validate(true)?;
            let last = self.read(name)?;
            if last.as_ref().map(|r| (&r.binding, &r.bytes))
                != previous.as_ref().map(|r| (&r.binding, &r.bytes))
            {
                return Err(error("state_file_conflict"));
            }
            if previous.is_some() {
                renameat(&self.file, &temporary, &self.file, name)
                    .map_err(|_| error("state_rename"))?;
            } else {
                renameat_with(
                    &self.file,
                    &temporary,
                    &self.file,
                    name,
                    RenameFlags::NOREPLACE,
                )
                .map_err(|_| error("state_no_clobber"))?;
            }
            io(self.file.sync_all())?;
            let current = self
                .read(name)?
                .ok_or_else(|| error("state_readback_missing"))?;
            if current.bytes != bytes {
                return Err(error("state_readback_mismatch"));
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = unlinkat(&self.file, &temporary, AtFlags::empty());
        }
        result
    }

    pub fn lock(&self) -> Result<WriterLock, ContractError> {
        self.validate(true)?;
        let file = match openat(
            &self.file,
            "writer.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        ) {
            Ok(file) => {
                io(self.file.sync_all())?;
                File::from(file)
            }
            Err(rustix::io::Errno::EXIST) => File::from(
                openat(
                    &self.file,
                    "writer.lock",
                    OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|_| error("writer_lock_open"))?,
            ),
            Err(_) => return Err(error("writer_lock_create")),
        };
        owner(&file, false, true)?;
        file.try_lock().map_err(|_| error("writer_busy"))?;
        Ok(WriterLock { file })
    }
}

pub(super) fn open_state(path: &Path, create: bool) -> Result<Directory, ContractError> {
    let parent = path
        .parent()
        .ok_or_else(|| error("state_parent_required"))?;
    let parent = canonical_dir(parent, false)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| error("invalid_state_path"))?;
    leaf(name)?;
    if create {
        match mkdirat(&parent.file, name, Mode::from_raw_mode(0o700)) {
            Ok(()) => io(parent.file.sync_all())?,
            Err(rustix::io::Errno::EXIST) => {}
            Err(_) => return Err(error("state_directory_create")),
        }
    }
    let file = File::from(
        openat(&parent.file, name, dir_flags(), Mode::empty())
            .map_err(|_| error("state_directory_open"))?,
    );
    owner(&file, true, true)?;
    let binding = identity(&file)?;
    Ok(Directory {
        file,
        path: parent.path.join(name),
        binding,
    })
}

pub(super) fn private_input(path: &Path) -> Result<Vec<u8>, ContractError> {
    let parent = canonical_dir(
        path.parent()
            .ok_or_else(|| error("input_parent_required"))?,
        false,
    )?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| error("input_name"))?;
    leaf(name)?;
    read_at(&parent.file, name.as_ref(), true, MAX_STATE_BYTES)?
        .map(|r| r.bytes)
        .ok_or_else(|| error("input_missing"))
}

pub(super) struct ReadFile {
    pub bytes: Vec<u8>,
    pub binding: Identity,
    pub mode: u32,
}

pub(super) fn read_at(
    parent: &File,
    name: &Path,
    private: bool,
    max: u64,
) -> Result<Option<ReadFile>, ContractError> {
    let mut file = match openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(file) => File::from(file),
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(_) => return Err(error("leaf_link_or_open")),
    };
    owner(&file, false, private)?;
    let m = io(file.metadata())?;
    if m.len() > max {
        return Err(error("file_too_large"));
    }
    let binding = identity(&file)?;
    let mut bytes = Vec::new();
    io((&mut file).take(max + 1).read_to_end(&mut bytes))?;
    owner(&file, false, private)?;
    if bytes.len() as u64 > max || identity(&file)? != binding {
        return Err(error("file_changed_during_read"));
    }
    Ok(Some(ReadFile {
        bytes,
        binding,
        mode: m.mode() & 0o777,
    }))
}

pub(super) fn leaf(name: &str) -> Result<(), ContractError> {
    if name.is_empty()
        || name.len() > 200
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', '\0'])
    {
        Err(error("invalid_file_name"))
    } else {
        Ok(())
    }
}

pub(super) fn relative(path: &Path) -> Result<Vec<String>, ContractError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(error("invalid_relative_path"));
    }
    let mut parts = Vec::new();
    for part in path.components() {
        if let Component::Normal(name) = part {
            let name = name.to_str().ok_or_else(|| error("non_utf8_path"))?;
            leaf(name)?;
            parts.push(name.to_owned());
        } else {
            return Err(error("invalid_relative_path"));
        }
    }
    if parts.len() > 32 {
        return Err(error("path_depth_limit"));
    }
    Ok(parts)
}

/// Root plus each relative parent is confined by a descriptor and current logical binding.
pub(super) fn target_parent(
    root: &Directory,
    relative_path: &Path,
) -> Result<Directory, ContractError> {
    root.validate(false)?;
    let parts = relative(relative_path)?;
    let mut file = io(root.file.try_clone())?;
    let mut path = root.path.clone();
    for part in &parts[..parts.len() - 1] {
        file = File::from(
            openat(&file, part, dir_flags(), Mode::empty())
                .map_err(|_| error("parent_link_or_missing"))?,
        );
        owner(&file, true, false)?;
        path.push(part);
    }
    let binding = identity(&file)?;
    let result = Directory {
        file,
        path,
        binding,
    };
    result.validate(false)?;
    Ok(result)
}

fn create_at(parent: &File, name: &str, mode: u32) -> Result<File, ContractError> {
    let file = File::from(
        openat(
            parent,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(mode as _),
        )
        .map_err(|_| error("candidate_create"))?,
    );
    // CREATE obeys umask; existing target mode is restored deliberately on the fd.
    rustix::fs::fchmod(&file, Mode::from_raw_mode(mode as _))
        .map_err(|_| error("candidate_mode"))?;
    owner(&file, false, false)?;
    Ok(file)
}

pub(super) struct Candidate {
    name: String,
    parent: File,
    renamed: bool,
}

impl Candidate {
    pub fn new(parent: &Directory, bytes: &[u8], mode: u32) -> Result<Self, ContractError> {
        parent.validate(false)?;
        let name = format!(".groundline-{}.tmp", Uuid::new_v4());
        let mut file = create_at(&parent.file, &name, 0o600)?;
        let candidate = Self {
            name,
            parent: io(parent.file.try_clone())?,
            renamed: false,
        };
        io(file.write_all(bytes))?;
        rustix::fs::fchmod(&file, Mode::from_raw_mode(mode as _))
            .map_err(|_| error("candidate_mode"))?;
        #[cfg(test)]
        fail(FaultPoint::CandidateSync, "")?;
        io(file.sync_all())?;
        Ok(candidate)
    }

    /// Caller rechecks baseline, registration and original bytes immediately before commit.
    pub fn commit(
        &mut self,
        parent: &Directory,
        leaf: &str,
        existed: bool,
    ) -> Result<(), ContractError> {
        parent.validate(false)?;
        if identity(&self.parent)? != parent.binding {
            return Err(error("candidate_parent_changed"));
        }
        if existed {
            renameat(&parent.file, &self.name, &parent.file, leaf)
                .map_err(|_| error("target_rename"))?;
        } else {
            renameat_with(
                &parent.file,
                &self.name,
                &parent.file,
                leaf,
                RenameFlags::NOREPLACE,
            )
            .map_err(|_| error("target_no_clobber"))?;
        }
        self.renamed = true;
        // An error here is a possibly applied outcome, never an unapplied claim.
        #[cfg(test)]
        fail(FaultPoint::TargetDirectorySync, leaf)?;
        io(parent.file.sync_all())
    }

    pub fn renamed(&self) -> bool {
        self.renamed
    }
}

impl Drop for Candidate {
    fn drop(&mut self) {
        if !self.renamed {
            let _ = unlinkat(&self.parent, &self.name, AtFlags::empty());
        }
    }
}

pub(super) fn remove(parent: &Directory, name: &str) -> Result<(), ContractError> {
    parent.validate(false)?;
    unlinkat(&parent.file, name, AtFlags::empty()).map_err(|_| error("target_remove"))?;
    io(parent.file.sync_all())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::TryLockError;
    use tempfile::tempdir;

    #[test]
    fn closing_a_locked_file_does_not_release_an_inherited_descriptor_lock() {
        let root = tempdir().unwrap();
        let dir = open_state(&root.path().join("state"), true).unwrap();
        let original = create_at(&dir.file, "proof.lock", 0o600).unwrap();
        original.try_lock().unwrap();
        let inherited = original.try_clone().unwrap();
        let contender = File::options()
            .read(true)
            .write(true)
            .open(dir.path.join("proof.lock"))
            .unwrap();
        assert!(matches!(
            contender.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        drop(original);
        assert!(matches!(
            contender.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        inherited.unlock().unwrap();
        contender.try_lock().unwrap();
        contender.unlock().unwrap();
    }

    #[test]
    fn writer_guard_releases_lock_while_an_inherited_descriptor_remains_open() {
        let root = tempdir().unwrap();
        let dir = open_state(&root.path().join("state"), true).unwrap();
        let guard = dir.lock().unwrap();
        let inherited = guard.file.try_clone().unwrap();
        assert_eq!(dir.lock().err().unwrap().0, "environment_writer_busy");

        drop(guard);
        // This descriptor remains valid, like a child suspended before exec.
        inherited.metadata().unwrap();
        let next_writer = dir.lock().unwrap();
        assert_eq!(dir.lock().err().unwrap().0, "environment_writer_busy");
        drop(inherited);
        assert_eq!(dir.lock().err().unwrap().0, "environment_writer_busy");
        drop(next_writer);
        let _third_writer = dir.lock().unwrap();
    }
}
