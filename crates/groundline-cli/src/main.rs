#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::{Duration as ChronoDuration, Utc};
use clap::{Parser, Subcommand};
use groundline_contracts::{ContractError, batch, efficiency};
use groundline_runtime::local_file::open_bounded_regular_file;
use groundline_runtime::{audit_store, platform};
use serde_json::{Value, json};

mod operations;

#[derive(Debug, Parser)]
#[command(
    name = "groundline",
    version,
    about = "Local-first GroundLine tools for Codex"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a bounded, read-only installation and local-state diagnostic.
    Doctor {
        #[arg(long)]
        plugin_root: Option<PathBuf>,
        #[arg(long)]
        codex_home: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Inventory Codex project surfaces without reading configuration values.
    ProjectAudit {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify the installed Codex package, native target, and checksums.
    ProviderSmoke {
        #[arg(long)]
        plugin_root: Option<PathBuf>,
        #[arg(long)]
        require_installed: bool,
        #[arg(long)]
        json: bool,
    },
    /// Build privacy-safe aggregate audits from the local Codex state store.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    /// Run deterministic Goal-boundary and efficiency contracts.
    Efficiency {
        #[command(subcommand)]
        command: EfficiencyCommand,
    },
    /// Report the binary-distribution target for this host.
    Platform {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AuditCommand {
    /// Audit completed root tasks in a bounded UTC window.
    Weekly {
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u16).range(1..=365))]
        days: u16,
        #[arg(long)]
        runtime_family: Option<String>,
        #[arg(long)]
        codex_home: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Audit open, resumed, delegated, and completed activity without overlap.
    Activity {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        runtime_family: Option<String>,
        #[arg(long)]
        codex_home: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum EfficiencyCommand {
    /// Assess a Goal batch without changing Codex or GroundLine state.
    Batch {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Simulate bounded efficiency scenarios from Codex-reported audits.
    Simulate {
        #[arg(long, required = true)]
        audit: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Fuse exact Codex usage with user-provided redacted boundary counts.
    Fuse {
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        chronicle: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Propose one bounded workflow change from a weekly aggregate.
    Recommend {
        #[arg(long)]
        audit: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Gate a redacted before-and-after aggregate comparison.
    Compare {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn default_codex_home() -> Result<PathBuf, ContractError> {
    if let Some(path) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    dirs::home_dir()
        .map(|home| home.join(".codex"))
        .ok_or_else(|| ContractError("codex_home_unavailable".to_owned()))
}

fn discover_plugin_root() -> Result<PathBuf, ContractError> {
    let current =
        std::env::current_dir().map_err(|_| ContractError("plugin_root_unavailable".to_owned()))?;
    if current.join(".codex-plugin/plugin.json").is_file() {
        return Ok(current);
    }
    let executable =
        std::env::current_exe().map_err(|_| ContractError("plugin_root_unavailable".to_owned()))?;
    executable
        .ancestors()
        .find(|ancestor| ancestor.join(".codex-plugin/plugin.json").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| ContractError("plugin_root_unavailable".to_owned()))
}

fn load_bounded(path: &Path, maximum_bytes: u64) -> Result<Vec<u8>, ContractError> {
    let mut file = open_bounded_regular_file(path, 1, maximum_bytes)
        .map_err(|_| ContractError("invalid_input_file".to_owned()))?;
    let mut bytes = Vec::with_capacity(
        file.metadata()
            .map_err(|_| ContractError("input_unavailable".to_owned()))?
            .len() as usize,
    );
    file.by_ref()
        .take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ContractError("input_unavailable".to_owned()))?;
    if bytes.is_empty() || bytes.len() as u64 > maximum_bytes {
        return Err(ContractError("invalid_input_file".to_owned()));
    }
    Ok(bytes)
}

fn load_object(path: &Path) -> Result<Value, ContractError> {
    let value: Value = serde_json::from_slice(&load_bounded(path, 2 * 1024 * 1024)?)
        .map_err(|_| ContractError("invalid_json".to_owned()))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(ContractError("input_not_object".to_owned()))
    }
}

fn emit(value: &Value, json_output: bool) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(value).expect("JSON value")
        );
    } else {
        println!(
            "GroundLine: {}",
            value
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("PASS")
        );
    }
}

fn failure(error: ContractError) -> Value {
    json!({
        "kind": "groundline-runtime-error",
        "schema": 1,
        "status": "FAIL",
        "error": error.0,
        "network_performed": false,
        "mutation_performed": false,
        "raw_content_emitted": false,
        "private_paths_emitted": false,
    })
}

fn run(cli: Cli) -> Result<(), ExitCode> {
    let result: Result<(Value, bool), ContractError> = match cli.command {
        Command::Doctor {
            plugin_root,
            codex_home,
            json,
        } => {
            let home = codex_home.map(Ok).unwrap_or_else(default_codex_home);
            let root = plugin_root.or_else(|| discover_plugin_root().ok());
            home.and_then(|home| operations::doctor(root.as_deref(), &home))
                .map(|value| (value, json))
        }
        Command::ProjectAudit { repo, json } => {
            operations::project_audit(&repo).map(|value| (value, json))
        }
        Command::ProviderSmoke {
            plugin_root,
            require_installed,
            json,
        } => plugin_root
            .map(Ok)
            .unwrap_or_else(discover_plugin_root)
            .and_then(|root| operations::provider_smoke(&root, require_installed))
            .map(|value| (value, json)),
        Command::Audit {
            command:
                AuditCommand::Weekly {
                    days,
                    runtime_family,
                    codex_home,
                    json,
                },
        } => {
            let end = Utc::now();
            codex_home
                .map(Ok)
                .unwrap_or_else(default_codex_home)
                .map_err(|_| audit_store::AuditStoreError::DatabaseNotFound)
                .and_then(|home| {
                    audit_store::collect_audit(
                        &home,
                        end - ChronoDuration::days(i64::from(days)),
                        end,
                        runtime_family.as_deref(),
                        true,
                    )
                })
                .map_err(audit_store::contract_error)
                .map(|value| (value, json))
        }
        Command::Audit {
            command:
                AuditCommand::Activity {
                    start,
                    end,
                    runtime_family,
                    codex_home,
                    json,
                },
        } => {
            let window = chrono::DateTime::parse_from_rfc3339(&start)
                .map(|value| value.with_timezone(&Utc))
                .ok()
                .zip(match end.as_deref() {
                    Some(value) => chrono::DateTime::parse_from_rfc3339(value)
                        .map(|value| value.with_timezone(&Utc))
                        .ok(),
                    None => Some(Utc::now()),
                });
            window
                .ok_or_else(|| ContractError("invalid_audit_window".to_owned()))
                .and_then(|(start, end)| {
                    codex_home
                        .map(Ok)
                        .unwrap_or_else(default_codex_home)
                        .and_then(|home| {
                            audit_store::collect_audit(
                                &home,
                                start,
                                end,
                                runtime_family.as_deref(),
                                false,
                            )
                            .map_err(audit_store::contract_error)
                        })
                })
                .map(|value| (value, json))
        }
        Command::Efficiency {
            command: EfficiencyCommand::Batch { input, json },
        } => load_object(&input)
            .and_then(|packet| batch::assess(&packet))
            .map(|value| (value, json)),
        Command::Efficiency {
            command: EfficiencyCommand::Simulate { audit, json },
        } => audit
            .iter()
            .map(|path| load_object(path))
            .collect::<Result<Vec<_>, _>>()
            .and_then(|audits| efficiency::simulate(&audits))
            .map(|value| (value, json)),
        Command::Efficiency {
            command:
                EfficiencyCommand::Fuse {
                    audit,
                    chronicle,
                    json,
                },
        } => load_object(&audit)
            .and_then(|audit| {
                load_object(&chronicle).and_then(|chronicle| efficiency::fuse(&audit, &chronicle))
            })
            .map(|value| (value, json)),
        Command::Efficiency {
            command: EfficiencyCommand::Recommend { audit, json },
        } => load_object(&audit)
            .and_then(|audit| efficiency::recommend_weekly_optimization(&audit))
            .map(|value| (value, json)),
        Command::Efficiency {
            command: EfficiencyCommand::Compare { input, json },
        } => load_object(&input)
            .and_then(|packet| efficiency::compare_aggregate_periods(&packet))
            .map(|value| (value, json)),
        Command::Platform { json } => platform::current_target()
            .and_then(|target| platform::packaged_binary_path(target).map(|path| (target, path)))
            .map(|(target, path)| {
                (
                    json!({
                        "kind": "groundline-platform",
                        "schema": 1,
                        "status": "PASS",
                        "target": target,
                        "packaged_binary": path.to_string_lossy(),
                        "network_performed": false,
                        "mutation_performed": false,
                    }),
                    json,
                )
            }),
    };

    match result {
        Ok((value, json_output)) => {
            emit(&value, json_output);
            Ok(())
        }
        Err(error) => {
            emit(&failure(error), true);
            Err(ExitCode::FAILURE)
        }
    }
}

fn main() -> ExitCode {
    run(Cli::parse()).map_or_else(|code| code, |()| ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::load_bounded;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn bounded_input_rejects_empty_oversized_and_symlink_files() {
        let root = tempdir().unwrap();
        let empty = root.path().join("empty.json");
        fs::write(&empty, b"").unwrap();
        assert!(load_bounded(&empty, 8).is_err());
        let large = root.path().join("large.json");
        fs::write(&large, b"123456789").unwrap();
        assert!(load_bounded(&large, 8).is_err());
        let valid = root.path().join("valid.json");
        fs::write(&valid, b"{}").unwrap();
        assert_eq!(load_bounded(&valid, 8).unwrap(), b"{}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let linked = root.path().join("linked.json");
            symlink(&valid, &linked).unwrap();
            assert!(load_bounded(&linked, 8).is_err());
        }
    }
}
