//! Project-scoped (non-global) skill install:
//! places the skill under `<project_path>/<agent.project_skills_dir>/<skill_id>`
//! using either symlink or copy.

use std::path::{Path, PathBuf};

use crate::db::repos::agents_repo;
use crate::db::{self, DbPool};

use crate::targets::{remote_join, ConnectedRemoteTarget, RemotePathInfo};

use super::centralize::{ensure_centralized, ensure_replaceable_target};
use super::error::InstallationError;
use super::fs_util::{copy_dir_all_blocking, create_symlink, run_blocking_fs, symlink_target_path};
use super::native::should_fallback_to_copy;
use super::remote::ensure_remote_centralized;
use super::skip::detect_existing_project_install;
use super::transport::InstallTransport;
use super::types::{InstallOutcome, InstallResult};

/// Transport dispatcher for project-scoped installs. Unlike agent installs,
/// the two halves keep their own orchestration (dispatcher-only convergence
/// for the pilot); callers stop branching on the active target regardless.
pub(crate) async fn install_central_skill_to_project(
    pool: &DbPool,
    transport: &InstallTransport,
    skill_id: &str,
    agent_id: &str,
    project_path: &str,
    method: &str,
) -> Result<InstallOutcome, InstallationError> {
    match transport {
        InstallTransport::Local => {
            install_central_skill_to_project_outcome_impl(
                pool,
                skill_id,
                agent_id,
                Path::new(project_path),
                method,
            )
            .await
        }
        InstallTransport::Remote(connection) => {
            install_central_skill_to_remote_project_outcome_impl(
                pool,
                connection,
                skill_id,
                agent_id,
                project_path,
                method,
            )
            .await
        }
    }
}

pub(crate) fn project_relative_skills_dir(agent: &db::Agent) -> Result<PathBuf, InstallationError> {
    if agent.id == "central" {
        return Err(InstallationError::CentralAgentProjectTarget);
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
                return Err(InstallationError::ProjectSkillsDirAbsolute(
                    agent.display_name.clone(),
                ));
            }
            return Ok(path);
        }
    }

    let global_dir = crate::paths::expand_home_path(&agent.global_skills_dir);
    let home_dir = crate::paths::resolve_home_dir();
    let relative = global_dir.strip_prefix(&home_dir).map_err(|_| {
        InstallationError::ProjectSkillsDirNotHomeRelative(agent.display_name.clone())
    })?;

    if relative.as_os_str().is_empty() {
        return Err(InstallationError::ProjectSkillsDirUndefined(
            agent.display_name.clone(),
        ));
    }

    Ok(relative.to_path_buf())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteProjectInstallPaths {
    pub project_path: String,
    pub project_skills_dir: String,
    pub target_path: String,
}

pub(crate) fn remote_project_relative_skills_dir(
    agent: &db::Agent,
) -> Result<String, InstallationError> {
    let relative = project_relative_skills_dir(agent)?;
    let normalized = relative.to_string_lossy().replace('\\', "/");
    let normalized = normalized.trim_matches('/').to_string();
    if normalized.is_empty() || normalized == "." {
        return Err(InstallationError::ProjectSkillsDirUndefined(
            agent.display_name.clone(),
        ));
    }
    Ok(normalized)
}

pub(crate) fn normalize_remote_project_path(remote_home: &str, project_path: &str) -> String {
    let expanded =
        crate::paths::expand_remote_home_path(project_path, remote_home).replace('\\', "/");
    let trimmed = expanded.trim_end_matches('/');
    if trimmed.is_empty() && expanded.starts_with('/') {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn remote_project_install_paths(
    remote_home: &str,
    project_path: &str,
    agent: &db::Agent,
    skill_id: &str,
) -> Result<RemoteProjectInstallPaths, InstallationError> {
    let project_path = normalize_remote_project_path(remote_home, project_path);
    if !project_path.starts_with('/') {
        return Err(InstallationError::RemoteProjectPathNotAbsolute(
            project_path,
        ));
    }

    let relative_skills_dir = remote_project_relative_skills_dir(agent)?;
    let project_skills_dir = remote_join(&project_path, &relative_skills_dir);
    let target_path = remote_join(&project_skills_dir, skill_id);

    Ok(RemoteProjectInstallPaths {
        project_path,
        project_skills_dir,
        target_path,
    })
}

pub(crate) fn remote_project_method(
    method: &str,
    symlink_allowed: bool,
) -> Result<&'static str, InstallationError> {
    if method == "symlink" {
        if !symlink_allowed {
            return Err(InstallationError::RemoteSymlinkDisabled);
        }
        Ok("symlink")
    } else {
        Ok("copy")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoteProjectExistingTargetAction {
    UseEmptyPath,
    ReplaceSymlink,
    Reject(String),
}

pub(crate) fn classify_remote_project_existing_target(
    target_path: &str,
    method: &str,
    path_info: Option<&RemotePathInfo>,
) -> RemoteProjectExistingTargetAction {
    let Some(path_info) = path_info else {
        return RemoteProjectExistingTargetAction::UseEmptyPath;
    };

    if path_info.file_type == "symlink" {
        return RemoteProjectExistingTargetAction::ReplaceSymlink;
    }

    let entry_type = match path_info.file_type.as_str() {
        "dir" => "directory",
        "file" => "file",
        _ => "entry",
    };
    RemoteProjectExistingTargetAction::Reject(format!(
        "A remote project {} already exists at '{}'. Delete it before installing with {}.",
        entry_type, target_path, method
    ))
}

async fn ensure_remote_project_dir(
    connection: &ConnectedRemoteTarget,
    project_path: &str,
) -> Result<(), InstallationError> {
    match connection
        .inspect_path(project_path)
        .await
        .map_err(|e| InstallationError::Remote(e.to_string()))?
    {
        Some(info) if info.file_type == "dir" => Ok(()),
        Some(_) => Err(InstallationError::RemoteProjectPathNotDirectory(
            project_path.to_string(),
        )),
        None => Err(InstallationError::RemoteProjectPathMissing(
            project_path.to_string(),
        )),
    }
}

async fn ensure_remote_project_target_replaceable(
    connection: &ConnectedRemoteTarget,
    target_path: &str,
    method: &str,
) -> Result<(), InstallationError> {
    let info = connection
        .inspect_path(target_path)
        .await
        .map_err(|e| InstallationError::Remote(e.to_string()))?;
    match classify_remote_project_existing_target(target_path, method, info.as_ref()) {
        RemoteProjectExistingTargetAction::UseEmptyPath
        | RemoteProjectExistingTargetAction::ReplaceSymlink => Ok(()),
        RemoteProjectExistingTargetAction::Reject(error) => {
            Err(InstallationError::RemoteTargetOccupied(error))
        }
    }
}

const REMOTE_PROJECT_INSTALL_SCRIPT: &str = r#"
set -eu

project_path=$1
canonical_dir=$2
target_path=$3
project_skills_dir=$4
method=$5

if [ ! -d "$project_path" ]; then
  printf 'Remote project path '\''%s'\'' is not a directory.\n' "$project_path" >&2
  exit 44
fi

if [ ! -e "$canonical_dir/SKILL.md" ]; then
  printf 'Central skill source not found at %s\n' "$canonical_dir/SKILL.md" >&2
  exit 42
fi

mkdir -p "$project_skills_dir"

if [ -L "$target_path" ]; then
  rm -f -- "$target_path"
elif [ -e "$target_path" ]; then
  if [ -d "$target_path" ]; then
    entry_type=directory
  elif [ -f "$target_path" ]; then
    entry_type=file
  else
    entry_type=entry
  fi
  printf 'A remote project %s already exists at '\''%s'\''. Delete it before installing with %s.\n' "$entry_type" "$target_path" "$method" >&2
  exit 43
fi

if [ "$method" = "symlink" ]; then
  ln -s "$canonical_dir" "$target_path"
else
  mkdir -p "$target_path"
  cp -R "$canonical_dir/." "$target_path/"
fi
"#;

pub(crate) async fn install_central_skill_to_remote_project_outcome_impl(
    pool: &DbPool,
    connection: &ConnectedRemoteTarget,
    skill_id: &str,
    agent_id: &str,
    project_path: &str,
    method: &str,
) -> Result<InstallOutcome, InstallationError> {
    let agent = agents_repo::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| InstallationError::AgentNotFound(agent_id.to_string()))?;
    let central = agents_repo::get_agent_by_id(pool, "central")
        .await?
        .ok_or(InstallationError::CentralAgentMissing)?;
    let canonical_dir = remote_join(&central.global_skills_dir, skill_id);

    let method = remote_project_method(method, connection.symlink_allowed())?;
    let paths =
        remote_project_install_paths(connection.remote_home(), project_path, &agent, skill_id)?;

    ensure_remote_project_dir(connection, &paths.project_path).await?;
    ensure_remote_centralized(connection, pool, skill_id, &canonical_dir).await?;
    ensure_remote_project_target_replaceable(connection, &paths.target_path, method).await?;

    connection
        .run_script(
            REMOTE_PROJECT_INSTALL_SCRIPT,
            &[
                &paths.project_path,
                &canonical_dir,
                &paths.target_path,
                &paths.project_skills_dir,
                method,
            ],
        )
        .await
        .map_err(|e| InstallationError::Remote(e.to_string()))
        .map(|_| {
            InstallOutcome::Installed(InstallResult {
                symlink_path: paths.target_path,
            })
        })
}

fn ensure_project_dir_sync(project_path: &Path) -> Result<(), InstallationError> {
    if !project_path.exists() {
        return Err(InstallationError::ProjectPathMissing(
            project_path.display().to_string(),
        ));
    }
    if !project_path.is_dir() {
        return Err(InstallationError::ProjectPathNotDirectory(
            project_path.display().to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn ensure_project_dir(project_path: &Path) -> Result<(), InstallationError> {
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
) -> Result<InstallOutcome, InstallationError> {
    ensure_project_dir(project_path).await?;

    let agent = agents_repo::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| InstallationError::AgentNotFound(agent_id.to_string()))?;
    let central = agents_repo::get_agent_by_id(pool, "central")
        .await?
        .ok_or(InstallationError::CentralAgentMissing)?;
    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);

    ensure_centralized(pool, skill_id, &canonical_dir).await?;

    let relative_skills_dir = project_relative_skills_dir(&agent)?;
    let project_skills_dir = project_path.join(relative_skills_dir);
    let target_path = project_skills_dir.join(skill_id);

    let project_skills_dir_for_create = project_skills_dir.clone();
    run_blocking_fs("project skills directory creation", move || {
        std::fs::create_dir_all(&project_skills_dir_for_create).map_err(|e| {
            InstallationError::io(
                format!(
                    "Failed to create project skills directory '{}'",
                    project_skills_dir_for_create.display()
                ),
                e,
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
