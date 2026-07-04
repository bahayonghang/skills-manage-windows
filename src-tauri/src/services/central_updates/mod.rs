//! Central updates service layer (Update Center).
//!
//! Owns Central skill update checks/updates, the GitHub snapshot cache, and
//! the local/remote filesystem façade used to hash and atomically replace
//! skill directories. Tauri IPC shells live in
//! `crate::commands::central_updates` and
//! `crate::commands::skill_update_inventory`.

mod core;
mod error;
pub(crate) mod fs;
mod snapshots;
mod types;

pub use error::CentralUpdatesError;
pub use snapshots::CentralUpdateSnapshotCache;
pub use types::{
    CentralSkillUpdateFailure, CentralSkillUpdateProgressPayload, CentralSkillUpdateResult,
    CentralSkillUpdateSkip, SkillUpdateStatus,
};

pub(crate) use core::{
    check_central_skill_updates_impl, emit_update_progress, error_state_from_assignment,
    get_central_skill_update_states_impl, keep_remote_missing_central_skills_impl,
    load_remote_skill_content, load_selected_central_skills, prepare_skill_updates,
    state_from_relocated_source, state_from_remote, remote_missing_state_from_assignment,
    unsupported_state_from_assignment, update_central_skills_impl, update_counters_for_state,
    update_one_skill, update_one_skill_with_options,
};
pub(crate) use fs::{normalize_repo_path, CentralFs};
pub(crate) use snapshots::{
    prepare_snapshots_for_repo_refs, prepare_snapshots_for_repo_refs_with_policy, repo_cache_key,
};
pub(crate) use types::{
    PreparedSkillUpdate, RemoteSkillLoadError, SnapshotCachePolicy, UpdateCounters,
};
