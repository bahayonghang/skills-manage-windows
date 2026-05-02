//! Local install/uninstall orchestration: symlink, copy, auto fallback,
//! and same-root native records.

use std::path::{Path, PathBuf};

use crate::db::{self, DbPool, SkillInstallation};

use super::centralize::{agents_share_skills_dir, ensure_centralized};
use super::fs_util::{copy_dir_all, create_symlink, remove_symlink_path, symlink_target_path};
use super::types::InstallResult;

/// Record a native installation: the agent's `global_skills_dir` is the same
/// canonical root as Central, so no symlink/copy is needed — only a DB row.
pub(crate) async fn record_native_installation(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    canonical_dir: &Path,
) -> Result<InstallResult, String> {
    let skill_md = canonical_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err(format!(
            "Canonical skill not found at '{}'",
            skill_md.display()
        ));
    }

    let installed_path = canonical_dir.to_string_lossy().into_owned();
    let installation = SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: installed_path.clone(),
        link_type: "native".to_string(),
        symlink_target: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(pool, &installation).await?;

    Ok(InstallResult {
        symlink_path: installed_path,
    })
}

/// Core install logic, separated from the Tauri layer for testability.
///
/// Creates a relative symlink at `agent.global_skills_dir/<skill_id>` that
/// points to the canonical skill directory `central.global_skills_dir/<skill_id>`.
///
/// Returns an error if:
/// - The agent or central agent is not found in the database.
/// - The canonical skill does not exist (no SKILL.md).
/// - A real (non-symlink) directory already exists at the target path.
/// - `agent_id` is "central" (would create a self-referencing symlink).
pub async fn install_skill_to_agent_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
) -> Result<InstallResult, String> {
    // Guard: cannot install to the central agent itself.
    if agent_id == "central" {
        return Err("Cannot install a skill to the central agent itself".to_string());
    }

    // 1. Look up the target agent.
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    // 2. Look up the central agent to determine the canonical root.
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;

    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);

    // 3. Ensure the skill exists in central (auto-centralize if needed).
    ensure_centralized(pool, skill_id, &canonical_dir).await?;

    if agents_share_skills_dir(&agent, &central) {
        return record_native_installation(pool, skill_id, agent_id, &canonical_dir).await;
    }

    // 4. Compute symlink location.
    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    let symlink_path = agent_dir.join(skill_id);

    // 5. Ensure the agent's skills directory exists.
    std::fs::create_dir_all(&agent_dir)
        .map_err(|e| format!("Failed to create agent skills directory: {}", e))?;

    // 6. Handle any existing entry at the symlink path.
    match std::fs::symlink_metadata(&symlink_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            // Remove stale symlink so we can replace it.
            remove_symlink_path(&symlink_path)?;
        }
        Ok(meta) if meta.is_dir() => {
            return Err(format!(
                "A real directory already exists at '{}'. Refusing to overwrite.",
                symlink_path.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "A file already exists at '{}'. Refusing to overwrite.",
                symlink_path.display()
            ));
        }
        Err(_) => {} // Path does not exist — proceed normally.
    }

    // 7. Compute the relative path from the agent directory to the canonical dir.
    let relative_target = symlink_target_path(&agent_dir, &canonical_dir);

    // 8. Create the symlink.
    create_symlink(&relative_target, &symlink_path)?;

    // 9. Persist the installation record.
    let installation = SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: symlink_path.to_string_lossy().into_owned(),
        link_type: "symlink".to_string(),
        symlink_target: Some(canonical_dir.to_string_lossy().into_owned()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(pool, &installation).await?;

    Ok(InstallResult {
        symlink_path: symlink_path.to_string_lossy().into_owned(),
    })
}

/// Try the symlink path; on Windows fall back to copy when the symlink call
/// fails (typically due to missing privileges or non-NTFS targets).
pub async fn install_skill_to_agent_auto_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
) -> Result<InstallResult, String> {
    match install_skill_to_agent_impl(pool, skill_id, agent_id).await {
        Ok(result) => Ok(result),
        Err(error) if should_fallback_to_copy(&error) => {
            install_skill_to_agent_copy_impl(pool, skill_id, agent_id).await
        }
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
pub(crate) fn should_fallback_to_copy(error: &str) -> bool {
    error.contains("Failed to create symlink")
}

#[cfg(not(windows))]
pub(crate) fn should_fallback_to_copy(_error: &str) -> bool {
    false
}

/// Core copy-install logic — copies the skill directory instead of symlinking.
///
/// Copies `central.global_skills_dir/<skill_id>` recursively into
/// `agent.global_skills_dir/<skill_id>`. Existing symlinks at the target are
/// replaced; existing real directories cause an error.
pub async fn install_skill_to_agent_copy_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
) -> Result<InstallResult, String> {
    // Guard: cannot install to the central agent itself.
    if agent_id == "central" {
        return Err("Cannot install a skill to the central agent itself".to_string());
    }

    // 1. Look up the target agent.
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    // 2. Look up the central agent to determine the canonical root.
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;

    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);

    // 3. Ensure the skill exists in central (auto-centralize if needed).
    ensure_centralized(pool, skill_id, &canonical_dir).await?;

    if agents_share_skills_dir(&agent, &central) {
        return record_native_installation(pool, skill_id, agent_id, &canonical_dir).await;
    }

    // 4. Compute target location.
    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    let target_path = agent_dir.join(skill_id);

    // 5. Ensure the agent's skills directory exists.
    std::fs::create_dir_all(&agent_dir)
        .map_err(|e| format!("Failed to create agent skills directory: {}", e))?;

    // 6. Handle any existing entry at the target path.
    match std::fs::symlink_metadata(&target_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            // Remove stale symlink so we can replace it with a real copy.
            remove_symlink_path(&target_path)?;
        }
        Ok(meta) if meta.is_dir() => {
            return Err(format!(
                "A real directory already exists at '{}'. Refusing to overwrite.",
                target_path.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "A file already exists at '{}'. Refusing to overwrite.",
                target_path.display()
            ));
        }
        Err(_) => {} // Path does not exist — proceed normally.
    }

    // 7. Recursively copy the canonical skill directory.
    copy_dir_all(&canonical_dir, &target_path)?;

    // 8. Persist the installation record.
    let installation = SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: target_path.to_string_lossy().into_owned(),
        link_type: "copy".to_string(),
        symlink_target: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(pool, &installation).await?;

    Ok(InstallResult {
        symlink_path: target_path.to_string_lossy().into_owned(),
    })
}

/// Dispatch by method string. Used by single-agent and batch IPC paths.
pub(crate) async fn install_central_skill_to_agent_by_method(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    method: &str,
) -> Result<InstallResult, String> {
    match method {
        "copy" => install_skill_to_agent_copy_impl(pool, skill_id, agent_id).await,
        "symlink" => install_skill_to_agent_impl(pool, skill_id, agent_id).await,
        _ => install_skill_to_agent_auto_impl(pool, skill_id, agent_id).await,
    }
}

/// Core uninstall logic, separated from the Tauri layer for testability.
///
/// Removes the symlink at `agent.global_skills_dir/<skill_id>` and deletes the
/// corresponding `skill_installations` record.
///
/// For symlinked skills: removes the symlink.
/// For copied skills: removes the copied directory (tracked in the DB as link_type='copy').
/// Refuses to delete real directories not tracked as copies in the DB.
pub async fn uninstall_skill_from_agent_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    // 1. Look up the agent.
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;

    if agent_id == "central" || agents_share_skills_dir(&agent, &central) {
        return Err(format!(
            "{} shares the Central Skills directory and cannot be uninstalled independently.",
            agent.display_name
        ));
    }

    // 2. Compute the expected install location.
    let install_path = PathBuf::from(&agent.global_skills_dir).join(skill_id);

    // 3. Look up the installation record to determine how it was installed.
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let record = installations.iter().find(|r| r.agent_id == agent_id);
    let link_type = record.map(|r| r.link_type.as_str()).unwrap_or("symlink");

    // 4. Inspect the entry at that path and remove it appropriately.
    match std::fs::symlink_metadata(&install_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            // Always safe to remove symlinks.
            remove_symlink_path(&install_path)
                .map_err(|e| e.replace("existing symlink", "symlink"))?;
        }
        Ok(meta) if meta.is_dir() => {
            // Only remove real directories that were explicitly installed as copies.
            if link_type == "copy" {
                std::fs::remove_dir_all(&install_path)
                    .map_err(|e| format!("Failed to remove copied skill directory: {}", e))?;
            } else {
                return Err(format!(
                    "Path '{}' exists but is not a symlink. Refusing to delete.",
                    install_path.display()
                ));
            }
        }
        Ok(_) => {
            return Err(format!(
                "Path '{}' exists but is not a symlink. Refusing to delete.",
                install_path.display()
            ));
        }
        Err(_) => {
            // Path doesn't exist — still clean up the DB record.
        }
    }

    // 5. Remove the installation record from the database.
    db::delete_skill_installation(pool, skill_id, agent_id).await?;

    Ok(())
}
