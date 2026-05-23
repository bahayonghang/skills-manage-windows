use serde::{Deserialize, Serialize};

use crate::commands::central_updates;
use crate::db::SkillUpdateState;
use crate::services::central_skills::BatchDeleteCentralSkillRequest;

/*
 * ========================================================================
 * 类型定义
 * ========================================================================
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRefreshScope {
    pub kind: SkillRefreshScopeKind,
    #[serde(default)]
    pub skill_ids: Option<Vec<String>>,
    #[serde(default)]
    pub repository_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillRefreshScopeKind {
    All,
    Skills,
    Repositories,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInventory {
    pub updatable: Vec<UpdatableSkill>,
    pub remote_added: Vec<RemoteAddedSkill>,
    pub remote_missing: Vec<RemoteMissingSkill>,
    pub platform_duplicates: Vec<PlatformDuplicateGroup>,
    /// Phase P2 始终空，留位给后续 orphan 扫描（broken symlink / 孤儿 .copy 目录）。
    pub orphans: Vec<OrphanSkillEntry>,
    pub failed_repositories: Vec<FailedRepository>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatableSkill {
    pub state: SkillUpdateState,
    pub repository_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAddedSkill {
    pub repository_id: String,
    pub source_path: String,
    pub skill_id: String,
    pub skill_name: String,
    pub conflict_existing_skill_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMissingSkill {
    pub state: SkillUpdateState,
    pub repository_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDuplicateGroup {
    pub agent_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub writable_paths: Vec<String>,
    pub plugin_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanSkillEntry {
    pub skill_id: String,
    pub broken_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedRepository {
    pub repository_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateDecisions {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDuplicateRemoval {
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
    pub failures: Vec<SkillUpdateApplyFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateApplyFailure {
    pub step: String,
    pub identifier: String,
    pub error: String,
}
