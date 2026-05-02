//! Auto-centralize logic: ensure that a skill exists under the central
//! directory before installing it elsewhere, and shared helpers for
//! detecting/handling existing entries at install targets.

use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};

use super::fs_util::{copy_dir_all, remove_symlink_path};

/// Ensure the skill exists in the central directory. If it doesn't, copy it
/// from its actual location (looked up in the database) and update the DB
/// record to mark it as central.
///
/// This enables installing platform-specific skills to other platforms:
/// the skill is first adopted into the central directory, then distributed
/// via symlink/copy as usual.
pub(crate) async fn ensure_centralized(
    pool: &DbPool,
    skill_id: &str,
    canonical_dir: &Path,
) -> Result<(), String> {
    if canonical_dir.join("SKILL.md").exists() {
        return Ok(());
    }

    // Look up the skill's actual file location from the database.
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found in database", skill_id))?;

    // Derive the source directory (parent of file_path).
    let source_file = PathBuf::from(&skill.file_path);
    let source_dir = source_file
        .parent()
        .ok_or_else(|| format!("Invalid file_path for skill '{}'", skill_id))?;

    if !source_file.exists() {
        return Err(format!(
            "Skill source not found at '{}'",
            source_file.display()
        ));
    }

    // Copy to central directory.
    copy_dir_all(source_dir, canonical_dir)?;

    // Update the DB record to reflect centralization.
    let mut updated = skill;
    updated.canonical_path = Some(canonical_dir.to_string_lossy().into_owned());
    updated.is_central = true;
    updated.file_path = canonical_dir
        .join("SKILL.md")
        .to_string_lossy()
        .into_owned();
    db::upsert_skill(pool, &updated).await?;

    Ok(())
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
pub(crate) fn ensure_replaceable_target(target_path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(target_path) {
        Ok(meta) if meta.file_type().is_symlink() => remove_symlink_path(target_path),
        Ok(meta) if meta.is_dir() => Err(format!(
            "A real directory already exists at '{}'. Refusing to overwrite.",
            target_path.display()
        )),
        Ok(_) => Err(format!(
            "A file already exists at '{}'. Refusing to overwrite.",
            target_path.display()
        )),
        Err(_) => Ok(()),
    }
}
