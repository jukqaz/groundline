use std::fs::File;
#[cfg(not(unix))]
use std::fs::OpenOptions;
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

#[cfg(windows)]
fn open_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(windows)]
fn open_no_follow_read_write(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_no_follow(path: &Path) -> io::Result<File> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symbolic links are not allowed",
        ));
    }
    OpenOptions::new().read(true).open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_no_follow_read_write(path: &Path) -> io::Result<File> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symbolic links are not allowed",
        ));
    }
    OpenOptions::new().read(true).write(true).open(path)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &std::fs::Metadata) -> bool {
    false
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
    if !metadata.file_type().is_file()
        || is_reparse_point(&metadata)
        || metadata.len() < minimum
        || metadata.len() > maximum
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file contract rejected",
        ));
    }
    Ok(file)
}

/// Open an owner-private directory without following its final link or reparse point.
pub fn open_private_directory(path: &Path) -> io::Result<File> {
    #[cfg(windows)]
    let file = {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        };
        OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?
    };
    #[cfg(not(windows))]
    let file = open_no_follow(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || is_reparse_point(&metadata) || !private_for_current_user(&file) {
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
    if !metadata.file_type().is_file()
        || is_reparse_point(&metadata)
        || !private_for_current_user(&file)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "private lock file rejected",
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn create_private_file(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_permissions::constants::{SeObjectType, SecurityInformation};
    use windows_permissions::utilities::current_process_sid;
    use windows_permissions::{LocalBox, SecurityDescriptor};
    use windows_sys::Win32::Storage::FileSystem::{FILE_GENERIC_WRITE, WRITE_DAC, WRITE_OWNER};

    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .access_mode(FILE_GENERIC_WRITE | WRITE_DAC | WRITE_OWNER)
        .open(path)?;
    let current_user = current_process_sid()?;
    let descriptor: LocalBox<SecurityDescriptor> = format!(
        "O:{0}D:P(A;;FA;;;{0})(A;;FA;;;SY)(A;;FA;;;BA)",
        current_user
    )
    .parse()?;
    let dacl = descriptor.dacl().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "private security descriptor is missing a DACL",
        )
    })?;
    windows_permissions::wrappers::SetSecurityInfo(
        &mut file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(current_user.as_ref()),
        None,
        Some(dacl),
        None,
    )?;
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
fn create_private_file(path: &Path) -> io::Result<File> {
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
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
        // std uses FileRenameInfoEx when Windows cannot replace an open reader
        // with MoveFileExW. Keep the private file's ACL and the atomic replacement.
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

#[cfg(windows)]
pub fn owned_by_current_user(file: &File) -> bool {
    use windows_permissions::constants::{SeObjectType, SecurityInformation};
    use windows_permissions::utilities::current_process_sid;

    let Ok(current_user) = current_process_sid() else {
        return false;
    };
    windows_permissions::wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner,
    )
    .ok()
    .and_then(|descriptor| {
        descriptor
            .owner()
            .map(|owner| owner == current_user.as_ref())
    })
    .unwrap_or(false)
}

#[cfg(not(any(unix, windows)))]
pub fn owned_by_current_user(_file: &File) -> bool {
    false
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

#[cfg(windows)]
pub fn private_for_current_user(file: &File) -> bool {
    use windows_permissions::LocalBox;
    use windows_permissions::constants::{AceType, SeObjectType, SecurityInformation};
    use windows_permissions::structures::{Sid, Trustee};
    use windows_permissions::utilities::current_process_sid;

    let Ok(current_user) = current_process_sid() else {
        return false;
    };
    let Ok(descriptor) = windows_permissions::wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    ) else {
        return false;
    };
    if descriptor.owner() != Some(current_user.as_ref()) {
        return false;
    }
    let Some(dacl) = descriptor.dacl() else {
        return false;
    };
    let Ok(system) = "SY".parse::<LocalBox<Sid>>() else {
        return false;
    };
    let Ok(administrators) = "BA".parse::<LocalBox<Sid>>() else {
        return false;
    };
    let allowed = [
        current_user.as_ref(),
        system.as_ref(),
        administrators.as_ref(),
    ];
    for index in 0..dacl.len() {
        let Some(ace) = dacl.get_ace(index) else {
            return false;
        };
        if !matches!(
            ace.ace_type(),
            AceType::ACCESS_ALLOWED_ACE_TYPE
                | AceType::ACCESS_ALLOWED_CALLBACK_ACE_TYPE
                | AceType::ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE
                | AceType::ACCESS_ALLOWED_OBJECT_ACE_TYPE
        ) {
            continue;
        }
        let Some(sid) = ace.sid() else {
            return false;
        };
        if allowed.contains(&sid) {
            continue;
        }
        let trustee: Trustee<'_> = sid.into();
        match dacl.effective_rights(&trustee) {
            Ok(rights) if rights.is_empty() => {}
            _ => return false,
        }
    }
    true
}

#[cfg(not(any(unix, windows)))]
pub fn private_for_current_user(_file: &File) -> bool {
    false
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

    #[cfg(windows)]
    #[test]
    fn readonly_destination_remains_protected() {
        let root = tempdir().unwrap();
        let path = root.path().join("state.json");
        atomic_write_private(&path, b"preserve").unwrap();
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = atomic_write_private(&path, b"replacement");
        fs::set_permissions(&path, original).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"preserve");
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
