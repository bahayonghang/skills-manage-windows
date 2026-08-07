use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::db::{self, Agent, DbPool};
use crate::services::central_skills::{
    self, BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult,
};
use crate::services::central_updates::{
    keep_remote_missing_central_skills_impl, normalize_repo_path,
    CentralRepositoryAdditionSkipRequest, CentralRepositoryAdditionUnskipRequest,
    CentralUpdatesError,
};
use crate::services::installation::{uninstall_skill, InstallTransport};
use crate::targets::{connect_remote_target, ActiveTarget};

use super::{
    DeletedPlatformCopyRemoval, PlatformDuplicateRemoval, SkillUpdateApplyFailure,
    SkillUpdateApplyResult,
};

/// 步骤1：keep_missing 解耦版。复用 `keep_remote_missing_central_skills_impl`，
/// 单元测试用空 app/state 也能跑通。
pub(crate) async fn apply_keep_missing_step(
    pool: &DbPool,
    keep_missing: &[String],
    result: &mut SkillUpdateApplyResult,
) {
    if keep_missing.is_empty() {
        return;
    }
    match keep_remote_missing_central_skills_impl(pool, keep_missing).await {
        Ok(kept) => result.kept_missing_skill_ids = kept,
        Err(error) => result.failures.push(SkillUpdateApplyFailure::new(
            "keep_missing",
            keep_missing.join(","),
            error.to_string(),
        )),
    }
}

/// 步骤2：delete_missing 解耦版。
pub(crate) async fn apply_delete_missing_step(
    pool: &DbPool,
    active_target: &ActiveTarget,
    delete_missing: &[BatchDeleteCentralSkillRequest],
    result: &mut SkillUpdateApplyResult,
) {
    if delete_missing.is_empty() {
        return;
    }
    let delete_outcome: Result<BatchDeleteCentralSkillResult, String> = match active_target {
        ActiveTarget::Local => {
            central_skills::delete_central_skills_impl(pool, delete_missing).await
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            central_skills::delete_central_skills_remote_impl(pool, active_target, delete_missing)
                .await
        }
    }
    .map_err(|e| e.to_string());
    match delete_outcome {
        Ok(batch) => {
            for ok in batch.succeeded {
                result.deleted_skill_ids.push(ok.skill_id);
            }
            for failure in batch.failed {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "delete_missing",
                    failure.skill_id,
                    failure.error,
                ));
            }
        }
        Err(error) => result.failures.push(SkillUpdateApplyFailure::new(
            "delete_missing",
            String::new(),
            error,
        )),
    }
}

/// 步骤3a：skip_additions 解耦版。
pub(crate) async fn apply_skip_addition_step(
    pool: &DbPool,
    skip_additions: Vec<CentralRepositoryAdditionSkipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in skip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "skip_addition",
                    format!("{}::{}", request.repository_id, request.source_path),
                    error.to_string(),
                ));
                continue;
            }
        };
        match db::upsert_skill_repository_sync_skip(
            pool,
            &request.repository_id,
            &source_path,
            &request.skill_id,
            &request.skill_name,
        )
        .await
        {
            Ok(saved) => {
                // pending_additions 已落 skip，删除 pending 行
                let _ = db::delete_pending_addition(pool, &saved.repository_id, &saved.source_path)
                    .await;
                result
                    .skipped_additions
                    .push(format!("{}::{}", saved.repository_id, saved.source_path));
            }
            Err(error) => result.failures.push(SkillUpdateApplyFailure::new(
                "skip_addition",
                format!("{}::{}", request.repository_id, source_path),
                error.to_string(),
            )),
        }
    }
}

/// 步骤3b：unskip_additions 解耦版。
pub(crate) async fn apply_unskip_addition_step(
    pool: &DbPool,
    unskip_additions: Vec<CentralRepositoryAdditionUnskipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in unskip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "unskip_addition",
                    format!("{}::{}", request.repository_id, request.source_path),
                    error.to_string(),
                ));
                continue;
            }
        };
        match db::delete_skill_repository_sync_skip(pool, &request.repository_id, &source_path)
            .await
        {
            Ok(_) => result
                .unskipped_additions
                .push(format!("{}::{}", request.repository_id, source_path)),
            Err(error) => result.failures.push(SkillUpdateApplyFailure::new(
                "unskip_addition",
                format!("{}::{}", request.repository_id, source_path),
                error.to_string(),
            )),
        }
    }
}

/// 步骤6：remove_platform_duplicates 解耦版。
pub(crate) async fn apply_remove_platform_duplicates_step(
    pool: &DbPool,
    removals: Vec<PlatformDuplicateRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
) {
    for removal in removals {
        if !is_agent_allowed(&removal.agent_id, allowed_agent_ids) {
            result.failures.push(SkillUpdateApplyFailure::new(
                "remove_platform_duplicate",
                format!("{}::{}", removal.agent_id, removal.skill_id),
                format!(
                    "Agent '{}' is outside the allowed platform scope.",
                    removal.agent_id
                ),
            ));
            continue;
        }
        let observations = match db::get_agent_skill_observations(pool, &removal.agent_id).await {
            Ok(rows) => rows,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "remove_platform_duplicate",
                    format!("{}::{}", removal.agent_id, removal.skill_id),
                    error.to_string(),
                ));
                continue;
            }
        };
        let obs_by_path = observations
            .into_iter()
            .filter(|o| o.skill_id == removal.skill_id)
            .map(|o| (o.dir_path.clone(), o))
            .collect::<HashMap<_, _>>();

        for path in removal.paths {
            let row_id = (removal.agent_id == "claude-code")
                .then(|| obs_by_path.get(&path).map(|o| o.row_id.clone()))
                .flatten();
            match uninstall_skill(
                pool,
                &InstallTransport::Local,
                &removal.skill_id,
                &removal.agent_id,
                row_id.as_deref(),
            )
            .await
            {
                Ok(()) => result.removed_platform_duplicate_paths.push(path),
                Err(error) => result.failures.push(SkillUpdateApplyFailure::new(
                    "remove_platform_duplicate",
                    format!("{}::{}::{}", removal.agent_id, removal.skill_id, path),
                    error.to_string(),
                )),
            }
        }
    }
}

/// 步骤7：remove_deleted_platform_copies 解耦版。
pub(crate) async fn apply_remove_deleted_platform_copies_step(
    pool: &DbPool,
    active_target: &ActiveTarget,
    removals: Vec<DeletedPlatformCopyRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
) {
    for removal in removals {
        if !is_agent_allowed(&removal.agent_id, allowed_agent_ids) {
            result.failures.push(SkillUpdateApplyFailure::new(
                "remove_deleted_platform_copy",
                format!("{}::{}", removal.agent_id, removal.skill_id),
                format!(
                    "Agent '{}' is outside the allowed platform scope.",
                    removal.agent_id
                ),
            ));
            continue;
        }
        for path in &removal.paths {
            match remove_deleted_platform_copy(pool, active_target, &removal, path).await {
                Ok(()) => result
                    .removed_deleted_platform_copy_paths
                    .push(path.clone()),
                Err(error) => result.failures.push(SkillUpdateApplyFailure::new(
                    "remove_deleted_platform_copy",
                    format!("{}::{}::{}", removal.agent_id, removal.skill_id, path),
                    error.to_string(),
                )),
            }
        }
    }
}

fn is_agent_allowed(agent_id: &str, allowed_agent_ids: Option<&HashSet<String>>) -> bool {
    allowed_agent_ids.is_none_or(|ids| ids.contains(agent_id))
}

async fn remove_deleted_platform_copy(
    pool: &DbPool,
    active_target: &ActiveTarget,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), CentralUpdatesError> {
    ensure_central_still_missing(pool, &removal.skill_id).await?;
    let agent = db::get_agent_by_id(pool, &removal.agent_id)
        .await?
        .ok_or_else(|| CentralUpdatesError::AgentNotFound(removal.agent_id.clone()))?;
    if removal.agent_id == "central" {
        return Err(CentralUpdatesError::CentralAgentPlatformCopy);
    }

    match active_target {
        ActiveTarget::Local => {
            remove_deleted_platform_copy_local(pool, &agent, removal, path).await
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            remove_deleted_platform_copy_remote(pool, active_target, &agent, removal, path).await
        }
    }
}

async fn ensure_central_still_missing(
    pool: &DbPool,
    skill_id: &str,
) -> Result<(), CentralUpdatesError> {
    if db::get_central_skills_by_ids(pool, &[skill_id.to_string()])
        .await?
        .is_empty()
    {
        Ok(())
    } else {
        Err(CentralUpdatesError::CentralSkillReappeared(
            skill_id.to_string(),
        ))
    }
}

async fn remove_deleted_platform_copy_local(
    pool: &DbPool,
    agent: &Agent,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), CentralUpdatesError> {
    let root = Path::new(&agent.global_skills_dir);
    let target = Path::new(path);
    ensure_local_child_path(root, target, &removal.agent_id)?;

    if removal.agent_id == "claude-code" {
        let observations = db::get_agent_skill_observations(pool, &removal.agent_id).await?;
        if let Some(obs) = observations.iter().find(|obs| {
            obs.skill_id == removal.skill_id && paths_equivalent_str(&obs.dir_path, path)
        }) {
            if obs.is_read_only || obs.source_kind == "plugin" {
                return Err(CentralUpdatesError::ReadOnlyPluginCopy);
            }
            uninstall_skill(
                pool,
                &InstallTransport::Local,
                &removal.skill_id,
                &removal.agent_id,
                Some(&obs.row_id),
            )
            .await?;
            return Ok(());
        }
    }

    let expected = root.join(&removal.skill_id);
    if !paths_equivalent_path(&expected, target) {
        return Err(CentralUpdatesError::NotManagedInstallPath {
            path: path.to_string(),
            skill_id: removal.skill_id.clone(),
            agent_id: removal.agent_id.clone(),
        });
    }

    uninstall_skill(
        pool,
        &InstallTransport::Local,
        &removal.skill_id,
        &removal.agent_id,
        None,
    )
    .await?;
    Ok(())
}

async fn remove_deleted_platform_copy_remote(
    pool: &DbPool,
    active_target: &ActiveTarget,
    agent: &Agent,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), CentralUpdatesError> {
    if path != crate::targets::remote_join(&agent.global_skills_dir, &removal.skill_id) {
        return Err(CentralUpdatesError::NotManagedRemoteInstallPath {
            path: path.to_string(),
            skill_id: removal.skill_id.clone(),
            agent_id: removal.agent_id.clone(),
        });
    }
    let path = ensure_remote_child_path(&agent.global_skills_dir, path, &removal.agent_id)?;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| CentralUpdatesError::Remote(e.to_string()))?;
    match connection.remove_tree(&path).await {
        Ok(()) => {
            db::delete_skill_installation(pool, &removal.skill_id, &removal.agent_id).await?;
            Ok(())
        }
        Err(error)
            if error
                .to_string()
                .to_ascii_lowercase()
                .contains("no such file") =>
        {
            db::delete_skill_installation(pool, &removal.skill_id, &removal.agent_id).await?;
            Ok(())
        }
        Err(error) => Err(CentralUpdatesError::Remote(error.to_string())),
    }
}

fn ensure_local_child_path(
    root: &Path,
    child: &Path,
    label: &str,
) -> Result<(), CentralUpdatesError> {
    if crate::paths::paths_equivalent(root, child) {
        return Err(CentralUpdatesError::PlatformRootDeletion(label.to_string()));
    }

    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_parent = child
        .parent()
        .ok_or_else(|| CentralUpdatesError::PathNoParent(child.display().to_string()))?;
    let child_parent_cmp = child_parent
        .canonicalize()
        .unwrap_or_else(|_| child_parent.to_path_buf());
    if !child_parent_cmp.starts_with(&root_cmp) {
        return Err(CentralUpdatesError::OutsidePlatformRoot {
            child: child.display().to_string(),
            root: root.display().to_string(),
        });
    }
    Ok(())
}

fn ensure_remote_child_path(
    root: &str,
    child: &str,
    label: &str,
) -> Result<String, CentralUpdatesError> {
    let root_cmp = normalize_remote_path(root)?;
    let child_cmp = normalize_remote_path(child)?;
    if root_cmp == "/" {
        return Err(CentralUpdatesError::RemoteRootDeletionScope(
            label.to_string(),
        ));
    }
    if root_cmp == child_cmp {
        return Err(CentralUpdatesError::RemoteRootDeletion {
            root: root_cmp,
            label: label.to_string(),
        });
    }
    let prefix = format!("{}/", root_cmp.trim_end_matches('/'));
    if !child_cmp.starts_with(&prefix) {
        return Err(CentralUpdatesError::OutsideRemoteRoot {
            child: child.to_string(),
            root: root.to_string(),
        });
    }
    Ok(child_cmp)
}

fn normalize_remote_path(path: &str) -> Result<String, CentralUpdatesError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') || trimmed.contains('\0') {
        return Err(CentralUpdatesError::InvalidRemotePath(path.to_string()));
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(CentralUpdatesError::RemotePathTraversal(path.to_string())),
            value => segments.push(value),
        }
    }
    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

fn paths_equivalent_str(left: &str, right: &str) -> bool {
    paths_equivalent_path(Path::new(left), Path::new(right))
}

fn paths_equivalent_path(left: &Path, right: &Path) -> bool {
    crate::paths::paths_equivalent(left, right)
}
