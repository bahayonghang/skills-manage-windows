//! services/installation: skill install orchestration.
//!
//! Layered modules:
//! - `fs_util`    : path/symlink/copy primitives
//! - `centralize` : auto-centralize skills into the canonical directory
//! - `transport`  : InstallTransport seam (Local / Remote adapters)
//! - `install`    : single business orchestration of install / uninstall
//! - `native`     : Local execution half (symlink, copy, fallback, native)
//! - `remote`     : Remote (SSH/WSL) execution half via remote shell script
//! - `project`    : project-scoped install (per-project skills directory)
//! - `batch`      : Cartesian batch install over (skills, agents) pairs
//! - `types`      : public IPC payload types

pub mod batch;
pub mod centralize;
pub(crate) mod directory_link;
pub mod error;
pub mod fs_util;
pub mod install;
pub mod native;
pub mod project;
pub mod remote;
pub mod skip;
pub mod transport;
pub mod types;

#[cfg(test)]
mod tests;

pub use batch::{batch_install_central_skills_impl, batch_uninstall_skills_from_agent_impl};
pub(crate) use batch::{batch_operation_status, dedupe_ordered};
pub use error::InstallationError;
pub(crate) use fs_util::copy_dir_all_blocking;
pub use fs_util::{copy_dir_all, create_symlink, make_relative_path, symlink_target_path};
pub use install::{install_skill, uninstall_skill};
pub use transport::InstallTransport;
pub use types::{
    BatchInstallResult, BatchUninstallSkillFailure, BatchUninstallSkillRequest,
    BatchUninstallSkillResult, BatchUninstallSkillSuccess, CentralBatchInstallFailure,
    CentralBatchInstallResult, CentralBatchInstallSkipped, CentralBatchInstallSuccess,
    FailedInstall, InstallOutcome, InstallResult, SkippedInstall,
};
