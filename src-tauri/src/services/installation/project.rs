//! Project-scoped (non-global) skill install:
//! places the skill under `<project_path>/<agent.project_skills_dir>/<skill_id>`
//! using either symlink or copy.

use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};

use super::centralize::{ensure_centralized, ensure_replaceable_target};
use super::fs_util::{copy_dir_all_blocking, create_symlink, run_blocking_fs, symlink_target_path};
use super::native::should_fallback_to_copy;
use super::skip::detect_existing_project_install;
use super::types::{InstallOutcome, InstallResult};

pub(crate) fn project_relative_skills_dir(agent: &db::Agent) -> Result<PathBuf, String> {
    if agent.id == "central" {
        return Err("Cannot install a project skill to the central agent itself".to_string());
    }

    if db::is_universal_project_agent(&agent.id) {
        return Ok(PathBuf::from(db::UNIVERSAL_PROJECT_SKILLS_DIR));
    }

    if let Some(project_skills_dir) = &agent.project_skills_dir {
        let trimmed = project_skills_dir.trim();
        if !trimmed.is_empty() {
            let relative = trimmed
                .strip_prefix("~/")
                .or_else(|| trimmed.strip_prefix("~\\"))
                .unwrap_or(trimmed);
            let path = PathBuf::from(relative);
            if path.is_absolute() {
                return Err(format!(
                    "Agent '{}' uses an absolute project skills directory pattern.",
                    agent.display_name
                ));
            }
            return Ok(path);
        }
    }

    let global_dir = crate::paths::expand_home_path(&agent.global_skills_dir);
    let home_dir = crate::paths::resolve_home_dir();
    let relative = global_dir.strip_prefix(&home_dir).map_err(|_| {
        format!(
            "Agent '{}' does not define a home-relative skills directory pattern.",
            agent.display_name
        )
    })?;

    if relative.as_os_str().is_empty() {
        return Err(format!(
            "Agent '{}' does not define a project skills directory pattern.",
            agent.display_name
        ));
    }

    Ok(relative.to_path_buf())
}

fn ensure_project_dir_sync(project_path: &Path) -> Result<(), String> {
    if !project_path.exists() {
        return Err(format!(
            "Project path '{}' does not exist.",
            project_path.display()
        ));
    }
    if !project_path.is_dir() {
        return Err(format!(
            "Project path '{}' is not a directory.",
            project_path.display()
        ));
    }
    Ok(())
}

pub(crate) async fn ensure_project_dir(project_path: &Path) -> Result<(), String> {
    let project_path = project_path.to_path_buf();
    run_blocking_fs("project directory inspection", move || {
        ensure_project_dir_sync(&project_path)
    })
    .await
}

pub(crate) async fn install_central_skill_to_project_outcome_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    project_path: &Path,
    method: &str,
) -> Result<InstallOutcome, String> {
    ensure_project_dir(project_path).await?;

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);

    ensure_centralized(pool, skill_id, &canonical_dir).await?;

    let relative_skills_dir = project_relative_skills_dir(&agent)?;
    let project_skills_dir = project_path.join(relative_skills_dir);
    let target_path = project_skills_dir.join(skill_id);

    let project_skills_dir_for_create = project_skills_dir.clone();
    run_blocking_fs("project skills directory creation", move || {
        std::fs::create_dir_all(&project_skills_dir_for_create).map_err(|e| {
            format!(
                "Failed to create project skills directory '{}': {}",
                project_skills_dir_for_create.display(),
                e
            )
        })
    })
    .await?;

    if let Some(skipped) =
        detect_existing_project_install(skill_id, agent_id, &target_path, &canonical_dir).await?
    {
        return Ok(InstallOutcome::Skipped(skipped));
    }

    ensure_replaceable_target(&target_path).await?;

    if method == "copy" {
        copy_dir_all_blocking(&canonical_dir, &target_path).await?;
    } else {
        let relative_target = symlink_target_path(&project_skills_dir, &canonical_dir);
        let symlink_result = {
            let target_path_for_create = target_path.clone();
            run_blocking_fs("project skill symlink creation", move || {
                create_symlink(&relative_target, &target_path_for_create)
            })
            .await
        };
        match symlink_result {
            Ok(()) => {}
            Err(error) if method != "symlink" && should_fallback_to_copy(&error) => {
                copy_dir_all_blocking(&canonical_dir, &target_path).await?;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(InstallOutcome::Installed(InstallResult {
        symlink_path: target_path.to_string_lossy().into_owned(),
    }))
}
