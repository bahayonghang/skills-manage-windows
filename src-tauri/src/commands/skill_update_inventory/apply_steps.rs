use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::commands::central_updates::{self, keep_remote_missing_central_skills_impl};
use crate::commands::central_updates_fs::normalize_repo_path;
use crate::db::{self, Agent, DbPool};
use crate::services::central_skills::{
    self, BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult,
};
use crate::services::installation::uninstall_skill_from_agent_with_row_impl;
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
        Err(error) => result.failures.push(SkillUpdateApplyFailure {
            step: "keep_missing".to_string(),
            identifier: keep_missing.join(","),
            error,
        }),
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
                result.failures.push(SkillUpdateApplyFailure {
                    step: "delete_missing".to_string(),
                    identifier: failure.skill_id,
                    error: failure.error,
                });
            }
        }
        Err(error) => result.failures.push(SkillUpdateApplyFailure {
            step: "delete_missing".to_string(),
            identifier: String::new(),
            error,
        }),
    }
}

/// 步骤3a：skip_additions 解耦版。
pub(crate) async fn apply_skip_addition_step(
    pool: &DbPool,
    skip_additions: Vec<central_updates::CentralRepositoryAdditionSkipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in skip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "skip_addition".to_string(),
                    identifier: format!("{}::{}", request.repository_id, request.source_path),
                    error,
                });
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
            Err(error) => result.failures.push(SkillUpdateApplyFailure {
                step: "skip_addition".to_string(),
                identifier: format!("{}::{}", request.repository_id, source_path),
                error: error.to_string(),
            }),
        }
    }
}

/// 步骤3b：unskip_additions 解耦版。
pub(crate) async fn apply_unskip_addition_step(
    pool: &DbPool,
    unskip_additions: Vec<central_updates::CentralRepositoryAdditionUnskipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in unskip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "unskip_addition".to_string(),
                    identifier: format!("{}::{}", request.repository_id, request.source_path),
                    error,
                });
                continue;
            }
        };
        match db::delete_skill_repository_sync_skip(pool, &request.repository_id, &source_path)
            .await
        {
            Ok(_) => result
                .unskipped_additions
                .push(format!("{}::{}", request.repository_id, source_path)),
            Err(error) => result.failures.push(SkillUpdateApplyFailure {
                step: "unskip_addition".to_string(),
                identifier: format!("{}::{}", request.repository_id, source_path),
                error: error.to_string(),
            }),
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
            result.failures.push(SkillUpdateApplyFailure {
                step: "remove_platform_duplicate".to_string(),
                identifier: format!("{}::{}", removal.agent_id, removal.skill_id),
                error: format!(
                    "Agent '{}' is outside the allowed platform scope.",
                    removal.agent_id
                ),
            });
            continue;
        }
        let observations = match db::get_agent_skill_observations(pool, &removal.agent_id).await {
            Ok(rows) => rows,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "remove_platform_duplicate".to_string(),
                    identifier: format!("{}::{}", removal.agent_id, removal.skill_id),
                    error: error.to_string(),
                });
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
            match uninstall_skill_from_agent_with_row_impl(
                pool,
                &removal.skill_id,
                &removal.agent_id,
                row_id.as_deref(),
            )
            .await
            {
                Ok(()) => result.removed_platform_duplicate_paths.push(path),
                Err(error) => result.failures.push(SkillUpdateApplyFailure {
                    step: "remove_platform_duplicate".to_string(),
                    identifier: format!("{}::{}::{}", removal.agent_id, removal.skill_id, path),
                    error: error.to_string(),
                }),
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
            result.failures.push(SkillUpdateApplyFailure {
                step: "remove_deleted_platform_copy".to_string(),
                identifier: format!("{}::{}", removal.agent_id, removal.skill_id),
                error: format!(
                    "Agent '{}' is outside the allowed platform scope.",
                    removal.agent_id
                ),
            });
            continue;
        }
        for path in &removal.paths {
            match remove_deleted_platform_copy(pool, active_target, &removal, path).await {
                Ok(()) => result
                    .removed_deleted_platform_copy_paths
                    .push(path.clone()),
                Err(error) => result.failures.push(SkillUpdateApplyFailure {
                    step: "remove_deleted_platform_copy".to_string(),
                    identifier: format!("{}::{}::{}", removal.agent_id, removal.skill_id, path),
                    error,
                }),
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
) -> Result<(), String> {
    ensure_central_still_missing(pool, &removal.skill_id).await?;
    let agent = db::get_agent_by_id(pool, &removal.agent_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Agent '{}' not found", removal.agent_id))?;
    if removal.agent_id == "central" {
        return Err("Central agent entries cannot be removed as platform copies.".to_string());
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

async fn ensure_central_still_missing(pool: &DbPool, skill_id: &str) -> Result<(), String> {
    if db::get_central_skills_by_ids(pool, &[skill_id.to_string()])
        .await
        .map_err(|e| e.to_string())?
        .is_empty()
    {
        Ok(())
    } else {
        Err(format!(
            "Central skill '{}' exists again; refusing to delete platform copy.",
            skill_id
        ))
    }
}

async fn remove_deleted_platform_copy_local(
    pool: &DbPool,
    agent: &Agent,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), String> {
    let root = Path::new(&agent.global_skills_dir);
    let target = Path::new(path);
    ensure_local_child_path(root, target, &removal.agent_id)?;

    if removal.agent_id == "claude-code" {
        let observations = db::get_agent_skill_observations(pool, &removal.agent_id)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(obs) = observations.iter().find(|obs| {
            obs.skill_id == removal.skill_id && paths_equivalent_str(&obs.dir_path, path)
        }) {
            if obs.is_read_only || obs.source_kind == "plugin" {
                return Err("Read-only plugin copies cannot be removed.".to_string());
            }
            uninstall_skill_from_agent_with_row_impl(
                pool,
                &removal.skill_id,
                &removal.agent_id,
                Some(&obs.row_id),
            )
            .await
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    let expected = root.join(&removal.skill_id);
    if !paths_equivalent_path(&expected, target) {
        return Err(format!(
            "Refusing to delete '{}' because it is not the managed install path for '{}' on '{}'.",
            path, removal.skill_id, removal.agent_id
        ));
    }

    uninstall_skill_from_agent_with_row_impl(pool, &removal.skill_id, &removal.agent_id, None)
        .await
        .map_err(|e| e.to_string())
}

async fn remove_deleted_platform_copy_remote(
    pool: &DbPool,
    active_target: &ActiveTarget,
    agent: &Agent,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), String> {
    if path != crate::targets::remote_join(&agent.global_skills_dir, &removal.skill_id) {
        return Err(format!(
            "Refusing to delete '{}' because it is not the managed remote install path for '{}' on '{}'.",
            path, removal.skill_id, removal.agent_id
        ));
    }
    let path = ensure_remote_child_path(&agent.global_skills_dir, path, &removal.agent_id)?;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| e.to_string())?;
    match connection.remove_tree(&path).await {
        Ok(()) => {
            db::delete_skill_installation(pool, &removal.skill_id, &removal.agent_id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(error) if error.to_string().to_ascii_lowercase().contains("no such file") => {
            db::delete_skill_installation(pool, &removal.skill_id, &removal.agent_id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn ensure_local_child_path(root: &Path, child: &Path, label: &str) -> Result<(), String> {
    if crate::paths::paths_equivalent(root, child) {
        return Err(format!(
            "Refusing to delete the platform skills root for {}",
            label
        ));
    }

    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_parent = child
        .parent()
        .ok_or_else(|| format!("Path '{}' has no parent", child.display()))?;
    let child_parent_cmp = child_parent
        .canonicalize()
        .unwrap_or_else(|_| child_parent.to_path_buf());
    if !child_parent_cmp.starts_with(&root_cmp) {
        return Err(format!(
            "Refusing to delete '{}' because it is outside platform skills root '{}'",
            child.display(),
            root.display()
        ));
    }
    Ok(())
}

fn ensure_remote_child_path(root: &str, child: &str, label: &str) -> Result<String, String> {
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

fn paths_equivalent_str(left: &str, right: &str) -> bool {
    paths_equivalent_path(Path::new(left), Path::new(right))
}

fn paths_equivalent_path(left: &Path, right: &Path) -> bool {
    crate::paths::paths_equivalent(left, right)
}
