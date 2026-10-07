//! Owner-private, descriptor-relative source cache. No user configuration is read.
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use groundline_contracts::ContractError;
use groundline_runtime::local_file::private_for_current_user;
use rustix::fs::{AtFlags, Mode, OFlags, mkdirat, open, openat, renameat, unlinkat};

use super::error;

fn absolute(path: &Path) -> Result<PathBuf, ContractError> {
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

fn directory(path: &Path, private: bool) -> Result<File, ContractError> {
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
                .map_err(|_| error("directory_link_or_missing"))?,
            );
        }
    }
    if private && !private_for_current_user(&directory) {
        return Err(error("private_directory_required"));
    }
    Ok(directory)
}

fn read_at(
    directory: &File,
    name: &std::ffi::OsStr,
    maximum: u64,
) -> Result<Vec<u8>, ContractError> {
    let mut file = File::from(
        openat(
            directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| error("input_unavailable"))?,
    );
    let before = file.metadata().map_err(|_| error("input_unavailable"))?;
    if !before.is_file()
        || before.nlink() != 1
        || !private_for_current_user(&file)
        || before.len() == 0
        || before.len() > maximum
    {
        return Err(error("private_regular_input_required"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("input_unavailable"))?;
    let after = file.metadata().map_err(|_| error("input_unavailable"))?;
    if bytes.len() as u64 > maximum
        || bytes.len() as u64 != after.len()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
        || after.nlink() != 1
        || !private_for_current_user(&file)
    {
        return Err(error("input_changed"));
    }
    Ok(bytes)
}

pub(super) fn read_private(path: &Path, maximum: u64) -> Result<Vec<u8>, ContractError> {
    let path = absolute(path)?;
    let parent = directory(path.parent().ok_or_else(|| error("invalid_path"))?, true)?;
    read_at(
        &parent,
        path.file_name().ok_or_else(|| error("invalid_path"))?,
        maximum,
    )
}

pub(super) struct State {
    path: PathBuf,
    directory: File,
    lock: File,
}

impl Drop for State {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}

impl State {
    pub(super) fn open(path: &Path) -> Result<Self, ContractError> {
        let path = absolute(path)?;
        let parent = directory(path.parent().ok_or_else(|| error("invalid_path"))?, false)?;
        if parent
            .metadata()
            .map_err(|_| error("state_unavailable"))?
            .uid()
            != rustix::process::geteuid().as_raw()
        {
            return Err(error("private_directory_required"));
        }
        let name = path.file_name().ok_or_else(|| error("invalid_path"))?;
        match mkdirat(&parent, name, Mode::from_raw_mode(0o700)) {
            Ok(()) => parent.sync_all().map_err(|_| error("state_unavailable"))?,
            Err(rustix::io::Errno::EXIST) => {}
            Err(_) => return Err(error("state_unavailable")),
        }
        let directory = directory(&path, true)?;
        let flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        let lock = match openat(&directory, ".sources.lock", flags, Mode::empty()) {
            Ok(fd) => File::from(fd),
            Err(rustix::io::Errno::NOENT) => File::from(
                openat(
                    &directory,
                    ".sources.lock",
                    flags | OFlags::CREATE | OFlags::EXCL,
                    Mode::from_raw_mode(0o600),
                )
                .map_err(|_| error("state_busy"))?,
            ),
            Err(_) => return Err(error("invalid_state_lock")),
        };
        let metadata = lock.metadata().map_err(|_| error("invalid_state_lock"))?;
        if !metadata.is_file() || metadata.nlink() != 1 || !private_for_current_user(&lock) {
            return Err(error("invalid_state_lock"));
        }
        lock.try_lock().map_err(|_| error("state_busy"))?;
        Ok(Self {
            path,
            directory,
            lock,
        })
    }

    fn check_binding(&self) -> Result<(), ContractError> {
        let current = directory(&self.path, true)?
            .metadata()
            .map_err(|_| error("state_binding_changed"))?;
        let held = self
            .directory
            .metadata()
            .map_err(|_| error("state_binding_changed"))?;
        if current.dev() != held.dev() || current.ino() != held.ino() {
            return Err(error("state_binding_changed"));
        }
        Ok(())
    }

    pub(super) fn read(&self, maximum: u64) -> Result<Option<Vec<u8>>, ContractError> {
        self.check_binding()?;
        match openat(
            &self.directory,
            "cache.json",
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Err(rustix::io::Errno::NOENT) => Ok(None),
            Err(_) => Err(error("invalid_cache_file")),
            Ok(fd) => {
                drop(fd);
                read_at(&self.directory, std::ffi::OsStr::new("cache.json"), maximum).map(Some)
            }
        }
    }

    pub(super) fn save(&self, bytes: &[u8]) -> Result<(), ContractError> {
        self.check_binding()?;
        // Validate any existing destination before replacement; a corrupt or
        // linked cache is never silently reset by this command.
        self.read(super::MAX_CACHE_BYTES)?;
        let name = format!(".sources-{}.tmp", uuid::Uuid::new_v4());
        let mut file = File::from(
            openat(
                &self.directory,
                name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| error("cache_write_failed"))?,
        );
        let result = (|| {
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| error("cache_write_failed"))?;
            if !private_for_current_user(&file) {
                return Err(error("cache_write_failed"));
            }
            self.check_binding()?;
            renameat(
                &self.directory,
                name.as_str(),
                &self.directory,
                "cache.json",
            )
            .map_err(|_| error("cache_write_failed"))?;
            self.directory
                .sync_all()
                .map_err(|_| error("cache_write_failed"))?;
            self.check_binding()
        })();
        if result.is_err() {
            let _ = unlinkat(&self.directory, name.as_str(), AtFlags::empty());
        }
        result
    }
}
