//! Schema initialization — split from `db/legacy.rs` per task_plan.md Phase 2b.
//!
//! 调度顺序按表依赖排：
//!
//! ```text
//! core         skills / skill_installations / agent_skill_observations / agents
//!  └─ collections  collections / collection_skills
//!     └─ metadata  repositories / update_states / tags / tag_links / ai_reviews
//!        └─ discovery  scan_directories / discovered_skills
//!           └─ settings   settings / operation_logs
//!              └─ marketplace  registries / skills / explanations + 8 列迁移
//! ```
//!
//! 所有 `CREATE TABLE` / `CREATE INDEX` 均带 `IF NOT EXISTS`，幂等可重复跑。
//! 同一业务域的 `ALTER TABLE` 增量列就近放在自家子模块里，避免一次集中跑导致
//! 调度链耦合。
//!
//! 对外仅暴露 [`init`]。子模块 `pub(super)` 收口在 schema 内。

pub(super) mod collections;
pub(super) mod core;
pub(super) mod discovery;
pub(super) mod marketplace;
pub(super) mod metadata;
pub(super) mod projects;
pub(super) mod saved_views;
pub(super) mod settings;
pub(super) mod usage;

use super::types::DbPool;

/// 全量建表 + 增量迁移。`init_database_with_agents` 在 seed 之前调用一次。
///
/// 顺序原则：
/// 1. 主键表 / 其他表会引用的表先建（如 `marketplace_skills` FK → `skill_registries`）
/// 2. 同一业务域的索引与 ALTER TABLE 跟在自家 CREATE TABLE 后面，避免漂移
/// 3. WAL：连接池构造时已设置，这里再次执行以兼容外部直接传入的池实例
pub(super) async fn init(pool: &DbPool) -> Result<(), String> {
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    core::init(pool).await?;
    collections::init(pool).await?;
    metadata::init(pool).await?;
    discovery::init(pool).await?;
    settings::init(pool).await?;
    marketplace::init(pool).await?;
    saved_views::init(pool).await?;
    projects::init(pool).await?;
    usage::init(pool).await?;

    Ok(())
}
