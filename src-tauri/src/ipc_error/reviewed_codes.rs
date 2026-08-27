/// Complete allowlist of locale-neutral IPC error codes that may cross into
/// structured Runtime diagnostics. Adding a code is an explicit review point;
/// syntax validity alone is never sufficient.
pub const REVIEWED_IPC_ERROR_CODES: &[&str] = &[
    "ai.client_build_failed",
    "ai.connect",
    "ai.dns",
    "ai.empty_response",
    "ai.invalid_api_key",
    "ai.missing_api_key",
    "ai.network",
    "ai.proxy",
    "ai.rate_limit",
    "ai.request_failed",
    "ai.response_error",
    "ai.response_parse_failed",
    "ai.response_read_failed",
    "ai.timeout",
    "ai.tls",
    "central.reset_failed",
    "central_operation.delete_restore_collision",
    "central_skills.budget_exceeded",
    "central_skills.database_failed",
    "central_skills.delete_failed",
    "central_skills.delete_preview_failed",
    "central_skills.force_delete_blocked",
    "central_skills.mutation_lock_failed",
    "central_skills.remote_failed",
    "central_updates.inventory_invariant",
    "central_updates.inventory_refresh_required",
    "central_updates.relocation_failed",
    "central_updates.repository_check_failed",
    "central_updates.skill_source_missing",
    "central_updates.snapshot_changed",
    "credential.ssh_password_unavailable",
    "github_import.access_denied",
    "github_import.archive_redirect_rejected",
    "github_import.archive_unavailable",
    "github_import.branch_conflict",
    "github_import.branch_invalid",
    "github_import.budget_exceeded",
    "github_import.configured_token_failed",
    "github_import.credential_unavailable",
    "github_import.duplicate_selection",
    "github_import.invalid_candidate",
    "github_import.invalid_url",
    "github_import.no_importable_skills",
    "github_import.preview_busy",
    "github_import.preview_capacity",
    "github_import.preview_cleanup_pending",
    "github_import.preview_commit_unresolved",
    "github_import.preview_expired",
    "github_import.preview_integrity",
    "github_import.preview_mismatch",
    "github_import.preview_missing",
    "github_import.rate_limited",
    "github_import.rename_conflict",
    "github_import.repo_not_found",
    "github_import.response_invalid",
    "github_import.selection_unavailable",
    "github_import.source_path_missing",
    "github_import.target_exists",
    "github_import.transport_failed",
    "input.invalid",
    "installation.pending_central_recovery",
    "internal.unexpected",
    "job.central_update_busy",
    "job.id_mismatch",
    "job.invalid_id",
    "job.portability_busy",
    "job.registry_unavailable",
    "local_archive.ambiguous_archive_layout",
    "local_archive.archive_changed_since_preview",
    "local_archive.archive_not_found",
    "local_archive.archive_read_failed",
    "local_archive.budget_exceeded",
    "local_archive.central_mutation",
    "local_archive.db",
    "local_archive.internal",
    "local_archive.invalid_archive_entry",
    "local_archive.invalid_skill_identifier",
    "local_archive.io",
    "local_archive.no_skill_manifest",
    "local_archive.path_conflict",
    "local_archive.remote_target_unsupported",
    "local_archive.rollback_failed",
    "local_archive.skill_frontmatter_missing",
    "local_archive.unsupported_archive_entry",
    "marketplace.identity_ambiguous",
    "marketplace.install_failed",
    "marketplace.install_unavailable",
    "marketplace.registry_disabled",
    "marketplace.registry_stale",
    "marketplace.source_unsupported",
    "operation.cancelled",
    "permission.denied",
    "portable_state.invalid_manifest_json",
    "portable_state.unsupported_export_kind",
    "portable_state.unsupported_export_version",
    "recovery.reconcile_guard_unavailable",
    "recovery.reconcile_preflight_blocked",
    "resource.conflict",
    "resource.not_found",
    "runtime.desktop_required",
    "skills_cli.agent_unmapped",
    "skills_cli.busy",
    "skills_cli.cancelled",
    "skills_cli.canonical_missing",
    "skills_cli.cli_failed",
    "skills_cli.cli_unavailable",
    "skills_cli.direct_copy_not_toggleable",
    "skills_cli.export_failed",
    "skills_cli.export_invalid",
    "skills_cli.local_target_only",
    "skills_cli.node_missing",
    "skills_cli.placement_conflict",
    "skills_cli.placement_unavailable",
    "skills_cli.preview_unparsed",
    "skills_cli.recovery_required",
    "skills_cli.remote_unavailable",
    "skills_cli.reveal_failed",
    "skills_cli.selection_empty",
    "skills_cli.skill_doc_invalid_utf8",
    "skills_cli.skill_doc_missing",
    "skills_cli.skill_doc_too_large",
    "skills_cli.skill_not_owned",
    "skills_cli.source_invalid",
    "skills_cli.timeout",
    "skills_cli.update_baseline_required",
    "skills_cli.update_check_failed",
    "skills_cli.update_integrity",
    "skills_cli.update_local_modified",
    "skills_cli.update_migration",
    "skills_cli.update_rate_limited",
    "skills_cli.update_recovery_required",
    "skills_cli.update_stale",
    "skills_cli.update_topology_conflict",
    "skills_cli.update_unsupported",
    "startup.rebuild_unavailable",
    "storage.unavailable",
];

pub fn is_reviewed_ipc_code(code: &str) -> bool {
    REVIEWED_IPC_ERROR_CODES.contains(&code)
}

pub(super) fn direct_public_message(code: &str) -> Option<&'static str> {
    match code {
        "credential.ssh_password_unavailable" => {
            Some("The SSH password is unavailable. Open Settings, save it, and retry.")
        }
        "input.invalid" => Some("The request is invalid."),
        "internal.unexpected" => Some("The operation failed. See runtime logs for details."),
        "job.central_update_busy" => Some("A Central update job is already running."),
        "job.id_mismatch" => Some("The cancellation request does not match the active job."),
        "job.invalid_id" => Some("The job identifier is invalid."),
        "job.registry_unavailable" => Some("The job registry is unavailable."),
        "marketplace.identity_ambiguous" => {
            Some("The Marketplace registry contains an ambiguous skill identity.")
        }
        "marketplace.install_failed" => Some("The Marketplace skill could not be installed."),
        "marketplace.install_unavailable" => {
            Some("Marketplace installation is unavailable for the selected target.")
        }
        "marketplace.registry_disabled" => Some("The Marketplace registry is disabled."),
        "marketplace.registry_stale" => {
            Some("The Marketplace cache is stale. Sync the registry and try again.")
        }
        "marketplace.source_unsupported" => {
            Some("The Marketplace registry source is not supported.")
        }
        "operation.cancelled" => Some("The operation was cancelled."),
        "permission.denied" => Some("Permission was denied."),
        "portable_state.invalid_manifest_json" => {
            Some("The SkillPort state file is not valid JSON.")
        }
        "portable_state.unsupported_export_kind" => {
            Some("The SkillPort state export kind is not supported.")
        }
        "portable_state.unsupported_export_version" => {
            Some("The SkillPort state export version is not supported.")
        }
        "resource.conflict" => Some("The requested resource conflicts with existing data."),
        "resource.not_found" => Some("The requested resource was not found."),
        "runtime.desktop_required" => Some("This operation requires the Tauri desktop runtime."),
        "storage.unavailable" => Some("Storage is unavailable."),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::REVIEWED_IPC_ERROR_CODES;

    #[test]
    fn reviewed_codes_are_unique_and_have_public_messages() {
        assert_eq!(
            REVIEWED_IPC_ERROR_CODES
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            REVIEWED_IPC_ERROR_CODES.len()
        );
        let missing = REVIEWED_IPC_ERROR_CODES
            .iter()
            .copied()
            .filter(|code| crate::ipc_error::public_message_for_code(code).is_none())
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "reviewed codes without messages: {missing:?}"
        );
    }
}
