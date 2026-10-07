//! Registered, owner-private environment plans. Native execution remains Codex-owned.

use clap::Subcommand;
use groundline_contracts::ContractError;
use serde_json::Value;
use std::path::{Path, PathBuf};

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
        /// Private learning state for a candidate trial or evidence-based adoption.
        #[arg(long)]
        learning_state: Option<PathBuf>,
        #[arg(long, requires = "learning_state", value_parser = ["trial", "adoption"])]
        intent: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Follow the common change through this device's plans, operations and evaluations.
    Status {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        learning_state: Option<PathBuf>,
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
    /// Export only the common baseline, its lineage and registered desired content.
    Export {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        proposal: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Inspect a pinned private bundle against explicitly supplied local bindings.
    InspectBundle {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        bundle_sha256: String,
        #[arg(long)]
        bindings: PathBuf,
        #[arg(long)]
        state_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Adopt a pinned baseline without writing targets or merging divergent history.
    Import {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        bundle_sha256: String,
        #[arg(long)]
        bindings: PathBuf,
        #[arg(long)]
        authority_ref: String,
        #[arg(
            long,
            conflicts_with = "expected_revision",
            required_unless_present = "expected_revision"
        )]
        new_device: bool,
        #[arg(long, requires = "expected_exception_revision")]
        expected_revision: Option<String>,
        #[arg(long, requires = "expected_revision", conflicts_with = "new_device")]
        expected_exception_revision: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Save an existing apply/rollback plan using an already imported bundle.
    PlanBundle {
        #[arg(long)]
        state_dir: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        bundle_sha256: String,
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        json: bool,
    },
}

/// Read observed local revisions for learning; content and private paths stay private.
pub(crate) fn learning_context(state: &Path, target_id: &str) -> Result<Value, ContractError> {
    #[cfg(unix)]
    {
        state::learning_context(state, target_id)
    }
    #[cfg(not(unix))]
    {
        let _ = (state, target_id);
        Err(ContractError("environment_unsupported_platform".into()))
    }
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
