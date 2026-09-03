//! Skill update inventory 纯读视图与清空的内核实现。
//!
//! 从 `commands/skill_update_inventory.rs` 原样迁出；Tauri 命令壳层在
//! `crate::commands::skill_update_inventory`，调用这里的 `*_impl`。两个函数都
//! 不触网，仅汇总/清理 DB 既有状态。

use super::*;
use crate::db::repos::pending_additions_repo;
use crate::db::repos::update_inventory_repo;
use crate::services::central_updates::CentralUpdatesError;

pub(crate) async fn get_skill_update_inventory_impl_scoped(
    pool: &DbPool,
    scope: Option<SkillRefreshScope>,
    cli_lock_protect: bool,
) -> Result<SkillUpdateInventory, CentralUpdatesError> {
    /*
     * ========================================================================
     * 步骤1：读 skill_update_states，分出 update_available / remote_missing
     * ========================================================================
     * 这是不触网的"读视图"，仅汇总 DB 既有状态。
     */

    let scope_filter = InventoryScopeFilter::from_scope(pool, scope.clone()).await?;
    pending_additions_repo::prune_orphaned_pending_additions(pool).await?;
    let entries = update_inventory_repo::list_skill_update_inventory_entries(
        pool,
        &inventory_id_for_scope(scope.as_ref()),
    )
    .await?;
    let (updatable, remote_missing, unsupported, failed_repositories, inventory_generated_at) =
        inventory_from_entries(entries)?;

    /*
     * ========================================================================
     * 步骤2：读 pending_additions 转 remote_added
     * ========================================================================
     */
    let pending = match &scope_filter.pending {
        PendingAdditionScope::All => pending_additions_repo::list_pending_additions(pool).await?,
        PendingAdditionScope::Repositories(repository_ids) => {
            let ids = repository_ids.iter().cloned().collect::<Vec<_>>();
            pending_additions_repo::list_pending_additions_for_repos(pool, &ids).await?
        }
        PendingAdditionScope::SkillIds(skill_ids) => {
            pending_additions_repo::list_pending_additions(pool)
                .await?
                .into_iter()
                .filter(|p| skill_ids.contains(&p.skill_id))
                .collect()
        }
        PendingAdditionScope::None => Vec::new(),
    };
    let remote_added = pending
        .into_iter()
        .map(|p| RemoteAddedSkill {
            repository_id: p.repository_id,
            source_path: p.source_path,
            skill_id: p.skill_id,
            skill_name: p.skill_name,
            conflict_existing_skill_id: p.conflict_existing_skill_id,
        })
        .collect();

    /*
     * ========================================================================
     * 步骤3：平台冗余 + 组装
     * ========================================================================
     */
    let platform_duplicates =
        scan_platform_duplicate_skills_with_pool(pool, scope_filter.agent_ids.clone()).await?;
    let deleted_platform_copies = scan_deleted_platform_copies_with_pool(
        pool,
        scope_filter.agent_ids.clone(),
        cli_lock_protect,
    )
    .await?;

    Ok(SkillUpdateInventory {
        updatable,
        remote_added,
        remote_missing,
        unsupported,
        platform_duplicates,
        deleted_platform_copies,
        orphans: Vec::new(),
        failed_repositories,
        snapshot_retry_attempted: None,
        snapshot_retry_recovered: None,
        generated_at: inventory_generated_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
    })
}

/// 内核版本：不依赖 `State<AppState>`，便于单元测试。
pub(crate) async fn clear_skill_update_inventory_impl(
    pool: &DbPool,
    scope: Option<SkillRefreshScope>,
) -> Result<(), CentralUpdatesError> {
    /*
     * ========================================================================
     * 清除 pending_additions —— 不动 update_states 以避免误删 update_available
     * ========================================================================
     * - None / All：清空全表
     * - Skills：与 pending 无关，noop
     * - Repositories：清这些 repo 的 pending 行
     */
    match scope {
        None => {
            update_inventory_repo::clear_all_skill_update_inventory(pool).await?;
            pending_additions_repo::clear_pending_additions(pool).await?;
        }
        Some(scope) => match scope.kind {
            SkillRefreshScopeKind::All => {
                update_inventory_repo::clear_all_skill_update_inventory(pool).await?;
                pending_additions_repo::clear_pending_additions(pool).await?;
            }
            SkillRefreshScopeKind::Skills => {
                let ids = normalize_ids(scope.skill_ids.unwrap_or_default());
                update_inventory_repo::delete_skill_update_inventory_entries_for_skills(pool, &ids)
                    .await?;
                pending_additions_repo::clear_pending_additions_for_skill_ids(pool, &ids).await?;
            }
            SkillRefreshScopeKind::Repositories => {
                let ids = normalize_ids(scope.repository_ids.unwrap_or_default());
                if !ids.is_empty() {
                    update_inventory_repo::delete_skill_update_inventory_entries_for_repositories(
                        pool, &ids,
                    )
                    .await?;
                    pending_additions_repo::clear_pending_additions_for_repos(pool, &ids).await?;
                }
            }
            SkillRefreshScopeKind::Platform => {
                update_inventory_repo::clear_skill_update_inventory_run(
                    pool,
                    &inventory_id_for_scope(Some(&scope)),
                )
                .await?;
            }
        },
    }
    Ok(())
}
