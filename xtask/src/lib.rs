#![forbid(unsafe_code)]

use thiserror::Error;

pub mod deploy;
pub mod secret_store;

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("deployment_failed")]
    DeploymentFailed,
    #[error("missing_deployment_input:{0}")]
    MissingDeploymentInput(&'static str),
    #[error("invalid_deployment_input:{0}")]
    InvalidDeploymentInput(&'static str),
    #[error("deployment_manifest_failed")]
    Manifest(#[from] serde_json::Error),
    #[error("invalid_runtime_configuration")]
    InvalidRuntimeConfiguration,
    #[error("deployment_connect_failed")]
    ConnectFailed,
    #[error("deployment_authentication_failed")]
    AuthenticationFailed,
    #[error("deployment_inspection_failed")]
    InspectionFailed,
    #[error("invalid_current_configuration")]
    InvalidCurrentConfiguration,
    #[error("deployment_verification_failed")]
    VerificationFailed,
    #[error("deployment_failed_rolled_back")]
    RolledBack,
    #[error("deployment_rollback_failed")]
    RollbackFailed,
    #[error("deployment_runtime_failed")]
    RuntimeFailed,
    #[error("deployment_timed_out")]
    DeploymentTimedOut,
}
