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
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
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
        tempfile::TempPath::try_from_path(&temporary)?
            .persist(path)
            .map_err(|error| error.error)?;
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

    use super::{atomic_write_private, open_bounded_regular_file, private_for_current_user};

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

    #[test]
    fn atomically_writes_private_state() {
        let root = tempdir().expect("temporary directory");
        let path = root.path().join("state.json");
        atomic_write_private(&path, b"{\"state\":1}\n").expect("private write");
        let file = open_bounded_regular_file(&path, 1, 64).expect("bounded state");
        assert!(private_for_current_user(&file));
        drop(file);
        atomic_write_private(&path, b"{\"state\":2}\n").expect("private replace");
        assert_eq!(fs::read(path).unwrap(), b"{\"state\":2}\n");
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
