use std::path::{Path, PathBuf};

use super::app_data_dir;

/// App-data directory for Skills CLI domain-local recovery manifests.
///
/// This is SkillPort-owned recovery state, not the official CLI lock and not
/// the Central `fs_db_operations` journal.
pub const SKILLS_CLI_DIR_NAME: &str = "skills-cli";
pub const SKILLS_CLI_REMOVE_RECOVERY_DIR_NAME: &str = "remove-recovery";
pub const SKILLS_CLI_UPDATE_RECOVERY_DIR_NAME: &str = "update-recovery";

pub fn skills_cli_remove_recovery_dir() -> PathBuf {
    skills_cli_remove_recovery_dir_from_app_data(&app_data_dir())
}

pub fn skills_cli_remove_recovery_dir_from_app_data(app_data: &Path) -> PathBuf {
    app_data
        .join(SKILLS_CLI_DIR_NAME)
        .join(SKILLS_CLI_REMOVE_RECOVERY_DIR_NAME)
}

pub fn skills_cli_update_recovery_dir() -> PathBuf {
    skills_cli_update_recovery_dir_from_app_data(&app_data_dir())
}

pub fn skills_cli_update_recovery_dir_from_app_data(app_data: &Path) -> PathBuf {
    app_data
        .join(SKILLS_CLI_DIR_NAME)
        .join(SKILLS_CLI_UPDATE_RECOVERY_DIR_NAME)
}
