//! Registered, owner-private environment plans. Native execution remains Codex-owned.

use clap::Subcommand;
use groundline_contracts::ContractError;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Register a portable desired baseline and this device's private path bindings.
    Register {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        bindings: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Compare registered desired bytes with disk; native activation stays separate.
    Inspect {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Save an exact, private plan bound to the current baseline and authority.
    Plan {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        proposal: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Apply a saved plan after checking revision, paths and original file identity.
    Apply {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Restore unchanged generated files, preserving edited consumers and dependencies.
    Rollback {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        operation_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Observe before/after/conflict after interruption without claiming native activation.
    Recover {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        operation_id: String,
        #[arg(long)]
        json: bool,
    },
}

#[cfg(unix)]
mod files;
#[cfg(unix)]
mod state;

pub(crate) fn run(command: Command) -> Result<Value, ContractError> {
    #[cfg(unix)]
    {
        state::run(command)
    }
    #[cfg(not(unix))]
    {
        let _ = command;
        Err(ContractError("environment_unsupported_platform".into()))
    }
}

/// Validate private operation records structurally; this does not authenticate their author.
pub(crate) fn validate_operation(value: &Value) -> Result<(), ContractError> {
    #[cfg(unix)]
    {
        state::validate_operation(value)
    }
    #[cfg(not(unix))]
    {
        let _ = value;
        Err(ContractError("environment_unsupported_platform".into()))
    }
}
