//! Skills CLI upstream update detection and journaled apply.
//!
//! Product argv never includes `--force`, `--keep-links`, or an unverified
//! full-SHA `skills add` source. Direct-copy refresh is UNVERIFIED and is
//! blocked before journal/spawn. Apply refreshes owned canonical files from a
//! pinned GitHub snapshot acquired over HTTP.

mod apply;
mod capability;
mod detect;
mod digest;
mod github;
mod source;
mod status;

#[cfg(test)]
mod tests;

pub(crate) use apply::{apply_updates, retry_update_recovery};
#[cfg(test)]
pub(crate) use apply::{set_apply_fault, ApplyFault};
pub use capability::{
    apply_argv_preview, update_capability_plan, CapabilitySupport, SkillsCliUpdateCapabilityPlan,
};
pub(crate) use detect::{check_updates, verify_update_baseline};
pub(crate) use digest::parse_remote_skill_hash_output;
pub(crate) use github::ProductionSkillsCliGithub;
#[cfg(test)]
pub(crate) use github::{FakeSkillsCliGithub, GithubObserveResult};

use serde::{Deserialize, Serialize};

use super::SkillsCliError;
use crate::db::DbPool;

pub const UPDATE_PROGRESS_EVENT: &str = "skills-cli://update-progress";
pub const UPDATE_LOCK_OPERATION: &str = "Skills CLI update apply";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SkillsCliUpdateStatus {
    NotChecked,
    Checking,
    Current,
    UpdateAvailable,
    LocalModified,
    BaselineRequired,
    Unsupported,
    RateLimited,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliUpdateBlocker {
    pub code: String,
    pub skill_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliUpdateSkillRow {
    pub skill_name: String,
    pub repository_key: Option<String>,
    pub normalized_source: Option<String>,
    pub skill_path: Option<String>,
    pub status: SkillsCliUpdateStatus,
    pub installed_revision_sha: Option<String>,
    pub observed_revision_sha: Option<String>,
    pub pending_revision_sha: Option<String>,
    pub installed_local_digest: Option<String>,
    pub observed_upstream_digest: Option<String>,
    pub pending_upstream_digest: Option<String>,
    pub is_stale: bool,
    pub last_error_code: Option<String>,
    pub change_summary: Vec<String>,
    pub blockers: Vec<SkillsCliUpdateBlocker>,
    pub argv_preview: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliUpdateRepositoryRow {
    pub repository_key: String,
    pub normalized_source: String,
    pub branch: String,
    pub observed_revision_sha: Option<String>,
    pub status: String,
    pub last_checked_at: Option<String>,
    pub last_error_code: Option<String>,
    pub rate_limit_reset_at: Option<String>,
    pub pending_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPendingRecovery {
    pub operation_id: String,
    pub phase: String,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliUpdateInventory {
    pub skills: Vec<SkillsCliUpdateSkillRow>,
    pub repositories: Vec<SkillsCliUpdateRepositoryRow>,
    pub last_success_at: Option<String>,
    pub pending_recovery: Option<SkillsCliPendingRecovery>,
    pub capability: SkillsCliUpdateCapabilityPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliUpdateProgress {
    pub job_id: String,
    pub phase: String,
    pub repository_total: u32,
    pub repository_completed: u32,
    pub current_repository_key: Option<String>,
    pub selected_total: u32,
    pub selected_completed: u32,
    pub terminal_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliApplySelection {
    pub skill_name: String,
    pub skill_path: String,
    pub expected_installed_revision: Option<String>,
    pub expected_installed_local_digest: Option<String>,
    pub expected_pending_revision: String,
    pub expected_pending_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliApplyUpdateRequest {
    pub job_id: String,
    pub repository_key: String,
    pub selections: Vec<SkillsCliApplySelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliApplyResult {
    pub applied_skill_names: Vec<String>,
    pub installed_revision_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliApplyRecoveryResult {
    pub operation_id: String,
    pub phase: String,
}

pub trait UpdateProgressEmitter: Send + Sync {
    fn emit_update_progress(&self, payload: &SkillsCliUpdateProgress);
}

pub struct NoopProgress;

impl UpdateProgressEmitter for NoopProgress {
    fn emit_update_progress(&self, _payload: &SkillsCliUpdateProgress) {}
}

pub fn map_db_error(_error: sqlx::Error) -> SkillsCliError {
    tracing::warn!("Skills CLI update database error");
    SkillsCliError::UpdateMigration
}

pub(crate) async fn load_update_inventory_for_pool(
    pool: &DbPool,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    detect::load_update_inventory(pool).await
}
