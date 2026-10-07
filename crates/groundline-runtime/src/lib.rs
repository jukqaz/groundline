#![forbid(unsafe_code)]

#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )
)))]
compile_error!("GroundLine supports only Apple Silicon macOS and Linux ARM64/x86-64");

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
#[cfg(feature = "audit-store")]
pub mod learning_response;
pub mod local_file;
pub mod platform;
#[cfg(feature = "audit-store")]
mod rollout;
#[cfg(feature = "tailnet-probe")]
pub mod tailnet;
