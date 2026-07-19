//! Service layer: business logic separated from Tauri command shells.
//!
//! Each service module owns the orchestration / domain logic for a slice
//! of functionality. `commands/*` modules call into services and translate
//! results into IPC responses (status, log records, error formatting).

pub mod ai_provider;
pub mod ai_tagging;
pub mod central_mutation;
pub mod central_skills;
pub mod central_store_location;
pub mod central_updates;
pub mod deep_link;
pub mod github_import;
pub mod installation;
pub mod local_archive_import;
pub mod local_remote_sync;
pub mod marketplace;
pub mod obsidian;
pub mod portable_state;
pub mod projects;
pub mod resource_budget;
pub mod scanner;
pub mod usage;
