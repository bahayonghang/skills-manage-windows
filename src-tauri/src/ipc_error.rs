use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

const INTERNAL_CODE: &str = "internal.unexpected";
const INTERNAL_MESSAGE: &str = "The operation failed. See runtime logs for details.";

/// Stable error envelope serialized across the Tauri command boundary.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    /// Operation Log row UUID used to correlate the rejection with audit and
    /// Runtime evidence. Missing for legacy/backend-internal failures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

pub type IpcResult<T> = Result<T, IpcError>;

impl IpcError {
    /// Construct an explicitly reviewed public error. Static inputs prevent
    /// runtime diagnostics from being passed through accidentally.
    pub fn new(code: &'static str, message: &'static str, retryable: bool) -> Self {
        debug_assert!(is_valid_code(code), "invalid IPC error code: {code}");
        Self {
            code: code.to_string(),
            message: message.to_string(),
            retryable,
            correlation_id: None,
        }
    }

    /// Attach a pre-generated operation/correlation UUID. Invalid values are
    /// rejected to avoid turning an untrusted string into diagnostic payload.
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        if let Ok(value) = uuid::Uuid::parse_str(correlation_id) {
            self.correlation_id = Some(value.to_string());
        } else {
            debug_assert!(false, "invalid IPC correlation id: {correlation_id}");
        }
        self
    }

    /// Locale-neutral code safe for allowlisted Runtime diagnostic fields.
    pub fn safe_code(&self) -> &str {
        if is_valid_code(&self.code) {
            &self.code
        } else {
            INTERNAL_CODE
        }
    }

    pub fn from_display(error: impl Display) -> Self {
        Self::from_legacy_boundary(error.to_string())
    }

    /// Narrow compatibility bridge for historical command errors. Only a
    /// frozen set of stable families is recognized, and the original text is
    /// never copied into the payload. Unknown failures use the fixed fallback.
    pub fn from_legacy_boundary(message: impl Into<String>) -> Self {
        let raw = message.into();
        let trimmed = raw.trim();

        if let Some((code, _)) = parse_coded_message(trimmed) {
            return legacy_code_message(code)
                .map(|message| Self {
                    code: code.to_string(),
                    message: message.to_string(),
                    retryable: retryable_for_code(code),
                    correlation_id: None,
                })
                .unwrap_or_else(unexpected);
        }

        let lower = trimmed.to_ascii_lowercase();
        if lower == "skillport state portability cancelled"
            || lower == "central update was cancelled."
        {
            return Self::new("operation.cancelled", "The operation was cancelled.", false);
        }
        if lower.starts_with("invalid skillport state json:") {
            return Self::new(
                "portable_state.invalid_manifest_json",
                "The SkillPort state file is not valid JSON.",
                false,
            );
        }
        if lower == "unsupported skillport state export kind" {
            return Self::new(
                "portable_state.unsupported_export_kind",
                "The SkillPort state export kind is not supported.",
                false,
            );
        }
        if lower.starts_with("unsupported skillport state export version:") {
            return Self::new(
                "portable_state.unsupported_export_version",
                "The SkillPort state export version is not supported.",
                false,
            );
        }
        if lower.starts_with("github api access was denied while ")
            && lower.contains("rate limit was exceeded")
        {
            return Self::new(
                "github_import.rate_limited",
                "GitHub rate limited the request. Try again later.",
                true,
            );
        }
        if lower.starts_with("github denied access while ")
            && lower.contains("a configured github token was used")
        {
            return Self::new(
                "github_import.configured_token_failed",
                "GitHub denied access to the repository.",
                false,
            );
        }
        if lower.starts_with("github denied access while ") {
            return Self::new(
                "github_import.access_denied",
                "GitHub denied access to the repository.",
                false,
            );
        }
        if lower.starts_with("ssh password for target ")
            && lower.ends_with(
                " is not available. open settings, enter the password for this target, save it, and retry.",
            )
        {
            return Self::new(
                "credential.ssh_password_unavailable",
                "The SSH password is unavailable. Open Settings, save it, and retry.",
                false,
            );
        }

        if let Some((code, message)) = legacy_plain_message(trimmed, &lower) {
            return Self::new(code, message, false);
        }

        unexpected()
    }
}

impl Display for IpcError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IpcError {}

impl From<String> for IpcError {
    fn from(value: String) -> Self {
        Self::from_legacy_boundary(value)
    }
}

impl From<&str> for IpcError {
    fn from(value: &str) -> Self {
        Self::from_legacy_boundary(value)
    }
}

fn parse_coded_message(message: &str) -> Option<(&str, &str)> {
    let (code, public_message) = message.split_once(':')?;
    is_valid_code(code).then_some((code, public_message.trim_start()))
}

fn is_valid_code(code: &str) -> bool {
    let mut segments = code.split('.');
    let Some(first) = segments.next() else {
        return false;
    };
    if !valid_code_segment(first) {
        return false;
    }
    let mut count = 1;
    for segment in segments {
        count += 1;
        if !valid_code_segment(segment) {
            return false;
        }
    }
    count >= 2
}

fn valid_code_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'_')
        && segment.as_bytes()[0].is_ascii_lowercase()
}

fn retryable_for_code(code: &str) -> bool {
    matches!(
        code,
        "github_import.rate_limited"
            | "github_import.transport_failed"
            | "github_import.archive_unavailable"
            | "skills_cli.busy"
            | "skills_cli.recovery_required"
            | "skills_cli.update_stale"
            | "skills_cli.update_rate_limited"
            | "skills_cli.update_recovery_required"
    )
}

fn legacy_plain_message(original: &str, lower: &str) -> Option<(&'static str, &'static str)> {
    let resource_prefix = [
        "saved view '",
        "tag group '",
        "collection '",
        "agent '",
        "skill '",
        "project '",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix));
    if (resource_prefix && lower.contains(" not found"))
        || matches!(lower, "registry not found" | "skill not found")
        || lower.starts_with("skill source not found at ")
    {
        return Some((
            "resource.not_found",
            "The requested resource was not found.",
        ));
    }

    if lower.contains("unique constraint failed") || lower.contains(" already exists") {
        return Some((
            "resource.conflict",
            "The requested resource conflicts with existing data.",
        ));
    }

    if lower.starts_with("permission denied") {
        return Some(("permission.denied", "Permission was denied."));
    }

    match original {
        "Collection name cannot be empty"
        | "Imported collection name cannot be empty"
        | "Agent ID cannot be empty"
        | "Agent display name cannot be empty"
        | "Agent global skills directory cannot be empty"
        | "Saved view name cannot be empty"
        | "Saved view query cannot be empty"
        | "Scan directory path cannot be empty"
        | "Tag group name cannot be empty"
        | "setting_value_invalid: The setting value is invalid." => {
            Some(("input.invalid", "The request is invalid."))
        }
        "setting_key_forbidden: This setting cannot be changed through the generic settings API." => {
            Some((
                "permission.denied",
                "This setting cannot be changed through the generic settings API.",
            ))
        }
        _ => None,
    }
}

/// Reviewed public message for a stable IPC code.
///
/// Exposed so surfaces other than the IPC envelope (Update Center inventory
/// rows, Operation Log summaries) can show the same reviewed sentence instead
/// of a domain error's Display text.
pub fn public_message_for_code(code: &str) -> Option<&'static str> {
    legacy_code_message(code)
}

fn legacy_code_message(code: &str) -> Option<&'static str> {
    match code {
        "ai.missing_api_key" => Some("Configure an AI API key before retrying."),
        "ai.rate_limit" => Some("The AI provider rate limited the request."),
        "ai.invalid_api_key" => Some("The AI provider rejected the configured API key."),
        "ai.request_failed"
        | "ai.client_build_failed"
        | "ai.response_error"
        | "ai.response_read_failed"
        | "ai.response_parse_failed"
        | "ai.proxy"
        | "ai.connect"
        | "ai.timeout"
        | "ai.dns"
        | "ai.tls"
        | "ai.network"
        | "ai.empty_response" => Some("The AI provider request failed."),
        "github_import.preview_missing" => {
            Some("GitHub preview is no longer available. Preview the repository again.")
        }
        "github_import.preview_expired" => {
            Some("GitHub preview expired. Preview the repository again.")
        }
        "github_import.preview_mismatch" => {
            Some("GitHub preview no longer matches the request. Preview again.")
        }
        "github_import.preview_integrity" => {
            Some("GitHub preview content changed. Preview the repository again.")
        }
        "github_import.preview_busy" => Some("GitHub preview is already being imported."),
        "github_import.preview_capacity" => {
            Some("GitHub preview capacity is full. Close an older preview and try again.")
        }
        "github_import.preview_cleanup_pending" => {
            Some("GitHub preview cleanup is still pending. Preview the repository again.")
        }
        "github_import.preview_commit_unresolved" => {
            Some("GitHub repository commit could not be resolved. Preview again.")
        }
        "github_import.branch_invalid" => Some("GitHub branch must be a safe single-segment name."),
        "github_import.branch_conflict" => {
            Some("GitHub branch in the repository URL does not match the selected branch.")
        }
        "github_import.no_importable_skills" => {
            Some("This GitHub repository does not contain an importable skill.")
        }
        "github_import.selection_unavailable" => {
            Some("The selected skill is no longer available in the repository preview.")
        }
        "github_import.invalid_candidate" => {
            Some("The repository contains a skill that cannot be imported.")
        }
        "github_import.source_path_missing" => {
            Some("The selected path no longer contains an importable skill.")
        }
        "github_import.target_exists" => {
            Some("The Central target directory already exists and cannot be overwritten.")
        }
        "github_import.duplicate_selection" => {
            Some("The same skill path was selected more than once in this import.")
        }
        "github_import.rename_conflict" => Some("The renamed skill id is not available."),
        "github_import.archive_redirect_rejected" => {
            Some("GitHub repository archive redirect was rejected.")
        }
        "github_import.transport_failed" => {
            Some("Could not reach GitHub. Check the network and try again.")
        }
        "github_import.rate_limited" => Some("GitHub rate limited the request. Try again later."),
        "github_import.access_denied" => Some(
            "GitHub denied anonymous access to the repository. Configure a token if the repository requires authentication.",
        ),
        "github_import.configured_token_failed" => Some(
            "GitHub denied the authenticated request. Check that the token owner can read the repository and that the token has the required permissions.",
        ),
        "github_import.repo_not_found" => Some("The GitHub repository was not found."),
        "github_import.archive_unavailable" => {
            Some("The GitHub repository archive is unavailable.")
        }
        "github_import.response_invalid" => Some("GitHub returned an unreadable response."),
        "github_import.invalid_url" => Some("The GitHub request address is not allowed."),
        "github_import.budget_exceeded" => {
            Some("The GitHub repository exceeds the import resource limits.")
        }
        "github_import.credential_unavailable" => {
            Some("The stored GitHub token could not be read. Save it again in Settings.")
        }
        "local_archive.archive_not_found" => Some("The selected archive was not found."),
        "local_archive.archive_read_failed" => Some("The selected archive could not be read."),
        "local_archive.archive_changed_since_preview" => {
            Some("The archive changed after it was previewed.")
        }
        "local_archive.ambiguous_archive_layout" => Some("The archive layout is ambiguous."),
        "local_archive.no_skill_manifest" => {
            Some("The archive does not contain an importable skill.")
        }
        "local_archive.invalid_archive_entry" => Some("The archive contains an unsafe path."),
        "local_archive.unsupported_archive_entry" => {
            Some("The archive contains an unsupported entry.")
        }
        "local_archive.budget_exceeded" => Some("The archive exceeds the import resource limits."),
        "local_archive.path_conflict" => {
            Some("The selected destination conflicts with existing data.")
        }
        "local_archive.skill_frontmatter_missing" => {
            Some("The skill manifest metadata is invalid.")
        }
        "local_archive.invalid_skill_identifier" => {
            Some("The requested skill identifier is invalid.")
        }
        "local_archive.io" => Some("A filesystem operation failed."),
        "local_archive.db" => Some("The skill database could not be updated."),
        "local_archive.central_mutation" => Some("Central is busy with another change."),
        "local_archive.rollback_failed" => Some("The previous skill could not be restored."),
        "local_archive.remote_target_unsupported" => {
            Some("Local ZIP import is unavailable for remote targets.")
        }
        "local_archive.internal" => Some("The local archive import failed."),
        "central_updates.repository_check_failed" => {
            Some("The repository could not be checked.")
        }
        "central_updates.skill_source_missing" => Some(
            "The tracked source path no longer contains a skill, and no unique new location was found.",
        ),
        "central_updates.relocation_failed" => {
            Some("The moved skill could not be reattached to its new location.")
        }
        "central_updates.inventory_invariant" => {
            Some("The update inventory could not be finalized.")
        }
        "central_updates.inventory_refresh_required" => {
            Some("Refresh the update inventory before importing this repository.")
        }
        "central_updates.snapshot_changed" => Some(
            "The repository content did not match the update inventory. Refresh and try again.",
        ),
        "central.reset_failed" => Some("Unknown-source Central skills could not be reset."),
        "central_skills.mutation_lock_failed" => {
            Some("Central is busy with another change. Try again shortly.")
        }
        "central_skills.delete_failed" | "central_skills.delete_preview_failed" => {
            Some("This Central skill could not be deleted.")
        }
        "central_skills.database_failed" => Some("This Central skill could not be deleted."),
        "central_skills.remote_failed" => Some("This Central skill could not be deleted."),
        "central_skills.budget_exceeded" => Some("This Central skill could not be deleted."),
        "central_skills.force_delete_blocked" => {
            Some("Force delete is not available for this Central skill.")
        }
        "central_operation.delete_restore_collision" => Some(
            "Central recovery evidence conflicts with the current files. Review and resolve the pending operation in Operation Logs.",
        ),
        "installation.pending_central_recovery" => {
            Some("Central recovery is pending for this skill.")
        }
        "recovery.reconcile_guard_unavailable" => {
            Some("Central recovery is busy. Try again shortly.")
        }
        "recovery.reconcile_preflight_blocked" => {
            Some("The prepared delete operation no longer passes reconciliation checks.")
        }
        "job.portability_busy" => Some("A portability job is already running."),
        "skills_cli.local_target_only" => {
            Some("Skills CLI is available only on the Local target.")
        }
        "skills_cli.node_missing" => Some("Node.js 22.20 or later is required."),
        "skills_cli.cli_unavailable" => {
            Some("The Skills CLI package could not be executed.")
        }
        "skills_cli.source_invalid" => Some("The skill source is not allowed."),
        "skills_cli.preview_unparsed" => Some("The skill preview could not be parsed."),
        "skills_cli.selection_empty" => {
            Some("Select at least one skill and one platform.")
        }
        "skills_cli.agent_unmapped" => {
            Some("That platform cannot be targeted by Skills CLI.")
        }
        "skills_cli.busy" => Some("Another skill operation is using this target."),
        "skills_cli.timeout" => Some("The Skills CLI command timed out."),
        "skills_cli.cancelled" => Some("The operation was cancelled."),
        "skills_cli.skill_not_owned" => Some("That skill is not managed by Skills CLI."),
        "skills_cli.canonical_missing" => Some("The skill folder is missing."),
        "skills_cli.skill_doc_missing" => Some("The SKILL.md file is missing."),
        "skills_cli.skill_doc_too_large" => Some("The SKILL.md file is too large to open."),
        "skills_cli.skill_doc_invalid_utf8" => Some("The SKILL.md file is not valid text."),
        "skills_cli.direct_copy_not_toggleable" => {
            Some("A copied skill folder cannot be linked or unlinked.")
        }
        "skills_cli.placement_conflict" => Some("The platform folder is in conflict."),
        "skills_cli.placement_unavailable" => Some("The platform folder is unavailable."),
        "skills_cli.export_invalid" => Some("The inventory export is invalid."),
        "skills_cli.export_failed" => Some("The inventory export could not be saved."),
        "skills_cli.reveal_failed" => Some("The skill folder could not be revealed."),
        "skills_cli.recovery_required" => {
            Some("A previous Skills CLI remove needs recovery.")
        }
        "skills_cli.update_stale" => {
            Some("The update is out of date. Refresh, then try again.")
        }
        "skills_cli.update_baseline_required" => {
            Some("This skill has no installed baseline, so it cannot be treated as current.")
        }
        "skills_cli.update_unsupported" => {
            Some("This skill source cannot be updated.")
        }
        "skills_cli.update_rate_limited" => {
            Some("GitHub rate limited the update check. Wait for the limit to reset, then retry.")
        }
        "skills_cli.update_check_failed" => {
            Some("The update check failed for this repository.")
        }
        "skills_cli.update_local_modified" => {
            Some("Local files differ from the installed baseline.")
        }
        "skills_cli.update_topology_conflict" => {
            Some("This skill's platform placement cannot be updated.")
        }
        "skills_cli.update_recovery_required" => {
            Some("A previous Skills CLI update needs recovery.")
        }
        "skills_cli.update_integrity" => {
            Some("The updated files did not pass the integrity check.")
        }
        "skills_cli.update_migration" => {
            Some("The Skills CLI update database is not available.")
        }
        "startup.rebuild_unavailable" => Some("Database rebuild is not available."),
        _ => None,
    }
}

fn unexpected() -> IpcError {
    IpcError::new(INTERNAL_CODE, INTERNAL_MESSAGE, false)
}

/// Preserve existing command internals and convert only the final Tauri
/// rejection boundary into [`IpcError`].
#[macro_export]
macro_rules! ipc_boundary {
    ($expression:expr) => {{
        let result: Result<_, String> = $expression;
        result.map_err($crate::ipc_error::IpcError::from)
    }};
}

#[macro_export]
macro_rules! ipc_boundary_async {
    ($body:block) => {{
        let result: Result<_, String> = (async move $body).await;
        result.map_err($crate::ipc_error::IpcError::from)
    }};
}

#[cfg(test)]
#[path = "ipc_error/redaction_contract_tests.rs"]
mod redaction_contract_tests;

#[cfg(test)]
mod tests;
