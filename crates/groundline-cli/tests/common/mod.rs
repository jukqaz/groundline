use std::path::Path;

/// Model a user-owned Codex file.
pub fn write_owned_config(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    let file =
        groundline_runtime::local_file::open_bounded_regular_file(path, 0, 512 * 1024).unwrap();
    assert!(groundline_runtime::local_file::owned_by_current_user(&file));
}
