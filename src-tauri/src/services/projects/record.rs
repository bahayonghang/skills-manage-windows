//! Record a Central project-path install into `projects` + psi.
//!
//! Installation owns the filesystem write. This helper only registers the
//! project row and upserts `project_skill_installations`. Local callers that
//! materialized a new target pass `fs_changed`; a later metadata failure
//! compensates that target. Remote callers must compensate on the remote
//! host themselves — this helper never uses local `Path::is_dir` for remote
//! paths.

use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::db::repos::projects_repo;
use crate::db::repos::skills_repo;
use crate::db::{DbPool, Project, ProjectSkillInstallation};

use super::crud::{
    add_project_impl, project_id_from_path, project_name_from_path, remove_project_skill_target,
    restore_project_skill_symlink,
};
use super::error::ProjectsError;

/// Whether the project path is a local directory or an already-normalized
/// remote POSIX path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRecordLocation {
    Local,
    Remote,
}

/// Inputs for [`record_project_skill_from_central_install`].
pub struct RecordProjectSkillFromCentralInstall<'a> {
    pub skill_id: &'a str,
    pub agent_id: &'a str,
    pub project_path: &'a str,
    pub location: ProjectRecordLocation,
    pub installed_path: &'a str,
    pub link_type: &'a str,
    pub symlink_target: Option<&'a str>,
    /// True when this call materialized a new target (Installed). False for
    /// skip-already-exists, which must not delete the user's existing files
    /// if the metadata write fails.
    pub fs_changed: bool,
    pub previous_symlink_target: Option<&'a str>,
}

/// Register the project (idempotent) and upsert a Central-origin psi row.
pub async fn record_project_skill_from_central_install(
    pool: &DbPool,
    params: RecordProjectSkillFromCentralInstall<'_>,
) -> Result<(), ProjectsError> {
    let result = persist_project_skill_row(pool, &params).await;
    if let Err(error) = result {
        if params.fs_changed && params.location == ProjectRecordLocation::Local {
            compensate_local_project_target(
                params.installed_path,
                params.link_type,
                params.previous_symlink_target,
            )
            .await?;
        }
        return Err(error);
    }
    Ok(())
}

async fn persist_project_skill_row(
    pool: &DbPool,
    params: &RecordProjectSkillFromCentralInstall<'_>,
) -> Result<(), ProjectsError> {
    let skill = skills_repo::get_skill_by_id(pool, params.skill_id)
        .await?
        .ok_or_else(|| ProjectsError::SkillNotFoundInCentral(params.skill_id.to_string()))?;

    let project = match params.location {
        ProjectRecordLocation::Local => add_project_impl(pool, params.project_path).await?,
        ProjectRecordLocation::Remote => {
            add_or_get_remote_project(pool, params.project_path).await?
        }
    };

    let installed_path = match params.location {
        ProjectRecordLocation::Local => crate::paths::normalize_stored_path(params.installed_path),
        ProjectRecordLocation::Remote => normalize_remote_stored_path(params.installed_path),
    };
    let file_path = match params.location {
        ProjectRecordLocation::Local => crate::paths::normalize_stored_path(
            &Path::new(params.installed_path)
                .join("SKILL.md")
                .to_string_lossy(),
        ),
        ProjectRecordLocation::Remote => {
            let installed = normalize_remote_stored_path(params.installed_path);
            if installed == "/" {
                "/SKILL.md".to_string()
            } else {
                format!("{installed}/SKILL.md")
            }
        }
    };

    let psi = ProjectSkillInstallation {
        project_id: project.id,
        skill_id: params.skill_id.to_string(),
        name: skill.name,
        description: skill.description,
        file_path,
        source_origin: "central".to_string(),
        agent_id: params.agent_id.to_string(),
        installed_path,
        link_type: params.link_type.to_string(),
        symlink_target: params.symlink_target.map(str::to_string),
        created_at: Utc::now().to_rfc3339(),
    };
    projects_repo::upsert_project_skill_installation(pool, &psi).await?;
    Ok(())
}

/// Insert or reuse a `projects` row for a normalized remote POSIX path.
/// Never checks `Path::is_dir` — the remote host owns existence.
async fn add_or_get_remote_project(
    pool: &DbPool,
    normalized_path: &str,
) -> Result<Project, ProjectsError> {
    let trimmed = normalized_path.trim();
    if trimmed.is_empty() {
        return Err(ProjectsError::ProjectPathEmpty);
    }

    if let Some(existing) = projects_repo::get_project_by_path(pool, trimmed).await? {
        return Ok(existing);
    }

    let project = Project {
        id: project_id_from_path(trimmed),
        path: trimmed.to_string(),
        name: project_name_from_path(trimmed),
        pinned: false,
        added_at: Utc::now().to_rfc3339(),
        last_scanned_at: None,
    };
    projects_repo::insert_project(pool, &project).await?;
    Ok(project)
}

fn normalize_remote_stored_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() && normalized.starts_with('/') {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

async fn compensate_local_project_target(
    installed_path: &str,
    link_type: &str,
    previous_symlink_target: Option<&str>,
) -> Result<(), ProjectsError> {
    let target_path = PathBuf::from(installed_path);
    remove_project_skill_target(target_path.clone(), link_type.to_string()).await?;
    if let Some(previous_target) = previous_symlink_target {
        restore_project_skill_symlink(target_path, PathBuf::from(previous_target)).await?;
    }
    Ok(())
}
