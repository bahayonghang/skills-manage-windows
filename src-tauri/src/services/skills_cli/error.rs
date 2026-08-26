//! Typed domain errors for the Skills CLI global management service.
//!
//! Every variant maps to a fixed `skills_cli.*` IPC code registered in
//! `crate::ipc_error`. Display text is a reviewed internal summary; the public
//! sentence lives in the IPC code table, so raw CLI stdout/stderr never crosses
//! the boundary.

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SkillsCliError {
    /// Active target is SSH/WSL. The command must not spawn or read local state.
    #[error("Skills CLI management is only available on the Local target")]
    LocalTargetOnly,

    /// The machine has no usable Node.js runtime (node not found on PATH).
    #[error("Node.js was not found on this machine")]
    NodeMissing,

    /// Node exists but is older than the version required by the PIN package.
    #[error("Node.js {required} or later is required (found {found})")]
    NodeTooOld {
        required: &'static str,
        found: String,
    },

    /// The pinned `skills` npm package could not be executed (npx entry
    /// script missing, spawn failure, or probe command failed).
    #[error("The pinned Skills CLI package could not be executed")]
    CliUnavailable,

    /// User-supplied source failed the character/shape whitelist.
    #[error("The skill source is not allowed")]
    SourceInvalid,

    /// The `--list` preview output could not be parsed for this PIN version.
    #[error("The skill preview output could not be parsed")]
    PreviewUnparsed,

    /// Install requested with no skills or no platforms selected.
    #[error("At least one skill and one platform must be selected")]
    SelectionEmpty,

    /// A selected SkillPort platform id has no mapped CLI `--agent` value.
    #[error("That platform cannot be targeted by the Skills CLI")]
    AgentUnmapped(String),

    /// The Local target mutation lock is held by another operation.
    #[error("Another skill operation is using this target")]
    Busy,

    /// The supervised process exceeded its policy deadline.
    #[error("The Skills CLI command timed out")]
    Timeout(Duration),

    /// The job was cancelled through its exclusive lease flag.
    #[error("The Skills CLI operation was cancelled")]
    Cancelled,

    /// The supervised process exceeded its bounded output capacity.
    #[error("The Skills CLI produced too much output")]
    OutputLimitExceeded { stream: &'static str },

    /// The CLI ran but exited with a failure status for the request.
    #[error("The Skills CLI command failed")]
    CliFailed,

    /// The `ls -g --json` listing could not be parsed for this PIN version.
    #[error("The Skills CLI listing could not be parsed")]
    ListUnparsed,

    /// Filesystem access failed while reading lock or runtime files.
    #[error("{context}: filesystem access failed")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// spawn_blocking join failure while resolving paths or links.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },

    #[error("That skill is not owned by the Skills CLI lock")]
    SkillNotOwned,

    #[error("The Skills CLI canonical folder is missing")]
    CanonicalMissing,

    #[error("The SKILL.md file is missing")]
    SkillDocMissing,

    #[error("The SKILL.md file is too large")]
    SkillDocTooLarge,

    #[error("The SKILL.md file is not valid UTF-8")]
    SkillDocInvalidUtf8,

    #[error("A copied skill folder cannot be linked or unlinked")]
    DirectCopyNotToggleable,

    #[error("The platform folder is in conflict")]
    PlacementConflict,

    #[error("The platform folder is unavailable")]
    PlacementUnavailable,

    #[error("The inventory export is invalid")]
    ExportInvalid,

    #[error("The inventory export could not be saved")]
    ExportFailed,

    #[error("The skill folder could not be revealed")]
    RevealFailed,

    #[error("A Skills CLI remove operation needs recovery")]
    RecoveryRequired,

    #[error("The Skills CLI update request no longer matches the current state")]
    UpdateStale,

    #[error("This skill has no installed update baseline")]
    UpdateBaselineRequired,

    #[error("This skill source cannot be updated")]
    UpdateUnsupported,

    #[error("GitHub rate limited the Skills CLI update check")]
    UpdateRateLimited { reset_at: Option<String> },

    #[error("The Skills CLI update check failed")]
    UpdateCheckFailed,

    #[error("The local skill files differ from the installed baseline")]
    UpdateLocalModified,

    #[error("The platform placement cannot be updated")]
    UpdateTopologyConflict,

    #[error("A Skills CLI update operation needs recovery")]
    UpdateRecoveryRequired,

    #[error("The Skills CLI update files failed an integrity check")]
    UpdateIntegrity,

    #[error("The Skills CLI update database is not available")]
    UpdateMigration,
}

impl SkillsCliError {
    /// Stable IPC error code. One row per variant; the public sentence lives
    /// in `crate::ipc_error::public_message_for_code`.
    pub fn ipc_code(&self) -> &'static str {
        match self {
            Self::LocalTargetOnly => "skills_cli.local_target_only",
            Self::NodeMissing | Self::NodeTooOld { .. } => "skills_cli.node_missing",
            Self::CliUnavailable => "skills_cli.cli_unavailable",
            Self::SourceInvalid => "skills_cli.source_invalid",
            Self::PreviewUnparsed => "skills_cli.preview_unparsed",
            Self::SelectionEmpty => "skills_cli.selection_empty",
            Self::AgentUnmapped(_) => "skills_cli.agent_unmapped",
            Self::Busy => "skills_cli.busy",
            Self::Timeout(_) => "skills_cli.timeout",
            Self::Cancelled => "skills_cli.cancelled",
            Self::OutputLimitExceeded { .. } | Self::CliFailed | Self::ListUnparsed => {
                "internal.unexpected"
            }
            Self::Io { .. } | Self::TaskJoin { .. } => "internal.unexpected",
            Self::SkillNotOwned => "skills_cli.skill_not_owned",
            Self::CanonicalMissing => "skills_cli.canonical_missing",
            Self::SkillDocMissing => "skills_cli.skill_doc_missing",
            Self::SkillDocTooLarge => "skills_cli.skill_doc_too_large",
            Self::SkillDocInvalidUtf8 => "skills_cli.skill_doc_invalid_utf8",
            Self::DirectCopyNotToggleable => "skills_cli.direct_copy_not_toggleable",
            Self::PlacementConflict => "skills_cli.placement_conflict",
            Self::PlacementUnavailable => "skills_cli.placement_unavailable",
            Self::ExportInvalid => "skills_cli.export_invalid",
            Self::ExportFailed => "skills_cli.export_failed",
            Self::RevealFailed => "skills_cli.reveal_failed",
            Self::RecoveryRequired => "skills_cli.recovery_required",
            Self::UpdateStale => "skills_cli.update_stale",
            Self::UpdateBaselineRequired => "skills_cli.update_baseline_required",
            Self::UpdateUnsupported => "skills_cli.update_unsupported",
            Self::UpdateRateLimited { .. } => "skills_cli.update_rate_limited",
            Self::UpdateCheckFailed => "skills_cli.update_check_failed",
            Self::UpdateLocalModified => "skills_cli.update_local_modified",
            Self::UpdateTopologyConflict => "skills_cli.update_topology_conflict",
            Self::UpdateRecoveryRequired => "skills_cli.update_recovery_required",
            Self::UpdateIntegrity => "skills_cli.update_integrity",
            Self::UpdateMigration => "skills_cli.update_migration",
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Busy
                | Self::RecoveryRequired
                | Self::UpdateStale
                | Self::UpdateRateLimited { .. }
                | Self::UpdateCheckFailed
                | Self::UpdateRecoveryRequired
        )
    }

    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }
}
