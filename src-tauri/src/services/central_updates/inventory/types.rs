use serde::{Deserialize, Serialize};

use crate::db::SkillUpdateState;
use crate::services::central_skills::{
    BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult, FailedCentralSkillDelete,
};
use crate::services::central_updates;
use crate::services::github_import::ImportedGitHubSkillSummary;

pub use crate::services::central_updates::types::UnsupportedSkillReasonCode;

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
    #[serde(default)]
    // Keep serde(default) deserialize-only in Specta's phased metadata.
    #[cfg_attr(feature = "ipc-codegen", serde(rename(deserialize = "unsupported")))]
    pub unsupported: Vec<UnsupportedSkill>,
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
    #[serde(default)]
    pub snapshot_retry_attempted: Option<u32>,
    #[serde(default)]
    pub snapshot_retry_recovered: Option<u32>,
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
pub struct UnsupportedSkill {
    pub skill_id: String,
    pub reason_code: UnsupportedSkillReasonCode,
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

/// What the Update Center may offer on a failed repository row.
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailedRepositoryRetry {
    /// Snapshot acquisition, relocation and addition-collection failures:
    /// running the same scope again can produce a different result.
    Retryable,
    /// The tracked source path is gone and no unique new path was found, so a
    /// user decision (keep or delete) is required in incremental mode.
    DecisionRequired,
    /// Entries persisted before this field existed. No in-place action.
    #[default]
    Unknown,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedRepository {
    pub repository_id: String,
    pub error: String,
    /// Stable IPC-style code for failures the domain classified, so the UI can
    /// localize the reason instead of showing backend English. `None` for the
    /// pre-existing reconciliation reasons that carry their own sentence, and
    /// for inventories persisted before this field existed.
    #[serde(default)]
    pub error_code: Option<String>,
    /// Static snapshot acquisition family. This is intentionally separate from
    /// the public code so transport subtypes remain diagnosable without raw
    /// request, response, URL, or status detail.
    #[serde(default)]
    pub diagnostic_category: Option<String>,
    #[serde(default)]
    pub retry: FailedRepositoryRetry,
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
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(serialize_with = "serialize_public_apply_error")]
    pub error: String,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_category: Option<String>,
}

impl SkillUpdateApplyFailure {
    pub fn new(step: impl Into<String>, identifier: impl Into<String>) -> Self {
        let (step, error_code) = controlled_apply_step(step.into());
        Self {
            step: step.to_string(),
            identifier: safe_logical_identifier(identifier.into()),
            phase: Some("decision_apply".to_string()),
            error: "This update item could not be applied.".to_string(),
            error_code: Some(error_code.to_string()),
            error_category: Some("central_updates.item_failure".to_string()),
        }
    }

    pub(crate) fn from_central_update(failure: central_updates::CentralSkillUpdateFailure) -> Self {
        Self {
            step: "update".to_string(),
            identifier: safe_logical_identifier(failure.skill_id),
            phase: Some(
                failure
                    .phase
                    .unwrap_or(central_updates::CentralUpdateFailurePhase::DecisionApply)
                    .as_str()
                    .to_string(),
            ),
            error: failure.error,
            error_code: Some(
                failure
                    .error_code
                    .unwrap_or_else(|| "central_updates.update_failed".to_string()),
            ),
            error_category: Some(
                failure
                    .error_category
                    .unwrap_or_else(|| "central_updates.item_failure".to_string()),
            ),
        }
    }

    pub(crate) fn from_central_delete(failure: FailedCentralSkillDelete) -> Self {
        Self {
            step: "delete_missing".to_string(),
            identifier: safe_logical_identifier(failure.skill_id),
            phase: Some(
                failure
                    .phase
                    .unwrap_or_else(|| "decision_apply".to_string()),
            ),
            error: failure.error,
            error_code: Some(
                failure
                    .error_code
                    .unwrap_or_else(|| "central_updates.delete_missing_failed".to_string()),
            ),
            error_category: Some(
                failure
                    .error_category
                    .unwrap_or_else(|| "central_updates.item_failure".to_string()),
            ),
        }
    }

    pub(crate) fn from_central_delete_error(
        identifier: impl Into<String>,
        error: crate::services::central_skills::CentralSkillsError,
    ) -> Self {
        Self {
            step: "delete_missing".to_string(),
            identifier: safe_logical_identifier(identifier.into()),
            phase: Some(error.delete_failure_phase().to_string()),
            error: error.public_delete_message().to_string(),
            error_code: Some(error.stable_delete_error_code()),
            error_category: Some(error.diagnostic_category().to_string()),
        }
    }

    pub(crate) fn from_central_error(
        step: impl Into<String>,
        identifier: impl Into<String>,
        phase: central_updates::CentralUpdateFailurePhase,
        error: central_updates::CentralUpdatesError,
    ) -> Self {
        let (step, _) = controlled_apply_step(step.into());
        Self {
            step: step.to_string(),
            identifier: safe_logical_identifier(identifier.into()),
            phase: Some(phase.as_str().to_string()),
            error: error.public_update_message().to_string(),
            error_code: Some(error.stable_error_code()),
            error_category: Some(error.diagnostic_category().to_string()),
        }
    }

    pub(crate) fn from_github_import(
        repository_id: impl Into<String>,
        error: crate::services::github_import::GithubImportError,
    ) -> Self {
        let error_code = error
            .ipc_error_code()
            .unwrap_or("central_updates.import_addition_failed");
        let public_error = crate::ipc_error::public_message_for_code(error_code)
            .unwrap_or("This update item could not be applied.");
        Self {
            step: "import_addition".to_string(),
            identifier: safe_logical_identifier(repository_id.into()),
            phase: Some("decision_apply".to_string()),
            error: public_error.to_string(),
            error_code: Some(error_code.to_string()),
            error_category: Some(error.diagnostic_category().to_string()),
        }
    }
}

fn controlled_apply_step(step: String) -> (&'static str, &'static str) {
    match step.as_str() {
        "keep_missing" => ("keep_missing", "central_updates.keep_missing_failed"),
        "delete_missing" => ("delete_missing", "central_updates.delete_missing_failed"),
        "skip_addition" => ("skip_addition", "central_updates.skip_addition_failed"),
        "unskip_addition" => ("unskip_addition", "central_updates.unskip_addition_failed"),
        "remove_platform_duplicate" => (
            "remove_platform_duplicate",
            "central_updates.remove_platform_duplicate_failed",
        ),
        "remove_deleted_platform_copy" => (
            "remove_deleted_platform_copy",
            "central_updates.remove_deleted_platform_copy_failed",
        ),
        "update" => ("update", "central_updates.update_failed"),
        "import_addition" => ("import_addition", "central_updates.import_addition_failed"),
        _ => ("unknown", "central_updates.item_failure"),
    }
}

pub(crate) fn safe_logical_identifier(identifier: String) -> String {
    let is_safe = !identifier.is_empty()
        && identifier.len() <= 160
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'));
    if is_safe {
        identifier
    } else {
        "batch".to_string()
    }
}

#[cfg(test)]
mod apply_failure_tests {
    use super::*;

    #[test]
    fn apply_failure_metadata_rejects_dynamic_step_and_identifier_text() {
        let failure = SkillUpdateApplyFailure::new(
            "token=secret",
            "https://example.invalid/C:/Users/private",
        );

        assert_eq!(failure.step, "unknown");
        assert_eq!(failure.identifier, "batch");
        assert_eq!(
            failure.error_code.as_deref(),
            Some("central_updates.item_failure")
        );
    }

    #[test]
    fn apply_failure_metadata_keeps_reviewed_logical_identifiers() {
        for identifier in [
            "skill-a",
            "github:owner-repo-main",
            "codex::skill-a",
            "batch",
        ] {
            let failure = SkillUpdateApplyFailure::new("update", identifier);
            assert_eq!(failure.identifier, identifier);
        }
    }

    #[test]
    fn global_delete_failures_keep_static_phase_code_and_category() {
        let cases = [
            (
                crate::services::central_skills::CentralSkillsError::CentralMutation(
                    crate::services::central_mutation::CentralMutationError::Busy {
                        operation: "secret operation",
                    },
                ),
                "mutation_lock",
                "central_skills.mutation_lock_failed",
                "central_skills.central_mutation",
            ),
            (
                crate::services::central_skills::CentralSkillsError::Db(sqlx::Error::RowNotFound),
                "recovery",
                "central_skills.database_failed",
                "central_skills.db",
            ),
            (
                crate::services::central_skills::CentralSkillsError::CentralOperation(
                    crate::services::central_operation::CentralOperationError::InvalidManifest(
                        "token=secret C:\\Users\\private".to_string(),
                    ),
                ),
                "recovery",
                "central_operation.invalid_manifest",
                "central_skills.central_operation",
            ),
            (
                crate::services::central_skills::CentralSkillsError::Remote(
                    "ssh://secret.example/private".to_string(),
                ),
                "prepare",
                "central_skills.remote_failed",
                "central_skills.remote",
            ),
        ];

        for (error, phase, code, category) in cases {
            let failure = SkillUpdateApplyFailure::from_central_delete_error("batch", error);
            assert_eq!(failure.phase.as_deref(), Some(phase));
            assert_eq!(failure.error_code.as_deref(), Some(code));
            assert_eq!(failure.error_category.as_deref(), Some(category));
            let serialized = serde_json::to_string(&failure).unwrap();
            for secret in [
                "secret operation",
                "token=secret",
                "Users",
                "secret.example",
            ] {
                assert!(!serialized.contains(secret));
            }
        }
    }

    #[test]
    fn import_addition_failure_maps_github_import_code_and_redacts_serialized_error() {
        let seeds = "token=secret https://example.invalid C:/Users/private";
        let failure = SkillUpdateApplyFailure::from_github_import(
            "github:emilkowalski-skill-main",
            crate::services::github_import::GithubImportError::AccessDenied(seeds.to_string()),
        );

        assert_eq!(failure.step, "import_addition");
        assert_eq!(failure.identifier, "github:emilkowalski-skill-main");
        assert_eq!(failure.phase.as_deref(), Some("decision_apply"));
        assert_eq!(
            failure.error_code.as_deref(),
            Some("github_import.access_denied")
        );
        assert_eq!(
            failure.error_category.as_deref(),
            Some("github_import.access_denied")
        );

        let unsafe_identifier = SkillUpdateApplyFailure::from_github_import(
            seeds,
            crate::services::github_import::GithubImportError::AccessDenied(seeds.to_string()),
        );
        assert_eq!(unsafe_identifier.identifier, "batch");

        let serialized = serde_json::to_string(&failure).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap()["error"],
            "This update item could not be applied."
        );
        assert!(!serialized.contains("token=secret"));
        assert!(!serialized.contains("example.invalid"));
        assert!(!serialized.contains("Users/private"));
    }
}

fn serialize_public_apply_error<S>(_: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str("This update item could not be applied.")
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
