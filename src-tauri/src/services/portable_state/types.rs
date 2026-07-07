use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{atomic::AtomicBool, Arc};

use crate::services::github_import::{DuplicateResolution, GitHubSkillImportSelection};

pub(crate) const EXPORT_KIND: &str = "skillport/state-export";
pub(crate) const EXPORT_VERSION: u32 = 1;
pub(crate) const REMOTE_CATALOG_CONCURRENCY_LIMIT: usize = 4;
pub(crate) const PORTABILITY_PROGRESS_EVENT: &str = "central://state-portability-progress";
pub(crate) const PORTABILITY_CANCELLED_MESSAGE: &str = "SkillPort state portability cancelled";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateExportOptions {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateManifest {
    pub kind: String,
    pub version: u32,
    pub exported_at: String,
    pub exported_from: ExportedFrom,
    pub github_sources: Vec<PortableGithubSource>,
    pub central_skills: Vec<PortableCentralSkill>,
    pub unrestorable_skills: Vec<PortableUnrestorableSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedFrom {
    pub app: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ExportedTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedTarget {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PortableStateTargetContext {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) label: String,
}

impl From<&PortableStateTargetContext> for ExportedTarget {
    fn from(target: &PortableStateTargetContext) -> Self {
        Self {
            id: target.id.clone(),
            kind: target.kind.clone(),
            label: target.label.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableGithubSource {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableCentralSkill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: PortableCentralSkillSource,
    pub tags: Vec<PortableSkillTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableCentralSkillSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub url: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableSkillTag {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableUnrestorableSkill {
    pub id: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportPreview {
    pub github_sources: Vec<SkillportStateSourcePreview>,
    pub skills: Vec<SkillportStateSkillPreview>,
    pub summary: SkillportStateImportPreviewSummary,
    pub warnings: Vec<SkillportStateImportPreviewWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportPreviewWarning {
    pub reason: String,
    pub detail: String,
    pub source_path: Option<String>,
    pub repo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateSourcePreview {
    pub name: String,
    pub url: String,
    pub status: SourcePreviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourcePreviewStatus {
    Exists,
    WillAdd,
    Duplicate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateSkillPreview {
    pub id: String,
    pub name: String,
    pub source_path: Option<String>,
    pub status: SkillPreviewStatus,
    pub existing_skill_id: Option<String>,
    pub reason: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillPreviewStatus {
    Ready,
    Conflict,
    Missing,
    Unrestorable,
    DuplicateSkipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportPreviewSummary {
    pub sources_to_add: usize,
    pub sources_existing: usize,
    pub sources_duplicate: usize,
    pub ready: usize,
    pub conflicts: usize,
    pub missing: usize,
    pub unrestorable: usize,
    pub duplicate_skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportResolution {
    pub skill_id: String,
    pub source_path: Option<String>,
    pub resolution: DuplicateResolution,
    pub renamed_skill_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportResult {
    pub sources_added: usize,
    pub sources_skipped: usize,
    pub imported_skills: Vec<SkillportStateImportedSkill>,
    pub skipped_skills: Vec<String>,
    pub failed_skills: Vec<SkillportStateImportFailure>,
    pub tags_restored: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportedSkill {
    pub source_path: String,
    pub imported_skill_id: String,
    pub skill_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportFailure {
    pub skill_id: String,
    pub source_path: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillportStatePortabilityPhase {
    Exporting,
    Previewing,
    Importing,
    Finalizing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillportStatePortabilityStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelling,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStatePortabilityProgressPayload {
    pub phase: SkillportStatePortabilityPhase,
    pub status: SkillportStatePortabilityStatus,
    pub total: usize,
    pub completed: usize,
    pub message: Option<String>,
    pub current_item: Option<String>,
    pub error: Option<String>,
}

pub(crate) type CancelFlag = Arc<AtomicBool>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RepoKey {
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) branch: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportGroup {
    pub(crate) repo_url: String,
    pub(crate) selections: Vec<GitHubSkillImportSelection>,
}

impl ImportGroup {
    pub(crate) fn selected_paths(&self) -> Vec<String> {
        self.selections
            .iter()
            .map(|selection| selection.source_path.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RemoteCatalogEntry {
    pub(crate) valid_source_paths: HashSet<String>,
    pub(crate) invalid_candidates: HashMap<String, RemoteCatalogInvalidCandidate>,
    pub(crate) repo_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteCatalogInvalidCandidate {
    pub(crate) reason: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SkillManifestKey {
    pub(crate) id: String,
    pub(crate) source_path: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PortabilityProgressUpdate<'a> {
    pub(crate) phase: SkillportStatePortabilityPhase,
    pub(crate) status: SkillportStatePortabilityStatus,
    pub(crate) total: usize,
    pub(crate) completed: usize,
    pub(crate) message: Option<&'a str>,
    pub(crate) current_item: Option<&'a str>,
    pub(crate) error: Option<&'a str>,
}
