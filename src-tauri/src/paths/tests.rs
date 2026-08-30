use super::*;
use std::ffi::OsString;
use tempfile::TempDir;

#[cfg(windows)]
#[test]
fn resolve_home_dir_prefers_userprofile_on_windows_even_when_home_is_set() {
    let home = resolve_home_dir_with(|key| match key {
        "HOME" => Some("/custom/home".to_string()),
        "USERPROFILE" => Some(r"C:\Users\fallback".to_string()),
        "HOMEDRIVE" => Some("D:".to_string()),
        "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
        _ => None,
    });

    assert_eq!(home, PathBuf::from(r"C:\Users\fallback"));
}

#[cfg(not(windows))]
#[test]
fn resolve_home_dir_prefers_home_on_non_windows() {
    let home = resolve_home_dir_with(|key| match key {
        "HOME" => Some("/custom/home".to_string()),
        "USERPROFILE" => Some(r"C:\Users\fallback".to_string()),
        "HOMEDRIVE" => Some("D:".to_string()),
        "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
        _ => None,
    });

    assert_eq!(home, PathBuf::from("/custom/home"));
}

#[cfg(windows)]
#[test]
fn resolve_home_dir_uses_home_drive_and_path_before_home_on_windows() {
    let home = resolve_home_dir_with(|key| match key {
        "HOME" => Some("/custom/home".to_string()),
        "USERPROFILE" => None,
        "HOMEDRIVE" => Some("D:".to_string()),
        "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
        _ => None,
    });

    assert_eq!(home, PathBuf::from(r"D:\Users\drive-path"));
}

#[test]
fn resolve_home_dir_falls_back_to_userprofile() {
    let home = resolve_home_dir_with(|key| match key {
        "HOME" => None,
        "USERPROFILE" => Some(r"C:\Users\lyh".to_string()),
        "HOMEDRIVE" => Some("D:".to_string()),
        "HOMEPATH" => Some(r"\Users\drive-path".to_string()),
        _ => None,
    });

    assert_eq!(home, PathBuf::from(r"C:\Users\lyh"));
}

#[cfg(windows)]
#[test]
fn resolve_home_dir_falls_back_to_home_drive_and_path() {
    let home = resolve_home_dir_with(|key| match key {
        "HOME" => Some("/custom/home".to_string()),
        "USERPROFILE" => None,
        "HOMEDRIVE" => Some("D:".to_string()),
        "HOMEPATH" => Some(r"\Users\lyh".to_string()),
        _ => None,
    });

    assert_eq!(home, PathBuf::from(r"D:\Users\lyh"));
}

#[test]
fn resolve_home_dir_uses_platform_temp_dir_as_last_resort() {
    let home = resolve_home_dir_with(|_| None);
    assert_eq!(home, std::env::temp_dir());
}

#[test]
fn resolve_home_dir_accepts_os_string_env_values() {
    let home =
        resolve_home_dir_from_env_vars(None, Some(OsString::from(r"C:\Users\alice")), None, None);

    assert_eq!(home, PathBuf::from(r"C:\Users\alice"));
}

#[test]
fn central_skills_dir_is_built_under_home() {
    let central = central_skills_dir_from_home(Path::new(r"C:\Users\lyh"));
    assert_eq!(
        central,
        PathBuf::from(r"C:\Users\lyh")
            .join(".skillsmanage")
            .join("skills")
    );
}

#[test]
fn universal_skills_dir_is_built_under_home() {
    let universal = universal_skills_dir_from_home(Path::new(r"C:\Users\lyh"));
    assert_eq!(
        universal,
        PathBuf::from(r"C:\Users\lyh")
            .join(".agents")
            .join("skills")
    );
}

#[test]
fn app_data_dir_is_built_under_home() {
    let app_dir = app_data_dir();
    assert!(app_dir.ends_with(".skillsmanage"));
}

#[cfg(windows)]
#[test]
fn app_data_dir_uses_legacy_home_db_when_preferred_windows_profile_is_empty() {
    let temp = TempDir::new().unwrap();
    let preferred_home = temp.path().join("userprofile-home");
    let legacy_home = temp.path().join("git-bash-home");
    std::fs::create_dir_all(&preferred_home).unwrap();
    let legacy_app_dir = app_data_dir_from_home(&legacy_home);
    std::fs::create_dir_all(&legacy_app_dir).unwrap();
    std::fs::write(legacy_app_dir.join(APP_DATABASE_FILE_NAME), b"legacy-db").unwrap();

    let app_dir = app_data_dir_from_env_vars(
        Some(OsString::from(legacy_home.as_os_str())),
        Some(OsString::from(preferred_home.as_os_str())),
        None,
        None,
    );

    assert!(paths_equivalent(&app_dir, &legacy_app_dir));
}

#[cfg(windows)]
#[test]
fn app_data_dir_prefers_windows_profile_when_it_already_has_a_db() {
    let temp = TempDir::new().unwrap();
    let preferred_home = temp.path().join("userprofile-home");
    let legacy_home = temp.path().join("git-bash-home");
    let preferred_app_dir = app_data_dir_from_home(&preferred_home);
    let legacy_app_dir = app_data_dir_from_home(&legacy_home);
    std::fs::create_dir_all(&preferred_app_dir).unwrap();
    std::fs::create_dir_all(&legacy_app_dir).unwrap();
    std::fs::write(
        preferred_app_dir.join(APP_DATABASE_FILE_NAME),
        b"preferred-db",
    )
    .unwrap();
    std::fs::write(legacy_app_dir.join(APP_DATABASE_FILE_NAME), b"legacy-db").unwrap();

    let app_dir = app_data_dir_from_env_vars(
        Some(OsString::from(legacy_home.as_os_str())),
        Some(OsString::from(preferred_home.as_os_str())),
        None,
        None,
    );

    assert!(paths_equivalent(&app_dir, &preferred_app_dir));
}

#[test]
fn expand_home_path_expands_unix_style_tilde() {
    let expanded = expand_home_path_with_home("~/.claude/skills", Path::new("/tmp/home"));
    assert_eq!(expanded, PathBuf::from("/tmp/home/.claude/skills"));
}

#[test]
fn expand_home_path_expands_windows_style_tilde() {
    let expanded = expand_home_path_with_home("~\\.claude\\skills", Path::new("C:\\Users\\alice"));
    assert_eq!(expanded, PathBuf::from("C:\\Users\\alice/.claude\\skills"));
}

#[test]
fn expand_home_path_leaves_absolute_paths_unchanged() {
    let expanded = expand_home_path_with_home("/opt/skills/custom", Path::new("/tmp/ignored-home"));
    assert_eq!(expanded, PathBuf::from("/opt/skills/custom"));
}

#[test]
fn expand_remote_home_path_uses_posix_separators() {
    let expanded = expand_remote_home_path("~/.agents/skills", "/home/alice");
    assert_eq!(expanded, "/home/alice/.agents/skills");
}

#[test]
fn remote_join_uses_posix_separators() {
    assert_eq!(
        remote_join("/home/alice", CENTRAL_SKILLS_REL_FROM_HOME),
        "/home/alice/.skillsmanage/skills"
    );
}

#[test]
fn remote_join_handles_root_parent_and_empty_child() {
    assert_eq!(remote_join("/", "skills"), "/skills");
    assert_eq!(remote_join("", "skills"), "/skills");
    assert_eq!(remote_join("/home/alice", ""), "/home/alice");
}

#[test]
fn remote_central_skills_root_matches_readme_semantics() {
    assert_eq!(
        remote_central_skills_root("/home/alice"),
        "/home/alice/.skillsmanage/skills"
    );
    assert_eq!(
        remote_central_skills_root("/home/alice/"),
        "/home/alice/.skillsmanage/skills"
    );
    assert_eq!(remote_central_skills_root("/"), "/.skillsmanage/skills");
}

#[test]
fn remote_repos_root_matches_readme_semantics() {
    assert_eq!(
        remote_repos_root("/home/alice"),
        "/home/alice/.skillsmanage/repos"
    );
}

#[test]
fn path_policy_dir_name_constants_are_pinned() {
    assert_eq!(APP_DATA_DIR_NAME, ".skillsmanage");
    assert_eq!(CENTRAL_SKILLS_REL_FROM_HOME, ".skillsmanage/skills");
    assert_eq!(REMOTE_REPOS_REL_FROM_HOME, ".skillsmanage/repos");
    assert_eq!(TARGETS_CACHE_DIR_NAME, "targets");
    assert_eq!(UNIVERSAL_AGENTS_DIR_NAME, ".agents");
    assert_eq!(UNIVERSAL_SKILLS_REL, ".agents/skills");
    assert_eq!(SKILLS_CLI_DIR_NAME, "skills-cli");
    assert_eq!(SKILLS_CLI_REMOVE_RECOVERY_DIR_NAME, "remove-recovery");
}

#[test]
fn skills_cli_remove_recovery_dir_is_under_app_data() {
    let app_data = Path::new("/tmp/.skillsmanage");
    assert_eq!(
        skills_cli_remove_recovery_dir_from_app_data(app_data),
        Path::new("/tmp/.skillsmanage/skills-cli/remove-recovery")
    );
}

#[test]
fn skills_cli_remove_recovery_local_path_has_no_target_subdirectory() {
    let app_data = Path::new("/tmp/.skillsmanage");
    let local = skills_cli_remove_recovery_dir_from_app_data(app_data);
    let remote = skills_cli_remove_recovery_dir_for_target_from_app_data(app_data, "ssh-cli-test");
    assert_eq!(
        local,
        Path::new("/tmp/.skillsmanage/skills-cli/remove-recovery")
    );
    assert_eq!(
        remote,
        Path::new("/tmp/.skillsmanage/skills-cli/remove-recovery/ssh-cli-test")
    );
    assert_ne!(local, remote);
}

#[test]
fn expand_remote_home_path_preserves_root_home() {
    let expanded = expand_remote_home_path("~", "/");
    assert_eq!(expanded, "/");
}

#[test]
fn expand_remote_home_path_leaves_absolute_paths_unchanged() {
    let expanded = expand_remote_home_path("/opt/skills/custom", "/home/alice");
    assert_eq!(expanded, "/opt/skills/custom");
}

#[test]
fn platform_global_skills_dir_resolves_agent_home_path() {
    let specs = [PlatformPathSpec {
        agent_id: "claude-code",
        global_skills_dir: "~/.claude/skills",
        project_skills_dir: Some(".claude/skills"),
    }];

    let resolved = platform_global_skills_dir("claude-code", &specs).unwrap();
    assert_eq!(resolved, expand_home_path("~/.claude/skills"));
}

#[test]
fn platform_project_skills_dir_keeps_relative_pattern() {
    let specs = [PlatformPathSpec {
        agent_id: "claude-code",
        global_skills_dir: "~/.claude/skills",
        project_skills_dir: Some(".claude/skills"),
    }];

    let resolved = platform_project_skills_dir("claude-code", &specs)
        .unwrap()
        .unwrap();
    assert_eq!(resolved, PathBuf::from(".claude/skills"));
}

#[test]
fn platform_paths_expand_remote_home_with_posix_separators() {
    let specs = [PlatformPathSpec {
        agent_id: "codex",
        global_skills_dir: "~/.codex/skills",
        project_skills_dir: Some("~\\.codex\\skills"),
    }];

    let global = platform_global_skills_dir_for_remote("codex", &specs, "/home/alice").unwrap();
    let project = platform_project_skills_dir_for_remote("codex", &specs, "/home/alice")
        .unwrap()
        .unwrap();
    assert_eq!(global, "/home/alice/.codex/skills");
    assert_eq!(project, "/home/alice/.codex/skills");
}

#[test]
fn platform_path_lookup_errors_for_unknown_agent() {
    let specs = [PlatformPathSpec {
        agent_id: "known",
        global_skills_dir: "~/.known/skills",
        project_skills_dir: None,
    }];

    let error = platform_global_skills_dir("missing", &specs).unwrap_err();
    assert!(error.to_string().contains("missing"));
}

#[test]
fn path_to_string_serializes_lossy_paths() {
    let path = Path::new(r"C:\Users\lyh\.agents\skills");
    assert_eq!(path_to_string(path), r"C:\Users\lyh\.agents\skills");
}

#[cfg(target_os = "macos")]
#[test]
fn normalize_stored_path_collapses_private_system_aliases() {
    assert_eq!(
        normalize_stored_path("/private/var/folders/demo"),
        "/var/folders/demo"
    );
    assert_eq!(normalize_stored_path("/private/tmp/demo"), "/tmp/demo");
}

#[cfg(target_os = "macos")]
#[test]
fn paths_equivalent_treats_private_system_aliases_as_same_location() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("demo");
    std::fs::create_dir_all(&path).unwrap();
    let canonical = path.canonicalize().unwrap();

    assert!(paths_equivalent(&path, &canonical));
}

#[test]
fn paths_equivalent_canonicalizes_existing_ancestor_for_missing_descendants() {
    let temp = TempDir::new().unwrap();
    let canonical_root = temp.path().canonicalize().unwrap();
    assert!(paths_equivalent(
        &temp.path().join("missing/child"),
        &canonical_root.join("missing/child"),
    ));
}

#[test]
fn paths_equivalent_ignores_trailing_separator() {
    assert!(paths_equivalent(
        Path::new(r"C:\Users\lyh\.agents\skills\"),
        Path::new(r"C:\Users\lyh\.agents\skills")
    ));
}
