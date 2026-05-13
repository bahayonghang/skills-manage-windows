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

/// Describes a target that was already installed and safely left in place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedInstall {
    pub agent_id: String,
    pub target_path: String,
    pub reason: String,
}

/// Result of a batch install across multiple agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInstallResult {
    pub succeeded: Vec<String>,
    pub skipped: Vec<SkippedInstall>,
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

/// Skipped item from a Central batch install request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralBatchInstallSkipped {
    pub skill_id: String,
    pub agent_id: String,
    pub target_path: String,
    pub reason: String,
}

/// Result of installing multiple Central skills to multiple targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralBatchInstallResult {
    pub succeeded: Vec<CentralBatchInstallSuccess>,
    pub skipped: Vec<CentralBatchInstallSkipped>,
    pub failed: Vec<CentralBatchInstallFailure>,
}

#[derive(Debug, Clone)]
pub(crate) enum InstallOutcome {
    Installed(InstallResult),
    Skipped(SkippedInstall),
}

impl InstallOutcome {
    pub(crate) fn into_install_result(self) -> InstallResult {
        match self {
            InstallOutcome::Installed(result) => result,
            InstallOutcome::Skipped(skipped) => InstallResult {
                symlink_path: skipped.target_path,
            },
        }
    }
}
