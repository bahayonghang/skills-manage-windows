//! Auto-centralize logic: ensure that a skill exists under the central
//! directory before installing it elsewhere, and shared helpers for
//! detecting/handling existing entries at install targets.

use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};

use super::error::InstallationError;
use super::fs_util::{
    copy_dir_all_blocking, path_exists_blocking, remove_symlink_path, run_blocking_fs,
};

/// Ensure the skill exists in the central directory. If it doesn't, copy it
/// from its actual location (looked up in the database) and update the DB
/// record to mark it as central.
///
/// Callers own the target mutation guard. Existing `SKILL.md` is not enough
/// to skip the database repair: a leftover copy with `is_central = false`
/// must still be upserted.
pub(crate) async fn ensure_centralized(
    pool: &DbPool,
    skill_id: &str,
    canonical_dir: &Path,
) -> Result<(), InstallationError> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| InstallationError::SkillNotFound(skill_id.to_string()))?;

    let canonical_exists = path_exists_blocking(&canonical_dir.join("SKILL.md")).await?;
    if canonical_exists && skill.is_central {
        return Ok(());
    }

    let copied_now = if canonical_exists {
        false
    } else {
        let source_file = PathBuf::from(&skill.file_path);
        let source_dir = source_file
            .parent()
            .ok_or_else(|| InstallationError::InvalidSkillFilePath(skill_id.to_string()))?;

        if !path_exists_blocking(&source_file).await? {
            return Err(InstallationError::SkillSourceMissing(
                source_file.display().to_string(),
            ));
        }

        copy_dir_all_blocking(source_dir, canonical_dir).await?;
        true
    };

    match persist_central_skill_row(pool, skill, canonical_dir).await {
        Ok(()) => Ok(()),
        Err(error) if copied_now => {
            let _ = remove_copied_canonical_dir(canonical_dir).await;
            Err(error)
        }
        Err(error) => Err(error),
    }
}

async fn persist_central_skill_row(
    pool: &DbPool,
    mut skill: db::Skill,
    canonical_dir: &Path,
) -> Result<(), InstallationError> {
    skill.canonical_path = Some(canonical_dir.to_string_lossy().into_owned());
    skill.is_central = true;
    skill.file_path = canonical_dir
        .join("SKILL.md")
        .to_string_lossy()
        .into_owned();
    db::upsert_skill(pool, &skill).await?;
    Ok(())
}

async fn remove_copied_canonical_dir(canonical_dir: &Path) -> Result<(), InstallationError> {
    let canonical_dir = canonical_dir.to_path_buf();
    run_blocking_fs("ensure_centralized compensate canonical copy", move || {
        match std::fs::symlink_metadata(&canonical_dir) {
            Ok(_) => std::fs::remove_dir_all(&canonical_dir).map_err(|e| {
                InstallationError::io(
                    format!(
                        "Failed to remove incomplete central copy '{}'",
                        canonical_dir.display()
                    ),
                    e,
                )
            }),
            Err(_) => Ok(()),
        }
    })
    .await
}

/// Two agents share a skills directory if their global_skills_dir resolves to
/// the same canonical filesystem path.
pub(crate) fn agents_share_skills_dir(agent: &db::Agent, central: &db::Agent) -> bool {
    crate::paths::paths_equivalent(
        Path::new(&agent.global_skills_dir),
        Path::new(&central.global_skills_dir),
    )
}

/// Make sure the install target slot is free for a fresh install.
///
/// - Symlink at the path: removed.
/// - Real directory or file: refused.
/// - Path missing: ok.
pub(crate) fn ensure_replaceable_target_sync(target_path: &Path) -> Result<(), InstallationError> {
    match std::fs::symlink_metadata(target_path) {
        Ok(meta) if meta.file_type().is_symlink() => remove_symlink_path(target_path)
            .map_err(|e| InstallationError::io("Failed to remove existing symlink", e)),
        Ok(meta) if meta.is_dir() => Err(InstallationError::TargetOccupied {
            entry_kind: "real directory",
            path: target_path.display().to_string(),
        }),
        Ok(_) => Err(InstallationError::TargetOccupied {
            entry_kind: "file",
            path: target_path.display().to_string(),
        }),
        Err(_) => Ok(()),
    }
}

pub(crate) async fn ensure_replaceable_target(target_path: &Path) -> Result<(), InstallationError> {
    let target_path = target_path.to_path_buf();
    run_blocking_fs("install target inspection", move || {
        ensure_replaceable_target_sync(&target_path)
    })
    .await
}
