//! Explicit, bounded repairs; native Codex still owns effective configuration.
use std::fs;
#[cfg(unix)]
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use groundline_cli::config_catalog::Catalog;
use groundline_contracts::ContractError;
use groundline_runtime::local_file::{
    atomic_write_private, create_private_new, open_bounded_regular_file,
    open_or_create_private_lock, owned_by_current_user,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::config_audit::{MAX_CONFIG_BYTES, inspect_catalog, load_catalog};

#[derive(Debug, Args)]
pub struct Options {
    #[arg(long)]
    config: PathBuf,
    /// Explicit native catalog JSON, or - for bounded stdin.
    #[arg(long)]
    catalog: PathBuf,
    /// Also remove positive context overrides after reviewing their intent.
    #[arg(long)]
    restore_native_context: bool,
    /// Apply the reviewed plan; otherwise only preview it, without writes.
    #[arg(long, requires_all = ["expect_plan", "backup"])]
    apply: bool,
    /// Plan digest returned by a preview with the same inputs and options.
    #[arg(long, requires = "apply")]
    expect_plan: Option<String>,
    /// New owner-private backup file in an existing directory; never overwrite.
    #[arg(long, requires = "apply")]
    backup: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Context {
    model_context_window: Option<toml::Spanned<i64>>,
    model_auto_compact_token_limit: Option<toml::Spanned<i64>>,
}

fn error(code: &str) -> ContractError {
    ContractError(format!("config_repair_{code}"))
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn canonical_target(path: &Path) -> Result<PathBuf, ContractError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|_| error("parent_unavailable"))?;
    let target = parent.join(path.file_name().ok_or_else(|| error("invalid_path"))?);
    if target.to_str().is_none() {
        return Err(error("invalid_path"));
    }
    Ok(target)
}

pub(crate) fn read_config(path: &Path) -> Result<Vec<u8>, ContractError> {
    let file = open_bounded_regular_file(path, 0, MAX_CONFIG_BYTES)
        .map_err(|_| error("invalid_config_file"))?;
    if !owned_by_current_user(&file) {
        return Err(error("config_owner_mismatch"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if file
            .metadata()
            .map_err(|_| error("invalid_config_file"))?
            .nlink()
            != 1
        {
            return Err(error("linked_config_file"));
        }
    }
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("config_unavailable"))?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(error("config_too_large"));
    }
    Ok(bytes)
}

fn candidate(config: &str, restore: bool) -> Result<(String, Vec<&'static str>), ContractError> {
    let context: Context = toml::from_str(config).map_err(|_| error("invalid_config"))?;
    let invalid_pair = matches!(
        (&context.model_context_window, &context.model_auto_compact_token_limit),
        (Some(window), Some(limit)) if limit.get_ref() > window.get_ref()
    );
    let fields = [
        ("model_context_window", context.model_context_window),
        (
            "model_auto_compact_token_limit",
            context.model_auto_compact_token_limit,
        ),
    ];
    let mut expected: toml::Table = toml::from_str(config).map_err(|_| error("invalid_config"))?;
    let mut ranges = Vec::new();
    let mut removed = Vec::new();
    for (key, value) in fields {
        if let Some(value) = value
            && (restore || invalid_pair || *value.get_ref() <= 0)
        {
            let span = value.span();
            // Integer assignments occupy one line. Use parser spans, then
            // prove semantic equality to removing ONLY the allowlisted keys.
            let start = config[..span.start].rfind('\n').map_or(0, |i| i + 1);
            let end = config[span.end..]
                .find('\n')
                .map_or(config.len(), |i| span.end + i + 1);
            ranges.push(start..end);
            expected.remove(key);
            removed.push(key);
        }
    }
    ranges.sort_by_key(|range| range.start);
    let mut updated = config.to_owned();
    for range in ranges.into_iter().rev() {
        updated.replace_range(range, "");
    }
    let actual: toml::Table = toml::from_str(&updated).map_err(|_| error("unsafe_edit"))?;
    if actual != expected {
        return Err(error("unsafe_edit"));
    }
    Ok((updated, removed))
}

fn changed_failure(report: &mut Value, code: &str) {
    report["status"] = json!("FAIL");
    report["error"] = json!(format!("config_repair_{code}"));
}

pub(crate) fn apply(
    config: &Path,
    backup: &Path,
    original: &[u8],
    updated: &[u8],
    report: &mut Value,
) -> Result<(), ContractError> {
    let backup = canonical_target(backup)?;
    if backup == config || fs::symlink_metadata(&backup).is_ok() {
        return Err(error("backup_exists_or_invalid"));
    }
    let mut name = config
        .file_name()
        .ok_or_else(|| error("invalid_path"))?
        .to_os_string();
    name.push(".groundline-repair.lock");
    let lock_path = config.with_file_name(name);
    if backup == lock_path {
        return Err(error("backup_exists_or_invalid"));
    }
    // Serializes GroundLine repairs. Codex/editors do not share this lock;
    // read again immediately before replacement and report that limitation.
    report["mutation_performed"] = Value::Null;
    let lock = open_or_create_private_lock(&lock_path).map_err(|_| error("lock_unavailable"))?;
    report["mutation_performed"] = json!(true);
    lock.try_lock().map_err(|_| error("config_busy"))?;
    if read_config(config)? != original {
        return Err(error("config_changed"));
    }
    let mut file =
        create_private_new(&backup).map_err(|_| error("backup_exists_or_unavailable"))?;
    if file
        .write_all(original)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        changed_failure(report, "backup_write_incomplete");
        return Ok(());
    }
    drop(file);
    #[cfg(unix)]
    File::open(backup.parent().ok_or_else(|| error("invalid_path"))?)
        .and_then(|dir| dir.sync_all())
        .map_err(|_| error("backup_sync_failed"))?;
    report["backup_written"] = json!(true);
    if read_config(config)? != original {
        return Err(error("config_changed"));
    }
    if atomic_write_private(config, updated).is_err() {
        // A rename can succeed even if subsequent directory sync fails.
        report["configuration_changed"] = Value::Null;
        changed_failure(report, "write_outcome_unverified");
        return Ok(());
    }
    report["configuration_changed"] = json!(true);
    if read_config(config)? != updated {
        changed_failure(report, "verification_failed");
        return Ok(());
    }
    report["status"] = report["audit_after"]["status"].clone();
    report["file_verified"] = json!(true);
    Ok(())
}

pub fn run(options: Options) -> Result<Value, ContractError> {
    let config_path = canonical_target(&options.config)?;
    let original = read_config(&config_path)?;
    let text = std::str::from_utf8(&original).map_err(|_| error("invalid_config"))?;
    let catalog = load_catalog(&options.catalog)?;
    let parsed_catalog = Catalog::parse(&catalog)?;
    let before = inspect_catalog(text, &parsed_catalog)?;
    let (updated, removed) = candidate(text, options.restore_native_context)?;
    let after = inspect_catalog(&updated, &parsed_catalog)?;
    let unresolved = before["findings"].as_array().is_some_and(|findings| {
        findings.iter().any(|f| {
            matches!(
                f["code"].as_str(),
                Some(
                    "profile_layer_not_resolved"
                        | "custom_provider_runtime_unverified"
                        | "catalog_override_requires_verification"
                )
            )
        })
    });
    // Bind review to this target, exact bytes, catalog, policy, and option.
    let plan = hash(
        &serde_json::to_vec(&json!([
            "groundline-config-repair-v1",
            config_path.to_str().ok_or_else(|| error("invalid_path"))?,
            hash(&original),
            hash(&catalog),
            options.restore_native_context,
            hash(updated.as_bytes())
        ]))
        .map_err(|_| error("invalid_path"))?,
    );
    let status = if unresolved || after["status"] == "FAIL" {
        "FAIL"
    } else if removed.is_empty() {
        after["status"].as_str().unwrap_or("FAIL")
    } else {
        "READY"
    };
    let mut report = json!({
        "kind":"groundline-config-repair", "schema":1, "status":status,
        "scope":"single_config_layer", "plan_sha256":plan,
        "removed_keys":removed, "audit_before":before, "audit_after":after,
        "mutation_performed":false, "configuration_changed":false,
        "backup_written":false, "file_verified":false,
        "native_schema_validation":"not_run", "effective_runtime_verified":false,
        "external_writers_locked":false, "network_performed":false,
        "configuration_values_emitted":false, "private_paths_emitted":false,
        "raw_content_emitted":false, "scripts_executed":false
    });
    if unresolved {
        report["error"] = json!("config_repair_resolve_native_layers_first");
    }
    if options.apply {
        if options.expect_plan.as_deref() != Some(&plan) {
            return Err(error("plan_changed"));
        }
        if status != "FAIL"
            && !removed.is_empty()
            && let Err(failure) = apply(
                &config_path,
                options
                    .backup
                    .as_deref()
                    .ok_or_else(|| error("backup_required"))?,
                &original,
                updated.as_bytes(),
                &mut report,
            )
        {
            report["status"] = json!("FAIL");
            report["error"] = json!(failure.0);
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_invalid_assignments_preserving_comments_crlf_and_nested_values() {
        let input = "# 한글\r\n\"model_context_window\" = 0 # stale\r\nmodel='gpt-6-astra'\r\n[other]\r\nmodel_context_window=123\r\ntext='''\r\nmodel_context_window=0\r\n'''\r\n";
        let (updated, removed) = candidate(input, false).unwrap();
        assert_eq!(removed, ["model_context_window"]);
        assert_eq!(
            updated,
            input.replace("\"model_context_window\" = 0 # stale\r\n", "")
        );
    }

    #[test]
    fn positive_overrides_require_explicit_restoration_and_invalid_pairs_clear_together() {
        let valid = "model_context_window=100\nmodel_auto_compact_token_limit=90";
        assert_eq!(candidate(valid, false).unwrap().0, valid);
        assert_eq!(candidate(valid, true).unwrap().0, "");
        assert_eq!(
            candidate(
                "model_context_window=100\nmodel_auto_compact_token_limit=200",
                false
            )
            .unwrap()
            .0,
            ""
        );
    }
}
