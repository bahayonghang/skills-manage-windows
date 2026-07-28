use serde::{Deserialize, Serialize};

use crate::db::SkillUpdateState;
use crate::services::central_skills::{
    BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult,
};
use crate::services::central_updates;
use crate::services::github_import::ImportedGitHubSkillSummary;

/*
 * ========================================================================
 * 类型定义
 * ========================================================================
 */

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRefreshScope {
    pub kind: SkillRefreshScopeKind,
    #[serde(default)]
    pub mode: Option<SkillRefreshMode>,
    #[serde(default)]
    pub cache_policy: Option<SkillRefreshCachePolicy>,
    #[serde(default)]
    pub skill_ids: Option<Vec<String>>,
    #[serde(default)]
    pub repository_ids: Option<Vec<String>>,
    #[serde(default)]
    pub agent_ids: Option<Vec<String>>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillRefreshScopeKind {
    All,
    Skills,
    Repositories,
    Platform,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillRefreshMode {
    Regular,
    Sync,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillRefreshCachePolicy {
    UseFresh,
    Bypass,
}

impl SkillRefreshCachePolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UseFresh => "use_fresh",
            Self::Bypass => "bypass",
        }
    }
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInventory {
    pub updatable: Vec<UpdatableSkill>,
    pub remote_added: Vec<RemoteAddedSkill>,
    pub remote_missing: Vec<RemoteMissingSkill>,
    pub platform_duplicates: Vec<PlatformDuplicateGroup>,
    #[serde(default)]
    // Keep serde(default) deserialize-only in Specta's phased metadata.
    #[cfg_attr(
        feature = "ipc-codegen",
        serde(rename(deserialize = "deletedPlatformCopies"))
    )]
    pub deleted_platform_copies: Vec<DeletedPlatformCopyGroup>,
    /// Phase P2 始终空，留位给后续 orphan 扫描（broken symlink / 孤儿 .copy 目录）。
    pub orphans: Vec<OrphanSkillEntry>,
    pub failed_repositories: Vec<FailedRepository>,
    pub generated_at: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatableSkill {
    pub state: SkillUpdateState,
    pub repository_id: Option<String>,
    #[serde(default)]
    pub diagnostics: Option<SkillUpdateDiagnostic>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAddedSkill {
    pub repository_id: String,
    pub source_path: String,
    pub skill_id: String,
    pub skill_name: String,
    pub conflict_existing_skill_id: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMissingSkill {
    pub state: SkillUpdateState,
    pub repository_id: Option<String>,
    #[serde(default)]
    pub diagnostics: Option<SkillUpdateDiagnostic>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDuplicateGroup {
    pub agent_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub writable_paths: Vec<String>,
    pub plugin_paths: Vec<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedPlatformCopyGroup {
    pub agent_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub writable_paths: Vec<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanSkillEntry {
    pub skill_id: String,
    pub broken_path: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedRepository {
    pub repository_id: String,
    pub error: String,
    #[serde(default)]
    pub diagnostics: Option<SkillUpdateDiagnostic>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateDiagnostic {
    pub source_url: Option<String>,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub source_path: Option<String>,
    pub local_hash: Option<String>,
    pub baseline_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub cache_policy: SkillRefreshCachePolicy,
    pub cache_hit: bool,
    pub snapshot_fetched_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateDecisions {
    #[serde(default)]
    pub allowed_agent_ids: Option<Vec<String>>,
    #[serde(default)]
    pub updates: Vec<String>,
    #[serde(default)]
    pub keep_missing: Vec<String>,
    #[serde(default)]
    pub delete_missing: Vec<BatchDeleteCentralSkillRequest>,
    #[serde(default)]
    pub import_additions: Vec<central_updates::CentralRepositoryAddedSkillSelection>,
    #[serde(default)]
    pub skip_additions: Vec<central_updates::CentralRepositoryAdditionSkipRequest>,
    #[serde(default)]
    pub unskip_additions: Vec<central_updates::CentralRepositoryAdditionUnskipRequest>,
    #[serde(default)]
    pub remove_platform_duplicates: Vec<PlatformDuplicateRemoval>,
    #[serde(default)]
    pub remove_deleted_platform_copies: Vec<DeletedPlatformCopyRemoval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDuplicateRemoval {
    pub agent_id: String,
    pub skill_id: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedPlatformCopyRemoval {
    pub agent_id: String,
    pub skill_id: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateApplyResult {
    pub updated_skill_ids: Vec<String>,
    pub kept_missing_skill_ids: Vec<String>,
    pub deleted_skill_ids: Vec<String>,
    pub imported_skill_ids: Vec<String>,
    pub skipped_additions: Vec<String>,
    pub unskipped_additions: Vec<String>,
    pub removed_platform_duplicate_paths: Vec<String>,
    pub removed_deleted_platform_copy_paths: Vec<String>,
    pub failures: Vec<SkillUpdateApplyFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateApplyFailure {
    pub step: String,
    pub identifier: String,
    pub error: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateRequest {
    pub skill_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub refresh_copy_installations: bool,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateResult {
    pub overwritten: Vec<ForceSkillUpdateSuccess>,
    pub skipped: Vec<ForceSkillUpdateSkip>,
    pub failed: Vec<ForceSkillUpdateFailure>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateSuccess {
    pub skill_id: String,
    pub repository_id: Option<String>,
    pub source_path: Option<String>,
    pub local_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub bytes_changed: bool,
    pub copy_installations_refreshed: bool,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateSkip {
    pub skill_id: String,
    pub reason: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateFailure {
    pub skill_id: String,
    pub repository_id: Option<String>,
    pub source_path: Option<String>,
    pub error: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceRepositoryMirrorRequest {
    pub repository_ids: Vec<String>,
    #[serde(default)]
    pub delete_missing: bool,
    #[serde(default)]
    pub import_added: bool,
    #[serde(default)]
    pub overwrite_tracked: bool,
    #[serde(default = "default_true")]
    pub remove_copy_installations_for_deleted: bool,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceRepositoryMirrorResult {
    pub overwritten: Vec<ForceSkillUpdateSuccess>,
    pub imported: Vec<ImportedGitHubSkillSummary>,
    pub deleted: BatchDeleteCentralSkillResult,
    pub skipped: Vec<ForceSkillUpdateSkip>,
    pub failed_repositories: Vec<FailedRepository>,
    pub failed_items: Vec<ForceSkillUpdateFailure>,
}

fn default_true() -> bool {
    true
}
