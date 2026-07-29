use crate::targets::{ConnectedRemoteTarget, TargetsError};

use super::error::CentralSkillsError;

const ROOT_RESOLUTION_FAILED: i32 = 41;
const ROOT_NOT_DIRECTORY: i32 = 42;
const CANDIDATE_RESOLUTION_FAILED: i32 = 43;
const CANONICAL_ESCAPE: i32 = 44;
const RESOLVER_UNAVAILABLE: i32 = 45;

const CANONICAL_GUARD_SCRIPT: &str = r#"if realpath -e / >/dev/null 2>&1; then
    resolver=gnu
elif realpath / >/dev/null 2>&1; then
    resolver=plain
else
    exit 45
fi

resolve_path() {
    if [ "$resolver" = gnu ]; then
        captured=$(realpath -e "$1" 2>/dev/null; printf '%03dX' "$?")
    else
        captured=$(realpath "$1" 2>/dev/null; printf '%03dX' "$?")
    fi

    output=${captured%????}
    trailer=${captured#"$output"}
    case "$trailer" in
        [0-9][0-9][0-9]X) ;;
        *) return 2 ;;
    esac
    status=${trailer%X}
    [ "$status" = 000 ] || return 1

    newline='
'
    case "$output" in
        *"$newline") ;;
        *) return 2 ;;
    esac
    resolved=${output%"$newline"}
    [ -n "$resolved" ] || return 2
}

resolve_path "$1" || exit 41
root_real=$resolved
[ -d "$root_real" ] || exit 42

resolve_path "$2" || exit 43
candidate_real=$resolved

if [ "$root_real" = / ]; then
    case "$candidate_real" in
        /*) ;;
        *) exit 44 ;;
    esac
elif [ "$candidate_real" != "$root_real" ]; then
    case "$candidate_real" in
        "$root_real"/*) ;;
        *) exit 44 ;;
    esac
fi

printf '%s\000' "$candidate_real"
"#;

pub(super) async fn resolve_remote_allowed_path(
    connection: &ConnectedRemoteTarget,
    access_root: &str,
    requested_path: &str,
) -> Result<String, CentralSkillsError> {
    let (root, candidate) = normalize_remote_allowed_path(access_root, requested_path)?;
    let output = connection
        .run_script(CANONICAL_GUARD_SCRIPT, &[&root, &candidate])
        .await
        .map_err(map_canonical_error)?;
    parse_canonical_candidate(&output)
}

fn normalize_remote_allowed_path(
    access_root: &str,
    requested_path: &str,
) -> Result<(String, String), CentralSkillsError> {
    let root = normalize_remote_posix_path(access_root)?;
    let candidate = if requested_path.is_empty() {
        root.clone()
    } else if requested_path.starts_with('/') {
        normalize_remote_posix_path(requested_path)?
    } else {
        normalize_remote_posix_path(&format!(
            "{}/{}",
            root.trim_end_matches('/'),
            requested_path
        ))?
    };
    if !remote_path_is_within(&root, &candidate) {
        return Err(CentralSkillsError::PathEscapesSkillRoot {
            path: candidate,
            root,
        });
    }
    Ok((root, candidate))
}

fn normalize_remote_posix_path(path: &str) -> Result<String, CentralSkillsError> {
    if path.is_empty() {
        return Err(CentralSkillsError::SkillPathContextEmpty);
    }
    if path.contains('\\') {
        return Err(CentralSkillsError::RemotePathBackslash(path.to_string()));
    }
    let is_absolute = path.starts_with('/');
    let mut segments = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(CentralSkillsError::RemoteParentTraversal(path.to_string()));
        }
        segments.push(segment);
    }
    let joined = segments.join("/");
    Ok(match (is_absolute, joined.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{joined}"),
        (false, true) => ".".to_string(),
        (false, false) => joined,
    })
}

fn remote_path_is_within(root: &str, candidate: &str) -> bool {
    if root == "/" {
        return candidate.starts_with('/');
    }
    candidate == root || candidate.starts_with(&format!("{root}/"))
}

fn parse_canonical_candidate(output: &str) -> Result<String, CentralSkillsError> {
    let Some(candidate) = output.strip_suffix('\0') else {
        return Err(CentralSkillsError::RemoteCanonicalProtocol);
    };
    if candidate.is_empty() || candidate.contains('\0') {
        return Err(CentralSkillsError::RemoteCanonicalProtocol);
    }
    Ok(candidate.to_string())
}

fn map_canonical_error(error: TargetsError) -> CentralSkillsError {
    if matches!(
        error,
        TargetsError::RemoteStdoutNotUtf8(_) | TargetsError::WslStdoutNotUtf8(_)
    ) {
        return CentralSkillsError::RemoteCanonicalProtocol;
    }

    let status = match &error {
        TargetsError::RemoteCommandFailed { status, .. }
        | TargetsError::WslCommandFailed { status, .. } => status.code(),
        _ => return CentralSkillsError::Remote(error.to_string()),
    };

    match status {
        Some(ROOT_RESOLUTION_FAILED) => CentralSkillsError::RemoteCanonicalRootResolution,
        Some(ROOT_NOT_DIRECTORY) => CentralSkillsError::RemoteCanonicalRootNotDirectory,
        Some(CANDIDATE_RESOLUTION_FAILED) => CentralSkillsError::RemoteCanonicalCandidateResolution,
        Some(CANONICAL_ESCAPE) => CentralSkillsError::RemoteCanonicalEscape,
        Some(RESOLVER_UNAVAILABLE) => CentralSkillsError::RemoteCanonicalResolverUnavailable,
        _ => CentralSkillsError::RemoteCanonicalResolution,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::targets::{
        ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget, RemoteTargetConfig,
        SshAuthMethod, TargetsError, WslTargetConfig,
    };
    use crate::test_support::{exit_status, FakeRunner};

    use super::*;

    fn ssh_connection(runner: Arc<FakeRunner>) -> ConnectedRemoteTarget {
        let target = RemoteTargetConfig {
            id: "ssh-test".to_string(),
            label: "SSH test".to_string(),
            host: "example.invalid".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        };
        ConnectedRemoteTarget::Ssh(ConnectedSshTarget::for_tests_with_runner(target, runner))
    }

    fn wsl_connection(runner: Arc<FakeRunner>) -> ConnectedRemoteTarget {
        let target = WslTargetConfig {
            id: "wsl-test".to_string(),
            label: "WSL test".to_string(),
            distribution: "Ubuntu-24.04".to_string(),
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        };
        ConnectedRemoteTarget::Wsl(ConnectedWslTarget::for_tests_with_runner(target, runner))
    }

    #[test]
    fn lexical_guard_accepts_relative_absolute_and_root_paths() {
        let root = "/home/alice/.skillsmanage/skills/skill-a";
        assert_eq!(
            normalize_remote_allowed_path(root, "docs/README.md").unwrap(),
            (root.to_string(), format!("{root}/docs/README.md"))
        );
        assert_eq!(
            normalize_remote_allowed_path(root, &format!("{root}/SKILL.md")).unwrap(),
            (root.to_string(), format!("{root}/SKILL.md"))
        );
        assert_eq!(
            normalize_remote_allowed_path(root, "").unwrap(),
            (root.to_string(), root.to_string())
        );
    }

    #[test]
    fn lexical_guard_rejects_escape_forms_and_prefix_trap() {
        let root = "/home/alice/skill-a";
        assert!(matches!(
            normalize_remote_allowed_path(root, "../secret"),
            Err(CentralSkillsError::RemoteParentTraversal(_))
        ));
        assert!(matches!(
            normalize_remote_allowed_path(root, "docs\\secret"),
            Err(CentralSkillsError::RemotePathBackslash(_))
        ));
        assert!(matches!(
            normalize_remote_allowed_path(root, "/etc/passwd"),
            Err(CentralSkillsError::PathEscapesSkillRoot { .. })
        ));
        assert!(matches!(
            normalize_remote_allowed_path(root, "/home/alice/skill-ab/file"),
            Err(CentralSkillsError::PathEscapesSkillRoot { .. })
        ));
    }

    #[test]
    fn lexical_guard_preserves_tabs_and_newlines() {
        let root = "/home/alice/skill[*?]\troot\n";
        let requested = "docs/file[*?]\tname\n";
        let (normalized_root, candidate) = normalize_remote_allowed_path(root, requested).unwrap();
        assert_eq!(normalized_root, root);
        assert_eq!(candidate, format!("{root}/{requested}"));
    }

    #[test]
    fn canonical_protocol_requires_exactly_one_terminal_nul() {
        assert_eq!(
            parse_canonical_candidate("/root/path\0").unwrap(),
            "/root/path"
        );
        assert_eq!(
            parse_canonical_candidate("/root/path\n\0").unwrap(),
            "/root/path\n"
        );
        assert!(matches!(
            parse_canonical_candidate("/root/path"),
            Err(CentralSkillsError::RemoteCanonicalProtocol)
        ));
        assert!(matches!(
            parse_canonical_candidate("/root/path\0extra\0"),
            Err(CentralSkillsError::RemoteCanonicalProtocol)
        ));
        assert!(matches!(
            parse_canonical_candidate("\0"),
            Err(CentralSkillsError::RemoteCanonicalProtocol)
        ));
    }

    #[test]
    fn canonical_exit_codes_map_without_exposing_stderr() {
        let cases = [
            (
                ROOT_RESOLUTION_FAILED,
                CentralSkillsError::RemoteCanonicalRootResolution,
            ),
            (
                ROOT_NOT_DIRECTORY,
                CentralSkillsError::RemoteCanonicalRootNotDirectory,
            ),
            (
                CANDIDATE_RESOLUTION_FAILED,
                CentralSkillsError::RemoteCanonicalCandidateResolution,
            ),
            (CANONICAL_ESCAPE, CentralSkillsError::RemoteCanonicalEscape),
            (
                RESOLVER_UNAVAILABLE,
                CentralSkillsError::RemoteCanonicalResolverUnavailable,
            ),
        ];

        for (code, expected) in cases {
            let error = map_canonical_error(TargetsError::RemoteCommandFailed {
                status: exit_status(code),
                detail: "sensitive resolver stderr".to_string(),
            });
            assert_eq!(
                std::mem::discriminant(&error),
                std::mem::discriminant(&expected)
            );
            assert!(!error.to_string().contains("sensitive"));
        }

        let unknown = map_canonical_error(TargetsError::WslCommandFailed {
            status: exit_status(99),
            detail: "sensitive unknown stderr".to_string(),
        });
        assert!(matches!(
            unknown,
            CentralSkillsError::RemoteCanonicalResolution
        ));
        assert!(!unknown.to_string().contains("sensitive"));

        let invalid_ssh = map_canonical_error(TargetsError::RemoteStdoutNotUtf8(
            String::from_utf8(vec![0xff]).unwrap_err(),
        ));
        let invalid_wsl = map_canonical_error(TargetsError::WslStdoutNotUtf8(
            String::from_utf8(vec![0xff]).unwrap_err(),
        ));
        assert!(matches!(
            invalid_ssh,
            CentralSkillsError::RemoteCanonicalProtocol
        ));
        assert!(matches!(
            invalid_wsl,
            CentralSkillsError::RemoteCanonicalProtocol
        ));
    }

    #[tokio::test]
    async fn canonical_guard_has_ssh_wsl_script_argument_and_policy_parity() {
        let ssh_runner = Arc::new(FakeRunner::new());
        let wsl_runner = Arc::new(FakeRunner::new());
        ssh_runner.push_success("/canonical/skill/docs/file\tname\n\0");
        wsl_runner.push_success("/canonical/skill/docs/file\tname\n\0");
        let ssh = ssh_connection(ssh_runner.clone());
        let wsl = wsl_connection(wsl_runner.clone());
        let root = "/install/skill[*?]\troot\n";
        let requested = "docs/file[*?]\tname\n";
        let candidate = format!("{root}/{requested}");

        let ssh_resolved = resolve_remote_allowed_path(&ssh, root, requested)
            .await
            .unwrap();
        let wsl_resolved = resolve_remote_allowed_path(&wsl, root, requested)
            .await
            .unwrap();

        assert_eq!(ssh_resolved, "/canonical/skill/docs/file\tname\n");
        assert_eq!(wsl_resolved, ssh_resolved);
        let ssh_calls = ssh_runner.calls();
        let wsl_calls = wsl_runner.calls();
        assert_eq!(ssh_calls.len(), 1);
        assert_eq!(wsl_calls.len(), 1);
        assert_eq!(
            ssh_calls[0].stdin.as_deref(),
            Some(CANONICAL_GUARD_SCRIPT.as_bytes())
        );
        assert_eq!(wsl_calls[0].stdin, ssh_calls[0].stdin);
        assert_eq!(ssh_calls[0].policy, wsl_calls[0].policy);
        assert_eq!(ssh_calls[0].policy.class.label(), "standard");
        assert_eq!(
            ssh_calls[0].args.last().map(String::as_str),
            Some(format!("sh -s -- '{root}' '{candidate}'").as_str())
        );
        assert_eq!(
            &wsl_calls[0].args[wsl_calls[0].args.len() - 5..],
            ["sh", "-s", "--", root, candidate.as_str()]
        );
    }

    #[tokio::test]
    async fn canonical_escape_stops_before_follow_up_operation() {
        let runner = Arc::new(FakeRunner::new());
        runner.push_output(CANONICAL_ESCAPE, "", "realpath detail must stay private");
        let connection = ssh_connection(runner.clone());

        let error = resolve_remote_allowed_path(&connection, "/root", "docs/passwd")
            .await
            .unwrap_err();

        assert!(matches!(error, CentralSkillsError::RemoteCanonicalEscape));
        assert_eq!(runner.calls().len(), 1);
    }
}
