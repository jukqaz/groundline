#![forbid(unsafe_code)]

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!("GroundLine supports only macOS and Linux");

#[cfg(feature = "audit-store")]
pub mod audit_store;
#[cfg(feature = "insights-state")]
pub mod checkpoint;
#[cfg(any(feature = "audit-store", feature = "insights-client"))]
pub mod environment;
#[cfg(feature = "insights-client")]
pub mod insights;
#[cfg(feature = "insights-state")]
pub mod insights_state;
#[cfg(feature = "insights-state")]
pub mod learning_boundary;
pub mod local_file;
pub mod platform;
#[cfg(feature = "audit-store")]
mod rollout;
#[cfg(feature = "tailnet-probe")]
pub mod tailnet;
