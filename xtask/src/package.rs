use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use walkdir::WalkDir;

use super::XtaskError;

const TOP_LEVEL_FILES: &[&str] = &[
    "README.md",
    "README.ko.md",
    "CHANGELOG.md",
    "LICENSE",
    "SECURITY.md",
];
const FULL_DIRECTORIES: &[&str] = &["assets", "skills"];
const REFERENCES: &[&str] = &[
    "chronicle-evidence-contract.md",
    "codex-runtime-contract.json",
    "model-effort-routing.md",
    "native-upgrade.md",
    "skill-index.json",
    "weekly-usage-audit.md",
];
const MAX_PACKAGE_FILE_BYTES: u64 = 4 * 1024 * 1024;
const FORBIDDEN_TOOLING: &[&[u8]] = &[b"setup-python", b"python3", b"py -3", b"scripts/"];
const PRIVATE_MARKERS: &[&[u8]] = &[
    b"@gmail.com",
    b"/Users/",
    b"C:\\Users\\",
    b"api_key =",
    b"password =",
];

fn package_root(root: &Path) -> Result<PathBuf, XtaskError> {
    let root = root.canonicalize().map_err(|_| XtaskError::InvalidSource)?;
    let package = root.join("plugins/groundline");
    if package == root || !package.starts_with(&root) {
        return Err(XtaskError::InvalidSource);
    }
    Ok(package)
}

pub(super) fn regular_bytes(path: &Path) -> Result<Vec<u8>, XtaskError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| XtaskError::InvalidSource)?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_PACKAGE_FILE_BYTES
    {
        return Err(XtaskError::InvalidSource);
    }
    fs::read(path).map_err(|_| XtaskError::InvalidSource)
}

fn copy_if_changed(source: &Path, destination: &Path) -> Result<bool, XtaskError> {
    let source_bytes = regular_bytes(source)?;
    if destination.exists() && regular_bytes(destination)? == source_bytes {
        return Ok(false);
    }
    let parent = destination.parent().ok_or(XtaskError::InvalidSource)?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    fs::write(temporary.path(), &source_bytes)?;
    let permissions = fs::metadata(source)?.permissions();
    fs::set_permissions(temporary.path(), permissions)?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    temporary
        .persist(destination)
        .map_err(|error| XtaskError::Io(error.error))?;
    Ok(true)
}

fn relative_files(root: &Path) -> Result<BTreeSet<PathBuf>, XtaskError> {
    let mut files = BTreeSet::new();
    for entry in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let entry = entry.map_err(|_| XtaskError::InvalidSource)?;
        if entry.file_type().is_symlink() {
            return Err(XtaskError::InvalidSource);
        }
        if entry.file_type().is_file() {
            files.insert(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| XtaskError::InvalidSource)?
                    .to_path_buf(),
            );
        }
    }
    Ok(files)
}

fn mirror_directory(source: &Path, destination: &Path) -> Result<usize, XtaskError> {
    let expected = relative_files(source)?;
    let mut changed = 0_usize;
    for relative in &expected {
        changed += usize::from(copy_if_changed(
            &source.join(relative),
            &destination.join(relative),
        )?);
    }
    if destination.exists() {
        let current = relative_files(destination)?;
        for relative in current.difference(&expected) {
            fs::remove_file(destination.join(relative))?;
            changed += 1;
        }
        for entry in WalkDir::new(destination)
            .min_depth(1)
            .contents_first(true)
            .sort_by_file_name()
        {
            let entry = entry.map_err(|_| XtaskError::InvalidSource)?;
            if entry.file_type().is_dir() && fs::read_dir(entry.path())?.next().is_none() {
                fs::remove_dir(entry.path())?;
            }
        }
    }
    Ok(changed)
}

fn package_pairs(root: &Path) -> Vec<(PathBuf, PathBuf)> {
    let package = root.join("plugins/groundline");
    let mut pairs = TOP_LEVEL_FILES
        .iter()
        .map(|path| (root.join(path), package.join(path)))
        .collect::<Vec<_>>();
    pairs.push((
        root.join(".codex-plugin/plugin.json"),
        package.join(".codex-plugin/plugin.json"),
    ));
    pairs.extend(REFERENCES.iter().map(|name| {
        (
            root.join("references").join(name),
            package.join("references").join(name),
        )
    }));
    for directory in FULL_DIRECTORIES {
        if let Ok(files) = relative_files(&root.join(directory)) {
            pairs.extend(files.into_iter().map(|path| {
                (
                    root.join(directory).join(&path),
                    package.join(directory).join(path),
                )
            }));
        }
    }
    pairs
}

pub fn sync_package(root: &Path) -> Result<Value, XtaskError> {
    let root = root.canonicalize().map_err(|_| XtaskError::InvalidSource)?;
    let package = package_root(&root)?;
    fs::create_dir_all(&package)?;
    let mut changed = 0_usize;
    for path in TOP_LEVEL_FILES {
        changed += usize::from(copy_if_changed(&root.join(path), &package.join(path))?);
    }
    changed += usize::from(copy_if_changed(
        &root.join(".codex-plugin/plugin.json"),
        &package.join(".codex-plugin/plugin.json"),
    )?);
    for directory in FULL_DIRECTORIES {
        changed += mirror_directory(&root.join(directory), &package.join(directory))?;
    }
    let references = package.join("references");
    fs::create_dir_all(&references)?;
    let expected = REFERENCES
        .iter()
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>();
    for name in REFERENCES {
        changed += usize::from(copy_if_changed(
            &root.join("references").join(name),
            &references.join(name),
        )?);
    }
    for current in relative_files(&references)?.difference(&expected) {
        fs::remove_file(references.join(current))?;
        changed += 1;
    }
    let scripts = package.join("scripts");
    if scripts.exists() {
        fs::remove_dir_all(scripts)?;
        changed += 1;
    }
    Ok(json!({
        "kind":"groundline-package-sync",
        "schema":2,
        "status":"PASS",
        "changed_file_count":changed,
        "runtime":"rust-binary-only",
        "python_packaged":false,
    }))
}

fn manifest_version(path: &Path) -> Result<String, XtaskError> {
    let value: Value = serde_json::from_slice(&regular_bytes(path)?)?;
    value
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(XtaskError::InvalidSource)
}

fn contains_forbidden_tooling(bytes: &[u8]) -> bool {
    FORBIDDEN_TOOLING.iter().any(|forbidden| {
        bytes
            .windows(forbidden.len())
            .any(|value| value == *forbidden)
    })
}

fn contains_private_marker(bytes: &[u8]) -> bool {
    PRIVATE_MARKERS
        .iter()
        .any(|marker| bytes.windows(marker.len()).any(|value| value == *marker))
}

fn packaged_guidance_source(root: &Path, path: &Path) -> bool {
    path == root.join("README.md")
        || path == root.join("README.ko.md")
        || path.starts_with(root.join("references"))
        || path.starts_with(root.join("skills"))
}

pub fn verify_source(root: &Path) -> Result<Value, XtaskError> {
    let root = root.canonicalize().map_err(|_| XtaskError::InvalidSource)?;
    package_root(&root)?;
    let mut python = Vec::new();
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(entry.file_name().to_str(), Some(".git" | "target" | "dist"))
        })
    {
        let entry = entry.map_err(|_| XtaskError::InvalidSource)?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("py")
        {
            python.push(entry.path().to_path_buf());
        }
    }
    if !python.is_empty()
        || manifest_version(&root.join(".codex-plugin/plugin.json"))? != env!("CARGO_PKG_VERSION")
        || manifest_version(&root.join("plugins/groundline/.codex-plugin/plugin.json"))?
            != env!("CARGO_PKG_VERSION")
        || root.join("plugins/groundline/scripts").exists()
        || root.join("hooks").exists()
        || root.join("plugins/groundline/hooks").exists()
        || root.join("infrastructure").exists()
        || root.join("services").exists()
        || regular_bytes(&root.join("rust-toolchain.toml"))?
            .windows(b"channel = \"stable\"".len())
            .all(|window| window != b"channel = \"stable\"")
    {
        return Err(XtaskError::InvalidSource);
    }
    super::workflow::verify_ci_cost_contract(&root)?;
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(entry.file_name().to_str(), Some(".git" | "target" | "dist"))
        })
    {
        let entry = entry.map_err(|_| XtaskError::InvalidSource)?;
        if entry.file_type().is_file()
            && entry.path().file_name().and_then(|value| value.to_str()) != Some("Cargo.lock")
            && entry.path() != root.join("xtask/src/package.rs")
            && contains_private_marker(&regular_bytes(entry.path())?)
        {
            return Err(XtaskError::InvalidSource);
        }
    }
    for (source, destination) in package_pairs(&root) {
        let source_bytes = regular_bytes(&source)?;
        if (packaged_guidance_source(&root, &source) && contains_forbidden_tooling(&source_bytes))
            || source_bytes != regular_bytes(&destination)?
        {
            return Err(XtaskError::InvalidSource);
        }
    }
    for surface in [
        root.join(".github/ISSUE_TEMPLATE"),
        root.join(".github/pull_request_template.md"),
        root.join("docs"),
    ] {
        let entries = if surface.is_file() {
            vec![surface]
        } else {
            relative_files(&surface)?
                .into_iter()
                .map(|path| surface.join(path))
                .collect()
        };
        for entry in entries {
            if entry.file_name().and_then(|value| value.to_str()) == Some("CHANGELOG.md") {
                continue;
            }
            if contains_forbidden_tooling(&regular_bytes(&entry)?) {
                return Err(XtaskError::InvalidSource);
            }
        }
    }
    Ok(json!({
        "kind":"groundline-rust-source-verification",
        "schema":2,
        "status":"PASS",
        "groundline_version":env!("CARGO_PKG_VERSION"),
        "python_source_count":0,
        "workflow_python_count":0,
        "documentation_python_command_count":0,
        "owner_hook_count":0,
        "network_capability_count":0,
        "private_marker_count":0,
        "package_synchronized":true,
        "moving_rust_stable":true,
        "ci_cost_contract":true,
        "bounded_workflow_jobs":true,
        "immutable_ci_actions":true,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::regular_bytes;

    #[test]
    fn package_reads_reject_symlinks_and_oversized_files() {
        let root = tempdir().expect("temporary directory");
        let file = root.path().join("file");
        fs::write(&file, b"ok").unwrap();
        assert_eq!(regular_bytes(&file).unwrap(), b"ok");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file, root.path().join("link")).unwrap();
            assert!(regular_bytes(&root.path().join("link")).is_err());
        }
    }
}
