use std::path::Path;

/// Model a user-owned Codex file. An elevated Windows runner's default token
/// owner can be the Administrators group, which the runtime correctly rejects.
pub fn write_owned_config(path: &Path, bytes: &[u8]) {
    #[cfg(windows)]
    {
        use std::io::Write;
        groundline_runtime::local_file::create_private_new(path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }
    #[cfg(not(windows))]
    std::fs::write(path, bytes).unwrap();
    let file =
        groundline_runtime::local_file::open_bounded_regular_file(path, 0, 512 * 1024).unwrap();
    assert!(groundline_runtime::local_file::owned_by_current_user(&file));
}
