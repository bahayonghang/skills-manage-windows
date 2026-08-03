//! Data types shared across the central updates domain: update statuses,
//! progress payloads, prepared-update carriers, and load-error classification.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::db::SkillUpdateStatus;
use crate::db::{Skill, SkillRepositoryAssignment, SkillUpdateState};
use crate::services::github_import::{GitHubRepoRef, RemoteSkillCandidate};

use super::fs::RemoteSkillFile;

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
    pub error: String,
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
