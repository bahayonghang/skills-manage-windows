//! Repository layer.
//!
//! Each submodule owns the SQL for one or a small group of related tables.
//! Public helpers are re-exported through `db/mod.rs` as a compatibility
//! surface for existing `use crate::db::{...}` call sites. Prefer adding new
//! SQL here and gradually migrating services to narrower repo imports.

pub(crate) mod agents_repo;
pub(crate) mod central_skills_page_repo;
pub(crate) mod collections_repo;
pub(crate) mod fs_db_operations_repo;
pub(crate) mod installations_repo;
pub(crate) mod observations_repo;
pub(crate) mod operation_logs_repo;
pub(crate) mod pending_additions_repo;
pub(crate) mod projects_repo;
pub(crate) mod repositories_repo;
pub(crate) mod repository_members_repo;
pub(crate) mod saved_views_repo;
pub(crate) mod scan_dirs_repo;
pub(crate) mod settings_repo;
pub(crate) mod skill_relations_repo;
pub(crate) mod skill_relations_spec;
pub(crate) mod skills_repo;
pub(crate) mod tag_groups_repo;
pub(crate) mod tags_repo;
pub(crate) mod update_inventory_repo;
pub(crate) mod update_states_repo;
pub(crate) mod usage_file_cache_repo;
pub(crate) mod usage_repo;
pub(crate) mod usage_stats_repo;
pub(crate) mod usage_unused_repo;
