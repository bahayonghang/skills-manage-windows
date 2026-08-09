use serde::{Deserialize, Serialize};

use crate::db::{Collection, SkillRepository, SkillRepositoryWithStats, SkillTag};

/// A Central Skill with a list of agent IDs that currently have this skill
/// installed (via symlink or copy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillWithLinks {
    pub id: String,
    pub uid: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillsPageRequest {
    pub query: Option<String>,
    #[serde(default, alias = "sources", alias = "repos")]
    pub source: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub install_state: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillsPage {
    pub items: Vec<SkillWithLinks>,
    pub total: usize,
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
    pub uid: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SkillRef {
    Uid(String),
    Slug(String),
    Name(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryTreeEntry {
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub symlink_target: Option<String>,
    pub children: Vec<DirectoryTreeEntry>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
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

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCentralSkillDelete {
    pub skill_id: String,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_category: Option<String>,
    #[serde(serialize_with = "serialize_public_delete_error")]
    #[cfg_attr(feature = "ipc-codegen", specta(type = String))]
    pub error: String,
}

impl FailedCentralSkillDelete {
    pub(crate) fn preview_fallback(skill_id: String) -> Self {
        Self {
            skill_id,
            phase: Some("prepare".to_string()),
            error_code: Some("central_skills.delete_preview_failed".to_string()),
            error_category: Some("central_skills.validation".to_string()),
            error: "This Central skill could not be deleted.".to_string(),
        }
    }

    pub(crate) fn from_error(
        skill_id: String,
        phase: &'static str,
        error: &super::error::CentralSkillsError,
    ) -> Self {
        Self {
            skill_id,
            phase: Some(phase.to_string()),
            error_code: Some(error.stable_delete_error_code()),
            error_category: Some(error.diagnostic_category().to_string()),
            error: error.public_delete_message().to_string(),
        }
    }
}

fn serialize_public_delete_error<S>(_: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str("This Central skill could not be deleted.")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillPreviewResult {
    pub previews: Vec<DeleteCentralSkillPreview>,
    pub failed: Vec<FailedCentralSkillDelete>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillRequest {
    pub skill_id: String,
    pub remove_agent_ids: Vec<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillSuccess {
    pub skill_id: String,
    pub removed_central_path: String,
    pub removed_agent_ids: Vec<String>,
    pub retained_agent_ids: Vec<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
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

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSkillRepositoryResult {
    pub repository: SkillRepository,
    pub deleted_repository: bool,
    pub delete_result: BatchDeleteCentralSkillResult,
}
