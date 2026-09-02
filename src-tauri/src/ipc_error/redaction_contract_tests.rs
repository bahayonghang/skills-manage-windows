use super::{unexpected, IpcError};

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
fn skills_cli_contract_codes_keep_reviewed_public_messages() {
    for (code, message, retryable) in [
        (
            "skills_cli.skill_not_owned",
            "That skill is not managed by Skills CLI.",
            false,
        ),
        (
            "skills_cli.canonical_missing",
            "The skill folder is missing.",
            false,
        ),
        (
            "skills_cli.skill_doc_missing",
            "The SKILL.md file is missing.",
            false,
        ),
        (
            "skills_cli.skill_doc_too_large",
            "The SKILL.md file is too large to open.",
            false,
        ),
        (
            "skills_cli.skill_doc_invalid_utf8",
            "The SKILL.md file is not valid text.",
            false,
        ),
        (
            "skills_cli.direct_copy_not_toggleable",
            "A copied skill folder cannot be linked or unlinked.",
            false,
        ),
        (
            "skills_cli.placement_conflict",
            "The platform folder is in conflict.",
            false,
        ),
        (
            "skills_cli.placement_unavailable",
            "The platform folder is unavailable.",
            false,
        ),
        (
            "skills_cli.export_invalid",
            "The inventory export is invalid.",
            false,
        ),
        (
            "skills_cli.export_failed",
            "The inventory export could not be saved.",
            false,
        ),
        (
            "skills_cli.reveal_failed",
            "The skill folder could not be revealed.",
            false,
        ),
        (
            "skills_cli.cli_failed",
            "The Skills CLI command did not complete successfully.",
            false,
        ),
        (
            "skills_cli.recovery_required",
            "A previous Skills CLI remove needs recovery.",
            true,
        ),
        (
            "skills_cli.update_stale",
            "The update is out of date. Refresh, then try again.",
            true,
        ),
        (
            "skills_cli.update_rate_limited",
            "GitHub rate limited the update check. Wait for the limit to reset, then retry.",
            true,
        ),
        (
            "skills_cli.update_recovery_required",
            "A previous Skills CLI update needs recovery.",
            true,
        ),
        (
            "skills_cli.update_unsupported",
            "This skill source cannot be updated.",
            false,
        ),
    ] {
        let error = IpcError::from(format!("{code}:C:\\\\Users\\\\secret\\\\SKILL.md"));
        assert_eq!(error.code, code);
        assert_eq!(error.message, message);
        assert_eq!(error.retryable, retryable);
        assert!(!error.message.contains("Users"));
        assert!(!serde_json::to_string(&error).unwrap().contains("secret"));
    }
}

#[test]
fn usage_remote_fatal_errors_keep_reviewed_redacted_payloads() {
    use crate::services::usage::UsageError;
    use crate::targets::TargetsError;

    let seeds = [
        r"/home/alice/.ssh/id_ed25519",
        "ssh -i private.pem host -- find /home/alice",
        "stderr: Permission denied for /var/secret.log",
        "alice@prod.example.invalid",
    ];
    let fixtures = [
        UsageError::from_remote(TargetsError::ProcessTimedOut {
            transport: "SSH",
            class: "probe",
            timeout_ms: 5_000,
        }),
        UsageError::from_remote(TargetsError::RemoteInspectFailed {
            path: seeds[0].to_string(),
            detail: format!("{}\n{}\n{}", seeds[1], seeds[2], seeds[3]),
        }),
        UsageError::from_remote(TargetsError::RemoteStdoutNotUtf8(
            String::from_utf8(vec![0xff]).unwrap_err(),
        )),
    ];
    let expected = [
        (
            "usage.remote_transport",
            "Remote usage refresh failed because the target is unavailable.",
            true,
        ),
        (
            "usage.remote_permission",
            "Remote usage refresh failed because access was denied.",
            false,
        ),
        (
            "usage.remote_protocol",
            "Remote usage refresh failed because the target protocol is invalid.",
            false,
        ),
    ];
    for (error, (code, message, retryable)) in fixtures.into_iter().zip(expected) {
        assert_eq!(error.stable_code(), code);
        assert_eq!(error.public_message(), message);
        assert_eq!(error.retryable(), retryable);
        let ipc = IpcError::new(
            error.stable_code(),
            error.public_message(),
            error.retryable(),
        );
        assert_eq!(ipc.code, code);
        assert_eq!(ipc.message, message);
        assert_eq!(ipc.retryable, retryable);
        let serialized = serde_json::to_string(&ipc).expect("serialize");
        for seed in seeds {
            assert!(!error.to_string().contains(seed), "Display leaked {seed}");
            assert!(
                !serialized.contains(seed),
                "IPC leaked {seed}: {serialized}"
            );
            assert!(!ipc.message.contains(seed));
        }
        let miswired = IpcError::from(format!(
            "{} {} {} {} {}",
            error, seeds[0], seeds[1], seeds[2], seeds[3]
        ));
        let miswired_json = serde_json::to_string(&miswired).expect("serialize");
        for seed in seeds {
            assert!(
                !miswired_json.contains(seed),
                "legacy IPC leaked {seed}: {miswired_json}"
            );
        }
    }
}
