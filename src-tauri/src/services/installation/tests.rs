#![cfg(test)]
#![allow(unused_imports)]

//! Integration tests for `services::installation`.
//!
//! Mirrors the original `commands::linker` test module after the Phase 3c
//! split: helpers spin up an in-memory SQLite pool, point Central / Claude /
//! Cursor / Codex agent rows at temporary directories, and then drive the
//! install / uninstall paths.

use std::fs;
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use tempfile::TempDir;

use crate::db::{self, AgentSkillObservation, DbPool, SkillInstallation};
use crate::targets::RemotePathInfo;

use super::batch::batch_install_central_skills_impl;
use super::fs_util::make_relative_path;
use super::native::{
    install_central_skill_to_agent_outcome_by_method, install_skill_to_agent_copy_impl,
    install_skill_to_agent_impl, uninstall_skill_from_agent_impl,
    uninstall_skill_from_agent_with_row_impl,
};
use super::project::{
    classify_remote_project_existing_target, install_central_skill_to_project_outcome_impl,
    normalize_remote_project_path, remote_project_install_paths, remote_project_method,
    remote_project_relative_skills_dir, RemoteProjectExistingTargetAction,
};
use super::remote::{classify_remote_existing_install_target, RemoteExistingInstallAction};
use super::types::BatchUninstallSkillRequest;
use super::types::{BatchInstallResult, FailedInstall, InstallOutcome};

// ── Test helpers ──────────────────────────────────────────────────────────

/// Create an in-memory SQLite pool with the full schema initialised and
/// the central/claude-code agent directories redirected to `central_dir`
/// and `agent_dir` respectively.
async fn setup_db(central_dir: &Path, agent_dir: &Path) -> DbPool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(central_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(agent_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    pool
}

async fn setup_db_with_codex(
    central_dir: &Path,
    claude_agent_dir: &Path,
    codex_agent_dir: &Path,
) -> DbPool {
    let pool = setup_db(central_dir, claude_agent_dir).await;

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'codex'")
        .bind(codex_agent_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    pool
}

/// Create a minimal skill directory containing a valid `SKILL.md`.
fn create_central_skill(central_dir: &Path, skill_id: &str) -> PathBuf {
    let skill_dir = central_dir.join(skill_id);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        format!(
            "---\nname: {}\ndescription: Test skill\n---\n\n# {}\n",
            skill_id, skill_id
        ),
    )
    .unwrap();
    skill_dir
}

fn create_user_skill(agent_dir: &Path, skill_id: &str) -> PathBuf {
    let skill_dir = agent_dir.join(skill_id);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        format!(
            "---\nname: {}\ndescription: User skill\n---\n\n# {}\n",
            skill_id, skill_id
        ),
    )
    .unwrap();
    skill_dir
}

#[cfg(unix)]
fn create_symlink_for_test(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(windows)]
fn create_symlink_for_test(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_dir(target, link).unwrap();
}

fn claude_observation(
    agent_dir: &Path,
    skill_id: &str,
    dir_path: &Path,
    source_kind: &str,
    is_read_only: bool,
) -> AgentSkillObservation {
    AgentSkillObservation {
        row_id: format!("claude-code::{}", dir_path.to_string_lossy()),
        agent_id: "claude-code".to_string(),
        skill_id: skill_id.to_string(),
        name: skill_id.to_string(),
        description: Some("Observed skill".to_string()),
        file_path: dir_path.join("SKILL.md").to_string_lossy().into_owned(),
        dir_path: dir_path.to_string_lossy().into_owned(),
        source_kind: source_kind.to_string(),
        source_root: if source_kind == "user" {
            agent_dir.to_string_lossy().into_owned()
        } else {
            dir_path
                .parent()
                .unwrap_or(dir_path)
                .to_string_lossy()
                .into_owned()
        },
        link_type: "native".to_string(),
        symlink_target: None,
        is_read_only,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

async fn point_codex_to_dir(pool: &DbPool, skills_dir: &Path) {
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'codex'")
        .bind(skills_dir.to_str().unwrap())
        .execute(pool)
        .await
        .unwrap();
}

// ── make_relative_path ────────────────────────────────────────────────────

#[test]
fn test_make_relative_path_sibling_dirs() {
    let from = Path::new("/home/user/claude/skills");
    let to = Path::new("/home/user/.agents/skills/my-skill");
    let rel = make_relative_path(from, to);
    assert_eq!(rel, PathBuf::from("../../.agents/skills/my-skill"));
}

#[test]
fn test_make_relative_path_same_parent() {
    let from = Path::new("/tmp/test/agent");
    let to = Path::new("/tmp/test/central/skill-x");
    let rel = make_relative_path(from, to);
    assert_eq!(rel, PathBuf::from("../central/skill-x"));
}

#[test]
fn test_make_relative_path_deep_nesting() {
    let from = Path::new("/a/b/c/d");
    let to = Path::new("/a/x/y");
    let rel = make_relative_path(from, to);
    assert_eq!(rel, PathBuf::from("../../../x/y"));
}

#[test]
fn test_remote_symlink_install_replaces_existing_symlink() {
    let info = RemotePathInfo {
        file_type: "symlink".to_string(),
        symlink_target: Some("/central/demo".to_string()),
    };

    let action =
        classify_remote_existing_install_target("/agent/demo", "symlink", Some(&info), None);

    assert_eq!(action, RemoteExistingInstallAction::RemoveSymlink);
}

#[test]
fn test_remote_symlink_install_replaces_managed_copy_dir() {
    let info = RemotePathInfo {
        file_type: "dir".to_string(),
        symlink_target: None,
    };
    let installation = SkillInstallation {
        skill_id: "demo".to_string(),
        agent_id: "codex".to_string(),
        installed_path: "/agent/demo".to_string(),
        link_type: "copy".to_string(),
        symlink_target: None,
        created_at: "2026-04-27T00:00:00Z".to_string(),
    };

    let action = classify_remote_existing_install_target(
        "/agent/demo",
        "symlink",
        Some(&info),
        Some(&installation),
    );

    assert_eq!(action, RemoteExistingInstallAction::RemoveManagedCopy);
}

#[test]
fn test_remote_symlink_install_rejects_unmanaged_dir() {
    let info = RemotePathInfo {
        file_type: "dir".to_string(),
        symlink_target: None,
    };

    let action =
        classify_remote_existing_install_target("/agent/demo", "symlink", Some(&info), None);

    match action {
        RemoteExistingInstallAction::Reject(error) => {
            assert!(error.contains("remote directory"));
            assert!(error.contains("/agent/demo"));
        }
        other => panic!("expected rejection, got {:?}", other),
    }
}

#[test]
fn test_remote_project_paths_use_agent_project_dir() {
    let agent = db::Agent {
        id: "claude-code".to_string(),
        display_name: "Claude Code".to_string(),
        category: "coding".to_string(),
        global_skills_dir: "/home/alice/.claude/skills".to_string(),
        project_skills_dir: Some(".claude/skills".to_string()),
        icon_name: None,
        is_detected: true,
        is_builtin: true,
        is_enabled: true,
    };

    let paths =
        remote_project_install_paths("/home/alice", "/work/demo/", &agent, "frontend-design")
            .unwrap();

    assert_eq!(paths.project_path, "/work/demo");
    assert_eq!(paths.project_skills_dir, "/work/demo/.claude/skills");
    assert_eq!(
        paths.target_path,
        "/work/demo/.claude/skills/frontend-design"
    );
}

#[test]
fn test_remote_project_paths_expand_home_and_universal_dir() {
    let agent = db::Agent {
        id: "codex".to_string(),
        display_name: "Codex".to_string(),
        category: "coding".to_string(),
        global_skills_dir: "/home/alice/.agents/skills".to_string(),
        project_skills_dir: Some(".codex/skills".to_string()),
        icon_name: None,
        is_detected: true,
        is_builtin: true,
        is_enabled: true,
    };

    assert_eq!(
        remote_project_relative_skills_dir(&agent).unwrap(),
        db::UNIVERSAL_PROJECT_SKILLS_DIR
    );
    let paths =
        remote_project_install_paths("/home/alice", "~/repo", &agent, "code-reviewer").unwrap();

    assert_eq!(paths.project_path, "/home/alice/repo");
    assert_eq!(
        paths.target_path,
        "/home/alice/repo/.agents/skills/code-reviewer"
    );
}

#[test]
fn test_remote_project_paths_use_grok_project_dir() {
    let agent = db::Agent {
        id: "grok".to_string(),
        display_name: "Grok".to_string(),
        category: "coding".to_string(),
        global_skills_dir: "/home/alice/.grok/skills".to_string(),
        project_skills_dir: Some(".grok/skills".to_string()),
        icon_name: None,
        is_detected: true,
        is_builtin: true,
        is_enabled: true,
    };

    assert_eq!(
        remote_project_relative_skills_dir(&agent).unwrap(),
        ".grok/skills"
    );
    let paths =
        remote_project_install_paths("/home/alice", "~/repo", &agent, "code-reviewer").unwrap();

    assert_eq!(paths.project_path, "/home/alice/repo");
    assert_eq!(
        paths.target_path,
        "/home/alice/repo/.grok/skills/code-reviewer"
    );
}

#[test]
fn test_remote_project_path_requires_absolute_posix_path() {
    let agent = db::Agent {
        id: "claude-code".to_string(),
        display_name: "Claude Code".to_string(),
        category: "coding".to_string(),
        global_skills_dir: "/home/alice/.claude/skills".to_string(),
        project_skills_dir: Some(".claude/skills".to_string()),
        icon_name: None,
        is_detected: true,
        is_builtin: true,
        is_enabled: true,
    };

    let error =
        remote_project_install_paths("/home/alice", "relative/repo", &agent, "demo").unwrap_err();

    assert!(error.contains("absolute POSIX path"));
}

#[test]
fn test_remote_project_install_replaces_existing_symlink_only() {
    let info = RemotePathInfo {
        file_type: "symlink".to_string(),
        symlink_target: Some("/central/demo".to_string()),
    };

    let action = classify_remote_project_existing_target(
        "/project/.agents/skills/demo",
        "copy",
        Some(&info),
    );

    assert_eq!(action, RemoteProjectExistingTargetAction::ReplaceSymlink);
}

#[test]
fn test_remote_project_install_rejects_existing_real_directory() {
    let info = RemotePathInfo {
        file_type: "dir".to_string(),
        symlink_target: None,
    };

    let action = classify_remote_project_existing_target(
        "/project/.agents/skills/demo",
        "copy",
        Some(&info),
    );

    match action {
        RemoteProjectExistingTargetAction::Reject(error) => {
            assert!(error.contains("remote project directory"));
            assert!(error.contains("/project/.agents/skills/demo"));
        }
        other => panic!("expected rejection, got {:?}", other),
    }
}

#[test]
fn test_remote_project_method_rejects_disabled_symlink() {
    assert_eq!(remote_project_method("copy", false).unwrap(), "copy");
    let error = remote_project_method("symlink", false).unwrap_err();
    assert!(error.contains("Remote symlink install is disabled"));
    assert_eq!(remote_project_method("symlink", true).unwrap(), "symlink");
}

#[test]
fn test_normalize_remote_project_path_keeps_root() {
    assert_eq!(normalize_remote_project_path("/home/alice", "/"), "/");
    assert_eq!(
        normalize_remote_project_path("/home/alice", "~\\repo\\demo\\"),
        "/home/alice/repo/demo"
    );
}

// ── install_skill_to_agent_impl ───────────────────────────────────────────

#[tokio::test]
async fn test_install_creates_symlink() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;

    create_central_skill(&central_dir, "my-skill");

    let result = install_skill_to_agent_impl(&pool, "my-skill", "claude-code").await;
    assert!(result.is_ok(), "install should succeed: {:?}", result);

    let symlink_path = agent_dir.join("my-skill");
    let meta = fs::symlink_metadata(&symlink_path).unwrap();
    assert!(meta.file_type().is_symlink(), "entry should be a symlink");
}

#[tokio::test]
async fn test_install_symlink_is_relative() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "rel-skill");

    install_skill_to_agent_impl(&pool, "rel-skill", "claude-code")
        .await
        .unwrap();

    let symlink_path = agent_dir.join("rel-skill");
    let link_target = fs::read_link(&symlink_path).unwrap();
    assert!(
        link_target.is_relative(),
        "symlink target should be relative, got {:?}",
        link_target
    );
}

#[tokio::test]
async fn test_install_symlink_resolves_correctly() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "resolve-skill");

    install_skill_to_agent_impl(&pool, "resolve-skill", "claude-code")
        .await
        .unwrap();

    let symlink_path = agent_dir.join("resolve-skill");
    // Following the symlink should give access to SKILL.md in the central dir.
    let skill_md = symlink_path.join("SKILL.md");
    assert!(
        skill_md.exists(),
        "SKILL.md should be accessible via symlink"
    );
}

#[tokio::test]
async fn test_install_creates_agent_dir_if_missing() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    // Do NOT pre-create agent_dir — install should create it.
    let agent_dir = tmp.path().join("new-agent-dir");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "dir-skill");

    let result = install_skill_to_agent_impl(&pool, "dir-skill", "claude-code").await;
    assert!(result.is_ok(), "install should create missing agent dir");
    assert!(agent_dir.exists(), "agent dir should have been created");
}

#[tokio::test]
async fn test_install_updates_db_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "db-skill");

    install_skill_to_agent_impl(&pool, "db-skill", "claude-code")
        .await
        .unwrap();

    let installations = db::get_skill_installations(&pool, "db-skill")
        .await
        .unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "claude-code");
    assert_eq!(installations[0].link_type, "symlink");
}

#[tokio::test]
async fn test_install_same_root_agent_records_native_without_symlink() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    point_codex_to_dir(&pool, &central_dir).await;
    let skill_dir = create_central_skill(&central_dir, "shared-root-skill");

    let result = install_skill_to_agent_impl(&pool, "shared-root-skill", "codex").await;
    assert!(
        result.is_ok(),
        "same-root install should succeed: {:?}",
        result
    );

    let meta = fs::symlink_metadata(&skill_dir).unwrap();
    assert!(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "same-root install must use the existing native directory"
    );

    let installations = db::get_skill_installations(&pool, "shared-root-skill")
        .await
        .unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "codex");
    assert_eq!(installations[0].link_type, "native");
    assert_eq!(
        installations[0].installed_path,
        skill_dir.to_string_lossy().into_owned()
    );
    assert!(installations[0].symlink_target.is_none());
}

#[tokio::test]
async fn test_install_fails_when_canonical_missing() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    // Do NOT create the skill in central_dir.

    let result = install_skill_to_agent_impl(&pool, "nonexistent-skill", "claude-code").await;
    assert!(
        result.is_err(),
        "install should fail if canonical skill missing"
    );
}

#[tokio::test]
async fn test_install_fails_for_unknown_agent() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "some-skill");

    let result = install_skill_to_agent_impl(&pool, "some-skill", "nonexistent-agent").await;
    assert!(result.is_err(), "install should fail for unknown agent");
}

#[tokio::test]
async fn test_install_to_central_agent_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &tmp.path().join("claude")).await;
    create_central_skill(&central_dir, "self-skill");

    let result = install_skill_to_agent_impl(&pool, "self-skill", "central").await;
    assert!(
        result.is_err(),
        "installing to 'central' should be rejected"
    );
}

#[tokio::test]
async fn test_install_replaces_existing_symlink() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "re-link-skill");

    // Install once.
    install_skill_to_agent_impl(&pool, "re-link-skill", "claude-code")
        .await
        .unwrap();

    // Install again — should replace the existing symlink without error.
    let result = install_skill_to_agent_impl(&pool, "re-link-skill", "claude-code").await;
    assert!(result.is_ok(), "re-install should succeed: {:?}", result);
}

#[tokio::test]
async fn test_install_refuses_to_overwrite_real_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "real-dir-skill");

    // Create a real (non-symlink) directory at the install location.
    fs::create_dir_all(agent_dir.join("real-dir-skill")).unwrap();

    let result = install_skill_to_agent_impl(&pool, "real-dir-skill", "claude-code").await;
    assert!(
        result.is_err(),
        "install should refuse to overwrite a real directory"
    );
}

// ── uninstall_skill_from_agent_impl ───────────────────────────────────────

#[tokio::test]
async fn test_uninstall_removes_symlink() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "uninstall-skill");

    install_skill_to_agent_impl(&pool, "uninstall-skill", "claude-code")
        .await
        .unwrap();

    let symlink_path = agent_dir.join("uninstall-skill");
    assert!(symlink_path.exists() || fs::symlink_metadata(&symlink_path).is_ok());

    uninstall_skill_from_agent_impl(&pool, "uninstall-skill", "claude-code")
        .await
        .unwrap();

    assert!(
        fs::symlink_metadata(&symlink_path).is_err(),
        "symlink should have been removed"
    );
}

#[tokio::test]
async fn test_uninstall_removes_db_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "db-uninstall-skill");

    install_skill_to_agent_impl(&pool, "db-uninstall-skill", "claude-code")
        .await
        .unwrap();

    uninstall_skill_from_agent_impl(&pool, "db-uninstall-skill", "claude-code")
        .await
        .unwrap();

    let installations = db::get_skill_installations(&pool, "db-uninstall-skill")
        .await
        .unwrap();
    assert!(installations.is_empty(), "DB record should be removed");
}

#[tokio::test]
async fn test_uninstall_refuses_real_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&agent_dir).unwrap();
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;

    // Place a real directory where the symlink would be.
    fs::create_dir_all(agent_dir.join("protected-skill")).unwrap();

    let result = uninstall_skill_from_agent_impl(&pool, "protected-skill", "claude-code").await;
    assert!(
        result.is_err(),
        "uninstall should refuse to delete a real directory"
    );

    // Ensure the directory still exists.
    assert!(
        agent_dir.join("protected-skill").is_dir(),
        "real directory should NOT have been deleted"
    );
}

#[tokio::test]
async fn test_uninstall_nonexistent_path_still_cleans_db() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;

    // Manually insert an installation record without creating the symlink.
    let installation = SkillInstallation {
        skill_id: "ghost-skill".to_string(),
        agent_id: "claude-code".to_string(),
        installed_path: agent_dir.join("ghost-skill").to_string_lossy().into_owned(),
        link_type: "symlink".to_string(),
        symlink_target: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(&pool, &installation)
        .await
        .unwrap();

    let result = uninstall_skill_from_agent_impl(&pool, "ghost-skill", "claude-code").await;
    assert!(result.is_ok(), "uninstall of missing path should succeed");

    let installations = db::get_skill_installations(&pool, "ghost-skill")
        .await
        .unwrap();
    assert!(installations.is_empty(), "DB record should be cleaned up");
}

#[tokio::test]
async fn test_uninstall_same_root_agent_is_rejected_without_deleting_central_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    point_codex_to_dir(&pool, &central_dir).await;
    let skill_dir = create_central_skill(&central_dir, "shared-root-uninstall-skill");

    install_skill_to_agent_impl(&pool, "shared-root-uninstall-skill", "codex")
        .await
        .unwrap();

    let result =
        uninstall_skill_from_agent_impl(&pool, "shared-root-uninstall-skill", "codex").await;
    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.contains("cannot be uninstalled independently")),
        "same-root uninstall should be rejected: {:?}",
        result
    );
    assert!(
        skill_dir.join("SKILL.md").exists(),
        "Central skill directory must not be deleted"
    );

    let installations = db::get_skill_installations(&pool, "shared-root-uninstall-skill")
        .await
        .unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "codex");
}

#[tokio::test]
async fn test_uninstall_claude_user_row_removes_observed_dir_and_observation() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    let skill_dir = create_user_skill(&agent_dir, "observed-user-skill");
    let observation =
        claude_observation(&agent_dir, "observed-user-skill", &skill_dir, "user", false);
    let row_id = observation.row_id.clone();
    db::upsert_agent_skill_observation(&pool, &observation)
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "observed-user-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: skill_dir.to_string_lossy().into_owned(),
            link_type: "native".to_string(),
            symlink_target: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    uninstall_skill_from_agent_with_row_impl(
        &pool,
        "observed-user-skill",
        "claude-code",
        Some(&row_id),
    )
    .await
    .unwrap();

    assert!(
        fs::symlink_metadata(&skill_dir).is_err(),
        "Claude user source directory should be deleted"
    );
    assert!(
        db::get_agent_skill_observation_by_row_id(&pool, &row_id)
            .await
            .unwrap()
            .is_none(),
        "Claude observation row should be deleted"
    );
    assert!(
        db::get_skill_installations(&pool, "observed-user-skill")
            .await
            .unwrap()
            .is_empty(),
        "installation record should be cleaned up"
    );
}

#[tokio::test]
async fn test_uninstall_claude_plugin_row_is_rejected_without_deleting_path() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    let plugin_dir = tmp
        .path()
        .join("plugin")
        .join("skills")
        .join("plugin-skill");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(
        plugin_dir.join("SKILL.md"),
        "---\nname: plugin-skill\n---\n\n# plugin\n",
    )
    .unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    let observation = claude_observation(&agent_dir, "plugin-skill", &plugin_dir, "plugin", true);
    let row_id = observation.row_id.clone();
    db::upsert_agent_skill_observation(&pool, &observation)
        .await
        .unwrap();

    let result = uninstall_skill_from_agent_with_row_impl(
        &pool,
        "plugin-skill",
        "claude-code",
        Some(&row_id),
    )
    .await;

    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.contains("read-only")),
        "plugin source rows should be rejected: {:?}",
        result
    );
    assert!(
        plugin_dir.join("SKILL.md").exists(),
        "plugin path must not be deleted"
    );
}

#[tokio::test]
async fn test_uninstall_claude_row_rejects_skill_id_mismatch() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    let skill_dir = create_user_skill(&agent_dir, "row-skill");
    let observation = claude_observation(&agent_dir, "row-skill", &skill_dir, "user", false);
    let row_id = observation.row_id.clone();
    db::upsert_agent_skill_observation(&pool, &observation)
        .await
        .unwrap();

    let result = uninstall_skill_from_agent_with_row_impl(
        &pool,
        "other-skill",
        "claude-code",
        Some(&row_id),
    )
    .await;

    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.contains("belongs to skill")),
        "mismatched row/skill should be rejected: {:?}",
        result
    );
    assert!(
        skill_dir.join("SKILL.md").exists(),
        "mismatched row should not delete the observed path"
    );
}

#[tokio::test]
async fn test_batch_uninstall_skills_from_agent_reports_partial_failure() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "batch-remove-ok");
    install_skill_to_agent_impl(&pool, "batch-remove-ok", "claude-code")
        .await
        .unwrap();
    fs::create_dir_all(agent_dir.join("protected-batch-dir")).unwrap();

    let result = super::batch::batch_uninstall_skills_from_agent_impl(
        &pool,
        "claude-code",
        vec![
            BatchUninstallSkillRequest {
                skill_id: "batch-remove-ok".to_string(),
                row_id: None,
            },
            BatchUninstallSkillRequest {
                skill_id: "protected-batch-dir".to_string(),
                row_id: None,
            },
        ],
    )
    .await;

    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "batch-remove-ok");
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_id, "protected-batch-dir");
    assert!(
        fs::symlink_metadata(agent_dir.join("batch-remove-ok")).is_err(),
        "successful batch item should remove the platform install"
    );
}

#[tokio::test]
async fn test_batch_uninstall_claude_user_rows_keep_row_identity() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    let first_dir = create_user_skill(&agent_dir, "same-name-a");
    let second_dir = create_user_skill(&agent_dir, "same-name-b");
    let first_observation = claude_observation(&agent_dir, "same-skill", &first_dir, "user", false);
    let second_observation =
        claude_observation(&agent_dir, "same-skill", &second_dir, "user", false);
    let first_row_id = first_observation.row_id.clone();
    let second_row_id = second_observation.row_id.clone();
    db::upsert_agent_skill_observation(&pool, &first_observation)
        .await
        .unwrap();
    db::upsert_agent_skill_observation(&pool, &second_observation)
        .await
        .unwrap();

    let result = super::batch::batch_uninstall_skills_from_agent_impl(
        &pool,
        "claude-code",
        vec![BatchUninstallSkillRequest {
            skill_id: "same-skill".to_string(),
            row_id: Some(first_row_id.clone()),
        }],
    )
    .await;

    assert_eq!(result.succeeded.len(), 1);
    assert!(result.failed.is_empty());
    assert!(fs::symlink_metadata(&first_dir).is_err());
    assert!(second_dir.join("SKILL.md").exists());
    assert!(
        db::get_agent_skill_observation_by_row_id(&pool, &first_row_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        db::get_agent_skill_observation_by_row_id(&pool, &second_row_id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_batch_uninstall_rejects_read_only_and_shared_root_rows() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    let plugin_dir = tmp
        .path()
        .join("plugin")
        .join("skills")
        .join("plugin-batch-skill");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&claude_dir).unwrap();
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(
        plugin_dir.join("SKILL.md"),
        "---\nname: plugin-batch-skill\n---\n\n# plugin\n",
    )
    .unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    let observation = claude_observation(
        &claude_dir,
        "plugin-batch-skill",
        &plugin_dir,
        "plugin",
        true,
    );
    let row_id = observation.row_id.clone();
    db::upsert_agent_skill_observation(&pool, &observation)
        .await
        .unwrap();
    point_codex_to_dir(&pool, &central_dir).await;
    create_central_skill(&central_dir, "shared-root-batch-skill");
    install_skill_to_agent_impl(&pool, "shared-root-batch-skill", "codex")
        .await
        .unwrap();

    let plugin_result = super::batch::batch_uninstall_skills_from_agent_impl(
        &pool,
        "claude-code",
        vec![BatchUninstallSkillRequest {
            skill_id: "plugin-batch-skill".to_string(),
            row_id: Some(row_id),
        }],
    )
    .await;
    let shared_result = super::batch::batch_uninstall_skills_from_agent_impl(
        &pool,
        "codex",
        vec![BatchUninstallSkillRequest {
            skill_id: "shared-root-batch-skill".to_string(),
            row_id: None,
        }],
    )
    .await;

    assert!(plugin_result.succeeded.is_empty());
    assert!(plugin_result.failed[0].error.contains("read-only"));
    assert!(plugin_dir.join("SKILL.md").exists());
    assert!(shared_result.succeeded.is_empty());
    assert!(shared_result.failed[0]
        .error
        .contains("cannot be uninstalled independently"));
    assert!(central_dir
        .join("shared-root-batch-skill")
        .join("SKILL.md")
        .exists());
}

// ── batch install ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_batch_install_multiple_agents() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    let cursor_dir = tmp.path().join("cursor");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;

    // Override cursor's dir too.
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(cursor_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    create_central_skill(&central_dir, "batch-skill");

    let result = batch_install_impl(
        &pool,
        "batch-skill",
        &["claude-code".to_string(), "cursor".to_string()],
    )
    .await;

    assert_eq!(result.succeeded.len(), 2);
    assert!(result.failed.is_empty());

    assert!(fs::symlink_metadata(claude_dir.join("batch-skill")).is_ok());
    assert!(fs::symlink_metadata(cursor_dir.join("batch-skill")).is_ok());
}

#[tokio::test]
async fn test_batch_install_partial_failure() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    create_central_skill(&central_dir, "partial-skill");

    let result = batch_install_impl(
        &pool,
        "partial-skill",
        &[
            "claude-code".to_string(),
            "nonexistent-agent".to_string(), // will fail
        ],
    )
    .await;

    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].agent_id, "nonexistent-agent");
}

#[tokio::test]
async fn test_batch_install_reports_existing_copy_as_skipped() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    let central_skill_dir = create_central_skill(&central_dir, "batch-existing-copy");
    let target_dir = claude_dir.join("batch-existing-copy");
    super::fs_util::copy_dir_all(&central_skill_dir, &target_dir).unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "batch-existing-copy".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: target_dir.to_string_lossy().into_owned(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let result =
        batch_install_impl(&pool, "batch-existing-copy", &["claude-code".to_string()]).await;

    assert!(result.succeeded.is_empty());
    assert_eq!(result.skipped.len(), 1);
    assert!(result.failed.is_empty());
    assert_eq!(result.skipped[0].agent_id, "claude-code");
    assert_eq!(result.skipped[0].reason, "already_installed");
    assert_eq!(PathBuf::from(&result.skipped[0].target_path), target_dir);
}

#[tokio::test]
async fn test_central_batch_install_multiple_skills_to_multiple_agents() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    let cursor_dir = tmp.path().join("cursor");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(cursor_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    create_central_skill(&central_dir, "batch-one");
    create_central_skill(&central_dir, "batch-two");

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["batch-one".to_string(), "batch-two".to_string()],
        vec!["claude-code".to_string(), "cursor".to_string()],
        "copy",
        None,
    )
    .await;

    assert_eq!(result.succeeded.len(), 4);
    assert!(result.failed.is_empty());
    assert!(claude_dir.join("batch-one").join("SKILL.md").exists());
    assert!(claude_dir.join("batch-two").join("SKILL.md").exists());
    assert!(cursor_dir.join("batch-one").join("SKILL.md").exists());
    assert!(cursor_dir.join("batch-two").join("SKILL.md").exists());
}

#[tokio::test]
async fn test_central_batch_install_skips_existing_db_copy_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    let central_skill_dir = create_central_skill(&central_dir, "existing-copy-skill");
    let target_dir = claude_dir.join("existing-copy-skill");
    super::fs_util::copy_dir_all(&central_skill_dir, &target_dir).unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "existing-copy-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: target_dir.to_string_lossy().into_owned(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["existing-copy-skill".to_string()],
        vec!["claude-code".to_string()],
        "symlink",
        None,
    )
    .await;

    assert!(result.succeeded.is_empty());
    assert_eq!(result.skipped.len(), 1);
    assert!(result.failed.is_empty());
    assert_eq!(result.skipped[0].reason, "already_installed");
    assert_eq!(PathBuf::from(&result.skipped[0].target_path), target_dir);
}

#[tokio::test]
async fn test_central_batch_install_skips_shared_target_record_and_adds_agent_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    let universal_dir = tmp.path().join("universal");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    point_codex_to_dir(&pool, &universal_dir).await;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(universal_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let central_skill_dir = create_central_skill(&central_dir, "shared-universal-skill");
    let target_dir = universal_dir.join("shared-universal-skill");
    super::fs_util::copy_dir_all(&central_skill_dir, &target_dir).unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-universal-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: target_dir.to_string_lossy().into_owned(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["shared-universal-skill".to_string()],
        vec!["codex".to_string()],
        "symlink",
        None,
    )
    .await;

    assert!(result.succeeded.is_empty());
    assert_eq!(result.skipped.len(), 1);
    assert!(result.failed.is_empty());
    assert_eq!(result.skipped[0].reason, "shared_target_record");

    let installations = db::get_skill_installations(&pool, "shared-universal-skill")
        .await
        .unwrap();
    assert!(installations
        .iter()
        .any(|record| record.agent_id == "cursor"));
    let codex_record = installations
        .iter()
        .find(|record| record.agent_id == "codex")
        .expect("codex record should be added");
    assert_eq!(PathBuf::from(&codex_record.installed_path), target_dir);
    assert_eq!(codex_record.link_type, "copy");
}

#[tokio::test]
async fn test_central_batch_install_refuses_different_existing_real_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(claude_dir.join("different-real-dir")).unwrap();

    let pool = setup_db(&central_dir, &claude_dir).await;
    create_central_skill(&central_dir, "different-real-dir");
    fs::write(
        claude_dir.join("different-real-dir").join("SKILL.md"),
        "---\nname: different\n---\n\n# different\n",
    )
    .unwrap();

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["different-real-dir".to_string()],
        vec!["claude-code".to_string()],
        "copy",
        None,
    )
    .await;

    assert!(result.succeeded.is_empty());
    assert!(result.skipped.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(result.failed[0].error.contains("Refusing to overwrite"));
}

#[tokio::test]
async fn test_project_install_creates_project_relative_skill_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "project-skill");

    let result = install_central_skill_to_project_outcome_impl(
        &pool,
        "project-skill",
        "claude-code",
        &project_dir,
        "copy",
    )
    .await
    .unwrap();

    let result = match result {
        InstallOutcome::Installed(result) => result,
        InstallOutcome::Skipped(skipped) => panic!("expected install, got skip: {:?}", skipped),
    };
    let target = project_dir
        .join(".claude")
        .join("skills")
        .join("project-skill");
    assert_eq!(PathBuf::from(result.symlink_path), target);
    assert!(target.join("SKILL.md").exists());
}

#[tokio::test]
async fn test_project_install_uses_agents_dir_for_universal_representative() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let codex_agent_dir = crate::paths::resolve_home_dir()
        .join(".agents")
        .join("skills");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let pool = setup_db_with_codex(&central_dir, &claude_agent_dir, &codex_agent_dir).await;
    create_central_skill(&central_dir, "universal-project-skill");

    let result = install_central_skill_to_project_outcome_impl(
        &pool,
        "universal-project-skill",
        "codex",
        &project_dir,
        "copy",
    )
    .await
    .unwrap();
    let result = match result {
        InstallOutcome::Installed(result) => result,
        InstallOutcome::Skipped(skipped) => panic!("expected install, got skip: {:?}", skipped),
    };
    let target = project_dir
        .join(".agents")
        .join("skills")
        .join("universal-project-skill");

    assert_eq!(PathBuf::from(result.symlink_path), target);
    assert!(target.join("SKILL.md").exists());
    assert!(
        !project_dir
            .join(".codex")
            .join("skills")
            .join("universal-project-skill")
            .exists(),
        "Universal project installs must not write the legacy .codex/skills path"
    );
}

#[tokio::test]
async fn test_project_install_uses_agents_dir_for_antigravity() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let codex_agent_dir = crate::paths::resolve_home_dir()
        .join(".agents")
        .join("skills");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let pool = setup_db_with_codex(&central_dir, &claude_agent_dir, &codex_agent_dir).await;
    create_central_skill(&central_dir, "antigravity-project-skill");

    let result = install_central_skill_to_project_outcome_impl(
        &pool,
        "antigravity-project-skill",
        "antigravity",
        &project_dir,
        "copy",
    )
    .await
    .unwrap();
    let result = match result {
        InstallOutcome::Installed(result) => result,
        InstallOutcome::Skipped(skipped) => panic!("expected install, got skip: {:?}", skipped),
    };
    let target = project_dir
        .join(".agents")
        .join("skills")
        .join("antigravity-project-skill");

    assert_eq!(PathBuf::from(result.symlink_path), target);
    assert!(target.join("SKILL.md").exists());
    assert!(
        !project_dir
            .join(".gemini")
            .join("antigravity")
            .join("skills")
            .join("antigravity-project-skill")
            .exists(),
        "Antigravity project installs must use the shared .agents/skills directory"
    );
}

#[tokio::test]
async fn test_project_install_uses_agents_dir_for_antigravity_cli() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let claude_agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let codex_agent_dir = crate::paths::resolve_home_dir()
        .join(".agents")
        .join("skills");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let pool = setup_db_with_codex(&central_dir, &claude_agent_dir, &codex_agent_dir).await;
    create_central_skill(&central_dir, "antigravity-cli-project-skill");

    let result = install_central_skill_to_project_outcome_impl(
        &pool,
        "antigravity-cli-project-skill",
        "antigravity-cli",
        &project_dir,
        "copy",
    )
    .await
    .unwrap();
    let result = match result {
        InstallOutcome::Installed(result) => result,
        InstallOutcome::Skipped(skipped) => panic!("expected install, got skip: {:?}", skipped),
    };
    let target = project_dir
        .join(".agents")
        .join("skills")
        .join("antigravity-cli-project-skill");

    assert_eq!(PathBuf::from(result.symlink_path), target);
    assert!(target.join("SKILL.md").exists());
    assert!(
        !project_dir
            .join(".gemini")
            .join("antigravity-cli")
            .join("skills")
            .join("antigravity-cli-project-skill")
            .exists(),
        "Antigravity CLI project installs must use the shared .agents/skills directory"
    );
}

#[tokio::test]
async fn test_project_install_uses_grok_project_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "grok-project-skill");

    let result = install_central_skill_to_project_outcome_impl(
        &pool,
        "grok-project-skill",
        "grok",
        &project_dir,
        "copy",
    )
    .await
    .unwrap();
    let result = match result {
        InstallOutcome::Installed(result) => result,
        InstallOutcome::Skipped(skipped) => panic!("expected install, got skip: {:?}", skipped),
    };
    let target = project_dir
        .join(".grok")
        .join("skills")
        .join("grok-project-skill");

    assert_eq!(PathBuf::from(result.symlink_path), target);
    assert!(target.join("SKILL.md").exists());
    assert!(
        !project_dir
            .join(".agents")
            .join("skills")
            .join("grok-project-skill")
            .exists(),
        "Grok project installs must stay in .grok/skills, not Universal Agents"
    );
}

#[tokio::test]
async fn test_project_install_refuses_existing_real_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    let existing_dir = project_dir
        .join(".claude")
        .join("skills")
        .join("existing-project-skill");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&existing_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "existing-project-skill");

    let result = install_central_skill_to_project_outcome_impl(
        &pool,
        "existing-project-skill",
        "claude-code",
        &project_dir,
        "copy",
    )
    .await;

    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.contains("Refusing to overwrite")),
        "project install should refuse existing real dir: {:?}",
        result
    );
    assert!(existing_dir.is_dir());
}

#[tokio::test]
async fn test_project_install_skips_existing_central_symlink() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    let project_skills_dir = project_dir.join(".claude").join("skills");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_skills_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    let central_skill_dir = create_central_skill(&central_dir, "project-symlink-skill");
    create_symlink_for_test(
        &central_skill_dir,
        &project_skills_dir.join("project-symlink-skill"),
    );

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["project-symlink-skill".to_string()],
        vec!["claude-code".to_string()],
        "symlink",
        Some(&project_dir),
    )
    .await;

    assert!(result.succeeded.is_empty());
    assert_eq!(result.skipped.len(), 1);
    assert!(result.failed.is_empty());
    assert_eq!(result.skipped[0].reason, "central_symlink");
}

#[tokio::test]
async fn test_project_install_skips_existing_matching_copy() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    let target_dir = project_dir
        .join(".claude")
        .join("skills")
        .join("project-copy-skill");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(target_dir.parent().unwrap()).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    let central_skill_dir = create_central_skill(&central_dir, "project-copy-skill");
    super::fs_util::copy_dir_all(&central_skill_dir, &target_dir).unwrap();

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["project-copy-skill".to_string()],
        vec!["claude-code".to_string()],
        "copy",
        Some(&project_dir),
    )
    .await;

    assert!(result.succeeded.is_empty());
    assert_eq!(result.skipped.len(), 1);
    assert!(result.failed.is_empty());
    assert_eq!(result.skipped[0].reason, "matching_copy");
}

#[tokio::test]
async fn test_project_install_refuses_existing_different_copy() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    let target_dir = project_dir
        .join(".claude")
        .join("skills")
        .join("project-different-skill");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "project-different-skill");
    fs::write(
        target_dir.join("SKILL.md"),
        "---\nname: project-different\n---\n\n# different\n",
    )
    .unwrap();

    let result = batch_install_central_skills_impl(
        &pool,
        vec!["project-different-skill".to_string()],
        vec!["claude-code".to_string()],
        "copy",
        Some(&project_dir),
    )
    .await;

    assert!(result.succeeded.is_empty());
    assert!(result.skipped.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(result.failed[0].error.contains("Refusing to overwrite"));
}

#[tokio::test]
async fn test_project_install_does_not_overwrite_global_installation_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = crate::paths::resolve_home_dir()
        .join(".claude")
        .join("skills");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "db-project-skill");
    let global_path = agent_dir.join("db-project-skill");
    let installation = SkillInstallation {
        skill_id: "db-project-skill".to_string(),
        agent_id: "claude-code".to_string(),
        installed_path: global_path.to_string_lossy().into_owned(),
        link_type: "copy".to_string(),
        symlink_target: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(&pool, &installation)
        .await
        .unwrap();

    let outcome = install_central_skill_to_project_outcome_impl(
        &pool,
        "db-project-skill",
        "claude-code",
        &project_dir,
        "copy",
    )
    .await
    .unwrap();
    assert!(matches!(outcome, InstallOutcome::Installed(_)));

    let installations = db::get_skill_installations(&pool, "db-project-skill")
        .await
        .unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "claude-code");
    assert_eq!(
        installations[0].installed_path,
        global_path.to_string_lossy()
    );
}

/// Helper that mirrors `batch_install_to_agents` but works with a raw pool
/// (no Tauri State).
async fn batch_install_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_ids: &[String],
) -> BatchInstallResult {
    let mut succeeded = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for agent_id in agent_ids {
        match install_central_skill_to_agent_outcome_by_method(pool, skill_id, agent_id, "symlink")
            .await
        {
            Ok(InstallOutcome::Installed(_)) => succeeded.push(agent_id.clone()),
            Ok(InstallOutcome::Skipped(item)) => skipped.push(item),
            Err(e) => failed.push(FailedInstall {
                agent_id: agent_id.clone(),
                error: e,
            }),
        }
    }

    BatchInstallResult {
        succeeded,
        skipped,
        failed,
    }
}

// ── install_skill_to_agent_copy_impl ──────────────────────────────────────

#[tokio::test]
async fn test_copy_install_creates_real_directory() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "copy-skill");

    let result = install_skill_to_agent_copy_impl(&pool, "copy-skill", "claude-code").await;
    assert!(result.is_ok(), "copy install should succeed: {:?}", result);

    let target = agent_dir.join("copy-skill");
    let meta = fs::symlink_metadata(&target).unwrap();
    // Must be a real directory — NOT a symlink.
    assert!(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "installed path should be a real directory, not a symlink"
    );
}

#[tokio::test]
async fn test_copy_install_files_are_copied() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;

    // Create skill with multiple files to verify all are copied.
    let skill_dir = central_dir.join("multi-file-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: multi-file-skill\ndescription: Test\n---\n",
    )
    .unwrap();
    fs::write(skill_dir.join("extra.txt"), "extra content").unwrap();

    install_skill_to_agent_copy_impl(&pool, "multi-file-skill", "claude-code")
        .await
        .unwrap();

    let installed_skill_dir = agent_dir.join("multi-file-skill");

    // Verify SKILL.md was copied.
    let skill_md = installed_skill_dir.join("SKILL.md");
    assert!(skill_md.exists(), "SKILL.md should be copied to agent dir");

    // Verify extra file was copied.
    let extra = installed_skill_dir.join("extra.txt");
    assert!(extra.exists(), "extra.txt should be copied to agent dir");
    assert_eq!(
        fs::read_to_string(&extra).unwrap(),
        "extra content",
        "copied file contents should match"
    );

    // Confirm that the installed path is NOT a symlink.
    let meta = fs::symlink_metadata(&installed_skill_dir).unwrap();
    assert!(
        !meta.file_type().is_symlink(),
        "installed directory must NOT be a symlink"
    );
}

#[tokio::test]
async fn test_copy_install_updates_db_with_copy_type() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "db-copy-skill");

    install_skill_to_agent_copy_impl(&pool, "db-copy-skill", "claude-code")
        .await
        .unwrap();

    let installations = db::get_skill_installations(&pool, "db-copy-skill")
        .await
        .unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "claude-code");
    assert_eq!(
        installations[0].link_type, "copy",
        "DB should record link_type as 'copy'"
    );
}

#[tokio::test]
async fn test_copy_install_same_root_agent_records_native_without_copying() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    point_codex_to_dir(&pool, &central_dir).await;
    let skill_dir = create_central_skill(&central_dir, "shared-root-copy-skill");

    let result = install_skill_to_agent_copy_impl(&pool, "shared-root-copy-skill", "codex").await;
    assert!(
        result.is_ok(),
        "same-root copy install should succeed: {:?}",
        result
    );

    let meta = fs::symlink_metadata(&skill_dir).unwrap();
    assert!(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "same-root copy install must keep the native Central directory"
    );

    let installations = db::get_skill_installations(&pool, "shared-root-copy-skill")
        .await
        .unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "codex");
    assert_eq!(installations[0].link_type, "native");
    assert!(installations[0].symlink_target.is_none());
}

#[tokio::test]
async fn test_copy_install_to_central_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &tmp.path().join("claude")).await;
    create_central_skill(&central_dir, "self-copy-skill");

    let result = install_skill_to_agent_copy_impl(&pool, "self-copy-skill", "central").await;
    assert!(
        result.is_err(),
        "copy install to 'central' should be rejected"
    );
}

#[tokio::test]
async fn test_copy_install_fails_when_canonical_missing() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    // Deliberately do NOT create the skill in central_dir.

    let result = install_skill_to_agent_copy_impl(&pool, "missing-skill", "claude-code").await;
    assert!(
        result.is_err(),
        "copy install should fail when canonical skill is missing"
    );
}

#[tokio::test]
async fn test_copy_install_refuses_to_overwrite_real_dir() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();
    fs::create_dir_all(&agent_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "existing-dir-skill");

    // Create a real directory at the target location.
    fs::create_dir_all(agent_dir.join("existing-dir-skill")).unwrap();

    let result = install_skill_to_agent_copy_impl(&pool, "existing-dir-skill", "claude-code").await;
    assert!(
        result.is_err(),
        "copy install should refuse to overwrite an existing real directory"
    );
}

// ── uninstall (copy) ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_uninstall_removes_copied_directory() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "uninstall-copy-skill");

    // First, install via copy.
    install_skill_to_agent_copy_impl(&pool, "uninstall-copy-skill", "claude-code")
        .await
        .unwrap();

    let target = agent_dir.join("uninstall-copy-skill");
    assert!(
        target.is_dir(),
        "copied directory should exist before uninstall"
    );

    // Now uninstall.
    uninstall_skill_from_agent_impl(&pool, "uninstall-copy-skill", "claude-code")
        .await
        .unwrap();

    assert!(
        fs::symlink_metadata(&target).is_err(),
        "copied directory should have been removed after uninstall"
    );
}

#[tokio::test]
async fn test_uninstall_copy_removes_db_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "db-copy-uninstall-skill");

    install_skill_to_agent_copy_impl(&pool, "db-copy-uninstall-skill", "claude-code")
        .await
        .unwrap();

    uninstall_skill_from_agent_impl(&pool, "db-copy-uninstall-skill", "claude-code")
        .await
        .unwrap();

    let installations = db::get_skill_installations(&pool, "db-copy-uninstall-skill")
        .await
        .unwrap();
    assert!(
        installations.is_empty(),
        "DB record should be removed after uninstall"
    );
}

#[tokio::test]
async fn test_uninstall_refuses_real_dir_without_copy_record() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&agent_dir).unwrap();
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;

    // Place a real directory with NO DB record as 'copy' type.
    fs::create_dir_all(agent_dir.join("protected-skill")).unwrap();

    let result = uninstall_skill_from_agent_impl(&pool, "protected-skill", "claude-code").await;
    assert!(
        result.is_err(),
        "uninstall should refuse to delete a real directory without a copy record"
    );

    // Ensure the directory still exists.
    assert!(
        agent_dir.join("protected-skill").is_dir(),
        "real directory should NOT have been deleted"
    );
}

#[tokio::test]
async fn test_batch_install_uses_copy_method() {
    let tmp = TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    let agent_dir = tmp.path().join("claude");
    fs::create_dir_all(&central_dir).unwrap();

    let pool = setup_db(&central_dir, &agent_dir).await;
    create_central_skill(&central_dir, "batch-copy-skill");

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for agent_id in &["claude-code".to_string()] {
        match install_skill_to_agent_copy_impl(&pool, "batch-copy-skill", agent_id).await {
            Ok(_) => succeeded.push(agent_id.clone()),
            Err(e) => failed.push(FailedInstall {
                agent_id: agent_id.clone(),
                error: e,
            }),
        }
    }

    assert_eq!(succeeded.len(), 1);
    assert!(failed.is_empty());

    // The installed directory must NOT be a symlink.
    let target = agent_dir.join("batch-copy-skill");
    let meta = fs::symlink_metadata(&target).unwrap();
    assert!(
        !meta.file_type().is_symlink(),
        "batch copy install should create a real directory"
    );
}
