use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const MANIFEST_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    CentralDelete,
    CentralUpdate,
}

impl OperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CentralDelete => "central_delete",
            Self::CentralUpdate => "central_update",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPhase {
    Prepared,
    FsStaged,
    FsSwapped,
    DbCommitted,
    CopiesPending,
    Completed,
    RolledBack,
}

impl OperationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::FsStaged => "fs_staged",
            Self::FsSwapped => "fs_swapped",
            Self::DbCommitted => "db_committed",
            Self::CopiesPending => "copies_pending",
            Self::Completed => "completed",
            Self::RolledBack => "rolled_back",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::RolledBack)
    }

    pub fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Prepared, Self::FsStaged | Self::RolledBack)
                | (
                    Self::FsStaged,
                    Self::FsSwapped | Self::DbCommitted | Self::RolledBack
                )
                | (Self::FsSwapped, Self::DbCommitted | Self::RolledBack)
                | (Self::DbCommitted, Self::CopiesPending | Self::Completed)
                | (Self::CopiesPending, Self::Completed)
        ) || self == next
    }
}

impl FromStr for OperationPhase {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "fs_staged" => Ok(Self::FsStaged),
            "fs_swapped" => Ok(Self::FsSwapped),
            "db_committed" => Ok(Self::DbCommitted),
            "copies_pending" => Ok(Self::CopiesPending),
            "completed" => Ok(Self::Completed),
            "rolled_back" => Ok(Self::RolledBack),
            other => Err(format!("unknown operation phase: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPath {
    pub original: String,
    pub backup: String,
    pub marker: String,
    pub expected_present: bool,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteManifest {
    pub version: i64,
    pub operation_id: String,
    pub paths: Vec<ManagedPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyProjection {
    pub target: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub version: i64,
    pub operation_id: String,
    pub target: String,
    pub staging: String,
    pub backup: String,
    pub marker: String,
    pub had_target: bool,
    pub old_fingerprint: Option<String>,
    pub new_fingerprint: String,
    pub copies: Vec<CopyProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum OperationManifest {
    Delete(DeleteManifest),
    Update(UpdateManifest),
}

impl OperationManifest {
    pub fn validate(&self, operation_id: &str) -> Result<(), String> {
        let (version, manifest_operation_id) = match self {
            Self::Delete(manifest) => (manifest.version, manifest.operation_id.as_str()),
            Self::Update(manifest) => (manifest.version, manifest.operation_id.as_str()),
        };
        if version != MANIFEST_VERSION {
            return Err(format!("unsupported manifest version {version}"));
        }
        if manifest_operation_id != operation_id {
            return Err("operation identity mismatch".to_string());
        }
        match self {
            Self::Delete(manifest) if manifest.paths.is_empty() => {
                Err("delete manifest has no paths".to_string())
            }
            Self::Delete(manifest)
                if manifest.paths.iter().any(|path| {
                    path.original.is_empty() || path.backup.is_empty() || path.marker.is_empty()
                }) =>
            {
                Err("delete manifest contains an empty path".to_string())
            }
            Self::Update(manifest)
                if manifest.target.is_empty()
                    || manifest.staging.is_empty()
                    || manifest.backup.is_empty()
                    || manifest.marker.is_empty() =>
            {
                Err("update manifest contains an empty path".to_string())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingOperationSummary {
    pub operation_id: String,
    pub target_id: String,
    pub target_kind: String,
    pub operation_kind: String,
    pub skill_id: String,
    pub phase: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedDeleteReconciliationPreview {
    pub operation_id: String,
    pub skill_id: String,
    pub eligible: bool,
    pub duplicate_path_count: usize,
    pub missing_unowned_path_count: usize,
    pub blocker_codes: Vec<String>,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDeleteRecoveryPreview {
    pub operation_id: String,
    pub operation_kind: String,
    pub phase: String,
    pub error_code: Option<String>,
    pub force_delete_eligible: bool,
    pub blocker_codes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_graph_rejects_commit_and_terminal_shortcuts() {
        assert!(OperationPhase::Prepared.permits(OperationPhase::FsStaged));
        assert!(!OperationPhase::Prepared.permits(OperationPhase::DbCommitted));
        assert!(!OperationPhase::FsStaged.permits(OperationPhase::Completed));
        assert!(OperationPhase::DbCommitted.permits(OperationPhase::CopiesPending));
        assert!(OperationPhase::CopiesPending.permits(OperationPhase::Completed));
        assert!(OperationPhase::Completed.is_terminal());
        assert!(OperationPhase::RolledBack.is_terminal());
    }
}
