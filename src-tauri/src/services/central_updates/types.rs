//! Data types shared across the central updates domain: update statuses,
//! progress payloads, prepared-update carriers, and load-error classification.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

pub use crate::db::SkillUpdateStatus;
use crate::db::{Skill, SkillRepositoryAssignment, SkillUpdateState};
use crate::services::github_import::{GitHubRepoRef, RemoteSkillCandidate};

use super::fs::{normalize_repo_path, RemoteSkillFile};
use super::CentralUpdatesError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CentralUpdateFailurePhase {
    MutationLock,
    Recovery,
    Prepare,
    Stage,
    DatabaseCommit,
    CopyRefresh,
    ResultFinalization,
    DecisionApply,
}

impl CentralUpdateFailurePhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MutationLock => "mutation_lock",
            Self::Recovery => "recovery",
            Self::Prepare => "prepare",
            Self::Stage => "stage",
            Self::DatabaseCommit => "database_commit",
            Self::CopyRefresh => "copy_refresh",
            Self::ResultFinalization => "result_finalization",
            Self::DecisionApply => "decision_apply",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CentralUpdateItemError {
    pub(crate) phase: CentralUpdateFailurePhase,
    error: Arc<CentralUpdatesError>,
}

impl CentralUpdateItemError {
    pub(crate) fn new(phase: CentralUpdateFailurePhase, error: CentralUpdatesError) -> Self {
        Self {
            phase,
            error: Arc::new(error),
        }
    }

    pub(crate) fn error(&self) -> &CentralUpdatesError {
        self.error.as_ref()
    }

    pub(crate) fn into_error(self) -> CentralUpdatesError {
        Arc::try_unwrap(self.error).unwrap_or_else(|error| {
            CentralUpdatesError::Batch(error.public_update_message().to_string())
        })
    }
}

/// Cache policy for GitHub repository snapshots during update checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotCachePolicy {
    UseFresh,
    Bypass,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateProgressPayload {
    pub job_id: String,
    pub phase: String,
    pub status: String,
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateFailure {
    pub skill_id: String,
    #[serde(default)]
    pub phase: Option<CentralUpdateFailurePhase>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_category: Option<String>,
    #[serde(serialize_with = "serialize_public_update_error")]
    pub error: String,
}

impl CentralSkillUpdateFailure {
    pub(crate) fn from_item_error(skill_id: String, error: &CentralUpdateItemError) -> Self {
        Self {
            skill_id,
            phase: Some(error.phase),
            error_code: Some(error.error().stable_error_code()),
            error_category: Some(error.error().diagnostic_category().to_string()),
            error: error.error().public_update_message().to_string(),
        }
    }

    pub(crate) fn decision_apply_fallback(skill_id: String) -> Self {
        Self {
            skill_id,
            phase: Some(CentralUpdateFailurePhase::DecisionApply),
            error_code: Some("central_updates.update_failed".to_string()),
            error_category: Some("central_updates.remote_skill_load".to_string()),
            error: "This update item could not be applied.".to_string(),
        }
    }
}

fn serialize_public_update_error<S>(_: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str("This update item could not be applied.")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateSkip {
    pub skill_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<CentralSkillUpdateFailure>,
    pub skipped: Vec<CentralSkillUpdateSkip>,
    pub states: Vec<SkillUpdateState>,
}

#[derive(Debug, Clone)]
pub(crate) struct GitHubUpdateSource {
    pub(crate) repo: GitHubRepoRef,
    pub(crate) source_path: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedSkillUpdate {
    pub(crate) skill: Skill,
    pub(crate) source: Option<GitHubUpdateSource>,
    pub(crate) assignment: SkillRepositoryAssignment,
    pub(crate) target_dir: Option<PathBuf>,
    pub(crate) previous_state: Option<SkillUpdateState>,
    pub(crate) reuse_previous_local_hash: bool,
    pub(crate) local_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteSkillContent {
    pub(crate) source: GitHubUpdateSource,
    pub(crate) candidate: RemoteSkillCandidate,
    pub(crate) files: Vec<RemoteSkillFile>,
    pub(crate) remote_hash: String,
    pub(crate) local_hash: String,
    pub(crate) target_dir: PathBuf,
    pub(crate) resolved_commit_sha: Option<String>,
    pub(crate) content_digest: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UpdateCounters {
    pub(crate) completed: usize,
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedSkillReasonCode {
    UnknownSource,
    UnsupportedSourceType,
    MissingSourcePath,
    UnsupportedSource,
}

pub(crate) fn unsupported_reason_code(
    assignment: &SkillRepositoryAssignment,
) -> UnsupportedSkillReasonCode {
    if assignment.is_source_unknown || assignment.repository.is_unknown {
        return UnsupportedSkillReasonCode::UnknownSource;
    }
    if assignment.repository.source_type != "github" {
        return UnsupportedSkillReasonCode::UnsupportedSourceType;
    }
    if normalized_github_source_path(assignment).is_none() {
        return UnsupportedSkillReasonCode::MissingSourcePath;
    }
    UnsupportedSkillReasonCode::UnsupportedSource
}

pub(crate) fn normalized_github_source_path(
    assignment: &SkillRepositoryAssignment,
) -> Option<String> {
    let source_path = assignment.source_path.as_deref()?.trim();
    if source_path.is_empty() {
        return None;
    }
    normalize_repo_path(source_path).ok()
}

/// Classification of a failed remote-skill load: `RemoteMissing` reasons are
/// user-facing "the upstream no longer has this skill" states, everything
/// else is a hard error. Payloads are plain strings because they are
/// persisted as `SkillUpdateState.error` reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteSkillLoadError {
    RemoteMissing(String),
    Other(String),
}

impl RemoteSkillLoadError {
    pub(crate) fn remote_missing(message: impl Into<String>) -> Self {
        Self::RemoteMissing(message.into())
    }

    pub(crate) fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::RemoteMissing(message) | Self::Other(message) => message,
        }
    }
}

#[cfg(test)]
mod progress_payload_tests {
    use super::*;

    #[test]
    fn progress_payload_serializes_job_id_as_camel_case() {
        let payload = CentralSkillUpdateProgressPayload {
            job_id: "update-job".to_string(),
            phase: "checking".to_string(),
            status: "started".to_string(),
            total: 0,
            completed: 0,
            succeeded: 0,
            failed: 0,
            skipped: 0,
            skill_id: None,
            skill_name: None,
            error: None,
        };
        let value = serde_json::to_value(payload).unwrap();
        assert_eq!(value["jobId"], "update-job");
        assert!(value.get("job_id").is_none());
    }
}
