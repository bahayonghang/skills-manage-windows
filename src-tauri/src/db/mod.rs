//! Database boundary.
//!
//! `pool`, `schema`, `migrations`, and `util` are internal plumbing for opening
//! and evolving the SQLite database. `types` contains shared row/domain structs,
//! while `repos/*` owns the SQL for each table family.
//!
//! The broad `crate::db::*` re-export surface is intentionally kept as a
//! compatibility layer while commands and services are migrated to narrower
//! repository imports. New DB helpers should live in the appropriate
//! `repos/*_repo.rs` module; avoid adding unrelated SQL to this top-level file.

mod migrations;
mod pool;
mod repos;
mod schema;
mod seed;
pub(crate) mod sqlite_batch;
mod types;
mod util;

#[cfg(test)]
mod tests;

pub(crate) use migrations::{
    inspect_database_integrity, open_database_for_startup, DatabaseOpenFailure, DatabaseOpenStage,
};
pub use migrations::{open_database, open_database_for_remote_home};
#[cfg(test)]
pub(crate) use pool::{create_memory_pool, create_memory_pool_single_conn, create_pool};
pub use repos::agents_repo::*;
pub use repos::collections_repo::*;
pub use repos::fs_db_operations_repo::*;
pub use repos::installations_repo::*;
pub use repos::observations_repo::*;
pub use repos::operation_logs_repo::*;
pub use repos::pending_additions_repo::*;
pub use repos::projects_repo::*;
pub use repos::repositories_repo::*;
pub use repos::saved_views_repo::*;
pub use repos::scan_dirs_repo::*;
pub use repos::settings_repo::*;
pub use repos::skill_relations_repo::{
    repair_orphan_skill_relations, OrphanRelationReport, OrphanRepairReport,
};
pub use repos::skills_repo::*;
pub use repos::tag_groups_repo::*;
pub use repos::tags_repo::*;
pub use repos::update_inventory_repo::*;
pub use repos::update_states_repo::*;
pub use repos::usage_repo::*;
pub use seed::*;
pub use types::*;
