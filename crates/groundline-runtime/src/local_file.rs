use std::fs::File;
#[cfg(not(unix))]
use std::fs::OpenOptions;
use std::io;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::open_bounded_regular_file;

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
