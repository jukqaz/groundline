use std::fs;
use std::io::Read;
use std::path::Path;

use groundline_contracts::ContractError;
use groundline_runtime::local_file::open_bounded_regular_file;
use groundline_runtime::platform::{current_target, packaged_insights_binary_path};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const MAX_BINARY_BYTES: u64 = 128 * 1024 * 1024;

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
    if manifest.name != "groundline-insights" || manifest.version != env!("CARGO_PKG_VERSION") {
        return Err(ContractError("plugin_version_mismatch".to_owned()));
    }

    let target = current_target()?;
    let relative_binary = packaged_insights_binary_path(target)?;
    let binary = root.join(&relative_binary);
    let installed = regular_file(&binary);
    if require_installed && !installed {
        return Err(ContractError("runtime_binary_missing".to_owned()));
    }

    let hooks = read_bounded(&root.join("hooks/hooks.json"), 128 * 1024)?;
    let hooks_value: Value = serde_json::from_slice(&hooks)
        .map_err(|_| ContractError("invalid_hook_manifest".to_owned()))?;
    let hook_text =
        String::from_utf8(hooks).map_err(|_| ContractError("invalid_hook_manifest".to_owned()))?;
    let hook_count = hooks_value
        .get("hooks")
        .and_then(Value::as_object)
        .map_or(0, serde_json::Map::len);
    if hook_count != 4
        || hook_text.contains(".py")
        || hook_text.contains("python")
        || !hook_text.contains("groundline-insights")
        || !hook_text.contains("checkpoint")
    {
        return Err(ContractError("invalid_hook_manifest".to_owned()));
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
        "kind":"groundline-insights-provider-smoke",
        "schema":3,
        "status":if installed { "PASS" } else { "UNVERIFIED" },
        "provider":"codex",
        "groundline_insights_version":env!("CARGO_PKG_VERSION"),
        "target":target,
        "runtime_binary_present":installed,
        "artifact_verified":artifact_verified,
        "hook_event_count":hook_count,
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
        "kind":"groundline-insights-doctor",
        "schema":3,
        "status":if plugin_status == "PASS" && state_store_present { "PASS" } else { "WARN" },
        "groundline_insights_version":env!("CARGO_PKG_VERSION"),
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
