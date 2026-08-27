use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{app_data_dir, resolve_home_dir};

/// App-data directory for Skills CLI domain-local recovery manifests.
///
/// This is SkillPort-owned recovery state, not the official CLI lock and not
/// the Central `fs_db_operations` journal.
pub const SKILLS_CLI_DIR_NAME: &str = "skills-cli";
pub const SKILLS_CLI_REMOVE_RECOVERY_DIR_NAME: &str = "remove-recovery";
pub const SKILLS_CLI_UPDATE_RECOVERY_DIR_NAME: &str = "update-recovery";

/// Local home for Skills CLI path resolution. Callers under
/// `services/skills_cli` must use this helper instead of naming
/// `resolve_home_dir` so that module stays free of host-home lookups.
pub fn skills_cli_local_home() -> PathBuf {
    resolve_home_dir()
}

pub fn skills_cli_remove_recovery_dir() -> PathBuf {
    skills_cli_remove_recovery_dir_from_app_data(&app_data_dir())
}

pub fn skills_cli_remove_recovery_dir_from_app_data(app_data: &Path) -> PathBuf {
    app_data
        .join(SKILLS_CLI_DIR_NAME)
        .join(SKILLS_CLI_REMOVE_RECOVERY_DIR_NAME)
}

/// Remote remove-recovery namespace. Local callers must keep using
/// [`skills_cli_remove_recovery_dir`] so existing manifests stay findable.
pub fn skills_cli_remove_recovery_dir_for_target(target_id: &str) -> PathBuf {
    skills_cli_remove_recovery_dir_for_target_from_app_data(&app_data_dir(), target_id)
}

pub fn skills_cli_remove_recovery_dir_for_target_from_app_data(
    app_data: &Path,
    target_id: &str,
) -> PathBuf {
    skills_cli_remove_recovery_dir_from_app_data(app_data)
        .join(sanitize_recovery_target_id(target_id))
}

fn sanitize_recovery_target_id(target_id: &str) -> String {
    let safe = !target_id.is_empty()
        && target_id != "."
        && target_id != ".."
        && !target_id.contains("..")
        && target_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if safe {
        target_id.to_string()
    } else {
        format!("{:x}", Sha256::digest(target_id.as_bytes()))
    }
}

pub fn skills_cli_update_recovery_dir() -> PathBuf {
    skills_cli_update_recovery_dir_from_app_data(&app_data_dir())
}

pub fn skills_cli_update_recovery_dir_from_app_data(app_data: &Path) -> PathBuf {
    app_data
        .join(SKILLS_CLI_DIR_NAME)
        .join(SKILLS_CLI_UPDATE_RECOVERY_DIR_NAME)
}
