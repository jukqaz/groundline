#![forbid(unsafe_code)]

#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "efficiency")]
pub mod efficiency;
#[cfg(feature = "version")]
pub mod version;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ContractError(pub String);
