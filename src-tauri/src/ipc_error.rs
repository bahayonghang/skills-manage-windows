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
        "github_import.archive_redirect_rejected" => {
            Some("GitHub repository archive redirect was rejected.")
        }
        "github_import.transport_failed" => {
            Some("Could not reach GitHub. Check the network and try again.")
        }
        "github_import.rate_limited" => Some("GitHub rate limited the request. Try again later."),
        "github_import.access_denied" => Some("GitHub denied access to the repository."),
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
        "central.reset_failed" => Some("Unknown-source Central skills could not be reset."),
        "central_skills.mutation_lock_failed" => {
            Some("Central is busy with another change. Try again shortly.")
        }
        "installation.pending_central_recovery" => {
            Some("Central recovery is pending for this skill.")
        }
        "recovery.reconcile_guard_unavailable" => {
            Some("Central recovery is busy. Try again shortly.")
        }
        "recovery.reconcile_preflight_blocked" => {
            Some("The prepared delete operation no longer passes reconciliation checks.")
        }
        "job.invalid_id" => Some("The job identifier is invalid."),
        "job.id_mismatch" => Some("The cancellation request does not match the active job."),
        "job.registry_unavailable" => Some("The job registry is unavailable."),
        "job.central_update_busy" => Some("A Central update job is already running."),
        "job.portability_busy" => Some("A portability job is already running."),
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
mod tests {
    use super::*;

    #[test]
    fn serializes_exact_camel_case_contract() {
        let value = serde_json::to_value(IpcError::new(
            "operation.cancelled",
            "Operation cancelled",
            false,
        ))
        .expect("serialize IPC error");
        assert_eq!(
            value,
            serde_json::json!({
                "code": "operation.cancelled",
                "message": "Operation cancelled",
                "retryable": false
            })
        );
    }

    #[test]
    fn known_coded_family_keeps_only_its_code_and_canonical_message() {
        let error = IpcError::from("ai.rate_limit:attacker-controlled detail".to_string());
        assert_eq!(error.code, "ai.rate_limit");
        assert_eq!(error.message, "The AI provider rate limited the request.");
        assert!(!error.message.contains("attacker"));
    }

    #[test]
    fn github_branch_codes_keep_reviewed_public_messages() {
        for (code, message) in [
            (
                "github_import.branch_invalid",
                "GitHub branch must be a safe single-segment name.",
            ),
            (
                "github_import.branch_conflict",
                "GitHub branch in the repository URL does not match the selected branch.",
            ),
        ] {
            let error = IpcError::from(format!("{code}:private branch detail"));
            assert_eq!(error.code, code);
            assert_eq!(error.message, message);
            assert!(!error.message.contains("private"));
        }
    }

    #[test]
    fn archive_redirect_code_keeps_only_the_reviewed_public_message() {
        let seeds = [
            "ghp_super_secret",
            "https://codeload.github.com/private/repo?token=secret",
            r"C:\Users\alice\private\SKILL.md",
            "private response body",
        ];
        for seed in seeds {
            let error = IpcError::from(format!("github_import.archive_redirect_rejected:{seed}"));
            assert_eq!(error.code, "github_import.archive_redirect_rejected");
            assert_eq!(
                error.message,
                "GitHub repository archive redirect was rejected."
            );
            assert!(!error.retryable);
            assert!(!serde_json::to_string(&error).unwrap().contains(seed));
        }
    }

    #[test]
    fn reset_failed_code_keeps_only_the_reviewed_public_message() {
        let seeds = [
            "ghp_super_secret",
            "https://github.com/private/repo",
            r"C:\Users\alice\.skillsmanage\skills\npx-skill",
        ];
        for seed in seeds {
            let error = IpcError::from(format!("central.reset_failed:{seed}"));
            assert_eq!(error.code, "central.reset_failed");
            assert_eq!(
                error.message,
                "Unknown-source Central skills could not be reset."
            );
            assert!(!error.retryable);
            assert!(!serde_json::to_string(&error).unwrap().contains(seed));
        }
    }

    #[test]
    fn unknown_coded_family_fails_closed() {
        let error = IpcError::from("attacker.injected:secret payload".to_string());
        assert_eq!(error, unexpected());
    }

    #[test]
    fn maps_only_narrow_behavior_compatibility_messages() {
        let cases = [
            (
                "SkillPort state portability cancelled",
                "operation.cancelled",
            ),
            (
                "Invalid SkillPort state JSON: expected value",
                "portable_state.invalid_manifest_json",
            ),
            (
                "Unsupported SkillPort state export kind",
                "portable_state.unsupported_export_kind",
            ),
            (
                "Unsupported SkillPort state export version: 99",
                "portable_state.unsupported_export_version",
            ),
            (
                "GitHub denied access while reading the repository (HTTP 403).",
                "github_import.access_denied",
            ),
            (
                "SSH password for target 'remote' is not available. Open Settings, enter the password for this target, save it, and retry.",
                "credential.ssh_password_unavailable",
            ),
        ];
        for (message, expected) in cases {
            assert_eq!(IpcError::from(message).code, expected);
        }
    }

    #[test]
    fn maps_reviewed_plain_errors_without_copying_dynamic_details() {
        let cases = [
            ("Saved view 'private-id' not found", "resource.not_found"),
            ("Tag group 'private-id' not found", "resource.not_found"),
            (
                "Collection 'private-id' not found after update",
                "resource.not_found",
            ),
            ("Agent 'private-id' not found", "resource.not_found"),
            (
                "Skill 'private-id' not found in central library",
                "resource.not_found",
            ),
            ("Project 'private-id' not found", "resource.not_found"),
            (
                "UNIQUE constraint failed: skills.private_column",
                "resource.conflict",
            ),
            (
                "Permission denied for C:\\private\\skill",
                "permission.denied",
            ),
            ("Saved view name cannot be empty", "input.invalid"),
        ];

        for (message, code) in cases {
            let error = IpcError::from(message);
            assert_eq!(error.code, code);
            assert!(!error.message.contains("private"));
        }
    }

    #[test]
    fn arbitrary_diagnostics_always_use_the_fixed_fallback() {
        let seeds = [
            r"C:\Users\alice\private\skill.md",
            r"C:/Users/alice/private/skill.md",
            r"..\alice\private\skill.md",
            "/home/alice/private/skill.md",
            "../alice/private/skill.md",
            "ssh -i private.pem host -- command",
            "stdout: first line\nstderr: second line",
            "ghp_super_secret",
            "sk-live-secret",
            "-----BEGIN PRIVATE KEY-----",
            "file content: private thesis text",
            "https://example.invalid/path?token=secret",
            "quoted data: 'private value'",
            "UNKNOWN_ENV=private-value",
            "resource not found: relative/private.txt",
            "request timed out after output=private-value",
        ];
        for seed in seeds {
            let error = IpcError::from(seed);
            let serialized = serde_json::to_string(&error).expect("serialize");
            assert_eq!(error, unexpected(), "unexpected classification for {seed}");
            assert!(!serialized.contains(seed), "leaked seed: {seed}");
        }
    }

    #[test]
    fn display_is_only_the_public_message() {
        let error = unexpected();
        assert_eq!(error.to_string(), INTERNAL_MESSAGE);
    }
}
