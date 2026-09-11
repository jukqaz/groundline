#![forbid(unsafe_code)]

#[cfg(feature = "guidance")]
pub mod skill;

#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "efficiency")]
pub mod efficiency;
#[cfg(feature = "insights")]
pub mod event;
#[cfg(feature = "insights")]
pub mod insights;
#[cfg(feature = "integrity")]
pub mod integrity;
#[cfg(any(feature = "audit", feature = "efficiency", feature = "insights"))]
pub mod model;
#[cfg(feature = "audit")]
pub mod rollout;
#[cfg(any(feature = "audit", feature = "insights"))]
mod usage;
#[cfg(feature = "version")]
pub mod version;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ContractError(pub String);
