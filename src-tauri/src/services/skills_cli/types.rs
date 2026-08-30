//! Skills CLI IPC payload types shared by commands and the service.

use serde::{Deserialize, Serialize};

// ─── IPC payload types ───────────────────────────────────────────────────────

/// Result of `skills_cli_doctor`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliDoctorReport {
    pub node_version: String,
    pub npm_spec: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum SkillsCliInstallKind {
    Canonical,
    Copy,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum SkillsCliSourceTypeBucket {
    Github,
    Gitlab,
    Git,
    Mintlify,
    Huggingface,
    Local,
    WellKnown,
    Unknown,
}

/// One global skill projected from lock v3 + filesystem (no CLI spawn).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliGlobalSkill {
    pub name: String,
    pub path: Option<String>,
    pub install_kind: SkillsCliInstallKind,
    pub scope: Option<String>,
    pub agents: Vec<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub source_type: Option<String>,
    pub source_type_bucket: SkillsCliSourceTypeBucket,
    pub canonical_path: Option<String>,
    pub folder_hash: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: Option<String>,
    pub placements: Vec<SkillsCliPlacement>,
}

/// Lock + filesystem snapshot returned by `skills_cli_list_global`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliGlobalSnapshot {
    pub skills: Vec<SkillsCliGlobalSkill>,
    pub canonical_root: String,
    pub lock_path: String,
}

/// One detected, mappable Local platform offered by the install flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliInstallTarget {
    pub id: String,
    pub display_name: String,
    pub icon_name: Option<String>,
    /// CLI `--agent` id this platform maps to.
    pub cli_agent: String,
    /// SkillPort enablement state; drives the default selection.
    pub is_enabled: bool,
    pub default_selected: bool,
}

/// Parsed result of `skills add <source> --list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliSourcePreview {
    pub source: String,
    pub skills: Vec<String>,
}

/// Summary of a completed global install.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliAddResult {
    pub installed_skills: u32,
    pub targeted_platforms: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SkillsCliPlacementState {
    ManagedLink,
    DirectCopy,
    Missing,
    Conflict,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SkillsCliManagedLinkKind {
    WindowsJunction,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacement {
    pub agent_id: String,
    pub display_name: String,
    pub target_path: String,
    pub state: SkillsCliPlacementState,
    pub managed_link_kind: Option<SkillsCliManagedLinkKind>,
    pub reason_code: Option<String>,
    /// Always `None` on Remote. Local platform origin lives on `SkillForAgent`.
    pub install_origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliSkillDoc {
    pub skill_name: String,
    pub content: String,
    pub byte_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliRemovePlacementSummary {
    pub agent_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacementConflict {
    pub agent_id: String,
    pub display_name: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliRemovePlan {
    pub skill_name: String,
    pub owned_canonical: bool,
    pub managed_placements: Vec<SkillsCliRemovePlacementSummary>,
    pub retained_direct_copies: Vec<SkillsCliRemovePlacementSummary>,
    pub conflicts: Vec<SkillsCliPlacementConflict>,
    pub confirmable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliRemoveResult {
    pub removed_canonical: bool,
    pub removed_managed_agent_ids: Vec<String>,
    pub retained_direct_copy_agent_ids: Vec<String>,
}

/// Batch result for Skills CLI link/unlink. Remote callers must use this
/// entry so round-trips stay `ceil(N / K) + C` instead of N handshakes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacementMutationOutcome {
    pub succeeded: Vec<SkillsCliPlacementMutationItem>,
    pub failed: Vec<SkillsCliPlacementMutationFailure>,
    pub skipped: Vec<SkillsCliPlacementMutationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacementMutationItem {
    pub skill_name: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacementMutationFailure {
    pub skill_name: String,
    pub agent_id: String,
    pub error_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacementBatchItem {
    pub skill_name: String,
    pub skillport_agent_id: String,
}
