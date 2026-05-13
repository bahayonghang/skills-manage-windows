//! Repository layer — Phase 2c.
//!
//! Each submodule owns the SQL for one or a small group of related tables.
//! Functions are re-exported through `db/mod.rs` so existing
//! `use crate::db::{...}` call sites in `commands/*` keep working without
//! per-call-site edits during the cutover.

pub(crate) mod agents_repo;
pub(crate) mod collections_repo;
pub(crate) mod discovered_repo;
pub(crate) mod installations_repo;
pub(crate) mod observations_repo;
pub(crate) mod operation_logs_repo;
pub(crate) mod projects_repo;
pub(crate) mod repositories_repo;
pub(crate) mod saved_views_repo;
pub(crate) mod scan_dirs_repo;
pub(crate) mod settings_repo;
pub(crate) mod skills_repo;
pub(crate) mod tag_groups_repo;
pub(crate) mod tags_repo;
pub(crate) mod update_states_repo;
