#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use xtask::DeployError;

#[derive(Debug, Parser)]
#[command(
    name = "groundline-deploy",
    about = "GroundLine Insights production deployment controller"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate credentials, the current app, migration plan, and live health without mutation.
    Preflight {
        #[arg(long, default_value = "infrastructure/truenas/compose.template.yaml")]
        compose_template: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Apply one immutable image if the live configuration still matches preflight.
    Apply {
        #[arg(long)]
        image: String,
        #[arg(long, default_value = "infrastructure/truenas/compose.template.yaml")]
        compose_template: PathBuf,
        #[arg(long)]
        expected_current_config_sha256: String,
        #[arg(long)]
        json: bool,
    },
}

fn failure_receipt(error: &DeployError, command: &str) -> Value {
    let (phase, reason, mutation_started, rollback, input) = match error {
        DeployError::MissingDeploymentInput(name) => (
            "input",
            "missing_deployment_input",
            false,
            "not_required",
            Some(*name),
        ),
        DeployError::InvalidDeploymentInput(name) => (
            "input",
            "invalid_deployment_input",
            false,
            "not_required",
            Some(*name),
        ),
        DeployError::InvalidRuntimeConfiguration | DeployError::Manifest(_) => (
            "input",
            "invalid_runtime_configuration",
            false,
            "not_required",
            None,
        ),
        DeployError::ConnectFailed => (
            "connect",
            "deployment_connect_failed",
            false,
            "not_required",
            None,
        ),
        DeployError::AuthenticationFailed => (
            "authenticate",
            "deployment_authentication_failed",
            false,
            "not_required",
            None,
        ),
        DeployError::InspectionFailed => (
            "inspect",
            "deployment_inspection_failed",
            false,
            "not_required",
            None,
        ),
        DeployError::InvalidCurrentConfiguration => (
            "plan",
            "invalid_current_configuration",
            false,
            "not_required",
            None,
        ),
        DeployError::VerificationFailed => (
            "verify",
            "deployment_verification_failed",
            false,
            "not_required",
            None,
        ),
        DeployError::RolledBack => (
            "verify",
            "deployment_failed_rolled_back",
            true,
            "succeeded",
            None,
        ),
        DeployError::RollbackFailed => (
            "rollback",
            "deployment_rollback_failed",
            true,
            "failed",
            None,
        ),
        DeployError::RuntimeFailed => (
            "runtime",
            "deployment_runtime_failed",
            false,
            "not_required",
            None,
        ),
        DeployError::DeploymentTimedOut => {
            ("timeout", "deployment_timed_out", true, "unknown", None)
        }
        DeployError::DeploymentFailed => {
            ("controller", "deployment_failed", false, "unknown", None)
        }
    };
    json!({
        "kind":format!("groundline-insights-deployment-{command}"),
        "schema":3,
        "status":"FAIL",
        "phase":phase,
        "reason":reason,
        "input":input,
        "mutation_started":mutation_started,
        "rollback":rollback,
        "configuration_printed":false,
        "private_url_printed":false,
        "secret_value_printed":false,
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (command, json, result) = match cli.command {
        Command::Preflight {
            compose_template,
            json,
        } => (
            "preflight",
            json,
            xtask::deploy::preflight(&compose_template),
        ),
        Command::Apply {
            image,
            compose_template,
            expected_current_config_sha256,
            json,
        } => (
            "apply",
            json,
            xtask::deploy::deploy(&image, &compose_template, &expected_current_config_sha256),
        ),
    };
    match result {
        Ok(receipt) => {
            if json {
                match serde_json::to_string_pretty(&receipt) {
                    Ok(value) => println!("{value}"),
                    Err(_) => return ExitCode::FAILURE,
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if json {
                match serde_json::to_string_pretty(&failure_receipt(&error, command)) {
                    Ok(value) => println!("{value}"),
                    Err(_) => return ExitCode::FAILURE,
                }
            } else {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DeployError, failure_receipt};

    #[test]
    fn apply_timeout_receipt_never_claims_that_mutation_or_rollback_did_not_happen() {
        let receipt = failure_receipt(&DeployError::DeploymentTimedOut, "apply");
        assert_eq!(receipt["phase"], "timeout");
        assert_eq!(receipt["mutation_started"], true);
        assert_eq!(receipt["rollback"], "unknown");
    }
}
