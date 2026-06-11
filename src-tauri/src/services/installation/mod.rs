//! services/installation: skill install orchestration.
//!
//! Layered modules:
//! - `fs_util`    : path/symlink/copy primitives
//! - `centralize` : auto-centralize skills into the canonical directory
//! - `native`     : local install/uninstall (symlink, copy, fallback, native)
//! - `remote`     : SSH-based install/uninstall via remote shell script
//! - `project`    : project-scoped install (per-project skills directory)
//! - `batch`      : Cartesian batch install over (skills, agents) pairs
//! - `types`      : public IPC payload types

pub mod batch;
pub mod centralize;
pub mod error;
pub mod fs_util;
pub mod native;
pub mod project;
pub mod remote;
pub mod skip;
pub mod types;

#[cfg(test)]
mod tests;

pub use batch::{batch_install_central_skills_impl, batch_uninstall_skills_from_agent_impl};
pub(crate) use batch::{batch_operation_status, dedupe_ordered};
pub use error::InstallationError;
pub use fs_util::{copy_dir_all, create_symlink, make_relative_path, symlink_target_path};
pub(crate) use fs_util::copy_dir_all_blocking;
pub(crate) use native::install_central_skill_to_agent_outcome_by_method;
pub use native::{
    install_skill_to_agent_auto_impl, install_skill_to_agent_copy_impl,
    install_skill_to_agent_impl, uninstall_skill_from_agent_impl,
    uninstall_skill_from_agent_with_row_impl,
};
pub(crate) use project::install_central_skill_to_remote_project_outcome_impl;
pub(crate) use remote::install_skill_to_agent_ssh_with_connection;
pub use remote::{
    install_skill_to_agent_remote_impl, install_skill_to_agent_ssh_impl,
    uninstall_skill_from_agent_remote_impl, uninstall_skill_from_agent_ssh_impl,
};
pub(crate) use types::InstallOutcome;
pub use types::{
    BatchInstallResult, BatchUninstallSkillFailure, BatchUninstallSkillRequest,
    BatchUninstallSkillResult, BatchUninstallSkillSuccess, CentralBatchInstallFailure,
    CentralBatchInstallResult, CentralBatchInstallSkipped, CentralBatchInstallSuccess,
    FailedInstall, InstallResult, SkippedInstall,
};
