//! Bounded filesystem access and typed skill metadata. No command execution.
use super::{Fingerprints, error, report};
use groundline_contracts::ContractError;
pub(super) use groundline_contracts::skill::valid_name;
use groundline_runtime::local_file::{create_private_new, open_bounded_regular_file};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

pub(super) const MAX_ENTRIES: usize = 16_384;
pub(super) const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Budget {
    pub(super) entries: usize,
    pub(super) bytes: u64,
}
#[derive(Default, Deserialize)]
struct Ui {
    #[serde(default)]
    policy: Policy,
}
#[derive(Default, Deserialize)]
struct Policy {
    allow_implicit_invocation: Option<bool>,
}

pub(super) fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['\\', ':'])
        && value
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
        && !value.chars().any(char::is_control)
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn ignored(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git" | ".DS_Store" | "__pycache__" | ".pytest_cache")
    )
}

pub(super) fn safe_directory(path: &Path) -> Result<PathBuf, ContractError> {
    // Parent aliases supplied by the owner (e.g. macOS /var) may canonicalize,
    // but the selected directory and all traversed children must not be links.
    let metadata = fs::symlink_metadata(path).map_err(|_| error("directory_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || reparse(&metadata) {
        return Err(error("linked_or_invalid_directory"));
    }
    path.canonicalize()
        .map_err(|_| error("directory_unavailable"))
}

#[cfg(windows)]
pub(super) fn reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
pub(super) fn reparse(_: &fs::Metadata) -> bool {
    false
}

fn bounded_bytes(path: &Path, budget: &mut Budget) -> Result<Vec<u8>, ContractError> {
    let mut file =
        open_bounded_regular_file(path, 0, MAX_FILE_BYTES).map_err(|_| error("invalid_file"))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("file_unavailable"))?;
    budget.bytes += bytes.len() as u64;
    if bytes.len() as u64 > MAX_FILE_BYTES || budget.bytes > MAX_TOTAL_BYTES {
        return Err(error("byte_limit"));
    }
    Ok(bytes)
}

pub(super) struct Inspected {
    pub(super) fingerprints: Fingerprints,
    pub(super) name: String,
    pub(super) description_chars: usize,
    pub(super) implicit: bool,
}

pub(super) fn inspect(root: &Path, budget: &mut Budget) -> Result<Inspected, ContractError> {
    let root = safe_directory(root)?;
    let mut fingerprints = Fingerprints::new();
    let mut entry = None;
    let mut ui = None;
    for item in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !ignored(e.file_name()))
    {
        let item = item.map_err(|_| error("walk_failed"))?;
        budget.entries += 1;
        if budget.entries > MAX_ENTRIES || item.depth() > 16 {
            return Err(error("entry_limit"));
        }
        let metadata = fs::symlink_metadata(item.path()).map_err(|_| error("file_unavailable"))?;
        if metadata.file_type().is_symlink() || reparse(&metadata) {
            return Err(error("linked_file"));
        }
        if metadata.is_dir() {
            continue;
        }
        if !metadata.is_file() {
            return Err(error("non_regular_file"));
        }
        let relative = item
            .path()
            .strip_prefix(&root)
            .map_err(|_| error("path_escape"))?;
        let key = relative
            .components()
            .map(|component| {
                let value = component
                    .as_os_str()
                    .to_str()
                    .ok_or_else(|| error("invalid_path"))?;
                if !safe_relative(value) {
                    return Err(error("invalid_path"));
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>, ContractError>>()?
            .join("/");
        if !safe_relative(&key) {
            return Err(error("invalid_path"));
        }
        if relative.components().any(|part| {
            part.as_os_str().to_str().is_some_and(|s| {
                s == ".env"
                    || s.starts_with(".env.")
                    || matches!(s, "auth.json" | "credentials.json")
            })
        }) {
            return Err(error("sensitive_file"));
        }
        // Check canonical containment as well as every walked entry's link type.
        if !item
            .path()
            .canonicalize()
            .map_err(|_| error("file_unavailable"))?
            .starts_with(&root)
        {
            return Err(error("path_escape"));
        }
        let bytes = bounded_bytes(item.path(), budget)?;
        fingerprints.insert(key.clone(), format!("{:x}", Sha256::digest(&bytes)));
        if key == "SKILL.md" {
            entry = Some(bytes);
        } else if key == "agents/openai.yaml" {
            ui = Some(bytes);
        }
    }
    let entry = String::from_utf8(entry.ok_or_else(|| error("missing_skill_entry"))?)
        .map_err(|_| error("invalid_metadata"))?;
    let meta = groundline_contracts::skill::parse(&entry)
        .map_err(|_| error("invalid_metadata"))?
        .metadata;
    let ui: Ui = match ui {
        Some(bytes) => {
            serde_saphyr::from_str(std::str::from_utf8(&bytes).map_err(|_| error("invalid_ui"))?)
                .map_err(|_| error("invalid_ui"))?
        }
        None => Ui::default(),
    };
    Ok(Inspected {
        fingerprints,
        name: meta.name,
        description_chars: meta.description.chars().count(),
        implicit: ui.policy.allow_implicit_invocation.unwrap_or(true),
    })
}

pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ContractError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| error("input_unavailable"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || reparse(&metadata) {
        return Err(error("invalid_input_file"));
    }
    let bytes = bounded_bytes(path, &mut Budget::default())?;
    serde_json::from_slice(&bytes).map_err(|_| error("invalid_json_contract"))
}

pub(super) fn write_new(
    bytes: &[u8],
    roots: &[PathBuf],
    output: &Path,
    count: usize,
) -> Result<Value, ContractError> {
    let parent = safe_directory(
        output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    if roots.iter().any(|root| parent.starts_with(root)) {
        return Err(error("output_inside_managed_root"));
    }
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(error("snapshot_size_limit"));
    }
    let output = parent.join(output.file_name().ok_or_else(|| error("invalid_output"))?);
    let mut file =
        create_private_new(&output).map_err(|_| error("output_exists_or_unavailable"))?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    let mut value = report("snapshot");
    value["mutation_performed"] = json!(true);
    value["private_receipt_written"] = json!(written.is_ok());
    value["skill_count"] = json!(count);
    value["installation_changed"] = json!(false);
    if written.is_err() {
        value["status"] = json!("FAIL");
        value["error"] = json!("guidance_receipt_write_incomplete");
    }
    Ok(value)
}
