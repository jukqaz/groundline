//! Explicit package files only; no directory scan, installation, or activation.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use groundline_contracts::ContractError;
use groundline_runtime::local_file::owned_by_current_user;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{MAX_FILE_BYTES, fingerprint, path_fingerprint};

pub(crate) struct Request<'a> {
    pub source_root: &'a Path,
    pub installed_root: &'a Path,
    pub files: &'a [PathBuf],
}

#[derive(Deserialize)]
struct Identity {
    name: String,
    version: String,
}

// Open each component relative to a directory descriptor. NOFOLLOW on every
// component keeps a symlink or a concurrent rename from redirecting a read
// outside the explicit package root. File bytes and paths never enter output.
fn read_relative(root: &Path, relative: &Path, limit: u64) -> Option<Vec<u8>> {
    use rustix::fs::{Mode, OFlags, open, openat};
    let mut directory = File::from(
        open(
            root,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .ok()?,
    );
    if !owned_by_current_user(&directory) {
        return None;
    }
    let components: Vec<_> = relative.components().collect();
    if components.is_empty()
        || components
            .iter()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return None;
        };
        let last = index + 1 == components.len();
        let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        if !last {
            flags |= OFlags::DIRECTORY;
        }
        let mut file = File::from(openat(&directory, *name, flags, Mode::empty()).ok()?);
        if last {
            let metadata = file.metadata().ok()?;
            if !metadata.is_file() || metadata.len() > limit || !owned_by_current_user(&file) {
                return None;
            }
            let mut bytes = Vec::new();
            Read::by_ref(&mut file)
                .take(limit + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            return (bytes.len() as u64 <= limit).then_some(bytes);
        }
        directory = file;
    }
    None
}

fn identity(root: &Path) -> Option<(Identity, String)> {
    let bytes = read_relative(root, Path::new(".codex-plugin/plugin.json"), 16 * 1024)?;
    let identity: Identity = serde_json::from_slice(&bytes).ok()?;
    if identity.name != "groundline"
        || identity.version.len() > 96
        || semver::Version::parse(&identity.version).is_err()
    {
        return None;
    }
    Some((identity, fingerprint(&bytes)))
}

pub(super) fn observe(request: Option<&Request<'_>>) -> Result<Value, ContractError> {
    let Some(request) = request else {
        return Ok(json!({"status":"NOT_REQUESTED","execution_effect_verified":false}));
    };
    if request.files.is_empty() || request.files.len() > 32 {
        return Err(ContractError(
            "environment_observation_comparison_file_limit".into(),
        ));
    }
    if request
        .source_root
        .canonicalize()
        .ok()
        .zip(request.installed_root.canonicalize().ok())
        .is_some_and(|(source, installed)| source == installed)
    {
        return Err(ContractError(
            "environment_observation_distinct_package_roots_required".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for path in request.files {
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            || !seen.insert(path)
        {
            return Err(ContractError(
                "environment_observation_comparison_relative_files_required".into(),
            ));
        }
    }
    let source_identity = identity(request.source_root);
    let installed_identity = identity(request.installed_root);
    let versions_match = source_identity
        .as_ref()
        .zip(installed_identity.as_ref())
        .map(|((source, _), (installed, _))| source.version == installed.version);
    let integrity = match crate::operations::provider_smoke(request.installed_root, true) {
        Ok(value) => json!({"status":"PASS","evidence":value}),
        Err(error) => {
            let (status, reason) = match error.0.as_str() {
                "runtime_binary_missing" => ("FAIL", "runtime_binary_missing"),
                "invalid_artifact_checksum" => ("FAIL", "invalid_artifact_checksum"),
                "plugin_version_mismatch" => ("FAIL", "plugin_version_mismatch"),
                _ => ("UNKNOWN", "package_integrity_unverified"),
            };
            json!({"status":status,"reason":reason})
        }
    };
    let files: Vec<_> = request
        .files
        .iter()
        .enumerate()
        .map(|(index, relative)| {
            let source = read_relative(request.source_root, relative, MAX_FILE_BYTES)
                .map(|bytes| fingerprint(&bytes));
            let installed = read_relative(request.installed_root, relative, MAX_FILE_BYTES)
                .map(|bytes| fingerprint(&bytes));
            let status = match source.as_ref().zip(installed.as_ref()) {
                Some((a, b)) if a == b => "MATCH",
                Some(_) => "DIFFERENT",
                None => "UNKNOWN",
            };
            json!({"file_index":index,"relative_path_fingerprint":path_fingerprint(relative),
            "source_sha256":source,"installed_sha256":installed,"status":status})
        })
        .collect();
    let status = if versions_match == Some(false)
        || files.iter().any(|file| file["status"] == "DIFFERENT")
    {
        "DIFFERENT"
    } else if integrity["status"] == "FAIL" {
        "FAIL"
    } else if versions_match.is_none()
        || integrity["status"] != "PASS"
        || files.iter().any(|file| file["status"] == "UNKNOWN")
    {
        "UNKNOWN"
    } else {
        "MATCH"
    };
    Ok(
        json!({"status":status,"scope":"explicit_package_files_and_installed_artifact",
        "native_enabled_installation_verified":false,
        "source_root_fingerprint":path_fingerprint(request.source_root),
        "installed_root_fingerprint":path_fingerprint(request.installed_root),
        "manifest_versions_match":versions_match,
        "source_manifest_sha256":source_identity.map(|(_, hash)| hash),
        "installed_manifest_sha256":installed_identity.map(|(_, hash)| hash),
        "installed_integrity":integrity,"files":files,
        "execution_effect_verified":false}),
    )
}

pub(super) fn finish(before: Value, mut after: Value) -> Value {
    if after["status"] == "NOT_REQUESTED" {
        return after;
    }
    let stable = before == after;
    after["stable_during_observation"] = json!(stable);
    if !stable {
        after["status"] = json!("UNKNOWN");
    }
    after
}

#[cfg(test)]
mod tests {
    use super::*;
    use groundline_runtime::platform::{current_target, packaged_binary_path};
    use std::fs;
    use std::os::unix::fs::symlink;

    fn fixture(root: &Path, content: &[u8]) {
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::create_dir_all(root.join("references")).unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            json!({"name":"groundline","version":env!("CARGO_PKG_VERSION")}).to_string(),
        )
        .unwrap();
        fs::write(root.join("references/guide.md"), content).unwrap();
        let target = current_target().unwrap();
        let binary = root.join(packaged_binary_path(target).unwrap());
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        let bytes = b"test artifact, never executed";
        fs::write(&binary, bytes).unwrap();
        let name = binary.file_name().unwrap().to_str().unwrap();
        let digest = fingerprint(bytes);
        fs::write(binary.parent().unwrap().join("manifest.json"), json!({
            "schema_version":1,"kind":"groundline-binary-artifact","groundline_version":env!("CARGO_PKG_VERSION"),
            "target":target,"executable":name,"size_bytes":bytes.len(),"sha256":digest}).to_string()).unwrap();
        fs::write(
            binary.with_file_name(format!("{name}.sha256")),
            format!("{digest}  {name}\n"),
        )
        .unwrap();
    }

    #[test]
    fn same_version_different_file_is_different_and_private() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("PRIVATE_SOURCE");
        let installed = root.path().join("PRIVATE_INSTALLED");
        fixture(&source, b"PRIVATE_CHANGED_GUIDANCE");
        fixture(&installed, b"PRIVATE_ORIGINAL_GUIDANCE");
        let files = [PathBuf::from("references/guide.md")];
        let request = Request {
            source_root: &source,
            installed_root: &installed,
            files: &files,
        };
        let result = observe(Some(&request)).unwrap();
        assert_eq!(result["manifest_versions_match"], true);
        assert_eq!(result["installed_integrity"]["status"], "PASS");
        assert_eq!(result["status"], "DIFFERENT");
        assert!(!result.to_string().contains("PRIVATE_"));
        assert!(!result.to_string().contains("guide.md"));
        fixture(&installed, b"PRIVATE_CHANGED_GUIDANCE");
        assert_eq!(observe(Some(&request)).unwrap()["status"], "MATCH");
    }

    #[test]
    fn missing_or_symlinked_files_never_match() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let installed = root.path().join("installed");
        fixture(&source, b"same");
        fixture(&installed, b"same");
        let files = [PathBuf::from("references/guide.md")];
        let request = Request {
            source_root: &source,
            installed_root: &installed,
            files: &files,
        };
        fs::remove_file(installed.join(&files[0])).unwrap();
        assert_eq!(observe(Some(&request)).unwrap()["status"], "UNKNOWN");
        symlink(source.join(&files[0]), installed.join(&files[0])).unwrap();
        assert_eq!(observe(Some(&request)).unwrap()["status"], "UNKNOWN");
        fs::remove_file(installed.join(&files[0])).unwrap();
        fs::remove_dir(installed.join("references")).unwrap();
        symlink(source.join("references"), installed.join("references")).unwrap();
        assert_eq!(observe(Some(&request)).unwrap()["status"], "UNKNOWN");
    }

    #[test]
    fn unsafe_duplicate_and_unbounded_file_requests_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        for files in [
            vec![],
            vec![PathBuf::from("/private")],
            vec![PathBuf::from("../private")],
            vec![PathBuf::from("references/a"), PathBuf::from("references/a")],
            (0..33)
                .map(|n| PathBuf::from(format!("references/{n}")))
                .collect(),
        ] {
            assert!(
                observe(Some(&Request {
                    source_root: &root.path().join("source"),
                    installed_root: &root.path().join("installed"),
                    files: &files
                }))
                .is_err()
            );
        }
        assert!(
            observe(Some(&Request {
                source_root: root.path(),
                installed_root: root.path(),
                files: &[PathBuf::from("references/guide.md")]
            }))
            .is_err()
        );
    }

    #[test]
    fn changing_files_and_broken_artifacts_do_not_pass() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let installed = root.path().join("installed");
        fixture(&source, b"same");
        fixture(&installed, b"same");
        let files = [PathBuf::from("references/guide.md")];
        let request = Request {
            source_root: &source,
            installed_root: &installed,
            files: &files,
        };
        let before = observe(Some(&request)).unwrap();
        fs::write(source.join(&files[0]), b"changed").unwrap();
        let after = observe(Some(&request)).unwrap();
        assert_eq!(finish(before, after)["status"], "UNKNOWN");
        fs::write(source.join(&files[0]), b"same").unwrap();
        let binary = installed.join(packaged_binary_path(current_target().unwrap()).unwrap());
        fs::write(binary, b"corrupt").unwrap();
        assert_eq!(observe(Some(&request)).unwrap()["status"], "FAIL");
    }

    #[test]
    fn different_versions_and_oversized_files_are_not_reported_as_matches() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let installed = root.path().join("installed");
        fixture(&source, b"same");
        fixture(&installed, b"same");
        let files = [PathBuf::from("references/guide.md")];
        let request = Request {
            source_root: &source,
            installed_root: &installed,
            files: &files,
        };
        fs::write(
            source.join(".codex-plugin/plugin.json"),
            json!({"name":"groundline","version":"0.0.1"}).to_string(),
        )
        .unwrap();
        let result = observe(Some(&request)).unwrap();
        assert_eq!(result["manifest_versions_match"], false);
        assert_eq!(result["files"][0]["status"], "MATCH");
        assert_eq!(result["status"], "DIFFERENT");
        fixture(&source, b"same");
        fs::File::create(source.join(&files[0]))
            .unwrap()
            .set_len(MAX_FILE_BYTES + 1)
            .unwrap();
        assert_eq!(observe(Some(&request)).unwrap()["status"], "UNKNOWN");
    }
}
