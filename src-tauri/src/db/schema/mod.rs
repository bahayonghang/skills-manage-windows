//! Frozen migration-1 legacy schema normalization.
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

use sqlx::SqliteConnection;

/// 在 migration 1 的单一事务连接上完成 legacy schema 归一化。
///
/// 顺序原则：
/// 1. 主键表 / 其他表会引用的表先建（如 `marketplace_skills` FK → `skill_registries`）
/// 2. 同一业务域的索引与 ALTER TABLE 跟在自家 CREATE TABLE 后面，避免漂移
pub(super) async fn init(connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    core::init(connection).await?;
    collections::init(connection).await?;
    metadata::init(connection).await?;
    discovery::init(connection).await?;
    settings::init(connection).await?;
    marketplace::init(connection).await?;
    saved_views::init(connection).await?;
    projects::init(connection).await?;
    usage::init(connection).await?;

    Ok(())
}
