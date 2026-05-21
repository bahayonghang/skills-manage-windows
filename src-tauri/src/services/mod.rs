//! Service layer: business logic separated from Tauri command shells.
//!
//! Each service module owns the orchestration / domain logic for a slice
//! of functionality. `commands/*` modules call into services and translate
//! results into IPC responses (status, log records, error formatting).

pub mod ai_provider;
pub mod ai_tagging;
pub mod central_skills;
pub mod github_import;
pub mod installation;
pub mod local_remote_sync;
pub mod marketplace;
pub mod obsidian;
pub mod portable_state;
pub mod projects;
pub mod scanner;
