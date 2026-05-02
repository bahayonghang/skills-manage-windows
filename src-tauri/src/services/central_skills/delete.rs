use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool, SkillRepository, SkillRepositoryWithStats};
use crate::targets::{connect_ssh_target, ConnectedSshTarget, RemoteTargetConfig};

use super::common::{installation_details, shared_root_agent_ids, unique_agent_ids};
use super::types::{
    BatchDeleteCentralSkillPreviewResult, BatchDeleteCentralSkillRequest,
    BatchDeleteCentralSkillResult, BatchDeleteCentralSkillSuccess, DeleteCentralSkillPreview,
    DeleteCentralSkillResult, DeleteSkillRepositoryPreview, DeleteSkillRepositoryResult,
    FailedCentralSkillDelete,
};

async fn get_deletable_repository_with_skill_ids(
    pool: &DbPool,
    repository_id: &str,
) -> Result<(SkillRepository, Vec<String>), String> {
    let repository = db::get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| format!("Repository '{}' not found", repository_id))?;
    if repository.id == db::LOCAL_UNKNOWN_REPOSITORY_ID || repository.is_unknown {
        return Err("The system unknown-source repository cannot be deleted".to_string());
    }

    let skill_ids = db::get_central_skill_ids_by_repository(pool, &repository.id).await?;
    Ok((repository, skill_ids))
}

fn repository_with_stats(
    repository: SkillRepository,
    skill_count: usize,
) -> SkillRepositoryWithStats {
    SkillRepositoryWithStats {
        repository,
        skill_count: skill_count as i64,
        unknown_skill_count: 0,
    }
}

fn build_repository_delete_requests(
    repository_id: &str,
    skill_ids: &[String],
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<Vec<BatchDeleteCentralSkillRequest>, String> {
    let valid_skill_ids: HashSet<&str> = skill_ids.iter().map(String::as_str).collect();
    let mut remove_agents_by_skill: HashMap<String, Vec<String>> = HashMap::new();

    for request in requests {
        if !valid_skill_ids.contains(request.skill_id.as_str()) {
            return Err(format!(
                "Skill '{}' does not belong to repository '{}'",
                request.skill_id, repository_id
            ));
        }

        let entry = remove_agents_by_skill
            .entry(request.skill_id.clone())
            .or_default();
        for agent_id in &request.remove_agent_ids {
            if !entry.contains(agent_id) {
                entry.push(agent_id.clone());
            }
        }
    }

    Ok(skill_ids
        .iter()
        .map(|skill_id| BatchDeleteCentralSkillRequest {
            skill_id: skill_id.clone(),
            remove_agent_ids: remove_agents_by_skill
                .remove(skill_id)
                .map(unique_agent_ids)
                .unwrap_or_default(),
        })
        .collect())
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_dir(path).map_err(|e| format!("Failed to remove symlink: {}", e))
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| format!("Failed to remove symlink: {}", e))
}

fn skill_delete_dir(skill: &db::Skill) -> Result<PathBuf, String> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| format!("Skill '{}' has no canonical directory", skill.id))
}

fn ensure_child_path(root: &Path, child: &Path, label: &str) -> Result<(), String> {
    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_cmp = child.canonicalize().unwrap_or_else(|_| child.to_path_buf());

    if crate::paths::paths_equivalent(&root_cmp, &child_cmp) {
        return Err(format!(
            "Refusing to delete the Central Skills root for {}",
            label
        ));
    }

    if !child_cmp.starts_with(&root_cmp) {
        return Err(format!(
            "Refusing to delete '{}' because it is outside Central Skills root '{}'",
            child.display(),
            root.display()
        ));
    }

    Ok(())
}

fn remove_skill_dir(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path).map_err(|e| {
            format!(
                "Failed to remove skill directory '{}': {}",
                path.display(),
                e
            )
        }),
        Ok(_) => Err(format!(
            "Path '{}' is not a directory. Refusing to delete.",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect '{}': {}", path.display(), error)),
    }
}

fn remove_installation_path(installation: &db::SkillInstallation) -> Result<(), String> {
    let path = Path::new(&installation.installed_path);
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() && installation.link_type == "copy" => {
            std::fs::remove_dir_all(path).map_err(|e| {
                format!(
                    "Failed to remove copied skill directory '{}': {}",
                    path.display(),
                    e
                )
            })
        }
        Ok(metadata) if metadata.is_dir() => Err(format!(
            "Path '{}' is not a managed copy. Refusing to delete.",
            path.display()
        )),
        Ok(_) => Err(format!(
            "Path '{}' is not a directory or symlink. Refusing to delete.",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect '{}': {}", path.display(), error)),
    }
}

fn remote_skill_delete_dir(skill: &db::Skill) -> Result<String, String> {
    skill
        .canonical_path
        .as_deref()
        .map(str::to_string)
        .or_else(|| crate::targets::remote_parent(&skill.file_path))
        .ok_or_else(|| format!("Skill '{}' has no canonical directory", skill.id))
}

fn normalize_remote_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') || trimmed.contains('\0') {
        return Err(format!("Invalid remote path '{}'", path));
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(format!("Remote path '{}' contains traversal", path)),
            value => segments.push(value),
        }
    }

    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

pub(super) fn ensure_remote_child_path(
    root: &str,
    child: &str,
    label: &str,
) -> Result<String, String> {
    let root_cmp = normalize_remote_path(root)?;
    let child_cmp = normalize_remote_path(child)?;

    if root_cmp == "/" {
        return Err(format!(
            "Refusing to delete under remote root for {}",
            label
        ));
    }

    if root_cmp == child_cmp {
        return Err(format!(
            "Refusing to delete the remote root '{}' for {}",
            root_cmp, label
        ));
    }

    let prefix = format!("{}/", root_cmp.trim_end_matches('/'));
    if !child_cmp.starts_with(&prefix) {
        return Err(format!(
            "Refusing to delete '{}' because it is outside remote root '{}'",
            child, root
        ));
    }

    Ok(child_cmp)
}

fn remote_shared_root_agent_ids(agents: &[db::Agent], central_root: &str) -> Vec<String> {
    let Ok(central_root) = normalize_remote_path(central_root) else {
        return Vec::new();
    };

    agents
        .iter()
        .filter(|agent| agent.id != "central")
        .filter_map(|agent| {
            normalize_remote_path(&agent.global_skills_dir)
                .ok()
                .filter(|path| *path == central_root)
                .map(|_| agent.id.clone())
        })
        .collect()
}

async fn preview_delete_central_skill_ssh_impl(
    pool: &DbPool,
    skill_id: &str,
) -> Result<DeleteCentralSkillPreview, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_skill_dir = remote_skill_delete_dir(&skill)?;
    let central_path =
        ensure_remote_child_path(&central.global_skills_dir, &central_skill_dir, skill_id)?;

    let agents = db::get_all_agents(pool).await?;
    let shared_root_agents = remote_shared_root_agent_ids(&agents, &central.global_skills_dir);
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let copy_installations = installation_details(
        installations
            .iter()
            .filter(|installation| installation.link_type == "copy")
            .cloned()
            .collect(),
    );
    let auto_removed_agent_ids = unique_agent_ids(
        installations
            .iter()
            .filter(|installation| installation.agent_id != "central")
            .filter(|installation| installation.link_type != "copy")
            .map(|installation| installation.agent_id.clone())
            .chain(shared_root_agents),
    );

    Ok(DeleteCentralSkillPreview {
        skill_id: skill.id,
        skill_name: skill.name,
        central_path,
        copy_installations,
        auto_removed_agent_ids,
    })
}

pub async fn preview_delete_central_skills_ssh_impl(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<BatchDeleteCentralSkillPreviewResult, String> {
    let mut previews = Vec::new();
    let mut failed = Vec::new();
    let mut seen = HashSet::new();

    for skill_id in skill_ids {
        if !seen.insert(skill_id.clone()) {
            continue;
        }

        match preview_delete_central_skill_ssh_impl(pool, skill_id).await {
            Ok(preview) => previews.push(preview),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: skill_id.clone(),
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillPreviewResult { previews, failed })
}

async fn remove_remote_installation_path(
    connection: &ConnectedSshTarget,
    installation: &db::SkillInstallation,
    agents_by_id: &HashMap<String, db::Agent>,
) -> Result<(), String> {
    let agent = agents_by_id
        .get(&installation.agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", installation.agent_id))?;
    let path = ensure_remote_child_path(
        &agent.global_skills_dir,
        &installation.installed_path,
        &installation.agent_id,
    )?;
    connection.remove_tree(&path).await
}

pub async fn delete_central_skill_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_skill_dir = remote_skill_delete_dir(&skill)?;
    let central_path =
        ensure_remote_child_path(&central.global_skills_dir, &central_skill_dir, skill_id)?;

    let remove_agent_set: HashSet<String> = remove_agent_ids.iter().cloned().collect();
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for agent_id in &remove_agent_set {
        let installation = installations
            .iter()
            .find(|item| item.agent_id == *agent_id)
            .ok_or_else(|| format!("Skill '{}' is not installed for '{}'", skill_id, agent_id))?;
        if installation.link_type != "copy" {
            return Err(format!(
                "Only copy installations can be selected for platform deletion: {}",
                agent_id
            ));
        }
    }

    let agents = db::get_all_agents(pool).await?;
    let agents_by_id: HashMap<String, db::Agent> = agents
        .into_iter()
        .map(|agent| (agent.id.clone(), agent))
        .collect();
    let connection = connect_ssh_target(target).await?;

    let mut removed_agent_ids = Vec::new();
    let mut retained_agent_ids = Vec::new();
    for installation in &installations {
        match installation.link_type.as_str() {
            "copy" if remove_agent_set.contains(&installation.agent_id) => {
                remove_remote_installation_path(&connection, installation, &agents_by_id).await?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "copy" => retained_agent_ids.push(installation.agent_id.clone()),
            "symlink" => {
                remove_remote_installation_path(&connection, installation, &agents_by_id).await?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "native" => {
                removed_agent_ids.push(installation.agent_id.clone());
            }
            _ => {
                retained_agent_ids.push(installation.agent_id.clone());
            }
        }
    }

    connection.remove_tree(&central_path).await?;
    db::delete_skill(pool, skill_id).await?;

    Ok(DeleteCentralSkillResult {
        removed_central_path: central_path,
        removed_agent_ids,
        retained_agent_ids,
    })
}

pub async fn delete_central_skills_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, String> {
    let mut ordered_requests: Vec<BatchDeleteCentralSkillRequest> = Vec::new();
    for request in requests {
        if let Some(existing) = ordered_requests
            .iter_mut()
            .find(|existing| existing.skill_id == request.skill_id)
        {
            for agent_id in &request.remove_agent_ids {
                if !existing.remove_agent_ids.contains(agent_id) {
                    existing.remove_agent_ids.push(agent_id.clone());
                }
            }
        } else {
            ordered_requests.push(BatchDeleteCentralSkillRequest {
                skill_id: request.skill_id.clone(),
                remove_agent_ids: unique_agent_ids(request.remove_agent_ids.clone()),
            });
        }
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for request in ordered_requests {
        match delete_central_skill_ssh_impl(
            pool,
            target,
            &request.skill_id,
            &request.remove_agent_ids,
        )
        .await
        {
            Ok(result) => succeeded.push(BatchDeleteCentralSkillSuccess {
                skill_id: request.skill_id,
                removed_central_path: result.removed_central_path,
                removed_agent_ids: result.removed_agent_ids,
                retained_agent_ids: result.retained_agent_ids,
            }),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: request.skill_id,
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillResult { succeeded, failed })
}

pub async fn preview_delete_central_skill_impl(
    pool: &DbPool,
    skill_id: &str,
) -> Result<DeleteCentralSkillPreview, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_root = PathBuf::from(&central.global_skills_dir);
    let central_skill_dir = skill_delete_dir(&skill)?;
    ensure_child_path(&central_root, &central_skill_dir, skill_id)?;

    let agents = db::get_all_agents(pool).await?;
    let shared_root_agents = shared_root_agent_ids(&agents);
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let copy_installations = installation_details(
        installations
            .iter()
            .filter(|installation| installation.link_type == "copy")
            .cloned()
            .collect(),
    );
    let auto_removed_agent_ids = unique_agent_ids(
        installations
            .iter()
            .filter(|installation| installation.agent_id != "central")
            .filter(|installation| installation.link_type != "copy")
            .map(|installation| installation.agent_id.clone())
            .chain(shared_root_agents),
    );

    Ok(DeleteCentralSkillPreview {
        skill_id: skill.id,
        skill_name: skill.name,
        central_path: central_skill_dir.to_string_lossy().into_owned(),
        copy_installations,
        auto_removed_agent_ids,
    })
}

pub async fn preview_delete_central_skills_impl(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<BatchDeleteCentralSkillPreviewResult, String> {
    let mut previews = Vec::new();
    let mut failed = Vec::new();
    let mut seen = HashSet::new();

    for skill_id in skill_ids {
        if !seen.insert(skill_id.clone()) {
            continue;
        }

        match preview_delete_central_skill_impl(pool, skill_id).await {
            Ok(preview) => previews.push(preview),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: skill_id.clone(),
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillPreviewResult { previews, failed })
}

pub async fn delete_central_skill_impl(
    pool: &DbPool,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_root = PathBuf::from(&central.global_skills_dir);
    let central_skill_dir = skill_delete_dir(&skill)?;
    ensure_child_path(&central_root, &central_skill_dir, skill_id)?;

    let remove_agent_set: HashSet<String> = remove_agent_ids.iter().cloned().collect();
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for agent_id in &remove_agent_set {
        let installation = installations
            .iter()
            .find(|item| item.agent_id == *agent_id)
            .ok_or_else(|| format!("Skill '{}' is not installed for '{}'", skill_id, agent_id))?;
        if installation.link_type != "copy" {
            return Err(format!(
                "Only copy installations can be selected for platform deletion: {}",
                agent_id
            ));
        }
    }

    let mut removed_agent_ids = Vec::new();
    let mut retained_agent_ids = Vec::new();
    for installation in &installations {
        match installation.link_type.as_str() {
            "copy" if remove_agent_set.contains(&installation.agent_id) => {
                remove_installation_path(installation)?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "copy" => retained_agent_ids.push(installation.agent_id.clone()),
            "symlink" => {
                remove_installation_path(installation)?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "native" => {
                removed_agent_ids.push(installation.agent_id.clone());
            }
            _ => {
                retained_agent_ids.push(installation.agent_id.clone());
            }
        }
    }

    remove_skill_dir(&central_skill_dir)?;
    db::delete_skill(pool, skill_id).await?;

    Ok(DeleteCentralSkillResult {
        removed_central_path: central_skill_dir.to_string_lossy().into_owned(),
        removed_agent_ids,
        retained_agent_ids,
    })
}

pub async fn delete_central_skills_impl(
    pool: &DbPool,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, String> {
    let mut ordered_requests: Vec<BatchDeleteCentralSkillRequest> = Vec::new();
    for request in requests {
        if let Some(existing) = ordered_requests
            .iter_mut()
            .find(|existing| existing.skill_id == request.skill_id)
        {
            for agent_id in &request.remove_agent_ids {
                if !existing.remove_agent_ids.contains(agent_id) {
                    existing.remove_agent_ids.push(agent_id.clone());
                }
            }
        } else {
            ordered_requests.push(BatchDeleteCentralSkillRequest {
                skill_id: request.skill_id.clone(),
                remove_agent_ids: unique_agent_ids(request.remove_agent_ids.clone()),
            });
        }
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for request in ordered_requests {
        match delete_central_skill_impl(pool, &request.skill_id, &request.remove_agent_ids).await {
            Ok(result) => succeeded.push(BatchDeleteCentralSkillSuccess {
                skill_id: request.skill_id,
                removed_central_path: result.removed_central_path,
                removed_agent_ids: result.removed_agent_ids,
                retained_agent_ids: result.retained_agent_ids,
            }),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: request.skill_id,
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillResult { succeeded, failed })
}

pub async fn preview_delete_skill_repository_impl(
    pool: &DbPool,
    repository_id: &str,
) -> Result<DeleteSkillRepositoryPreview, String> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_preview = preview_delete_central_skills_impl(pool, &skill_ids).await?;

    Ok(DeleteSkillRepositoryPreview {
        repository: repository_with_stats(repository, skill_ids.len()),
        delete_preview,
    })
}

pub async fn preview_delete_skill_repository_ssh_impl(
    pool: &DbPool,
    repository_id: &str,
) -> Result<DeleteSkillRepositoryPreview, String> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_preview = preview_delete_central_skills_ssh_impl(pool, &skill_ids).await?;

    Ok(DeleteSkillRepositoryPreview {
        repository: repository_with_stats(repository, skill_ids.len()),
        delete_preview,
    })
}

pub async fn delete_skill_repository_impl(
    pool: &DbPool,
    repository_id: &str,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<DeleteSkillRepositoryResult, String> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_requests = build_repository_delete_requests(&repository.id, &skill_ids, requests)?;
    let delete_result = delete_central_skills_impl(pool, &delete_requests).await?;
    let deleted_repository = if delete_result.failed.is_empty() {
        if skill_ids.is_empty() {
            db::delete_empty_skill_repository(pool, &repository.id).await?
        } else {
            db::prune_empty_skill_repositories(pool).await?;
            db::get_skill_repository_by_id(pool, &repository.id)
                .await?
                .is_none()
        }
    } else {
        false
    };

    Ok(DeleteSkillRepositoryResult {
        repository,
        deleted_repository,
        delete_result,
    })
}

pub async fn delete_skill_repository_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    repository_id: &str,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<DeleteSkillRepositoryResult, String> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_requests = build_repository_delete_requests(&repository.id, &skill_ids, requests)?;
    let delete_result = delete_central_skills_ssh_impl(pool, target, &delete_requests).await?;
    let deleted_repository = if delete_result.failed.is_empty() {
        if skill_ids.is_empty() {
            db::delete_empty_skill_repository(pool, &repository.id).await?
        } else {
            db::prune_empty_skill_repositories(pool).await?;
            db::get_skill_repository_by_id(pool, &repository.id)
                .await?
                .is_none()
        }
    } else {
        false
    };

    Ok(DeleteSkillRepositoryResult {
        repository,
        deleted_repository,
        delete_result,
    })
}
