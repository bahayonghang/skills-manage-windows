use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};
use crate::targets::{connect_remote_target, ActiveTarget, RemoteTargetConfig};

use super::common::{installation_details, shared_root_agent_ids, unique_agent_ids};
use super::error::CentralSkillsError;
use super::types::{
    BatchDeleteCentralSkillPreviewResult, BatchDeleteCentralSkillRequest,
    BatchDeleteCentralSkillResult, BatchDeleteCentralSkillSuccess, DeleteCentralSkillPreview,
    DeleteCentralSkillResult, FailedCentralSkillDelete,
};

mod repository;

pub use repository::{
    delete_skill_repository_impl, delete_skill_repository_remote_impl,
    delete_skill_repository_ssh_impl, preview_delete_skill_repository_impl,
    preview_delete_skill_repository_ssh_impl,
};

fn skill_delete_dir(skill: &db::Skill) -> Result<PathBuf, CentralSkillsError> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| CentralSkillsError::SkillNoCanonicalDir(skill.id.clone()))
}

fn ensure_child_path(root: &Path, child: &Path, label: &str) -> Result<(), CentralSkillsError> {
    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_cmp = child.canonicalize().unwrap_or_else(|_| child.to_path_buf());

    if crate::paths::paths_equivalent(&root_cmp, &child_cmp) {
        return Err(CentralSkillsError::CentralRootDeleteRefused(
            label.to_string(),
        ));
    }

    if !child_cmp.starts_with(&root_cmp) {
        return Err(CentralSkillsError::OutsideCentralRoot {
            path: child.display().to_string(),
            root: root.display().to_string(),
        });
    }

    Ok(())
}

fn validate_skill_dir_removal(path: &Path) -> Result<(), CentralSkillsError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_dir() => Ok(()),
        Ok(_) => Err(CentralSkillsError::NotADirectoryDeleteRefused(
            path.display().to_string(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CentralSkillsError::io(
            format!("Failed to inspect '{}'", path.display()),
            error,
        )),
    }
}

fn validate_installation_removal(
    installation: &db::SkillInstallation,
) -> Result<(), CentralSkillsError> {
    let path = Path::new(&installation.installed_path);
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(()),
        Ok(metadata) if metadata.is_dir() && installation.link_type == "copy" => Ok(()),
        Ok(metadata) if metadata.is_dir() => Err(CentralSkillsError::NotAManagedCopy(
            path.display().to_string(),
        )),
        Ok(_) => Err(CentralSkillsError::NotDirectoryOrSymlink(
            path.display().to_string(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CentralSkillsError::io(
            format!("Failed to inspect '{}'", path.display()),
            error,
        )),
    }
}

fn remote_skill_delete_dir(skill: &db::Skill) -> Result<String, CentralSkillsError> {
    skill
        .canonical_path
        .as_deref()
        .map(str::to_string)
        .or_else(|| crate::targets::remote_parent(&skill.file_path))
        .ok_or_else(|| CentralSkillsError::SkillNoCanonicalDir(skill.id.clone()))
}

fn normalize_remote_path(path: &str) -> Result<String, CentralSkillsError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') || trimmed.contains('\0') {
        return Err(CentralSkillsError::InvalidRemotePath(path.to_string()));
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(CentralSkillsError::RemotePathTraversal(path.to_string())),
            value => segments.push(value),
        }
    }

    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

async fn insert_delete_operation(
    pool: &DbPool,
    target: &ActiveTarget,
    skill_id: &str,
    batch_id: Option<&str>,
    operation_id: &str,
    manifest: &crate::services::central_operation::DeleteManifest,
) -> Result<(), CentralSkillsError> {
    let manifest_json = serde_json::to_string(
        &crate::services::central_operation::OperationManifest::Delete(manifest.clone()),
    )
    .map_err(|error| {
        crate::services::central_operation::CentralOperationError::InvalidManifest(
            error.to_string(),
        )
    })?;
    let target_kind = match target {
        ActiveTarget::Local => "local",
        ActiveTarget::Ssh(_) => "ssh",
        ActiveTarget::Wsl(_) => "wsl",
    };
    let old_fingerprint = manifest
        .paths
        .iter()
        .find_map(|path| path.fingerprint.as_deref());
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: operation_id,
            batch_id,
            target_id: target.id(),
            target_kind,
            operation_kind: crate::services::central_operation::OperationKind::CentralDelete
                .as_str(),
            skill_id,
            manifest_version: crate::services::central_operation::MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint,
            new_fingerprint: None,
        },
    )
    .await?;
    Ok(())
}

async fn record_recovery_error(
    pool: &DbPool,
    operation_id: &str,
    error: &crate::services::central_operation::CentralOperationError,
) -> Result<(), CentralSkillsError> {
    db::record_fs_db_operation_error(pool, operation_id, error.code(), &error.redacted_message())
        .await?;
    Ok(())
}

pub(super) fn ensure_remote_child_path(
    root: &str,
    child: &str,
    label: &str,
) -> Result<String, CentralSkillsError> {
    let root_cmp = normalize_remote_path(root)?;
    let child_cmp = normalize_remote_path(child)?;

    if root_cmp == "/" {
        return Err(CentralSkillsError::RemoteRootDeleteRefused(
            label.to_string(),
        ));
    }

    if root_cmp == child_cmp {
        return Err(CentralSkillsError::RemoteRootDeletion {
            root: root_cmp,
            label: label.to_string(),
        });
    }

    let prefix = format!("{}/", root_cmp.trim_end_matches('/'));
    if !child_cmp.starts_with(&prefix) {
        return Err(CentralSkillsError::OutsideRemoteRoot {
            path: child.to_string(),
            root: root.to_string(),
        });
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
) -> Result<DeleteCentralSkillPreview, CentralSkillsError> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| CentralSkillsError::SkillNotFound(skill_id.to_string()))?;
    if !skill.is_central {
        return Err(CentralSkillsError::NotCentralSkill(skill_id.to_string()));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(CentralSkillsError::CentralAgentMissing)?;
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
) -> Result<BatchDeleteCentralSkillPreviewResult, CentralSkillsError> {
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
                error: error.to_string(),
            }),
        }
    }

    Ok(BatchDeleteCentralSkillPreviewResult { previews, failed })
}

pub async fn delete_central_skill_remote_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, CentralSkillsError> {
    delete_central_skill_remote_with_batch(pool, active_target, skill_id, remove_agent_ids, None)
        .await
}

async fn delete_central_skill_remote_with_batch(
    pool: &DbPool,
    active_target: &ActiveTarget,
    skill_id: &str,
    remove_agent_ids: &[String],
    batch_id: Option<&str>,
) -> Result<DeleteCentralSkillResult, CentralSkillsError> {
    let _mutation_guard = crate::services::central_mutation::acquire_target_mutation_guard(
        active_target,
        "delete Central skill",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    crate::services::central_operation::recover_pending_operations_under_guard(pool, active_target)
        .await?;
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| CentralSkillsError::SkillNotFound(skill_id.to_string()))?;
    if !skill.is_central {
        return Err(CentralSkillsError::NotCentralSkill(skill_id.to_string()));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(CentralSkillsError::CentralAgentMissing)?;
    let central_skill_dir = remote_skill_delete_dir(&skill)?;
    let central_path =
        ensure_remote_child_path(&central.global_skills_dir, &central_skill_dir, skill_id)?;

    let remove_agent_set: HashSet<String> = remove_agent_ids.iter().cloned().collect();
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for agent_id in &remove_agent_set {
        let installation = installations
            .iter()
            .find(|item| item.agent_id == *agent_id)
            .ok_or_else(|| CentralSkillsError::SkillNotInstalledForAgent {
                skill_id: skill_id.to_string(),
                agent_id: agent_id.clone(),
            })?;
        if installation.link_type != "copy" {
            return Err(CentralSkillsError::OnlyCopyInstallationsDeletable(
                agent_id.clone(),
            ));
        }
    }

    let agents = db::get_all_agents(pool).await?;
    let agents_by_id: HashMap<String, db::Agent> = agents
        .into_iter()
        .map(|agent| (agent.id.clone(), agent))
        .collect();
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| CentralSkillsError::Remote(e.to_string()))?;

    let mut removed_agent_ids = Vec::new();
    let mut retained_agent_ids = Vec::new();
    let mut removable_paths = Vec::new();
    for installation in &installations {
        match installation.link_type.as_str() {
            "copy" if remove_agent_set.contains(&installation.agent_id) => {
                let agent = agents_by_id.get(&installation.agent_id).ok_or_else(|| {
                    CentralSkillsError::AgentNotFound(installation.agent_id.clone())
                })?;
                removable_paths.push(ensure_remote_child_path(
                    &agent.global_skills_dir,
                    &installation.installed_path,
                    &installation.agent_id,
                )?);
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "copy" => retained_agent_ids.push(installation.agent_id.clone()),
            "symlink" => {
                let agent = agents_by_id.get(&installation.agent_id).ok_or_else(|| {
                    CentralSkillsError::AgentNotFound(installation.agent_id.clone())
                })?;
                removable_paths.push(ensure_remote_child_path(
                    &agent.global_skills_dir,
                    &installation.installed_path,
                    &installation.agent_id,
                )?);
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
    removable_paths.push(central_path.clone());
    let operation_id = uuid::Uuid::new_v4().to_string();
    let manifest = crate::services::central_operation::build_remote_delete_manifest(
        &connection,
        &operation_id,
        removable_paths,
    )
    .await?;
    insert_delete_operation(
        pool,
        active_target,
        skill_id,
        batch_id,
        &operation_id,
        &manifest,
    )
    .await?;
    if let Err(error) =
        crate::services::central_operation::stage_delete_remote(&connection, &manifest).await
    {
        record_recovery_error(pool, &operation_id, &error).await?;
        return Err(error.into());
    }
    db::transition_fs_db_operation(pool, &operation_id, "prepared", "fs_staged").await?;
    db::commit_delete_fs_db_operation(pool, &operation_id, skill_id).await?;
    if let Err(error) =
        crate::services::central_operation::finalize_delete_remote(&connection, &manifest).await
    {
        record_recovery_error(pool, &operation_id, &error).await?;
        return Err(error.into());
    }
    db::transition_fs_db_operation(pool, &operation_id, "db_committed", "completed").await?;

    Ok(DeleteCentralSkillResult {
        removed_central_path: central_path,
        removed_agent_ids,
        retained_agent_ids,
    })
}

pub async fn delete_central_skill_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, CentralSkillsError> {
    let active_target = ActiveTarget::Ssh(Box::new(target.clone()));
    delete_central_skill_remote_impl(pool, &active_target, skill_id, remove_agent_ids).await
}

pub async fn delete_central_skills_remote_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
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
    let batch_id = uuid::Uuid::new_v4().to_string();
    for request in ordered_requests {
        match delete_central_skill_remote_with_batch(
            pool,
            active_target,
            &request.skill_id,
            &request.remove_agent_ids,
            Some(&batch_id),
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
                error: error.to_string(),
            }),
        }
    }

    Ok(BatchDeleteCentralSkillResult { succeeded, failed })
}

pub async fn delete_central_skills_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    let active_target = ActiveTarget::Ssh(Box::new(target.clone()));
    delete_central_skills_remote_impl(pool, &active_target, requests).await
}

pub async fn preview_delete_central_skill_impl(
    pool: &DbPool,
    skill_id: &str,
) -> Result<DeleteCentralSkillPreview, CentralSkillsError> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| CentralSkillsError::SkillNotFound(skill_id.to_string()))?;
    if !skill.is_central {
        return Err(CentralSkillsError::NotCentralSkill(skill_id.to_string()));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(CentralSkillsError::CentralAgentMissing)?;
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
) -> Result<BatchDeleteCentralSkillPreviewResult, CentralSkillsError> {
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
                error: error.to_string(),
            }),
        }
    }

    Ok(BatchDeleteCentralSkillPreviewResult { previews, failed })
}

pub async fn delete_central_skill_impl(
    pool: &DbPool,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, CentralSkillsError> {
    delete_central_skill_with_batch(pool, skill_id, remove_agent_ids, None).await
}

async fn delete_central_skill_with_batch(
    pool: &DbPool,
    skill_id: &str,
    remove_agent_ids: &[String],
    batch_id: Option<&str>,
) -> Result<DeleteCentralSkillResult, CentralSkillsError> {
    let active_target = ActiveTarget::Local;
    let _mutation_guard = crate::services::central_mutation::acquire_target_mutation_guard(
        &active_target,
        "delete Central skill",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    crate::services::central_operation::recover_pending_operations_under_guard(
        pool,
        &active_target,
    )
    .await?;
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| CentralSkillsError::SkillNotFound(skill_id.to_string()))?;
    if !skill.is_central {
        return Err(CentralSkillsError::NotCentralSkill(skill_id.to_string()));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(CentralSkillsError::CentralAgentMissing)?;
    let central_root = PathBuf::from(&central.global_skills_dir);
    let central_skill_dir = skill_delete_dir(&skill)?;
    ensure_child_path(&central_root, &central_skill_dir, skill_id)?;

    let remove_agent_set: HashSet<String> = remove_agent_ids.iter().cloned().collect();
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for agent_id in &remove_agent_set {
        let installation = installations
            .iter()
            .find(|item| item.agent_id == *agent_id)
            .ok_or_else(|| CentralSkillsError::SkillNotInstalledForAgent {
                skill_id: skill_id.to_string(),
                agent_id: agent_id.clone(),
            })?;
        if installation.link_type != "copy" {
            return Err(CentralSkillsError::OnlyCopyInstallationsDeletable(
                agent_id.clone(),
            ));
        }
    }

    let mut removed_agent_ids = Vec::new();
    let mut retained_agent_ids = Vec::new();
    let mut removable_paths = Vec::new();
    for installation in &installations {
        match installation.link_type.as_str() {
            "copy" if remove_agent_set.contains(&installation.agent_id) => {
                validate_installation_removal(installation)?;
                removable_paths.push(PathBuf::from(&installation.installed_path));
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "copy" => retained_agent_ids.push(installation.agent_id.clone()),
            "symlink" => {
                validate_installation_removal(installation)?;
                removable_paths.push(PathBuf::from(&installation.installed_path));
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "native" => removed_agent_ids.push(installation.agent_id.clone()),
            _ => retained_agent_ids.push(installation.agent_id.clone()),
        }
    }
    validate_skill_dir_removal(&central_skill_dir)?;
    removable_paths.push(central_skill_dir.clone());
    let operation_id = uuid::Uuid::new_v4().to_string();
    let manifest = crate::services::central_operation::build_local_delete_manifest(
        &operation_id,
        removable_paths,
    )
    .await?;
    insert_delete_operation(
        pool,
        &active_target,
        skill_id,
        batch_id,
        &operation_id,
        &manifest,
    )
    .await?;
    if let Err(error) = crate::services::central_operation::stage_delete_local(&manifest).await {
        record_recovery_error(pool, &operation_id, &error).await?;
        return Err(error.into());
    }
    db::transition_fs_db_operation(pool, &operation_id, "prepared", "fs_staged").await?;
    db::commit_delete_fs_db_operation(pool, &operation_id, skill_id).await?;
    if let Err(error) = crate::services::central_operation::finalize_delete_local(&manifest).await {
        record_recovery_error(pool, &operation_id, &error).await?;
        return Err(error.into());
    }
    db::transition_fs_db_operation(pool, &operation_id, "db_committed", "completed").await?;

    Ok(DeleteCentralSkillResult {
        removed_central_path: central_skill_dir.to_string_lossy().into_owned(),
        removed_agent_ids,
        retained_agent_ids,
    })
}

pub async fn delete_central_skills_impl(
    pool: &DbPool,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
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
    let batch_id = uuid::Uuid::new_v4().to_string();
    for request in ordered_requests {
        match delete_central_skill_with_batch(
            pool,
            &request.skill_id,
            &request.remove_agent_ids,
            Some(&batch_id),
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
                error: error.to_string(),
            }),
        }
    }

    Ok(BatchDeleteCentralSkillResult { succeeded, failed })
}
