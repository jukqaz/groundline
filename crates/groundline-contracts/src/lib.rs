#![forbid(unsafe_code)]

pub mod artifact;

#[cfg(feature = "guidance")]
pub mod skill;

#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "efficiency")]
pub mod delivery;
#[cfg(feature = "efficiency")]
pub mod efficiency;
#[cfg(feature = "insights")]
pub mod event;
#[cfg(feature = "insights")]
pub mod grafana;
#[cfg(feature = "insights")]
pub mod insights;
#[cfg(feature = "integrity")]
pub mod integrity;
#[cfg(feature = "efficiency")]
pub mod learning;
#[cfg(any(feature = "audit", feature = "efficiency", feature = "insights"))]
pub mod model;
#[cfg(feature = "audit")]
pub mod rollout;
#[cfg(feature = "efficiency")]
pub mod routing;
#[cfg(any(feature = "audit", feature = "insights"))]
mod usage;
#[cfg(feature = "version")]
pub mod version;
#[cfg(feature = "efficiency")]
pub mod weekly_review;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ContractError(pub String);
