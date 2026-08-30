use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use groundline_contracts::ContractError;
use groundline_runtime::local_file::open_bounded_regular_file;
use groundline_runtime::platform::{current_target, packaged_binary_path};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const MAX_BINARY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PROJECT_FILES: usize = 100_000;

const INSIGHTS_PROFILE: &str = "groundline/insights/owner-profile.json";
const INSIGHTS_STATE_ROOT: &str = "groundline/insights";

#[derive(Debug, Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct ArtifactManifest {
    schema_version: u8,
    kind: String,
    groundline_version: String,
    target: String,
    executable: String,
    size_bytes: u64,
    sha256: String,
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, ContractError> {
    let mut file = open_bounded_regular_file(path, 1, maximum)
        .map_err(|_| ContractError("invalid_runtime_layout".to_owned()))?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ContractError("invalid_runtime_layout".to_owned()))?;
    if bytes.len() as u64 > maximum {
        return Err(ContractError("invalid_runtime_layout".to_owned()));
    }
    Ok(bytes)
}

fn regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn sha256(path: &Path) -> Result<(String, u64), ContractError> {
    let mut file = open_bounded_regular_file(path, 1, MAX_BINARY_BYTES)
        .map_err(|_| ContractError("invalid_runtime_binary".to_owned()))?;
    let mut digest = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| ContractError("invalid_runtime_binary".to_owned()))?;
        if count == 0 {
            break;
        }
        size = size
            .checked_add(count as u64)
            .ok_or_else(|| ContractError("invalid_runtime_binary".to_owned()))?;
        digest.update(&buffer[..count]);
    }
    Ok((format!("{:x}", digest.finalize()), size))
}

pub fn provider_smoke(root: &Path, require_installed: bool) -> Result<Value, ContractError> {
    let root = root
        .canonicalize()
        .map_err(|_| ContractError("plugin_root_unavailable".to_owned()))?;
    let manifest: PluginManifest = serde_json::from_slice(&read_bounded(
        &root.join(".codex-plugin/plugin.json"),
        MAX_MANIFEST_BYTES,
    )?)
    .map_err(|_| ContractError("invalid_plugin_manifest".to_owned()))?;
    if manifest.name != "groundline" || manifest.version != env!("CARGO_PKG_VERSION") {
        return Err(ContractError("plugin_version_mismatch".to_owned()));
    }

    let target = current_target()?;
    let relative_binary = packaged_binary_path(target)?;
    let binary = root.join(&relative_binary);
    let installed = regular_file(&binary);
    if require_installed && !installed {
        return Err(ContractError("runtime_binary_missing".to_owned()));
    }

    let hook_manifest_present = root.join("hooks/hooks.json").exists();
    if hook_manifest_present {
        return Err(ContractError("owner_hook_not_allowed".to_owned()));
    }

    let mut artifact_verified = false;
    if installed {
        let executable = relative_binary
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ContractError("invalid_runtime_binary".to_owned()))?;
        let target_root = binary
            .parent()
            .ok_or_else(|| ContractError("invalid_runtime_layout".to_owned()))?;
        let artifact: ArtifactManifest = serde_json::from_slice(&read_bounded(
            &target_root.join("manifest.json"),
            MAX_MANIFEST_BYTES,
        )?)
        .map_err(|_| ContractError("invalid_artifact_manifest".to_owned()))?;
        let (binary_sha256, binary_size) = sha256(&binary)?;
        let checksum = String::from_utf8(read_bounded(
            &target_root.join(format!("{executable}.sha256")),
            256,
        )?)
        .map_err(|_| ContractError("invalid_artifact_checksum".to_owned()))?;
        artifact_verified = artifact.schema_version == 1
            && artifact.kind == "groundline-binary-artifact"
            && artifact.groundline_version == env!("CARGO_PKG_VERSION")
            && artifact.target == target
            && artifact.executable == executable
            && artifact.size_bytes == binary_size
            && artifact.sha256 == binary_sha256
            && checksum == format!("{binary_sha256}  {executable}\n");
        if !artifact_verified {
            return Err(ContractError("invalid_artifact_checksum".to_owned()));
        }
    }

    Ok(json!({
        "kind":"groundline-provider-smoke",
        "schema":3,
        "status":if installed { "PASS" } else { "UNVERIFIED" },
        "provider":"codex",
        "groundline_version":env!("CARGO_PKG_VERSION"),
        "target":target,
        "runtime_binary_present":installed,
        "artifact_verified":artifact_verified,
        "hook_event_count":0,
        "owner_hook_present":hook_manifest_present,
        "network_capability_present":false,
        "python_runtime_required":false,
        "mutation_performed":false,
        "private_paths_emitted":false,
    }))
}

pub fn doctor(plugin_root: Option<&Path>, codex_home: &Path) -> Result<Value, ContractError> {
    let platform = current_target()?;
    let state_store_present = regular_file(&codex_home.join("state_5.sqlite"));
    let plugin = plugin_root
        .map(|root| provider_smoke(root, false))
        .transpose()?;
    let plugin_status = plugin
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("UNVERIFIED");
    Ok(json!({
        "kind":"groundline-doctor",
        "schema":3,
        "status":if plugin_status == "PASS" && state_store_present { "PASS" } else { "WARN" },
        "groundline_version":env!("CARGO_PKG_VERSION"),
        "platform_target":platform,
        "plugin_status":plugin_status,
        "codex_state_store_present":state_store_present,
        "rust_runtime":true,
        "python_runtime_required":false,
        "network_performed":false,
        "mutation_performed":false,
        "private_paths_emitted":false,
    }))
}

/// Report only privacy-safe evidence that the optional Insights integration left locally.
/// Plugin installation and hook trust stay provider-owned and are intentionally not inferred.
pub fn integration_status(codex_home: &Path, integration: &str) -> Result<Value, ContractError> {
    if integration != "insights" && integration != "groundline-insights" {
        return Err(ContractError("unsupported_integration".to_owned()));
    }
    let root = codex_home.join(INSIGHTS_STATE_ROOT);
    let profile_configured = regular_file(&codex_home.join(INSIGHTS_PROFILE));
    let mut state_directory_count = 0_u64;
    let mut identity_present = false;
    let mut consent_present = false;
    let mut token_present = false;
    let mut pending_event_count = 0_u64;

    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                continue;
            }
            state_directory_count = state_directory_count.saturating_add(1);
            identity_present |= regular_file(&path.join("identity.json"));
            consent_present |= regular_file(&path.join("consent.json"));
            token_present |= regular_file(&path.join("collector-token"));
            if let Ok(outbox) = fs::read_dir(path.join("outbox")) {
                pending_event_count = pending_event_count.saturating_add(
                    outbox
                        .flatten()
                        .filter(|item| regular_file(&item.path()))
                        .count() as u64,
                );
            }
        }
    }

    let observed = profile_configured || state_directory_count > 0;
    Ok(json!({
        "kind":"groundline-integration-status",
        "schema":1,
        "status":"PASS",
        "integration":"groundline-insights",
        "integration_contract":1,
        "local_state":if observed { "observed" } else { "not_observed" },
        "state_observed":observed,
        "state_directory_count":state_directory_count,
        "owner_profile_configured":profile_configured,
        "collector_identity_present":identity_present,
        "consent_status":if consent_present { "active" } else { "missing" },
        "collector_token_present":token_present,
        "pending_event_count":pending_event_count,
        "plugin_installation_status":"provider_check_required",
        "hook_trust_status":"provider_check_required",
        "endpoint_emitted":false,
        "collector_id_emitted":false,
        "timestamps_emitted":false,
        "network_performed":false,
        "mutation_performed":false,
        "private_paths_emitted":false,
        "secret_value_printed":false,
    }))
}

fn ignored(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry.file_type().is_dir()
        && matches!(
            entry.file_name().to_str(),
            Some(
                ".git"
                    | ".hg"
                    | ".svn"
                    | ".dart_tool"
                    | ".next"
                    | ".venv"
                    | "build"
                    | "dist"
                    | "node_modules"
                    | "target"
                    | "vendor"
            )
        )
}

fn surface(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    if matches!(name, "AGENTS.md" | "AGENTS.override.md") {
        return Some("guidance");
    }
    if name == ".worktreeinclude" {
        return Some("worktree_include");
    }
    let parent = path.parent()?.file_name()?.to_str()?;
    match (
        parent,
        name,
        path.extension().and_then(|value| value.to_str()),
    ) {
        (".codex", "config.toml", _) => Some("config"),
        (".codex", "hooks.json", _) => Some("hooks"),
        (".codex-plugin", "plugin.json", _) => Some("plugin"),
        ("agents", _, Some("toml")) => Some("agent"),
        ("rules", _, Some("rules")) => Some("rule"),
        (_, "SKILL.md", _) => Some("skill"),
        _ => None,
    }
}

pub fn project_audit(repo: &Path) -> Result<Value, ContractError> {
    let repo = repo
        .canonicalize()
        .map_err(|_| ContractError("repository_unavailable".to_owned()))?;
    if !repo.is_dir() {
        return Err(ContractError("repository_unavailable".to_owned()));
    }
    let mut counts = BTreeMap::<&'static str, u64>::new();
    let mut scanned = 0_usize;
    for entry in WalkDir::new(&repo)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !ignored(entry))
    {
        let entry = entry.map_err(|_| ContractError("project_audit_failed".to_owned()))?;
        if entry.file_type().is_symlink() || !entry.file_type().is_file() {
            continue;
        }
        scanned = scanned
            .checked_add(1)
            .ok_or_else(|| ContractError("project_too_large".to_owned()))?;
        if scanned > MAX_PROJECT_FILES {
            return Err(ContractError("project_too_large".to_owned()));
        }
        if let Some(kind) = surface(entry.path()) {
            *counts.entry(kind).or_default() += 1;
        }
    }
    let worktree_include = counts.get("worktree_include").copied().unwrap_or(0);
    Ok(json!({
        "kind":"groundline-project-audit",
        "schema":2,
        "status":"PASS",
        "provider":"codex",
        "surface_counts":counts,
        "worktree_include_present":worktree_include > 0,
        "worktree_include_recommendation":if worktree_include > 0 { "present" } else { "evaluate_if_untracked_local_files_are_required" },
        "scanned_file_count":scanned,
        "network_performed":false,
        "mutation_performed":false,
        "private_paths_emitted":false,
        "configuration_values_emitted":false,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{integration_status, project_audit};

    #[test]
    fn project_audit_counts_surfaces_without_emitting_paths_or_values() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("AGENTS.md"), "private-value").unwrap();
        fs::write(root.path().join(".worktreeinclude"), ".env.local").unwrap();
        fs::create_dir(root.path().join("node_modules")).unwrap();
        fs::write(root.path().join("node_modules/AGENTS.md"), "ignored").unwrap();
        let result = project_audit(root.path()).unwrap();
        let encoded = serde_json::to_string(&result).unwrap();
        assert_eq!(result["surface_counts"]["guidance"], 1);
        assert_eq!(result["surface_counts"]["worktree_include"], 1);
        assert!(!encoded.contains("private-value"));
        assert!(!encoded.contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn integration_status_reports_presence_without_emitting_private_values() {
        let home = tempdir().unwrap();
        let state = home.path().join("groundline/insights/codex_app-desktop");
        fs::create_dir_all(state.join("outbox")).unwrap();
        fs::write(state.join("identity.json"), "private-collector-id").unwrap();
        fs::write(state.join("consent.json"), "private-consent-id").unwrap();
        fs::write(state.join("collector-token"), "private-token").unwrap();
        fs::write(state.join("outbox/event.json"), "private-event").unwrap();
        let result = integration_status(home.path(), "insights").unwrap();
        let encoded = serde_json::to_string(&result).unwrap();
        assert_eq!(result["state_observed"], true);
        assert_eq!(result["pending_event_count"], 1);
        assert!(!encoded.contains("private-"));
        assert!(!encoded.contains(home.path().to_string_lossy().as_ref()));
    }
}
