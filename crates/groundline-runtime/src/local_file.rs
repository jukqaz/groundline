use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(unix)]
fn open_no_follow(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};

    rustix::fs::open(
        path,
        // Reject special files via fstat without first blocking on a FIFO open.
        // NONBLOCK has no effect on ordinary files; NOFOLLOW remains mandatory.
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(io::Error::from)
}

#[cfg(unix)]
fn open_no_follow_read_write(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};

    rustix::fs::open(
        path,
        OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(io::Error::from)
}

pub fn open_bounded_regular_file(path: &Path, minimum: u64, maximum: u64) -> io::Result<File> {
    if minimum > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid file size bounds",
        ));
    }
    let file = open_no_follow(path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() < minimum || metadata.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file contract rejected",
        ));
    }
    Ok(file)
}

/// Open an owner-private directory without following its final symbolic link.
pub fn open_private_directory(path: &Path) -> io::Result<File> {
    let file = open_no_follow(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || !private_for_current_user(&file) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "private directory required",
        ));
    }
    Ok(file)
}

fn private_temporary_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"))?;
    let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), sequence)))
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
}

pub fn create_private_new(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    create_private_file(path)
}

pub fn open_or_create_private_lock(path: &Path) -> io::Result<File> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
    std::fs::create_dir_all(parent)?;
    if std::fs::symlink_metadata(parent)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symbolic-link parent rejected",
        ));
    }
    let file = match create_private_file(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            open_no_follow_read_write(path)?
        }
        Err(error) => return Err(error),
    };
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || !private_for_current_user(&file) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "private lock file rejected",
        ));
    }
    Ok(file)
}

/// Atomically replace one local state file with owner-private contents.
pub fn atomic_write_private(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
    std::fs::create_dir_all(parent)?;
    if std::fs::symlink_metadata(parent)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symbolic-link parent rejected",
        ));
    }
    let temporary = private_temporary_path(path)?;
    let result = (|| {
        let mut file = create_private_file(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        if !private_for_current_user(&file) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "private file permissions rejected",
            ));
        }
        drop(file);
        std::fs::rename(&temporary, path)?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
pub fn owned_by_current_user(file: &File) -> bool {
    use std::os::unix::fs::MetadataExt;

    file.metadata()
        .map(|metadata| metadata.uid() == rustix::process::geteuid().as_raw())
        .unwrap_or(false)
}

#[cfg(unix)]
pub fn private_for_current_user(file: &File) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    file.metadata()
        .map(|metadata| {
            let effective_user = rustix::process::geteuid().as_raw();
            metadata.uid() == effective_user && metadata.permissions().mode() & 0o077 == 0
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        atomic_write_private, open_bounded_regular_file, owned_by_current_user,
        private_for_current_user,
    };

    #[test]
    fn accepts_only_regular_files_within_the_size_contract() {
        let root = tempdir().expect("temporary directory");
        let file = root.path().join("state.json");
        fs::write(&file, b"bounded").expect("fixture");
        open_bounded_regular_file(&file, 1, 16).expect("bounded regular file");
        assert!(open_bounded_regular_file(&file, 8, 16).is_err());
        assert!(open_bounded_regular_file(&file, 1, 6).is_err());
        assert!(open_bounded_regular_file(root.path(), 0, 16).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_fifo_before_waiting_for_a_writer() {
        use rustix::fs::{Mode, OFlags, open};
        use std::sync::mpsc;
        use std::time::Duration;

        let root = tempdir().unwrap();
        let path = root.path().join("state.fifo");
        // rustix does not expose mkfifoat on Apple targets. The POSIX utility
        // creates this test fixture on both macOS and Linux, without unsafe FFI.
        assert!(
            std::process::Command::new("mkfifo")
                .args(["-m", "600"])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let input = path.clone();
        let (sender, receiver) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            sender
                .send(open_bounded_regular_file(&input, 0, 16).is_err())
                .unwrap();
        });
        let result = receiver.recv_timeout(Duration::from_secs(2));
        // Release a regressed blocking reader before failing, rather than
        // leaving a detached thread or hanging the test suite.
        let rescue = result
            .is_err()
            .then(|| open(&path, OFlags::RDWR | OFlags::NONBLOCK, Mode::empty()).unwrap());
        reader.join().unwrap();
        drop(rescue);
        assert!(result.unwrap(), "FIFO must be rejected without a writer");
    }

    #[test]
    fn atomically_writes_private_state() {
        let root = tempdir().expect("temporary directory");
        let path = root.path().join("state.json");
        atomic_write_private(&path, b"{\"state\":1}\n").expect("private write");
        let file = open_bounded_regular_file(&path, 1, 64).expect("bounded state");
        assert!(owned_by_current_user(&file));
        assert!(private_for_current_user(&file));
        drop(file);
        atomic_write_private(&path, b"{\"state\":2}\n").expect("private replace");
        assert_eq!(fs::read(path).unwrap(), b"{\"state\":2}\n");
    }

    #[test]
    fn replacement_preserves_open_readers_and_private_permissions() {
        use std::io::Read;

        let root = tempdir().unwrap();
        let path = root.path().join("state.json");
        atomic_write_private(&path, b"before").unwrap();
        let mut reader = open_bounded_regular_file(&path, 1, 64).unwrap();
        atomic_write_private(&path, b"after").unwrap();
        let mut previous = Vec::new();
        reader.read_to_end(&mut previous).unwrap();
        assert_eq!(previous, b"before");
        let current = open_bounded_regular_file(&path, 1, 64).unwrap();
        assert!(private_for_current_user(&current));
        assert!(owned_by_current_user(&current));
        assert_eq!(fs::read(&path).unwrap(), b"after");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_replacement_preserves_existing_data_and_removes_temporary_file() {
        let root = tempdir().unwrap();
        let path = root.path().join("directory");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("existing"), b"preserve").unwrap();
        assert!(atomic_write_private(&path, b"replacement").is_err());
        assert_eq!(fs::read(path.join("existing")).unwrap(), b"preserve");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_at_open_time() {
        use std::os::unix::fs::symlink;

        let root = tempdir().expect("temporary directory");
        let file = root.path().join("state.json");
        let link = root.path().join("state-link.json");
        fs::write(&file, b"bounded").expect("fixture");
        symlink(&file, &link).expect("fixture symlink");
        assert!(open_bounded_regular_file(&link, 1, 16).is_err());
    }
}
