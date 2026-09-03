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
pub(crate) mod inventory;
mod repository_sync;
mod snapshots;
mod types;

pub use error::CentralUpdatesError;
pub use repository_sync::{
    CentralRemoteAddedSkill, CentralRemoteMissingSkill, CentralRepositoryAddedSkillSelection,
    CentralRepositoryAdditionSkipRequest, CentralRepositoryAdditionUnskipRequest,
    CentralRepositorySyncApplyResult, CentralRepositorySyncDecisions, CentralRepositorySyncFailure,
    CentralRepositorySyncPreview, CentralRepositorySyncSummary,
};
pub use snapshots::CentralUpdateSnapshotCache;
pub use types::{
    CentralSkillUpdateFailure, CentralSkillUpdateProgressPayload, CentralSkillUpdateResult,
    CentralSkillUpdateSkip, CentralUpdateFailurePhase, SkillUpdateStatus,
};

pub(crate) use core::{
    check_central_skill_updates_impl, get_central_skill_update_states_impl,
    journaled_central_content_upsert, journaled_central_content_upsert_with_fs,
    keep_remote_missing_central_skills_impl, load_remote_skill_content, prepare_skill_updates,
    recover_pending_update_operation, recover_pending_update_operations,
    state_from_relocated_source, state_from_remote, update_central_skills_impl,
    update_skills_batch, JournaledCentralContentUpsert, SkillUpdatePlan,
};
pub(crate) use fs::{normalize_repo_path, CentralFs};
pub(crate) use repository_sync::{
    apply_central_repository_sync_impl, check_central_repository_sync_impl,
    collect_remote_added_skills,
};
pub(crate) use snapshots::{
    prepare_snapshots_for_repo_refs_with_policy, repo_cache_key, SnapshotProgressEvent,
    SnapshotProgressReporter, SnapshotProgressStatus,
};
pub(crate) use types::{PreparedSkillUpdate, RemoteSkillLoadError, SnapshotCachePolicy};
