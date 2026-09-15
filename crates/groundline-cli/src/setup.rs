//! One portable installation policy, applied only by an explicit setup command.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use groundline_contracts::ContractError;
use groundline_runtime::local_file::{create_private_new, open_or_create_private_lock};
use serde_json::{Value, json};
use toml_edit::{DocumentMut, Item};

use crate::config_audit::{inspect, load_catalog};
use crate::config_repair::{canonical_target, read_config};

const DEFAULTS: &str = include_str!("../../../plugins/groundline/config/setup-defaults.toml");
const RETIRED_HOOKS: [&str; 4] = [
    "groundline@groundline:hooks/hooks.json:session_start:0:0",
    "groundline@groundline:hooks/hooks.json:session_end:0:0",
    "groundline@groundline:hooks/hooks.json:post_compact:0:0",
    "groundline@groundline:hooks/hooks.json:stop:0:0",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum Preset {
    #[default]
    Preserve,
    Astra,
}

#[derive(Debug, Args)]
pub struct Options {
    /// Defaults to CODEX_HOME, then the current user's .codex directory.
    #[arg(long)]
    codex_home: Option<PathBuf>,
    /// Native debug models JSON, or - for bounded stdin. No model fallback.
    #[arg(long)]
    catalog: PathBuf,
    /// Preserve existing choices and native defaults, or explicitly select Astra / xhigh / Fast off.
    #[arg(long, value_enum, default_value = "preserve")]
    preset: Preset,
    /// Explicitly select a model from the supplied native catalog.
    #[arg(long)]
    model: Option<String>,
    /// Explicitly select a supported reasoning effort.
    #[arg(long)]
    effort: Option<String>,
    #[arg(long, value_parser = ["default", "fast"])]
    service_tier: Option<String>,
    /// Explicitly remove root context overrides so Codex owns context sizing.
    #[arg(long)]
    restore_native_context: bool,
    /// Apply the selected policy with a private backup; unchanged files are not rewritten.
    #[arg(long)]
    apply: bool,
}

fn error(code: &str) -> ContractError {
    ContractError(format!("setup_{code}"))
}

fn candidate(
    config: &str,
    defaults: &toml::Table,
    restore_context: bool,
) -> Result<(String, Vec<String>), ContractError> {
    let mut doc: DocumentMut = config.parse().map_err(|_| error("invalid_config"))?;
    let mut expected: toml::Table = toml::from_str(config).map_err(|_| error("invalid_config"))?;
    // A profile/custom provider/catalog can change the meaning of this baseline.
    // Stop before any edit; never silently rewrite an unsupported configuration.
    if expected.contains_key("profile")
        || expected.contains_key("model_catalog_json")
        || expected
            .get("model_provider")
            .is_some_and(|p| p.as_str() != Some("openai"))
    {
        return Err(error("effective_layer_requires_review"));
    }
    let mut changes = Vec::new();
    for (key, value) in defaults {
        if expected.get(key) == Some(value) {
            continue;
        }
        if expected.get(key).is_some_and(|v| !v.is_str()) {
            return Err(error("unsupported_setting_type"));
        }
        let mut replacement =
            toml_edit::Value::from(value.as_str().ok_or_else(|| error("invalid_policy"))?);
        if let Some(old) = doc.get(key).and_then(Item::as_value) {
            *replacement.decor_mut() = old.decor().clone();
        }
        if let Some(item) = doc.get_mut(key) {
            // Replacing the map entry would discard the key's quoted spelling
            // and leading comments. Keep the key and update only its value.
            *item = Item::Value(replacement);
        } else {
            doc.as_table_mut().insert(key, Item::Value(replacement));
        }
        expected.insert(key.clone(), value.clone());
        changes.push(format!("set_{key}"));
    }
    for key in ["model_context_window", "model_auto_compact_token_limit"]
        .into_iter()
        .filter(|_| restore_context)
    {
        if let Some(value) = expected.get(key) {
            if !value.is_integer() {
                return Err(error("unsupported_setting_type"));
            }
            expected.remove(key);
            doc.remove(key);
            changes.push(format!("restore_native_{key}"));
        }
    }
    if let Some(state) = expected
        .get_mut("hooks")
        .and_then(toml::Value::as_table_mut)
        .and_then(|h| h.get_mut("state"))
        .and_then(toml::Value::as_table_mut)
    {
        for key in RETIRED_HOOKS {
            if let Some(entry) = state.get(key) {
                let valid = entry.as_table().is_some_and(|entry| {
                    !entry.is_empty()
                        && entry.iter().all(|(key, value)| match key.as_str() {
                            "enabled" => value.is_bool(),
                            "trusted_hash" => value
                                .as_str()
                                .and_then(|s| s.strip_prefix("sha256:"))
                                .is_some_and(|s| {
                                    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
                                }),
                            _ => false,
                        })
                });
                if !valid {
                    return Err(error("unsupported_hook_state"));
                }
                state.remove(key);
                doc.get_mut("hooks")
                    .and_then(Item::as_table_like_mut)
                    .and_then(|h| h.get_mut("state"))
                    .and_then(Item::as_table_like_mut)
                    .ok_or_else(|| error("unsupported_hook_state"))?
                    .remove(key);
                changes.push("remove_retired_core_hook_trust".to_owned());
            }
        }
    }
    if changes.is_empty() {
        return Ok((config.to_owned(), changes));
    }
    let mut updated = doc.to_string();
    // Preserve CRLF files; do not alter newlines inside multiline string values.
    if config.contains("\r\n") && !config.replace("\r\n", "").contains('\n') {
        updated = updated.replace("\r\n", "\n").replace('\n', "\r\n");
    }
    let actual: toml::Table = toml::from_str(&updated).map_err(|_| error("unsafe_edit"))?;
    if actual != expected {
        return Err(error("unsafe_edit"));
    }
    Ok((updated, changes))
}

fn create_config(path: &Path, updated: &[u8], report: &mut Value) -> Result<(), ContractError> {
    report["mutation_performed"] = Value::Null;
    let lock =
        open_or_create_private_lock(&path.with_file_name("config.toml.groundline-repair.lock"))
            .map_err(|_| error("lock_unavailable"))?;
    report["mutation_performed"] = json!(true);
    lock.try_lock().map_err(|_| error("config_busy"))?;
    // Exclusive creation cannot replace a concurrently created file or symlink.
    let mut file = create_private_new(path).map_err(|_| error("config_created_concurrently"))?;
    report["configuration_changed"] = Value::Null;
    file.write_all(updated)
        .and_then(|()| file.sync_all())
        .map_err(|_| error("write_outcome_unverified"))?;
    drop(file);
    #[cfg(unix)]
    fs::File::open(path.parent().ok_or_else(|| error("invalid_path"))?)
        .and_then(|dir| dir.sync_all())
        .map_err(|_| error("write_outcome_unverified"))?;
    report["configuration_changed"] = json!(true);
    if read_config(path)? != updated {
        return Err(error("verification_failed"));
    }
    report["file_verified"] = json!(true);
    report["status"] = report["audit_after"]["status"].clone();
    Ok(())
}

pub fn run(options: Options) -> Result<Value, ContractError> {
    let home = options
        .codex_home
        .map(Ok)
        .unwrap_or_else(crate::default_codex_home)?;
    // Codex's native installer creates the home first. Do not manufacture a home
    // hierarchy through possibly linked/nonexistent parents during setup.
    let path = canonical_target(&home.join("config.toml"))?;
    let exists = match fs::symlink_metadata(&path) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err(error("config_unavailable")),
    };
    let original = if exists {
        read_config(&path)?
    } else {
        Vec::new()
    };
    let config = std::str::from_utf8(&original).map_err(|_| error("invalid_config"))?;
    if options.preset == Preset::Astra
        && (options.model.is_some() || options.effort.is_some() || options.service_tier.is_some())
    {
        return Err(error("conflicting_policy"));
    }
    let mut defaults: toml::Table = if options.preset == Preset::Astra {
        toml::from_str(DEFAULTS).map_err(|_| error("invalid_policy"))?
    } else {
        toml::Table::new()
    };
    for (key, value) in [
        ("model", options.model),
        ("model_reasoning_effort", options.effort),
        ("service_tier", options.service_tier),
    ] {
        if let Some(value) = value {
            defaults.insert(key.to_owned(), toml::Value::String(value));
        }
    }
    let (updated, changes) = candidate(config, &defaults, options.restore_native_context)?;
    let after = inspect(&updated, &load_catalog(&options.catalog)?)?;
    let blocked = after["status"] == "FAIL";
    let mut report = json!({
        "kind":"groundline-setup", "schema":2,
        "status":if blocked { "FAIL" } else if changes.is_empty() { after["status"].as_str().unwrap_or("FAIL") } else { "READY" },
        "mode":if options.apply { "apply" } else { "preview" },
        "preset":if options.preset == Preset::Astra { "astra" } else { "preserve" },
        "model_policy":if defaults.contains_key("model") { "explicit_selection" } else { "existing_or_native_default" },
        "migration_review":{
            "guide":"references/installation-alignment.md#existing-settings-and-migration",
            "native_schema_check_required":true,
            "guidance_review":"groundline:align-agent-home",
            "unknown_settings_preserved":true,
            "context_overrides_removed_only_when_requested":true
        },
        "changes":changes, "audit_after":after,
        "configuration_existed":exists, "configuration_changed":false,
        "backup_written":false, "backup_file":null, "file_verified":exists && !blocked && changes.is_empty(),
        "external_writers_locked":false, "effective_runtime_verified":false,
        "network_performed":false, "mutation_performed":false,
        "raw_content_emitted":false, "private_paths_emitted":false
    });
    if !options.apply || blocked || changes.is_empty() {
        return Ok(report);
    }
    let result = if exists {
        // Same directory avoids following a separately configured backup tree.
        // The random filename is public; contents remain owner-private.
        let name = format!("config.toml.groundline-backup-{}", uuid::Uuid::new_v4());
        report["backup_file"] = json!(name);
        crate::config_repair::apply(
            &path,
            &path.with_file_name(name),
            &original,
            updated.as_bytes(),
            &mut report,
        )
    } else {
        create_config(&path, updated.as_bytes(), &mut report)
    };
    if let Err(e) = result {
        report["status"] = json!("FAIL");
        report["error"] = json!(e.0);
    } else if report["audit_after"]["status"] == "REVIEW_REQUIRED" {
        report["status"] = json!("REVIEW_REQUIRED");
    }
    Ok(report)
}
