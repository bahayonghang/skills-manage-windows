//! Database module — split from a single 5612-line `db.rs` per task_plan.md Phase 2.
//!
//! 当前阶段（Phase 2b 完成）：
//! - `types`：所有 struct / 公共常量 / `DbPool` 类型别名
//! - `pool`：`create_pool` 连接池工厂（WAL 模式）
//! - `schema`：建表 / 索引 / ALTER TABLE 增量列，按业务域拆分（core /
//!   collections / metadata / discovery / settings / marketplace）
//! - `migrations`：`ensure_column` 等 schema 演进工具
//! - `legacy`：seed / agent 注册表 / 业务 CRUD / 测试，等待 Phase 2c 拆 repos/*
//!
//! 上层通过 `pub use` 在 `crate::db` 顶层重导出 types / pool / legacy 公开符号；
//! `schema` 与 `migrations` 仅供 `legacy::init_database_with_agents` 调度使用，
//! 不对外暴露。下游引用路径 `crate::db::Foo` 保持不变。
//!
//! 演进路径：
//! - Phase 2c：repos/* 接管 173 处 sqlx::query()，commands 改调 repo
//! - Phase 2d：删除 legacy.rs
//!
//! 下游约束：本模块对外仅暴露 `crate::db::Foo` 顶级符号，下游代码不得直接
//! 引用 `crate::db::legacy::Foo` 或 `crate::db::types::Foo`。

mod migrations;
mod pool;
mod repos;
mod schema;
mod seed;
mod types;
mod util;

#[cfg(test)]
mod tests;

pub use pool::*;
pub use repos::agents_repo::*;
pub use repos::collections_repo::*;
pub use repos::installations_repo::*;
pub use repos::observations_repo::*;
pub use repos::operation_logs_repo::*;
pub use repos::projects_repo::*;
pub use repos::repositories_repo::*;
pub use repos::saved_views_repo::*;
pub use repos::scan_dirs_repo::*;
pub use repos::settings_repo::*;
pub use repos::skills_repo::*;
pub use repos::tag_groups_repo::*;
pub use repos::tags_repo::*;
pub use repos::update_states_repo::*;
pub use seed::*;
pub use types::*;
