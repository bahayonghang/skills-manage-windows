use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleTool {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "omp")]
    Omp,
}

impl RuleTool {
    pub const ALL: [Self; 2] = [Self::ClaudeCode, Self::Omp];
    pub fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Omp => "omp",
        }
    }
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleTargetState {
    Linked,
    Absent,
    NativeEquivalent,
    Conflict,
    Broken,
    Unreadable,
    Unsupported,
    RecoveryRequired,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTargetInfo {
    pub tool: RuleTool,
    pub path: String,
    pub supported: bool,
    pub error_code: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTargetStatus {
    pub tool: RuleTool,
    pub path: String,
    pub state: RuleTargetState,
    pub fingerprint: Option<String>,
    pub error_code: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleCompatibility {
    Supported,
    Unsupported,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSummary {
    pub name: String,
    pub title: String,
    pub description: String,
    pub bytes: u32,
    pub revision: String,
    pub compatibility: RuleCompatibility,
    pub targets: Vec<RuleTargetStatus>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDetail {
    pub name: String,
    pub title: String,
    pub description: String,
    pub bytes: u32,
    pub revision: String,
    pub compatibility: RuleCompatibility,
    pub targets: Vec<RuleTargetStatus>,
    pub body: String,
    pub source: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesSnapshot {
    pub root_path: String,
    pub targets: Vec<RuleTargetInfo>,
    pub rules: Vec<RuleSummary>,
    pub recovery_operations: Vec<RuleRecoveryOperation>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSourceFingerprint {
    pub tool: RuleTool,
    pub fingerprint: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleImportStatus {
    New,
    Same,
    Conflict,
    Unsupported,
    Failed,
    Imported,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleImportPreviewEntry {
    pub name: String,
    pub status: RuleImportStatus,
    pub source_fingerprints: Vec<RuleSourceFingerprint>,
    pub description: String,
    pub error_code: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesImportPreview {
    pub entries: Vec<RuleImportPreviewEntry>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleImportEntry {
    pub name: String,
    pub source_fingerprints: Vec<RuleSourceFingerprint>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleImportResultEntry {
    pub name: String,
    pub status: RuleImportStatus,
    pub error_code: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesImportResult {
    pub entries: Vec<RuleImportResultEntry>,
    pub operation_id: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleOperationKind {
    Enable,
    Disable,
    Delete,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRecoveryOperation {
    pub operation_id: String,
    pub name: String,
    pub kind: RuleOperationKind,
    pub phase: String,
    pub tool: Option<RuleTool>,
    pub backup_path: Option<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleMutationResult {
    pub operation_id: String,
    pub name: String,
    pub recovery_required: bool,
}
