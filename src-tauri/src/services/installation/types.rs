//! Public IPC return types for skill installation operations.
//!
//! Re-exported from `services::installation` and bridged out to
//! `commands::linker::*` via `pub use` so that frontend TS bindings keep
//! seeing them under the original `commands::linker::*` path.

use serde::{Deserialize, Serialize};

/// Result of a single skill install operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub symlink_path: String,
}

/// Result of a batch install across multiple agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInstallResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<FailedInstall>,
}

/// Describes a single failed install within a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedInstall {
    pub agent_id: String,
    pub error: String,
}

/// Successful item from a Central batch install request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralBatchInstallSuccess {
    pub skill_id: String,
    pub agent_id: String,
    pub target_path: String,
}

/// Failed item from a Central batch install request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralBatchInstallFailure {
    pub skill_id: String,
    pub agent_id: String,
    pub error: String,
}

/// Result of installing multiple Central skills to multiple targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralBatchInstallResult {
    pub succeeded: Vec<CentralBatchInstallSuccess>,
    pub failed: Vec<CentralBatchInstallFailure>,
}
