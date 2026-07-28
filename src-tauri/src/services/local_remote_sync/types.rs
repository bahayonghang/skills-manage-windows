use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemoteSyncPreviewRequest {
    pub target_id: String,
    #[serde(default)]
    pub repo_path: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemoteSyncApplyRequest {
    pub target_id: String,
    #[serde(default)]
    pub repo_path: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LocalRemoteSyncItemStatus {
    Add,
    Update,
    Skip,
    Error,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LocalRemoteSyncItemKind {
    Repo,
    Skill,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemoteSyncItemPreview {
    pub id: String,
    pub label: String,
    pub kind: LocalRemoteSyncItemKind,
    pub local_path: String,
    pub remote_path: String,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub file_count: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub byte_count: u64,
    pub local_hash: String,
    pub remote_hash: Option<String>,
    pub status: LocalRemoteSyncItemStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemoteSyncPreview {
    pub target_id: String,
    pub target_label: String,
    pub repo_root: String,
    pub skills_root: String,
    pub repo: LocalRemoteSyncItemPreview,
    pub skills: Vec<LocalRemoteSyncItemPreview>,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub total_file_count: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub total_byte_count: u64,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemoteSyncFailure {
    pub id: String,
    pub label: String,
    pub target_path: String,
    pub error: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemoteSyncApplyResult {
    pub target_id: String,
    pub target_label: String,
    pub synced_repo: Option<LocalRemoteSyncItemPreview>,
    pub synced_skills: Vec<LocalRemoteSyncItemPreview>,
    pub skipped_skills: Vec<LocalRemoteSyncItemPreview>,
    pub failed: Vec<LocalRemoteSyncFailure>,
}

#[derive(Debug, Clone)]
pub struct SnapshotFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LocalSnapshot {
    pub id: String,
    pub label: String,
    pub root: PathBuf,
    pub files: Vec<SnapshotFile>,
    pub file_count: usize,
    pub byte_count: u64,
    pub hash: String,
}
