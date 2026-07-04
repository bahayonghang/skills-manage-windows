//! Skill update inventory 纯读视图与清空的内核实现。
//!
//! 从 `commands/skill_update_inventory.rs` 原样迁出；Tauri 命令壳层在
//! `crate::commands::skill_update_inventory`，调用这里的 `*_impl`。两个函数都
//! 不触网，仅汇总/清理 DB 既有状态。

use super::*;
use crate::services::central_updates::CentralUpdatesError;

pub(crate) async fn get_skill_update_inventory_impl_scoped(
    pool: &DbPool,
    scope: Option<SkillRefreshScope>,
) -> Result<SkillUpdateInventory, CentralUpdatesError> {
    /*
     * ========================================================================
     * 步骤1：读 skill_update_states，分出 update_available / remote_missing
     * ========================================================================
     * 这是不触网的"读视图"，仅汇总 DB 既有状态。
     */

    let scope_filter = InventoryScopeFilter::from_scope(pool, scope.clone()).await?;
    db::prune_orphaned_pending_additions(pool).await?;
    let entries =
        db::list_skill_update_inventory_entries(pool, &inventory_id_for_scope(scope.as_ref()))
            .await?;
    let (updatable, remote_missing, failed_repositories, inventory_generated_at) =
        inventory_from_entries(entries)?;

    /*
     * ========================================================================
     * 步骤2：读 pending_additions 转 remote_added
     * ========================================================================
     */
    let pending = match &scope_filter.pending {
        PendingAdditionScope::All => db::list_pending_additions(pool).await?,
        PendingAdditionScope::Repositories(repository_ids) => {
            let ids = repository_ids.iter().cloned().collect::<Vec<_>>();
            db::list_pending_additions_for_repos(pool, &ids).await?
        }
        PendingAdditionScope::SkillIds(skill_ids) => db::list_pending_additions(pool)
            .await?
            .into_iter()
            .filter(|p| skill_ids.contains(&p.skill_id))
            .collect(),
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
    let deleted_platform_copies =
        scan_deleted_platform_copies_with_pool(pool, scope_filter.agent_ids.clone()).await?;

    Ok(SkillUpdateInventory {
        updatable,
        remote_added,
        remote_missing,
        platform_duplicates,
        deleted_platform_copies,
        orphans: Vec::new(),
        failed_repositories,
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
            db::clear_all_skill_update_inventory(pool).await?;
            db::clear_pending_additions(pool).await?;
        }
        Some(scope) => match scope.kind {
            SkillRefreshScopeKind::All => {
                db::clear_all_skill_update_inventory(pool).await?;
                db::clear_pending_additions(pool).await?;
            }
            SkillRefreshScopeKind::Skills => {
                let ids = normalize_ids(scope.skill_ids.unwrap_or_default());
                db::delete_skill_update_inventory_entries_for_skills(pool, &ids).await?;
                db::clear_pending_additions_for_skill_ids(pool, &ids).await?;
            }
            SkillRefreshScopeKind::Repositories => {
                let ids = normalize_ids(scope.repository_ids.unwrap_or_default());
                if !ids.is_empty() {
                    db::delete_skill_update_inventory_entries_for_repositories(pool, &ids).await?;
                    db::clear_pending_additions_for_repos(pool, &ids).await?;
                }
            }
            SkillRefreshScopeKind::Platform => {
                db::clear_skill_update_inventory_run(pool, &inventory_id_for_scope(Some(&scope)))
                    .await?;
            }
        },
    }
    Ok(())
}
