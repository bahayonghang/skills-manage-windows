//! Skill Update Inventory —— Phase P2 of Update Mechanism Overhaul。
//!
//! 在旧的 `check_central_skill_updates` / `check_central_repository_sync` 之上
//! 提供面向"更新中心"统一面板的内核集合：
//!
//! - `refresh_skill_update_inventory_impl`：拉远端并写入 update-inventory 表与
//!   remote-addition bucket，返回一份完整 inventory；不更新安装 baseline。
//! - `get_skill_update_inventory_impl_scoped`：纯读视图，不触网。
//! - `clear_skill_update_inventory_impl`：清空所选 scope 的 inventory bucket，
//!   不删除技能、不改 `skill_update_states` baseline。
//! - `apply_skill_update_decisions_impl`：把用户在面板里勾的决策一次性应用，
//!   每步独立 partial success，复用 keep/delete/import/update/uninstall 既有 impl。
//! - `force_update_central_skills_impl` / `force_mirror_central_repositories_impl`：
//!   显式救援动作，绕过检测结果从远端覆盖/镜像同步。
//! - `scan_platform_duplicate_skills_with_pool`：观察各 agent 是否同 skill_id 同时有
//!   writable 与 plugin readonly 两份，给前端去重弹窗用。
//!
//! 不重新实现业务逻辑，只组合既有 helper。旧命令仍并行存在以保证不破坏前端；
//! Tauri IPC 壳层在 `crate::commands::skill_update_inventory`。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;

use chrono::Utc;
use tauri::AppHandle;

use crate::db::{self, DbPool, SkillRepositoryPendingAddition};
use crate::services::github_import;
use crate::targets::ActiveTarget;

use super::core::{
    error_state_from_assignment, load_remote_skill_content, prepare_skill_updates,
    remote_missing_state_from_assignment, state_from_remote, unsupported_state_from_assignment,
    update_central_skills_impl,
};
use super::error::CentralUpdatesError;
use super::fs::{normalize_repo_path, CentralFs};
use super::repository_sync::{collect_remote_added_skills, CentralRepositorySyncFailure};
use super::snapshots::{
    prepare_snapshots_for_repo_refs_collecting_failures, repo_cache_key,
    CentralUpdateSnapshotCache, SnapshotProgressEvent, SnapshotProgressReporter,
};
use super::types::{
    unsupported_reason_code, CentralUpdateFailurePhase, RemoteSkillLoadError, SkillUpdateStatus,
    SnapshotCachePolicy,
};

mod apply_steps;
mod force;
mod persistence;
mod relocation;
mod repositories;
mod retry;
mod scan;
mod scope;
mod types;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use apply_steps::*;
pub(crate) use force::*;
use persistence::*;
use relocation::*;
pub(crate) use repositories::*;
pub(crate) use retry::*;
pub(crate) use scan::*;
pub(crate) use scope::*;
pub use types::*;
pub(crate) use view::*;

/// An update state paired with the repository assignment that produced it.
///
/// Scope controls which skills are checked; it must not erase repository
/// ownership while the state moves through relocation and inventory building.
pub(super) struct RepositoryOwnedUpdateState {
    pub repository_id: String,
    pub state: crate::db::SkillUpdateState,
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
    snapshots_cache: &CentralUpdateSnapshotCache,
    scope: SkillRefreshScope,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SkillUpdateInventory, CentralUpdatesError> {
    let mode = scope.mode.unwrap_or(SkillRefreshMode::Sync);
    let cache_policy = scope
        .cache_policy
        .unwrap_or(SkillRefreshCachePolicy::Bypass);
    let inventory = compute_skill_update_inventory(
        pool,
        fs,
        auth_token,
        client,
        snapshots_cache,
        &scope,
        progress,
    )
    .await?;
    persist_refresh_inventory(pool, &scope, mode, cache_policy, &inventory).await?;
    Ok(inventory)
}

/// Build an inventory without persisting it. Retry merges several computed
/// slices into one stored inventory, so the write stays with the caller.
pub(crate) async fn compute_skill_update_inventory(
    pool: &DbPool,
    fs: &CentralFs,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    scope: &SkillRefreshScope,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SkillUpdateInventory, CentralUpdatesError> {
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
                .await?
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
                let ids = db::get_central_skill_ids_by_repository(pool, repository_id).await?;
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
     * refresh 会计算 scope 内每个 skill，但只把当前 inventory bucket
     * 持久化；安装 baseline 仍由成功的 apply/update 维护。
     */
    let skills = if let Some(ids) = &skill_ids_filter {
        if ids.is_empty() {
            Vec::new()
        } else {
            db::get_central_skills_by_ids(pool, ids).await?
        }
    } else {
        db::get_central_skills(pool).await?
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

    // 2.2 快照失败按仓库结算，不终止整轮。检查范围覆盖全部可同步 GitHub 仓库，
    // 任意一个仓库不可达就丢弃其余仓库的结果会让整轮检查无任何持久化产出。
    let mut repository_id_by_snapshot_key = HashMap::<String, String>::new();
    for (repository, repo) in &valid_repositories {
        repository_id_by_snapshot_key
            .entry(repo_cache_key(repo))
            .or_insert_with(|| repository.id.clone());
    }
    for prepared_skill in &prepared {
        if let Some(repo) = prepared_repo_ref(prepared_skill) {
            repository_id_by_snapshot_key
                .entry(repo_cache_key(&repo))
                .or_insert_with(|| prepared_skill.assignment.repository.id.clone());
        }
    }

    let acquisition = prepare_snapshots_for_repo_refs_collecting_failures(
        client,
        auth_token,
        &snapshot_repos,
        snapshots_cache,
        snapshot_cache_policy(cache_policy),
        progress.clone(),
    )
    .await?;
    let snapshot_retry_attempted =
        Some(u32::try_from(acquisition.retry_attempted).unwrap_or(u32::MAX));
    let snapshot_retry_recovered =
        Some(u32::try_from(acquisition.retry_recovered).unwrap_or(u32::MAX));
    let snapshots = acquisition.snapshots;
    let snapshot_failures = acquisition
        .failures
        .into_iter()
        .map(|failure| {
            let key = repo_cache_key(&failure.repo);
            let (error, error_code) = failed_repository_reason(&failure.error);
            FailedRepository {
                repository_id: repository_id_by_snapshot_key
                    .get(&key)
                    .cloned()
                    .unwrap_or(key),
                error,
                error_code,
                diagnostic_category: Some(failure.error.snapshot_diagnostic_category().to_string()),
                retry: FailedRepositoryRetry::Retryable,
                diagnostics: None,
            }
        })
        .collect::<Vec<_>>();
    if let Some(progress) = &progress {
        progress(SnapshotProgressEvent::finalizing(
            snapshots.len(),
            snapshots.len(),
        ));
    }

    /*
     * ========================================================================
     * 步骤3：算出每个 skill 的 update state，区分 updatable / remote_missing
     * ========================================================================
     * refresh 只构建当前 inventory；skill_update_states 是 apply/update 成功后的
     * 安装 baseline，此处不能写入。up_to_date 不是可操作结果，unsupported
     * 则必须保留在 inventory 中供用户查看。
     */
    let repo_ref_by_id = valid_repositories
        .iter()
        .map(|(repository, repo)| (repository.id.clone(), repo.clone()))
        .collect::<HashMap<_, _>>();

    let mut updatable = Vec::new();
    let mut remote_missing_states = Vec::new();
    let mut unsupported = Vec::new();
    let mut failed_repositories = snapshot_failures;
    let mut prepared_by_skill_id = HashMap::new();
    // Regular mode has no remote-addition listing to pair a vanished path with,
    // so these are resolved against the repository snapshot after the loop.
    let mut pending_relocations = Vec::new();

    for prepared_skill in prepared {
        let skill_id = prepared_skill.skill.id.clone();
        let repository_id = prepared_skill.assignment.repository.id.clone();
        let load_result = load_remote_skill_content(&prepared_skill, &snapshots);

        if mode == SkillRefreshMode::Regular
            && matches!(load_result, Err(RemoteSkillLoadError::RemoteMissing(_)))
        {
            pending_relocations.push(PendingRelocation {
                skill_id: skill_id.clone(),
                repository_id: prepared_skill.assignment.repository.id.clone(),
            });
            prepared_by_skill_id.insert(skill_id, prepared_skill);
            continue;
        }

        let skill = &prepared_skill.skill;
        let state_result = match load_result {
            Ok(Some(remote)) => state_from_remote(skill, &remote, false),
            Ok(None) => unsupported_state_from_assignment(skill, &prepared_skill.assignment, None),
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => {
                remote_missing_state_from_assignment(skill, &prepared_skill.assignment, &reason)
            }
            Err(RemoteSkillLoadError::Other(error)) => {
                error_state_from_assignment(skill, &prepared_skill.assignment, &error)
            }
        };

        match state_result.status {
            SkillUpdateStatus::UpdateAvailable => {
                let diagnostics = Some(diagnostic_from_state(&state_result, cache_policy, false));
                updatable.push(UpdatableSkill {
                    state: state_result,
                    repository_id: Some(repository_id),
                    diagnostics,
                });
            }
            SkillUpdateStatus::RemoteMissing => {
                remote_missing_states.push(RepositoryOwnedUpdateState {
                    repository_id,
                    state: state_result,
                });
            }
            SkillUpdateStatus::Unsupported => {
                unsupported.push(UnsupportedSkill {
                    skill_id: skill_id.clone(),
                    reason_code: unsupported_reason_code(&prepared_skill.assignment),
                });
            }
            _ => {
                // up_to_date / error / cancelled 不进入 actionable inventory
            }
        }
        prepared_by_skill_id.insert(skill_id, prepared_skill);
    }

    if !pending_relocations.is_empty() {
        resolve_regular_mode_relocations(
            pool,
            &pending_relocations,
            &prepared_by_skill_id,
            &snapshots,
            &mut updatable,
            &mut failed_repositories,
        )
        .await?;
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
        let mut failed_collector = Vec::<CentralRepositorySyncFailure>::new();
        let collection = collect_remote_added_skills(
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
            db::upsert_pending_addition(pool, &addition).await?;
        }
        for item in &collection.skipped_remote_added {
            let source_path = normalize_repo_path(&item.preview.source_path)?;
            db::delete_pending_addition(pool, &item.repository_id, &source_path).await?;
        }
        for item in remote_added_items {
            remote_added.push(remote_added_from_item(item));
        }
        for failure in failed_collector {
            // The collector's text can carry transport detail, so only the
            // reviewed sentence for the stable code is surfaced.
            failed_repositories.push(FailedRepository {
                repository_id: failure.repository_id,
                error: repository_check_failed_message(),
                error_code: Some(REPOSITORY_CHECK_FAILED_CODE.to_string()),
                diagnostic_category: None,
                retry: FailedRepositoryRetry::Retryable,
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
        remote_missing_states
            .into_iter()
            .map(|owned| RemoteMissingSkill {
                diagnostics: Some(diagnostic_from_state(&owned.state, cache_policy, false)),
                repository_id: Some(owned.repository_id),
                state: owned.state,
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
            db::set_repository_last_synced_at(pool, repository_id, &now).await?;
        }
    }

    // Inventory entries are keyed by (bucket, repository_id), so a repository
    // may only appear once. Snapshot acquisition failures are seeded first, so
    // the root cause wins over the downstream reasons it produced.
    let mut seen_failed_repositories = HashSet::new();
    failed_repositories.retain(|item| seen_failed_repositories.insert(item.repository_id.clone()));

    Ok(SkillUpdateInventory {
        updatable,
        remote_added,
        remote_missing,
        unsupported,
        platform_duplicates,
        deleted_platform_copies,
        orphans: Vec::new(),
        failed_repositories,
        snapshot_retry_attempted,
        snapshot_retry_recovered,
        generated_at: now,
    })
}

/// 内核版本：把面板决策一次性应用。步骤 5 直接调 `update_central_skills_impl`
/// 复用完整的 progress event / counters / cancel 流程，不再经由旧命令壳层。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_skill_update_decisions_impl(
    app: Option<&AppHandle>,
    job_id: &str,
    pool: &DbPool,
    active_target: &ActiveTarget,
    fs: &CentralFs,
    cancel: &AtomicBool,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    decisions: SkillUpdateDecisions,
) -> Result<SkillUpdateApplyResult, CentralUpdatesError> {
    /*
     * ========================================================================
     * 顺序：keep_missing → delete_missing → skip/unskip → import → updates
     *      → remove_platform_duplicates
     * ========================================================================
     * 每一步独立 partial success：单项失败不中断其他步骤，只记 failures。
     */
    let mut result = SkillUpdateApplyResult::default();
    let allowed_agent_ids = normalize_optional_id_set(decisions.allowed_agent_ids.as_deref());

    // 步骤1：keep_missing
    apply_keep_missing_step(pool, &decisions.keep_missing, &mut result).await;
    // 步骤2：delete_missing
    apply_delete_missing_step(pool, active_target, &decisions.delete_missing, &mut result).await;
    // 步骤3：skip/unskip pending_additions
    apply_skip_addition_step(pool, decisions.skip_additions, &mut result).await;
    apply_unskip_addition_step(pool, decisions.unskip_additions, &mut result).await;

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
                        pool,
                        &repository_id,
                        &source_path,
                        &source_path,
                        &source_path,
                    )
                    .await;
                    let _ = db::delete_pending_addition(pool, &repository_id, &source_path).await;
                }
            } else {
                import_selections.push(selection);
            }
        }
        if import_selections.is_empty() {
            continue;
        }

        let repository = match load_repository_for_import_addition(pool, &repository_id).await? {
            Some(repository) => repository,
            None => {
                continue;
            }
        };
        let repo_url = match repository_import_url(&repository) {
            Some(url) => url,
            None => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "import_addition",
                    repository.id,
                ));
                continue;
            }
        };

        let outcome = match active_target {
            ActiveTarget::Local => {
                github_import::import_github_repo_skills_with_auth(
                    pool,
                    &repo_url,
                    import_selections,
                    app,
                    auth_token,
                )
                .await
            }
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                github_import::import_github_repo_skills_remote_with_auth(
                    pool,
                    active_target,
                    &repo_url,
                    import_selections,
                    app,
                    auth_token,
                )
                .await
            }
        };

        match outcome {
            Ok(import_result) => {
                for imported in &import_result.imported_skills {
                    let source_path = normalize_repo_path(&imported.source_path)?;
                    let _ =
                        db::delete_skill_repository_sync_skip(pool, &repository_id, &source_path)
                            .await;
                    let _ = db::delete_pending_addition(pool, &repository_id, &source_path).await;
                    result
                        .imported_skill_ids
                        .push(imported.imported_skill_id.clone());
                }
            }
            Err(error) => result
                .failures
                .push(SkillUpdateApplyFailure::from_github_import(
                    repository.id,
                    error,
                )),
        }
    }

    /*
     * ========================================================================
     * 步骤5：updates —— 调 update_central_skills_impl 复用既有循环
     * ========================================================================
     * impl 做了 progress event / counters / cancel 等完整流程，没必要重写。
     */
    if !decisions.updates.is_empty() {
        match update_central_skills_impl(
            app,
            job_id,
            pool,
            fs,
            cancel,
            auth_token,
            client,
            snapshots_cache,
            decisions.updates.clone(),
        )
        .await
        {
            Ok(update_result) => {
                result.updated_skill_ids = update_result.succeeded;
                for failure in update_result.failed {
                    result
                        .failures
                        .push(SkillUpdateApplyFailure::from_central_update(failure));
                }
            }
            Err(error) => result
                .failures
                .push(SkillUpdateApplyFailure::from_central_error(
                    "update",
                    "batch",
                    CentralUpdateFailurePhase::DecisionApply,
                    error,
                )),
        }
    }

    // 步骤6：remove_platform_duplicates
    apply_remove_platform_duplicates_step(
        pool,
        decisions.remove_platform_duplicates,
        &mut result,
        allowed_agent_ids.as_ref(),
    )
    .await;
    // 步骤7：remove_deleted_platform_copies
    apply_remove_deleted_platform_copies_step(
        pool,
        active_target,
        decisions.remove_deleted_platform_copies,
        &mut result,
        allowed_agent_ids.as_ref(),
    )
    .await;

    prune_applied_inventory_entries(pool, &result).await;

    Ok(result)
}

/*
 * ========================================================================
 * 内部 helpers
 * ========================================================================
 */

async fn load_repository_for_import_addition(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Option<db::SkillRepository>, CentralUpdatesError> {
    match db::get_skill_repository_by_id(pool, repository_id).await? {
        Some(repository) => Ok(Some(repository)),
        None => {
            db::clear_pending_additions_for_repos(pool, &[repository_id.to_string()]).await?;
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

/// Reduce a snapshot acquisition failure to what the inventory may persist and
/// show: a stable code plus its reviewed public sentence. The domain error's
/// Display text is never used, because it can carry a URL, path, mirror label,
/// or transport detail.
fn failed_repository_reason(error: &CentralUpdatesError) -> (String, Option<String>) {
    match error
        .reviewed_operation_failure()
        .and_then(|(code, _)| crate::ipc_error::public_message_for_code(code).map(|m| (code, m)))
    {
        Some((code, message)) => (message.to_string(), Some(code.to_string())),
        None => (
            "The repository could not be checked.".to_string(),
            Some("central_updates.repository_check_failed".to_string()),
        ),
    }
}
