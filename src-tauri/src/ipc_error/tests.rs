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
fn correlation_id_is_additive_and_camel_case() {
    let operation_id = uuid::Uuid::new_v4().to_string();
    let value = serde_json::to_value(
        IpcError::new("operation.cancelled", "Operation cancelled", false)
            .with_correlation_id(&operation_id),
    )
    .expect("serialize IPC error");
    assert_eq!(value["correlationId"], operation_id);
    assert_eq!(value.as_object().unwrap().len(), 4);
}

#[test]
fn legacy_three_field_payload_deserializes_without_correlation() {
    let value = serde_json::json!({
        "code": "operation.cancelled",
        "message": "Operation cancelled",
        "retryable": false
    });
    let error: IpcError = serde_json::from_value(value).expect("deserialize legacy payload");
    assert!(error.correlation_id.is_none());
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
        (
            "github_import.no_importable_skills",
            "This GitHub repository does not contain an importable skill.",
        ),
        (
            "github_import.selection_unavailable",
            "The selected skill is no longer available in the repository preview.",
        ),
        (
            "github_import.invalid_candidate",
            "The repository contains a skill that cannot be imported.",
        ),
        (
            "github_import.source_path_missing",
            "The selected path no longer contains an importable skill.",
        ),
        (
            "github_import.target_exists",
            "The Central target directory already exists and cannot be overwritten.",
        ),
        (
            "github_import.duplicate_selection",
            "The same skill path was selected more than once in this import.",
        ),
        (
            "github_import.rename_conflict",
            "The renamed skill id is not available.",
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
fn display_is_only_the_public_message() {
    let error = unexpected();
    assert_eq!(error.to_string(), INTERNAL_MESSAGE);
}

#[test]
fn delete_restore_collision_is_not_internal_unexpected() {
    let seeds = [
        r"C:\Users\alice\.skillsmanage\skills\yao-meta",
        "ghp_super_secret",
        "manifest_json",
    ];
    for seed in seeds {
        let error = IpcError::from(format!("central_operation.delete_restore_collision:{seed}"));
        assert_eq!(error.code, "central_operation.delete_restore_collision");
        assert_eq!(
                error.message,
                "Central recovery evidence conflicts with the current files. Review and resolve the pending operation in Operation Logs."
            );
        assert!(!error.retryable);
        assert!(!serde_json::to_string(&error).unwrap().contains(seed));
    }
    let uncoded = IpcError::from(
        "Central operation recovery collision (delete_restore_collision)".to_string(),
    );
    assert_eq!(uncoded.code, INTERNAL_CODE);
}

#[test]
fn force_delete_blocked_keeps_reviewed_public_message() {
    let error = IpcError::from(
        "central_skills.force_delete_blocked:This Central skill could not be deleted.".to_string(),
    );
    assert_eq!(error.code, "central_skills.force_delete_blocked");
    assert_eq!(
        error.message,
        "Force delete is not available for this Central skill."
    );
    assert!(!error.retryable);
}

#[test]
fn generic_delete_failed_codes_are_not_internal_unexpected() {
    for code in [
        "central_skills.delete_failed",
        "central_skills.delete_preview_failed",
        "central_skills.database_failed",
        "central_skills.remote_failed",
        "central_skills.budget_exceeded",
    ] {
        let error = IpcError::from(format!("{code}:Skill 'yao-meta' not found"));
        assert_eq!(error.code, code);
        assert_eq!(error.message, "This Central skill could not be deleted.");
        assert!(!serde_json::to_string(&error).unwrap().contains("yao-meta"));
    }
}
