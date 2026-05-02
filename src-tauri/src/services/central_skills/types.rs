use serde::{Deserialize, Serialize};

use crate::db::{Collection, SkillRepository, SkillRepositoryWithStats, SkillTag};

/// A Central Skill with a list of agent IDs that currently have this skill
/// installed (via symlink or copy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillWithLinks {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub canonical_path: Option<String>,
    pub is_central: bool,
    pub source: Option<String>,
    pub scanned_at: String,
    pub created_at: String,
    pub updated_at: String,
    /// Agent IDs that have an installation record for this skill.
    pub linked_agents: Vec<String>,
    /// Agent IDs that use the Central skills directory as their own root.
    pub shared_root_agents: Vec<String>,
    pub repository: Option<SkillRepository>,
    pub tags: Vec<SkillTag>,
    pub source_path: Option<String>,
    pub is_source_unknown: bool,
}

/// An installation record enriched with the `installed_at` timestamp for
/// the skill detail IPC response. This is the frontend-facing version of
/// `db::SkillInstallation` — `created_at` from the DB is exposed as
/// `installed_at` for clarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstallationDetail {
    pub skill_id: String,
    pub agent_id: String,
    pub installed_path: String,
    pub link_type: String,
    pub symlink_target: Option<String>,
    /// ISO 8601 timestamp of when the skill was first installed.
    pub installed_at: String,
}

/// A skill with full installation details across all platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    pub id: String,
    pub row_id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub canonical_path: Option<String>,
    pub is_central: bool,
    pub source: Option<String>,
    pub scanned_at: String,
    pub source_kind: Option<String>,
    pub source_root: Option<String>,
    pub is_read_only: bool,
    pub conflict_group: Option<String>,
    pub conflict_count: i64,
    /// All installation records for this skill across agents.
    pub installations: Vec<SkillInstallationDetail>,
    /// Collections this skill currently belongs to.
    pub collections: Vec<Collection>,
    pub repository: Option<SkillRepository>,
    pub tags: Vec<SkillTag>,
    pub source_path: Option<String>,
    pub is_source_unknown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCentralSkillResult {
    pub removed_central_path: String,
    pub removed_agent_ids: Vec<String>,
    pub retained_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCentralSkillPreview {
    pub skill_id: String,
    pub skill_name: String,
    pub central_path: String,
    pub copy_installations: Vec<SkillInstallationDetail>,
    pub auto_removed_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCentralSkillDelete {
    pub skill_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillPreviewResult {
    pub previews: Vec<DeleteCentralSkillPreview>,
    pub failed: Vec<FailedCentralSkillDelete>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillRequest {
    pub skill_id: String,
    pub remove_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillSuccess {
    pub skill_id: String,
    pub removed_central_path: String,
    pub removed_agent_ids: Vec<String>,
    pub retained_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillResult {
    pub succeeded: Vec<BatchDeleteCentralSkillSuccess>,
    pub failed: Vec<FailedCentralSkillDelete>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSkillRepositoryPreview {
    pub repository: SkillRepositoryWithStats,
    pub delete_preview: BatchDeleteCentralSkillPreviewResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSkillRepositoryResult {
    pub repository: SkillRepository,
    pub deleted_repository: bool,
    pub delete_result: BatchDeleteCentralSkillResult,
}
