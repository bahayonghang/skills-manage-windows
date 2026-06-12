//! Skill Update Inventory —— Phase P2 of Update Mechanism Overhaul。
//!
//! 在旧的 `check_central_skill_updates` / `check_central_repository_sync` 之上
//! 提供面向"更新中心"统一面板的命令集合：
//!
//! - `refresh_skill_update_inventory`：拉远端并写入 update-inventory 表与
//!   remote-addition bucket，返回一份完整 inventory；不更新安装 baseline。
//! - `get_skill_update_inventory`：纯读视图，不触网。
//! - `clear_skill_update_inventory`：清空所选 scope 的 inventory bucket，
//!   不删除技能、不改 `skill_update_states` baseline。
//! - `apply_skill_update_decisions`：把用户在面板里勾的决策一次性应用，
//!   每步独立 partial success，复用 keep/delete/import/update/uninstall 既有 impl。
//! - `force_update_central_skills` / `force_mirror_central_repositories`：
//!   显式救援动作，绕过检测结果从远端覆盖/镜像同步。
//! - `scan_platform_duplicate_skills`：观察各 agent 是否同 skill_id 同时有
//!   writable 与 plugin readonly 两份，给前端去重弹窗用。
//!
//! 不重新实现业务逻辑，只组合既有 helper。旧命令仍并行存在以保证不破坏前端。

use std::collections::HashMap;

use chrono::Utc;
use tauri::{AppHandle, State};

use crate::commands::central_updates::{
    self, error_state_from_assignment, load_remote_skill_content, prepare_skill_updates,
    prepare_snapshots_for_repo_refs_with_policy, remote_missing_state_from_assignment,
    state_from_remote, unsupported_state_from_assignment, RemoteSkillLoadError, SkillUpdateStatus,
    SnapshotCachePolicy,
};
use crate::commands::central_updates_fs::{normalize_repo_path, CentralFs};
use crate::commands::github_import;
use crate::db::{self, DbPool, SkillRepositoryPendingAddition};
use crate::targets::ActiveTarget;
use crate::AppState;

mod apply_steps;
mod force;
mod persistence;
mod relocation;
mod repositories;
mod scan;
mod scope;
mod types;
mod view;

pub(crate) use apply_steps::*;
pub(crate) use force::*;
use persistence::*;
pub(crate) use relocation::*;
pub(crate) use repositories::*;
pub(crate) use scan::*;
pub(crate) use scope::*;
pub use types::*;
pub(crate) use view::*;

/*
 * ========================================================================
 * 命令实现
 * ========================================================================
 */

#[tauri::command]
pub async fn refresh_skill_update_inventory(
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
) -> Result<SkillUpdateInventory, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?).await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    refresh_skill_update_inventory_impl(
        &pool,
        &fs,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        scope,
    )
    .await
}

/// 内核版本：不依赖 `State<AppState>`，便于单元测试注入 pool / 预填 snapshot 缓存。
///
/// 当 `snapshots_cache` 已用 `cache.insert(repo_cache_key(&repo), snapshot)` 预填时，
/// 内部 `prepare_snapshots_for_repo_refs` 跳过网络下载直接复用缓存命中，所以测试
/// 可以完全离线运行。
pub(crate) async fn refresh_skill_update_inventory_impl(
    pool: &DbPool,
    fs: &CentralFs,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &crate::CentralUpdateSnapshotCache,
    scope: SkillRefreshScope,
) -> Result<SkillUpdateInventory, String> {
    /*
     * ========================================================================
     * 步骤1：根据 scope 解析目标 skills 和 repositories
     * ========================================================================
     * 数据源：
     * 1) All：全部 central skill_ids + 全部 syncable github repo_ids
     * 2) Skills：仅指定 skill_ids，不扫 repo additions
     * 3) Repositories：仅指定 repo_ids，skill_ids 从这些 repo members 推导
     * 4) Platform：从当前平台 agent_ids 已安装/已观测到的 central skills 推导
     */
    let mode = scope.mode.unwrap_or(SkillRefreshMode::Sync);
    let cache_policy = scope
        .cache_policy
        .unwrap_or(SkillRefreshCachePolicy::Bypass);
    let include_sync_buckets = mode == SkillRefreshMode::Sync;

    // 1.1 决定 repo_ids 与 skill_ids
    let (skill_ids_filter, repository_ids, platform_agent_ids): (
        Option<Vec<String>>,
        Vec<String>,
        Option<Vec<String>>,
    ) = match scope.kind {
        SkillRefreshScopeKind::All => {
            let repo_ids = db::get_skill_repositories_with_stats(pool)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|r| !r.repository.is_unknown && r.repository.source_type == "github")
                .map(|r| r.repository.id)
                .collect::<Vec<_>>();
            (None, repo_ids, None)
        }
        SkillRefreshScopeKind::Skills => {
            let ids = normalize_ids(scope.skill_ids.clone().unwrap_or_default());
            (Some(ids), Vec::new(), None)
        }
        SkillRefreshScopeKind::Repositories => {
            let repo_ids = normalize_ids(scope.repository_ids.clone().unwrap_or_default());
            let mut skill_ids = Vec::new();
            for repository_id in &repo_ids {
                let ids = db::get_central_skill_ids_by_repository(pool, repository_id)
                    .await
                    .map_err(|e| e.to_string())?;
                skill_ids.extend(ids);
            }
            (Some(normalize_ids(skill_ids)), repo_ids, None)
        }
        SkillRefreshScopeKind::Platform => {
            let agent_ids = normalize_ids(scope.agent_ids.clone().unwrap_or_default());
            let skill_ids = central_skill_ids_for_agents(pool, &agent_ids).await?;
            (Some(skill_ids), Vec::new(), Some(agent_ids))
        }
    };

    /*
     * ========================================================================
     * 步骤2：准备 skills + snapshots，拉远端 hash 对比
     * ========================================================================
     * 复用 prepare_skill_updates / prepare_snapshots_for_repo_refs。
     * refresh 会持久化每个已检查 skill 的最新状态（包括 up_to_date /
     * unsupported / error），这样旧的 update_available / remote_missing
     * 不会在后续 get_inventory 纯读视图中残留。
     */
    let skills = if let Some(ids) = &skill_ids_filter {
        if ids.is_empty() {
            Vec::new()
        } else {
            db::get_central_skills_by_ids(pool, ids)
                .await
                .map_err(|e| e.to_string())?
        }
    } else {
        db::get_central_skills(pool)
            .await
            .map_err(|e| e.to_string())?
    };

    let valid_repositories =
        load_syncable_github_repositories(pool, &repository_ids, auth_token).await?;

    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, false).await?;

    // 2.1 把 skill 携带的 repo refs 和 scope 指定的 repo refs 合并去快照
    let mut snapshot_repos = prepared
        .iter()
        .filter_map(prepared_repo_ref)
        .collect::<Vec<_>>();
    snapshot_repos.extend(valid_repositories.iter().map(|(_, repo)| repo.clone()));

    let snapshots = prepare_snapshots_for_repo_refs_with_policy(
        client,
        auth_token,
        &snapshot_repos,
        snapshots_cache,
        snapshot_cache_policy(cache_policy),
    )
    .await?;

    /*
     * ========================================================================
     * 步骤3：算出每个 skill 的 update state，区分 updatable / remote_missing
     * ========================================================================
     * 每个已检查 skill 都写入 skill_update_states，inventory 只返回
     * actionable 桶。
     */
    let repo_by_id = valid_repositories
        .iter()
        .map(|(repository, _)| (repository.id.clone(), repository.clone()))
        .collect::<HashMap<_, _>>();
    let repo_ref_by_id = valid_repositories
        .iter()
        .map(|(repository, repo)| (repository.id.clone(), repo.clone()))
        .collect::<HashMap<_, _>>();

    let mut updatable = Vec::new();
    let mut remote_missing_states = Vec::new();
    let mut failed_repositories = Vec::new();
    let mut prepared_by_skill_id = HashMap::new();

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        let state_result = match load_remote_skill_content(&prepared_skill, &snapshots) {
            Ok(Some(remote)) => state_from_remote(skill, &remote, false),
            Ok(None) => unsupported_state_from_assignment(skill, &prepared_skill.assignment, None),
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => match mode {
                SkillRefreshMode::Sync => {
                    remote_missing_state_from_assignment(skill, &prepared_skill.assignment, &reason)
                }
                SkillRefreshMode::Regular => {
                    failed_repositories.push(FailedRepository {
                        repository_id: prepared_skill.assignment.repository.id.clone(),
                        error: format!(
                            "{} Switch to incremental and removal mode to decide whether to keep or delete '{}'.",
                            reason, skill.id
                        ),
                        diagnostics: Some(diagnostic_from_state(
                            &error_state_from_assignment(skill, &prepared_skill.assignment, &reason),
                            cache_policy,
                            false,
                        )),
                    });
                    error_state_from_assignment(skill, &prepared_skill.assignment, &reason)
                }
            },
            Err(RemoteSkillLoadError::Other(error)) => {
                error_state_from_assignment(skill, &prepared_skill.assignment, &error)
            }
        };

        match state_result.status.parse::<SkillUpdateStatus>().ok() {
            Some(SkillUpdateStatus::UpdateAvailable) => {
                let repository_id = repository_id_for_state(&repo_by_id, &state_result);
                let diagnostics = Some(diagnostic_from_state(&state_result, cache_policy, false));
                updatable.push(UpdatableSkill {
                    state: state_result,
                    repository_id,
                    diagnostics,
                });
            }
            Some(SkillUpdateStatus::RemoteMissing) => {
                remote_missing_states.push(state_result);
            }
            _ => {
                // up_to_date / unsupported / error / cancelled 不进入 inventory
            }
        }
        prepared_by_skill_id.insert(skill.id.clone(), prepared_skill);
    }

    /*
     * ========================================================================
     * 步骤4：发现远端新增 skill（仅当 scope 涉及 repos 时）
     * ========================================================================
     * 复用 collect_remote_added_skills，只有未跳过的 remote_added 写入
     * pending_additions。已 skip 的 addition 由 skill_repository_sync_skips
     * 持久化，不再回写 pending，避免 reload 后重新变成可操作新增项。
     */
    let mut remote_added = Vec::new();

    if include_sync_buckets && !repository_ids.is_empty() {
        let mut failed_collector = Vec::<central_updates::CentralRepositorySyncFailure>::new();
        let collection = central_updates::collect_remote_added_skills(
            pool,
            &repository_ids,
            &valid_repositories,
            &snapshots,
            &mut failed_collector,
        )
        .await?;

        let mut remote_added_items = collection.remote_added;
        let mut relocation_ctx = RelocationContext {
            pool,
            prepared_by_skill_id: &prepared_by_skill_id,
            snapshots: &snapshots,
            repo_by_id: &repo_by_id,
            repo_ref_by_id: &repo_ref_by_id,
            remote_missing_states: &mut remote_missing_states,
            remote_added_items: &mut remote_added_items,
            updatable: &mut updatable,
            failed_repositories: &mut failed_repositories,
        };
        reconcile_relocated_remote_skills(&mut relocation_ctx).await?;

        let now = Utc::now().to_rfc3339();
        for item in &remote_added_items {
            let source_path = normalize_repo_path(&item.preview.source_path)?;
            let addition = SkillRepositoryPendingAddition {
                repository_id: item.repository_id.clone(),
                source_path: source_path.clone(),
                skill_id: item.preview.skill_id.clone(),
                skill_name: item.preview.skill_name.clone(),
                conflict_existing_skill_id: item
                    .preview
                    .conflict
                    .as_ref()
                    .map(|c| c.existing_skill_id.clone()),
                discovered_at: now.clone(),
            };
            db::upsert_pending_addition(pool, &addition)
                .await
                .map_err(|e| e.to_string())?;
        }
        for item in &collection.skipped_remote_added {
            let source_path = normalize_repo_path(&item.preview.source_path)?;
            db::delete_pending_addition(pool, &item.repository_id, &source_path)
                .await
                .map_err(|e| e.to_string())?;
        }
        for item in remote_added_items {
            remote_added.push(remote_added_from_item(item));
        }
        for failure in failed_collector {
            failed_repositories.push(FailedRepository {
                repository_id: failure.repository_id,
                error: failure.error,
                diagnostics: None,
            });
        }
    }

    /*
     * ========================================================================
     * 步骤5：build remote_missing + 平台冗余 + 更新 repo.last_synced_at
     * ========================================================================
     */
    let remote_missing = if include_sync_buckets {
        central_updates::build_remote_missing_skills(&repo_by_id, remote_missing_states)
            .into_iter()
            .map(|item| RemoteMissingSkill {
                diagnostics: Some(diagnostic_from_state(&item.state, cache_policy, false)),
                repository_id: item.repository_id,
                state: item.state,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let platform_duplicates = if include_sync_buckets {
        scan_platform_duplicate_skills_with_pool(pool, platform_agent_ids.clone()).await?
    } else {
        Vec::new()
    };
    let deleted_platform_copies = if include_sync_buckets {
        scan_deleted_platform_copies_with_pool(pool, platform_agent_ids.clone()).await?
    } else {
        Vec::new()
    };

    let now = Utc::now().to_rfc3339();
    if include_sync_buckets {
        for repository_id in &repository_ids {
            db::set_repository_last_synced_at(pool, repository_id, &now)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let inventory = SkillUpdateInventory {
        updatable,
        remote_added,
        remote_missing,
        platform_duplicates,
        deleted_platform_copies,
        orphans: Vec::new(),
        failed_repositories,
        generated_at: now,
    };
    persist_refresh_inventory(pool, &scope, mode, cache_policy, &inventory).await?;
    Ok(inventory)
}

#[tauri::command]
pub async fn get_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> Result<SkillUpdateInventory, String> {
    let pool = state.active_db().await?;
    get_skill_update_inventory_impl_scoped(&pool, scope).await
}

#[tauri::command]
pub async fn clear_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    clear_skill_update_inventory_impl(&pool, scope).await
}

#[tauri::command]
#[allow(deprecated)]
pub async fn apply_skill_update_decisions(
    app: AppHandle,
    state: State<'_, AppState>,
    decisions: SkillUpdateDecisions,
) -> Result<SkillUpdateApplyResult, String> {
    /*
     * ========================================================================
     * 顺序：keep_missing → delete_missing → skip/unskip → import → updates
     *      → remove_platform_duplicates
     * ========================================================================
     * 每一步独立 partial success：单项失败不中断其他步骤，只记 failures。
     */
    let pool = state.active_db().await?;
    let active_target = state.active_target().await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;

    let mut result = SkillUpdateApplyResult::default();
    let allowed_agent_ids = normalize_optional_id_set(decisions.allowed_agent_ids.as_deref());

    // 步骤1：keep_missing
    apply_keep_missing_step(&pool, &decisions.keep_missing, &mut result).await;
    // 步骤2：delete_missing
    apply_delete_missing_step(
        &pool,
        &active_target,
        &decisions.delete_missing,
        &mut result,
    )
    .await;
    // 步骤3：skip/unskip pending_additions
    apply_skip_addition_step(&pool, decisions.skip_additions, &mut result).await;
    apply_unskip_addition_step(&pool, decisions.unskip_additions, &mut result).await;

    /*
     * ========================================================================
     * 步骤4：import additions —— 复用 import_github_repo_skills_*_with_auth
     * ========================================================================
     */
    for addition in decisions.import_additions {
        let repository_id = addition.repository_id.clone();
        let mut import_selections = Vec::new();
        for selection in addition.selections {
            if selection.resolution == github_import::DuplicateResolution::Skip {
                if let Ok(source_path) = normalize_repo_path(&selection.source_path) {
                    let _ = db::upsert_skill_repository_sync_skip(
                        &pool,
                        &repository_id,
                        &source_path,
                        &source_path,
                        &source_path,
                    )
                    .await;
                    let _ = db::delete_pending_addition(&pool, &repository_id, &source_path).await;
                }
            } else {
                import_selections.push(selection);
            }
        }
        if import_selections.is_empty() {
            continue;
        }

        let repository = match load_repository_for_import_addition(&pool, &repository_id).await? {
            Some(repository) => repository,
            None => {
                continue;
            }
        };
        let repo_url = match repository_import_url(&repository) {
            Some(url) => url,
            None => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "import_addition".to_string(),
                    identifier: repository.id,
                    error: "GitHub repository URL is unavailable.".to_string(),
                });
                continue;
            }
        };

        let outcome = match &active_target {
            ActiveTarget::Local => {
                github_import::import_github_repo_skills_with_auth(
                    &pool,
                    &repo_url,
                    import_selections,
                    Some(&app),
                    auth.as_deref(),
                )
                .await
            }
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                github_import::import_github_repo_skills_remote_with_auth(
                    &pool,
                    &active_target,
                    &repo_url,
                    import_selections,
                    addition.preview_workspace_id.as_deref(),
                    Some(&app),
                    auth.as_deref(),
                )
                .await
            }
        };

        match outcome {
            Ok(import_result) => {
                for imported in &import_result.imported_skills {
                    let source_path = normalize_repo_path(&imported.source_path)?;
                    let _ =
                        db::delete_skill_repository_sync_skip(&pool, &repository_id, &source_path)
                            .await;
                    let _ = db::delete_pending_addition(&pool, &repository_id, &source_path).await;
                    result
                        .imported_skill_ids
                        .push(imported.imported_skill_id.clone());
                }
            }
            Err(error) => result.failures.push(SkillUpdateApplyFailure {
                step: "import_addition".to_string(),
                identifier: repository.id,
                error: error.to_string(),
            }),
        }
    }

    /*
     * ========================================================================
     * 步骤5：updates —— 调旧的 update_central_skills 命令复用既有循环
     * ========================================================================
     * 旧命令做了 progress event / counters / cancel 等完整流程，没必要重写。
     */
    if !decisions.updates.is_empty() {
        match central_updates::update_central_skills(
            app.clone(),
            state.clone(),
            decisions.updates.clone(),
        )
        .await
        {
            Ok(update_result) => {
                result.updated_skill_ids = update_result.succeeded;
                for failure in update_result.failed {
                    result.failures.push(SkillUpdateApplyFailure {
                        step: "update".to_string(),
                        identifier: failure.skill_id,
                        error: failure.error,
                    });
                }
            }
            Err(error) => result.failures.push(SkillUpdateApplyFailure {
                step: "update".to_string(),
                identifier: decisions.updates.join(","),
                error,
            }),
        }
    }

    // 步骤6：remove_platform_duplicates
    apply_remove_platform_duplicates_step(
        &pool,
        decisions.remove_platform_duplicates,
        &mut result,
        allowed_agent_ids.as_ref(),
    )
    .await;
    // 步骤7：remove_deleted_platform_copies
    apply_remove_deleted_platform_copies_step(
        &pool,
        &active_target,
        decisions.remove_deleted_platform_copies,
        &mut result,
        allowed_agent_ids.as_ref(),
    )
    .await;

    prune_applied_inventory_entries(&pool, &result).await;

    Ok(result)
}

#[tauri::command]
pub async fn force_update_central_skills(
    state: State<'_, AppState>,
    request: ForceSkillUpdateRequest,
) -> Result<ForceSkillUpdateResult, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?).await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    force_update_central_skills_impl(
        &pool,
        &fs,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        SnapshotCachePolicy::Bypass,
        request,
    )
    .await
}

#[tauri::command]
pub async fn force_mirror_central_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ForceRepositoryMirrorRequest,
) -> Result<ForceRepositoryMirrorResult, String> {
    let pool = state.active_db().await?;
    let active_target = state.active_target().await?;
    let fs = CentralFs::from_active_target(active_target.clone()).await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    force_mirror_central_repositories_impl(
        Some(&app),
        &pool,
        &active_target,
        &fs,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        SnapshotCachePolicy::Bypass,
        request,
    )
    .await
}

#[tauri::command]
pub async fn scan_platform_duplicate_skills(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<PlatformDuplicateGroup>, String> {
    let pool = state.active_db().await?;
    scan_platform_duplicate_skills_with_pool(&pool, agent_ids).await
}

#[tauri::command]
pub async fn scan_deleted_platform_copies(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<DeletedPlatformCopyGroup>, String> {
    let pool = state.active_db().await?;
    scan_deleted_platform_copies_with_pool(&pool, agent_ids).await
}

/*
 * ========================================================================
 * 内部 helpers
 * ========================================================================
 */

async fn load_repository_for_import_addition(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Option<db::SkillRepository>, String> {
    match db::get_skill_repository_by_id(pool, repository_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(repository) => Ok(Some(repository)),
        None => {
            db::clear_pending_additions_for_repos(pool, &[repository_id.to_string()])
                .await
                .map_err(|e| e.to_string())?;
            Ok(None)
        }
    }
}

fn snapshot_cache_policy(policy: SkillRefreshCachePolicy) -> SnapshotCachePolicy {
    match policy {
        SkillRefreshCachePolicy::UseFresh => SnapshotCachePolicy::UseFresh,
        SkillRefreshCachePolicy::Bypass => SnapshotCachePolicy::Bypass,
    }
}

#[cfg(test)]
mod tests;
