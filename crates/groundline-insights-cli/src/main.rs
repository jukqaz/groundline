#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use groundline_contracts::{ContractError, insights};
use groundline_runtime::local_file::open_bounded_regular_file;
use groundline_runtime::{
    checkpoint, insights as insights_runtime, insights_state, platform, tailnet,
};
use serde_json::{Value, json};

mod operations;

#[derive(Debug, Parser)]
#[command(
    name = "groundline-insights",
    version,
    about = "GroundLine Insights private collector"
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
    /// Verify the installed Codex package, hook, native target, and checksums.
    ProviderSmoke {
        #[arg(long)]
        plugin_root: Option<PathBuf>,
        #[arg(long)]
        require_installed: bool,
        #[arg(long)]
        json: bool,
    },
    /// Validate privacy-safe Insights contracts without network access.
    Insights {
        #[command(subcommand)]
        command: InsightsCommand,
    },
    /// Operate the event-driven self-hosted Insights collector.
    Worker {
        #[command(subcommand)]
        command: WorkerCommand,
    },
    /// Detach one fail-open worker from a trusted Codex lifecycle hook.
    #[command(hide = true)]
    Checkpoint {
        trigger: String,
        #[arg(long, hide = true)]
        plugin_root: Option<PathBuf>,
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Report the bounded binary-distribution target for this host.
    Platform {
        #[arg(long)]
        json: bool,
    },
    /// Report a privacy-bounded local Tailnet state without network probes.
    TailnetStatus {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum WorkerCommand {
    /// Install a validated owner-local collection profile with private permissions.
    Configure {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Collect and upload one non-overlapping activity window.
    RunOnce {
        #[arg(long, default_value = "manual", hide = true)]
        trigger: String,
        #[arg(long, hide = true)]
        plugin_root: Option<PathBuf>,
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Run the initial all-history synchronization through the same durable path.
    BackfillHistory {
        #[arg(long)]
        confirm_rebuild: bool,
        #[arg(long, hide = true)]
        plugin_root: Option<PathBuf>,
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Inspect local collection, Tailnet, outbox, and lifecycle state.
    Status {
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Enable automatic native lifecycle collection.
    Enable {
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Disable automatic collection without deleting local evidence.
    Disable {
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum InsightsCommand {
    /// Fetch one strict privacy-safe report from the configured Tailnet service.
    FetchReport {
        #[arg(long, value_parser = parse_report_days, default_value_t = 7)]
        days: u16,
        #[arg(long)]
        json: bool,
        #[arg(long, hide = true)]
        plugin_root: Option<PathBuf>,
        #[arg(long, hide = true)]
        codex_home: Option<PathBuf>,
    },
    /// Validate one exact schema-3 weekly owner report.
    ValidateReport {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
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

fn parse_report_days(value: &str) -> Result<u16, &'static str> {
    match value {
        "7" => Ok(7),
        "30" => Ok(30),
        "90" => Ok(90),
        _ => Err("expected one of 7, 30, or 90"),
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
        "kind": "groundline-rust-runtime-error",
        "schema": 1,
        "status": "FAIL",
        "error": error.0,
        "mutation_performed": false,
        "raw_content_emitted": false,
    })
}

fn runtime_failure(error: &insights_runtime::InsightsRuntimeError) -> Value {
    json!({
        "kind": "groundline-rust-runtime-error",
        "schema": 1,
        "status": "FAIL",
        "error": error.to_string(),
        "network_performed": error.network_performed(),
        "mutation_performed": false,
        "raw_content_emitted": false,
        "private_paths_emitted": false,
        "secret_value_printed": false,
    })
}

fn state_failure(error: &insights_state::StateError) -> Value {
    json!({
        "kind": "groundline-insights-worker-error",
        "schema": 1,
        "status": "FAIL",
        "result_code": error.to_string(),
        "network_performed": error.network_performed(),
        "mutation_performed": error.mutation_performed(),
        "raw_content_emitted": false,
        "private_paths_emitted": false,
        "secret_value_printed": false,
    })
}

async fn run(cli: Cli) -> Result<(), ExitCode> {
    match cli.command {
        Command::Doctor {
            plugin_root,
            codex_home,
            json,
        } => {
            let home = codex_home
                .map(Ok)
                .unwrap_or_else(insights_runtime::default_codex_home);
            let root = plugin_root
                .map(Ok)
                .or_else(|| Some(insights_runtime::discover_plugin_root()))
                .transpose()
                .ok()
                .flatten();
            match home
                .map_err(|_| ContractError("codex_home_unavailable".to_owned()))
                .and_then(|home| operations::doctor(root.as_deref(), &home))
            {
                Ok(result) => {
                    emit(&result, json);
                    Ok(())
                }
                Err(error) => {
                    emit(&failure(error), json);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::ProviderSmoke {
            plugin_root,
            require_installed,
            json,
        } => {
            let root = plugin_root
                .map(Ok)
                .unwrap_or_else(insights_runtime::discover_plugin_root);
            match root
                .map_err(|_| ContractError("plugin_root_unavailable".to_owned()))
                .and_then(|root| operations::provider_smoke(&root, require_installed))
            {
                Ok(result) => {
                    emit(&result, json);
                    Ok(())
                }
                Err(error) => {
                    emit(&failure(error), json);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Insights {
            command:
                InsightsCommand::FetchReport {
                    days,
                    json,
                    plugin_root,
                    codex_home,
                },
        } => {
            let roots = plugin_root
                .map(Ok)
                .unwrap_or_else(insights_runtime::discover_plugin_root)
                .and_then(|root| {
                    codex_home
                        .map(Ok)
                        .unwrap_or_else(insights_runtime::default_codex_home)
                        .map(|home| (root, home))
                });
            match roots {
                Ok((root, home)) => {
                    match insights_runtime::fetch_weekly_report(&root, &home, days).await {
                        Ok(report) => match serde_json::to_value(report) {
                            Ok(result) => {
                                emit(&result, json);
                                Ok(())
                            }
                            Err(_) => Err(ExitCode::FAILURE),
                        },
                        Err(error) => {
                            emit(&runtime_failure(&error), json);
                            Err(ExitCode::FAILURE)
                        }
                    }
                }
                Err(error) => {
                    emit(&runtime_failure(&error), json);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Insights {
            command: InsightsCommand::ValidateReport { input, json },
        } => match load_bounded(&input, insights::MAX_WEEKLY_REPORT_BYTES as u64)
            .and_then(|bytes| insights::WeeklyReport::from_slice(&bytes))
            .and_then(|report| {
                serde_json::to_value(report)
                    .map_err(|_| ContractError("invalid_weekly_report".to_owned()))
            }) {
            Ok(result) => {
                emit(&result, json);
                Ok(())
            }
            Err(error) => {
                emit(&failure(error), json);
                Err(ExitCode::FAILURE)
            }
        },
        Command::Worker {
            command: WorkerCommand::Configure { input, codex_home },
        } => {
            let home = codex_home
                .map(Ok)
                .unwrap_or_else(insights_runtime::default_codex_home);
            let result = home
                .map_err(|_| insights_state::StateError::LocalState)
                .and_then(|home| {
                    load_bounded(&input, 16 * 1024)
                        .map_err(|_| insights_state::StateError::InvalidProfile)
                        .and_then(|bytes| insights_state::configure_profile(&home, &bytes))
                });
            match result {
                Ok(result) => {
                    emit(&result, true);
                    Ok(())
                }
                Err(error) => {
                    emit(&state_failure(&error), true);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Worker {
            command:
                WorkerCommand::RunOnce {
                    trigger,
                    plugin_root,
                    codex_home,
                },
        } => {
            let result = match insights_state::resolve_roots(plugin_root, codex_home) {
                Ok((root, home)) => insights_state::run_once(&root, &home, &trigger).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(result) => {
                    emit(&result, true);
                    Ok(())
                }
                Err(error) => {
                    emit(&state_failure(&error), true);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Worker {
            command:
                WorkerCommand::BackfillHistory {
                    confirm_rebuild,
                    plugin_root,
                    codex_home,
                },
        } => {
            if !confirm_rebuild {
                emit(
                    &json!({"status":"FAIL","result_code":"historical_backfill_confirmation_required"}),
                    true,
                );
                return Err(ExitCode::FAILURE);
            }
            let result = match insights_state::resolve_roots(plugin_root, codex_home) {
                Ok((root, home)) => insights_state::run_once(&root, &home, "history_sync").await,
                Err(error) => Err(error),
            };
            match result {
                Ok(result) => {
                    emit(&result, true);
                    Ok(())
                }
                Err(error) => {
                    emit(&state_failure(&error), true);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Worker {
            command: WorkerCommand::Status { codex_home },
        } => {
            let home = codex_home
                .map(Ok)
                .unwrap_or_else(insights_runtime::default_codex_home);
            match home
                .map_err(|_| insights_state::StateError::LocalState)
                .and_then(|home| insights_state::status(&home))
            {
                Ok(result) => {
                    emit(&result, true);
                    Ok(())
                }
                Err(error) => {
                    emit(&state_failure(&error), true);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Worker {
            command: WorkerCommand::Enable { codex_home },
        } => {
            let home = codex_home
                .map(Ok)
                .unwrap_or_else(insights_runtime::default_codex_home);
            match home
                .map_err(|_| insights_state::StateError::LocalState)
                .and_then(|home| insights_state::enable(&home))
            {
                Ok(result) => {
                    emit(&result, true);
                    Ok(())
                }
                Err(error) => {
                    emit(&state_failure(&error), true);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Worker {
            command: WorkerCommand::Disable { codex_home },
        } => {
            let home = codex_home
                .map(Ok)
                .unwrap_or_else(insights_runtime::default_codex_home);
            match home
                .map_err(|_| insights_state::StateError::LocalState)
                .and_then(|home| insights_state::disable(&home))
            {
                Ok(result) => {
                    emit(&result, true);
                    Ok(())
                }
                Err(error) => {
                    emit(&state_failure(&error), true);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::Checkpoint {
            trigger,
            plugin_root,
            codex_home,
        } => {
            if !checkpoint::valid_trigger(&trigger) {
                return Err(ExitCode::FAILURE);
            }
            let home = codex_home
                .map(Ok)
                .unwrap_or_else(insights_runtime::default_codex_home)
                .map_err(|_| ExitCode::FAILURE)?;
            match insights_state::checkpoint_enabled(&home) {
                Ok(false) => Ok(()),
                Ok(true) => checkpoint::spawn_worker(&trigger, plugin_root.as_deref(), Some(&home))
                    .map_err(|_| ExitCode::FAILURE),
                Err(_) => Err(ExitCode::FAILURE),
            }
        }
        Command::Platform { json: json_output } => {
            match platform::current_target().and_then(|target| {
                platform::packaged_insights_binary_path(target).map(|path| (target, path))
            }) {
                Ok((target, path)) => {
                    let result = json!({
                        "kind": "groundline-platform",
                        "schema": 1,
                        "status": "PASS",
                        "target": target,
                        "packaged_binary": path.to_string_lossy(),
                        "mutation_performed": false,
                    });
                    emit(&result, json_output);
                    Ok(())
                }
                Err(error) => {
                    emit(&failure(error), json_output);
                    Err(ExitCode::FAILURE)
                }
            }
        }
        Command::TailnetStatus { json } => {
            emit(&tailnet::probe(), json);
            Ok(())
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::load_bounded;

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
