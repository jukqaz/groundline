//! Read-only access to preserved records, without profile or settings mutation.
use clap::Subcommand;
use groundline_contracts::ContractError;
use serde_json::Value;
use std::path::PathBuf;
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Read preserved conversation history and current improvement summaries.
    Status {
        #[arg(long)]
        state: PathBuf,
    },
}
pub(crate) fn run(command: Command) -> Result<Value, ContractError> {
    match command {
        Command::Status { state } => crate::improvement::status(&state),
    }
}
