use super::delete::ensure_remote_child_path;
use super::files::{open_in_file_manager_checked_impl, read_file_by_path_impl};
use super::query::{get_skill_detail_with_row_impl, get_skills_by_agent_impl};
use super::types::{BatchDeleteCentralSkillRequest, SkillWithLinks};
use super::*;
use crate::db::{self, AgentSkillObservation, Skill, SkillInstallation};
use chrono::Utc;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

async fn set_test_central_root(pool: &SqlitePool, root: &Path) {
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(root.to_string_lossy().into_owned())
        .execute(pool)
        .await
        .unwrap();
}

fn write_test_skill_dir(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: test skill\n---\n",
    )
    .unwrap();
}

fn make_directory_tree_fixture(root: &Path) {
    fs::create_dir_all(root.join("examples").join("nested")).unwrap();
    fs::write(root.join("SKILL.md"), "# Demo").unwrap();
    fs::write(root.join("examples").join("demo.md"), "demo").unwrap();
    fs::write(root.join("examples").join("nested").join("deep.txt"), "deep").unwrap();
}

fn make_central_skill_at(id: &str, name: &str, dir: &Path) -> Skill {
    Skill {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(format!("Desc for {}", name)),
        file_path: dir.join("SKILL.md").to_string_lossy().into_owned(),
        canonical_path: Some(dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
    }
}

fn make_installation_at(
    skill_id: &str,
    agent_id: &str,
    dir: &Path,
    link_type: &str,
    symlink_target: Option<&Path>,
) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: dir.to_string_lossy().into_owned(),
        link_type: link_type.to_string(),
        symlink_target: symlink_target.map(|path| path.to_string_lossy().into_owned()),
        created_at: Utc::now().to_rfc3339(),
    }
}

fn make_skill(id: &str, name: &str, is_central: bool) -> Skill {
    Skill {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(format!("Desc for {}", name)),
        file_path: format!("/tmp/{}/SKILL.md", id),
        canonical_path: if is_central {
            Some(format!("/tmp/central/{}", id))
        } else {
            None
        },
        is_central,
        source: if is_central {
            Some("native".to_string())
        } else {
            Some("copy".to_string())
        },
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
    }
}

fn make_remote_central_skill(id: &str, dir: &str) -> Skill {
    Skill {
        id: id.to_string(),
        name: id.to_string(),
        description: Some(format!("Desc for {}", id)),
        file_path: format!("{}/SKILL.md", dir.trim_end_matches('/')),
        canonical_path: Some(dir.to_string()),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
    }
}

fn make_remote_installation(
    skill_id: &str,
    agent_id: &str,
    installed_path: &str,
    link_type: &str,
) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: installed_path.to_string(),
        link_type: link_type.to_string(),
        symlink_target: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

fn make_observation(
    row_id: &str,
    skill_id: &str,
    name: &str,
    dir_path: &str,
    source_kind: &str,
    read_only: bool,
) -> AgentSkillObservation {
    AgentSkillObservation {
        row_id: row_id.to_string(),
        agent_id: "claude-code".to_string(),
        skill_id: skill_id.to_string(),
        name: name.to_string(),
        description: Some(format!("{source_kind} copy")),
        file_path: format!("{dir_path}/SKILL.md"),
        dir_path: dir_path.to_string(),
        source_kind: source_kind.to_string(),
        source_root: if source_kind == "user" {
            "/tmp/.claude/skills".to_string()
        } else {
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0".to_string()
        },
        link_type: "copy".to_string(),
        symlink_target: None,
        is_read_only: read_only,
        scanned_at: Utc::now().to_rfc3339(),
    }
}

#[test]
fn test_remote_child_path_guard_normalizes_and_rejects_unsafe_paths() {
    assert_eq!(
        ensure_remote_child_path(
            "/home/alice/.skillsmanage/skills/",
            "/home/alice/.skillsmanage/skills/demo",
            "demo",
        )
        .unwrap(),
        "/home/alice/.skillsmanage/skills/demo"
    );

    assert!(ensure_remote_child_path(
        "/home/alice/.skillsmanage/skills",
        "/home/alice/.skillsmanage/skills",
        "root",
    )
    .is_err());
    assert!(ensure_remote_child_path(
        "/home/alice/.skillsmanage/skills",
        "/home/alice/other/demo",
        "outside",
    )
    .is_err());
    assert!(ensure_remote_child_path(
        "/home/alice/.skillsmanage/skills",
        "/home/alice/.skillsmanage/skills/../other",
        "traversal",
    )
    .is_err());
}

#[tokio::test]
async fn test_preview_remote_delete_uses_remote_paths_and_installations() {
    let pool = setup_test_db().await;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind("/home/alice/.skillsmanage/skills")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind("/home/alice/.agents/skills")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind("/home/alice/.claude/skills")
        .execute(&pool)
        .await
        .unwrap();

    db::upsert_skill(
        &pool,
        &make_remote_central_skill(
            "remote-delete",
            "/home/alice/.skillsmanage/skills/remote-delete",
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_remote_installation(
            "remote-delete",
            "cursor",
            "/home/alice/.agents/skills/remote-delete",
            "copy",
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_remote_installation(
            "remote-delete",
            "claude-code",
            "/home/alice/.claude/skills/remote-delete",
            "symlink",
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_remote_installation(
            "remote-delete",
            "central",
            "/home/alice/.skillsmanage/skills/remote-delete",
            "native",
        ),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_ssh_impl(&pool, &["remote-delete".to_string()])
        .await
        .unwrap();

    assert!(result.failed.is_empty());
    assert_eq!(
        result.previews[0].central_path,
        "/home/alice/.skillsmanage/skills/remote-delete"
    );
    assert_eq!(result.previews[0].copy_installations[0].agent_id, "cursor");
    assert_eq!(
        result.previews[0].auto_removed_agent_ids,
        vec!["claude-code"]
    );
}

#[tokio::test]
async fn test_preview_remote_delete_rejects_central_path_outside_remote_root() {
    let pool = setup_test_db().await;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind("/home/alice/.skillsmanage/skills")
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_skill(
        &pool,
        &make_remote_central_skill("outside-remote", "/tmp/outside-remote"),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_ssh_impl(&pool, &["outside-remote".to_string()])
        .await
        .unwrap();

    assert!(result.previews.is_empty());
    assert_eq!(result.failed[0].skill_id, "outside-remote");
    assert!(result.failed[0].error.contains("outside remote root"));
}

// ── get_skills_by_agent ───────────────────────────────────────────────────

#[tokio::test]
async fn test_get_skills_by_agent_returns_correct_skills() {
    let pool = setup_test_db().await;

    let skill_a = make_skill("skill-a", "Skill A", false);
    let skill_b = make_skill("skill-b", "Skill B", false);
    db::upsert_skill(&pool, &skill_a).await.unwrap();
    db::upsert_skill(&pool, &skill_b).await.unwrap();

    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "skill-a".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/claude/skill-a/SKILL.md".to_string(),
            link_type: "symlink".to_string(),
            symlink_target: Some("/tmp/central/skill-a".to_string()),
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let skills = db::get_skills_by_agent(&pool, "claude-code").await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "skill-a");
}

#[tokio::test]
async fn test_get_skills_by_agent_empty_for_unknown_agent() {
    let pool = setup_test_db().await;
    let skills = db::get_skills_by_agent(&pool, "nonexistent-agent")
        .await
        .unwrap();
    assert!(skills.is_empty());
}

// ── get_central_skills ────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_central_skills_includes_linked_agents() {
    let pool = setup_test_db().await;

    let central_skill = make_skill("central-a", "Central A", true);
    db::upsert_skill(&pool, &central_skill).await.unwrap();

    // Install to claude-code and cursor.
    for agent_id in &["claude-code", "cursor"] {
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "central-a".to_string(),
                agent_id: agent_id.to_string(),
                installed_path: format!("/tmp/{}/central-a/SKILL.md", agent_id),
                link_type: "symlink".to_string(),
                symlink_target: Some("/tmp/central/central-a".to_string()),
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();
    }

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(skills_with_links.len(), 1);

    let mut linked = skills_with_links[0].linked_agents.clone();
    linked.sort();
    let mut expected_linked: Vec<String> = vec!["claude-code".to_string(), "cursor".to_string()];
    expected_linked.sort();
    assert_eq!(linked, expected_linked);

    let mut shared = skills_with_links[0].shared_root_agents.clone();
    shared.sort();
    assert!(shared.is_empty());
}

#[tokio::test]
async fn test_get_central_skills_no_links() {
    let pool = setup_test_db().await;

    let central_skill = make_skill("central-solo", "Solo Central", true);
    db::upsert_skill(&pool, &central_skill).await.unwrap();

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(skills_with_links.len(), 1);
    let mut linked = skills_with_links[0].linked_agents.clone();
    linked.sort();
    assert!(linked.is_empty());

    let mut shared = skills_with_links[0].shared_root_agents.clone();
    shared.sort();
    assert!(shared.is_empty());
}

#[tokio::test]
async fn test_get_central_skills_ignores_claude_plugin_observations() {
    let pool = setup_test_db().await;

    let central_skill = make_skill("shared-skill", "Shared Skill", true);
    db::upsert_skill(&pool, &central_skill).await.unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(skills_with_links.len(), 1);
    assert_eq!(
        {
            let mut linked = skills_with_links[0].linked_agents.clone();
            linked.sort();
            linked
        },
        Vec::<String>::new(),
        "plugin observations must not pollute linked_agents state"
    );
    let mut shared = skills_with_links[0].shared_root_agents.clone();
    shared.sort();
    assert!(shared.is_empty());
}

#[tokio::test]
async fn test_get_central_skills_excludes_non_central() {
    let pool = setup_test_db().await;

    let central = make_skill("c-skill", "Central", true);
    let non_central = make_skill("nc-skill", "Non-Central", false);
    db::upsert_skill(&pool, &central).await.unwrap();
    db::upsert_skill(&pool, &non_central).await.unwrap();

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(
        skills_with_links.len(),
        1,
        "only central skills should be returned"
    );
    assert_eq!(skills_with_links[0].id, "c-skill");
}

// ── get_skill_detail ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_central_skill_rejects_non_central_skill() {
    let pool = setup_test_db().await;
    let skill = make_skill("plain-skill", "Plain Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let error = delete_central_skill_impl(&pool, "plain-skill", &[])
        .await
        .unwrap_err();

    assert!(error.contains("is not a Central skill"));
}

#[tokio::test]
async fn test_delete_central_skill_rejects_path_outside_central_root() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let outside_dir = temp.path().join("outside").join("outside-skill");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&outside_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("outside-skill", "Outside Skill", &outside_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let error = delete_central_skill_impl(&pool, "outside-skill", &[])
        .await
        .unwrap_err();

    assert!(error.contains("outside Central Skills root"));
    assert!(outside_dir.exists());
    assert!(db::get_skill_by_id(&pool, "outside-skill")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_delete_central_skill_removes_selected_copy_and_retains_unselected_copy() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("central-delete");
    let removed_copy_dir = temp.path().join("cursor").join("central-delete");
    let retained_copy_dir = temp.path().join("claude").join("central-delete");
    let missing_symlink_path = temp.path().join("codex").join("central-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    write_test_skill_dir(&removed_copy_dir);
    write_test_skill_dir(&retained_copy_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("central-delete", "Central Delete", &central_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("central-delete", "cursor", &removed_copy_dir, "copy", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "central-delete",
            "claude-code",
            &retained_copy_dir,
            "copy",
            None,
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "central-delete",
            "codex",
            &missing_symlink_path,
            "symlink",
            Some(&central_dir),
        ),
    )
    .await
    .unwrap();

    let result = delete_central_skill_impl(&pool, "central-delete", &["cursor".to_string()])
        .await
        .unwrap();

    assert_eq!(
        result.removed_central_path,
        central_dir.to_string_lossy().into_owned()
    );
    let mut removed_agent_ids = result.removed_agent_ids;
    removed_agent_ids.sort();
    assert_eq!(
        removed_agent_ids,
        vec!["codex".to_string(), "cursor".to_string()]
    );
    assert_eq!(result.retained_agent_ids, vec!["claude-code".to_string()]);
    assert!(!central_dir.exists());
    assert!(!removed_copy_dir.exists());
    assert!(retained_copy_dir.exists());
    assert!(db::get_skill_by_id(&pool, "central-delete")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_installations(&pool, "central-delete")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_preview_delete_central_skills_reports_copies_and_preview_failures() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("preview-delete");
    let copy_dir = temp.path().join("cursor").join("preview-delete");
    let missing_symlink_path = temp.path().join("codex").join("preview-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    write_test_skill_dir(&copy_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("preview-delete", "Preview Delete", &central_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("preview-delete", "cursor", &copy_dir, "copy", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "preview-delete",
            "codex",
            &missing_symlink_path,
            "symlink",
            Some(&central_dir),
        ),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_impl(
        &pool,
        &["preview-delete".to_string(), "missing-delete".to_string()],
    )
    .await
    .unwrap();

    assert_eq!(result.previews.len(), 1);
    assert_eq!(result.previews[0].skill_id, "preview-delete");
    assert_eq!(result.previews[0].copy_installations.len(), 1);
    assert_eq!(result.previews[0].copy_installations[0].agent_id, "cursor");
    assert_eq!(result.previews[0].auto_removed_agent_ids, vec!["codex"]);
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_id, "missing-delete");
}

#[tokio::test]
async fn test_preview_delete_central_skills_reports_auto_links_without_central_self() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("linked-delete");
    let symlink_path = temp.path().join("codex").join("linked-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("linked-delete", "Linked Delete", &central_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("linked-delete", "central", &central_dir, "native", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "linked-delete",
            "codex",
            &symlink_path,
            "symlink",
            Some(&central_dir),
        ),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_impl(&pool, &["linked-delete".to_string()])
        .await
        .unwrap();

    assert!(result.failed.is_empty());
    assert_eq!(result.previews.len(), 1);
    assert!(result.previews[0].copy_installations.is_empty());
    assert_eq!(result.previews[0].auto_removed_agent_ids, vec!["codex"]);
}

#[tokio::test]
async fn test_batch_delete_central_skills_keeps_partial_failures_isolated() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let valid_dir = central_root.join("valid-delete");
    let outside_dir = temp.path().join("outside").join("unsafe-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&valid_dir);
    write_test_skill_dir(&outside_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("valid-delete", "Valid Delete", &valid_dir),
    )
    .await
    .unwrap();
    db::upsert_skill(
        &pool,
        &make_central_skill_at("unsafe-delete", "Unsafe Delete", &outside_dir),
    )
    .await
    .unwrap();

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "valid-delete".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "unsafe-delete".to_string(),
                remove_agent_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "valid-delete");
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_id, "unsafe-delete");
    assert!(!valid_dir.exists());
    assert!(outside_dir.exists());
    assert!(db::get_skill_by_id(&pool, "valid-delete")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_by_id(&pool, "unsafe-delete")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_batch_delete_central_skills_dedupes_and_merges_copy_agents() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("dedupe-delete");
    let cursor_copy_dir = temp.path().join("cursor").join("dedupe-delete");
    let claude_copy_dir = temp.path().join("claude").join("dedupe-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    write_test_skill_dir(&cursor_copy_dir);
    write_test_skill_dir(&claude_copy_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("dedupe-delete", "Dedupe Delete", &central_dir),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("dedupe-delete", "cursor", &cursor_copy_dir, "copy", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "dedupe-delete",
            "claude-code",
            &claude_copy_dir,
            "copy",
            None,
        ),
    )
    .await
    .unwrap();

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "dedupe-delete".to_string(),
                remove_agent_ids: vec!["cursor".to_string()],
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "dedupe-delete".to_string(),
                remove_agent_ids: vec!["claude-code".to_string(), "cursor".to_string()],
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.succeeded.len(), 1);
    assert!(result.failed.is_empty());
    let mut removed_agent_ids = result.succeeded[0].removed_agent_ids.clone();
    removed_agent_ids.sort();
    assert_eq!(
        removed_agent_ids,
        vec!["claude-code".to_string(), "cursor".to_string()]
    );
    assert!(!central_dir.exists());
    assert!(!cursor_copy_dir.exists());
    assert!(!claude_copy_dir.exists());
}

#[tokio::test]
async fn test_delete_skill_repository_removes_repository_skills_and_record() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("repo-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("repo-delete", "Repo Delete", &central_dir),
    )
    .await
    .unwrap();
    let repository = db::create_or_update_skill_repository(
        &pool,
        Some("github-repo-delete"),
        "owner/repo-delete",
        "github",
        Some("owner"),
        Some("repo-delete"),
        Some("main"),
        Some("https://github.com/owner/repo-delete"),
        false,
    )
    .await
    .unwrap();
    db::assign_skills_to_repository(
        &pool,
        &repository.id,
        &["repo-delete".to_string()],
        Some("skills/repo-delete"),
    )
    .await
    .unwrap();

    let preview = preview_delete_skill_repository_impl(&pool, &repository.id)
        .await
        .unwrap();
    assert_eq!(preview.repository.skill_count, 1);
    assert_eq!(preview.delete_preview.previews.len(), 1);

    let result = delete_skill_repository_impl(&pool, &repository.id, &[])
        .await
        .unwrap();

    assert!(result.deleted_repository);
    assert!(result.delete_result.failed.is_empty());
    assert_eq!(result.delete_result.succeeded.len(), 1);
    assert!(!central_dir.exists());
    assert!(db::get_skill_by_id(&pool, "repo-delete")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_delete_skill_repository_rejects_unknown_repository() {
    let pool = setup_test_db().await;

    let error = delete_skill_repository_impl(&pool, db::LOCAL_UNKNOWN_REPOSITORY_ID, &[])
        .await
        .unwrap_err();

    assert!(error.contains("cannot be deleted"));
    assert!(
        db::get_skill_repository_by_id(&pool, db::LOCAL_UNKNOWN_REPOSITORY_ID)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_delete_skill_repository_keeps_record_on_partial_failure() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let valid_dir = central_root.join("repo-valid-delete");
    let outside_dir = temp.path().join("outside").join("repo-unsafe-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&valid_dir);
    write_test_skill_dir(&outside_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("repo-valid-delete", "Repo Valid Delete", &valid_dir),
    )
    .await
    .unwrap();
    db::upsert_skill(
        &pool,
        &make_central_skill_at("repo-unsafe-delete", "Repo Unsafe Delete", &outside_dir),
    )
    .await
    .unwrap();
    let repository = db::create_or_update_skill_repository(
        &pool,
        Some("github-repo-partial"),
        "owner/repo-partial",
        "github",
        Some("owner"),
        Some("repo-partial"),
        Some("main"),
        Some("https://github.com/owner/repo-partial"),
        false,
    )
    .await
    .unwrap();
    db::assign_skills_to_repository(
        &pool,
        &repository.id,
        &[
            "repo-valid-delete".to_string(),
            "repo-unsafe-delete".to_string(),
        ],
        Some("skills/repo-partial"),
    )
    .await
    .unwrap();

    let result = delete_skill_repository_impl(&pool, &repository.id, &[])
        .await
        .unwrap();

    assert!(!result.deleted_repository);
    assert_eq!(result.delete_result.succeeded.len(), 1);
    assert_eq!(result.delete_result.failed.len(), 1);
    assert!(!valid_dir.exists());
    assert!(outside_dir.exists());
    assert!(db::get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_get_skill_detail_returns_installations() {
    let pool = setup_test_db().await;

    let skill = make_skill("detail-skill", "Detail Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let now = Utc::now().to_rfc3339();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "detail-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/claude/detail-skill/SKILL.md".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: now.clone(),
        },
    )
    .await
    .unwrap();

    let detail = get_skill_detail_impl(&pool, "detail-skill").await.unwrap();
    assert_eq!(detail.id, "detail-skill");
    assert_eq!(detail.installations.len(), 1);
    assert_eq!(detail.installations[0].agent_id, "claude-code");
    // installed_at should be populated from created_at
    assert!(
        !detail.installations[0].installed_at.is_empty(),
        "installed_at must be set"
    );
    assert!(
        detail.collections.is_empty(),
        "skill should have no collections by default"
    );
}

#[tokio::test]
async fn test_get_skill_detail_returns_collections() {
    let pool = setup_test_db().await;

    let skill = make_skill("detail-skill", "Detail Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let alpha = db::create_collection(&pool, "Alpha", Some("First collection"))
        .await
        .unwrap();
    let beta = db::create_collection(&pool, "Beta", None).await.unwrap();

    db::add_skill_to_collection(&pool, &alpha.id, "detail-skill")
        .await
        .unwrap();
    db::add_skill_to_collection(&pool, &beta.id, "detail-skill")
        .await
        .unwrap();

    let detail = get_skill_detail_impl(&pool, "detail-skill").await.unwrap();
    let collection_names: Vec<&str> = detail.collections.iter().map(|c| c.name.as_str()).collect();

    assert_eq!(collection_names, vec!["Alpha", "Beta"]);
}

#[tokio::test]
async fn test_get_skill_detail_not_found() {
    let pool = setup_test_db().await;
    let result = get_skill_detail_impl(&pool, "nonexistent").await;
    assert!(result.is_err(), "should error for unknown skill_id");
}

// ── read_skill_content ────────────────────────────────────────────────────

#[tokio::test]
async fn test_read_skill_content_returns_file_content() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let skill_dir = tmp.path().join("my-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    let skill_md_path = skill_dir.join("SKILL.md");
    let expected_content = "---\nname: My Skill\n---\n\n# My Skill\n\nContent here.";
    fs::write(&skill_md_path, expected_content).unwrap();

    let skill = Skill {
        id: "my-skill".to_string(),
        name: "My Skill".to_string(),
        description: None,
        file_path: skill_md_path.to_string_lossy().into_owned(),
        canonical_path: None,
        is_central: false,
        source: None,
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
    };
    db::upsert_skill(&pool, &skill).await.unwrap();

    let content = read_skill_content_impl(&pool, "my-skill").await.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_skill_content_file_not_found() {
    let pool = setup_test_db().await;

    let skill = Skill {
        id: "missing-file-skill".to_string(),
        name: "Missing File".to_string(),
        description: None,
        file_path: "/nonexistent/SKILL.md".to_string(),
        canonical_path: None,
        is_central: false,
        source: None,
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
    };
    db::upsert_skill(&pool, &skill).await.unwrap();

    let result = read_skill_content_impl(&pool, "missing-file-skill").await;
    assert!(result.is_err(), "should error when file does not exist");
}

// ── Testable core implementations (without Tauri State) ───────────────────

async fn get_central_skills_impl(pool: &SqlitePool) -> Result<Vec<SkillWithLinks>, String> {
    super::get_central_skills_impl(pool).await
}

async fn get_skill_detail_impl(pool: &SqlitePool, skill_id: &str) -> Result<SkillDetail, String> {
    super::get_skill_detail_with_row_impl(pool, skill_id, None, None).await
}

async fn read_skill_content_impl(pool: &SqlitePool, skill_id: &str) -> Result<String, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    std::fs::read_to_string(&skill.file_path)
        .map_err(|e| format!("Failed to read '{}': {}", skill.file_path, e))
}

// ── Regression: get_skills_by_agent_impl returns installation metadata ─────

/// `get_skills_by_agent_impl` must return `SkillForAgent` objects that
/// include `link_type`, `dir_path`, and `symlink_target` from the
/// installation record so the frontend `SkillCard` can show the correct
/// source indicator.
#[tokio::test]
async fn test_get_skills_by_agent_impl_includes_installation_metadata() {
    let pool = setup_test_db().await;

    let skill = make_skill("meta-skill", "Meta Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "meta-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/claude/meta-skill".to_string(),
            link_type: "symlink".to_string(),
            symlink_target: Some("/tmp/central/meta-skill".to_string()),
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let skills = get_skills_by_agent_impl(&pool, "claude-code")
        .await
        .unwrap();
    assert_eq!(skills.len(), 1, "should find one skill for claude-code");

    let s = &skills[0];
    assert_eq!(s.id, "meta-skill");
    assert_eq!(
        s.link_type, "symlink",
        "link_type must come from installation record"
    );
    assert_eq!(
        s.dir_path, "/tmp/claude/meta-skill",
        "dir_path must be installed_path from installation record"
    );
    assert_eq!(
        s.symlink_target.as_deref(),
        Some("/tmp/central/meta-skill"),
        "symlink_target must be forwarded from installation record"
    );
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_empty_for_unknown_agent() {
    let pool = setup_test_db().await;
    let skills = get_skills_by_agent_impl(&pool, "nobody").await.unwrap();
    assert!(
        skills.is_empty(),
        "no skills for an agent with no installations"
    );
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_claude_uses_observations_for_duplicate_rows() {
    let pool = setup_test_db().await;

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let mut skills = get_skills_by_agent_impl(&pool, "claude-code")
        .await
        .unwrap();
    skills.sort_by(|a, b| a.dir_path.cmp(&b.dir_path));

    assert_eq!(
        skills.len(),
        2,
        "Claude queries should surface duplicate logical skills from different sources"
    );
    assert_eq!(skills[0].id, "shared-skill");
    assert_eq!(skills[1].id, "shared-skill");
    assert_ne!(skills[0].dir_path, skills[1].dir_path);
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_claude_includes_source_identity_and_conflict_grouping() {
    let pool = setup_test_db().await;

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let mut skills = get_skills_by_agent_impl(&pool, "claude-code")
        .await
        .unwrap();
    skills.sort_by(|a, b| a.dir_path.cmp(&b.dir_path));

    assert_eq!(skills.len(), 2);
    assert_eq!(
        skills[0].row_id,
        "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
    );
    assert_eq!(
        skills[1].row_id,
        "claude-code::/tmp/.claude/skills/shared-skill"
    );
    assert_eq!(skills[0].source_kind.as_deref(), Some("plugin"));
    assert_eq!(skills[1].source_kind.as_deref(), Some("user"));
    assert_eq!(
        skills[0].source_root.as_deref(),
        Some("/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0")
    );
    assert_eq!(
        skills[1].source_root.as_deref(),
        Some("/tmp/.claude/skills")
    );
    assert!(skills[0].is_read_only);
    assert!(!skills[1].is_read_only);
    assert_eq!(
        skills[0].conflict_group.as_deref(),
        Some("claude-code::shared-skill")
    );
    assert_eq!(
        skills[1].conflict_group.as_deref(),
        Some("claude-code::shared-skill")
    );
    assert_eq!(skills[0].conflict_count, 2);
    assert_eq!(skills[1].conflict_count, 2);
}

#[tokio::test]
async fn test_get_skill_detail_with_row_impl_claude_plugin_row_uses_selected_observation() {
    let pool = setup_test_db().await;

    let skill = make_skill("shared-skill", "Shared Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/.claude/skills/shared-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let collection = db::create_collection(&pool, "Alpha", None).await.unwrap();
    db::add_skill_to_collection(&pool, &collection.id, "shared-skill")
        .await
        .unwrap();

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let detail = get_skill_detail_with_row_impl(
        &pool,
        "shared-skill",
        Some("claude-code"),
        Some("claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"),
    )
    .await
    .unwrap();

    assert_eq!(
        detail.row_id,
        "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
    );
    assert_eq!(
        detail.dir_path,
        "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
    );
    assert_eq!(
        detail.file_path,
        "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill/SKILL.md"
    );
    assert_eq!(detail.source_kind.as_deref(), Some("plugin"));
    assert_eq!(
        detail.source_root.as_deref(),
        Some("/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0")
    );
    assert!(detail.is_read_only);
    assert_eq!(detail.conflict_count, 2);
    assert_eq!(
        detail.conflict_group.as_deref(),
        Some("claude-code::shared-skill")
    );
    assert!(
        detail.installations.is_empty(),
        "plugin detail should not expose manageable installations"
    );
    assert!(
        detail.collections.is_empty(),
        "plugin detail should not expose collection management state"
    );
}

#[tokio::test]
async fn test_get_skill_detail_with_row_impl_claude_user_row_keeps_manageable_state() {
    let pool = setup_test_db().await;

    let skill = make_skill("shared-skill", "Shared Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/.claude/skills/shared-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let collection = db::create_collection(&pool, "Alpha", None).await.unwrap();
    db::add_skill_to_collection(&pool, &collection.id, "shared-skill")
        .await
        .unwrap();

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let detail = get_skill_detail_with_row_impl(
        &pool,
        "shared-skill",
        Some("claude-code"),
        Some("claude-code::/tmp/.claude/skills/shared-skill"),
    )
    .await
    .unwrap();

    assert_eq!(
        detail.row_id,
        "claude-code::/tmp/.claude/skills/shared-skill"
    );
    assert_eq!(detail.dir_path, "/tmp/.claude/skills/shared-skill");
    assert_eq!(detail.source_kind.as_deref(), Some("user"));
    assert!(!detail.is_read_only);
    assert_eq!(detail.conflict_count, 2);
    assert_eq!(detail.installations.len(), 1);
    assert_eq!(detail.collections.len(), 1);
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_copy_link_type() {
    let pool = setup_test_db().await;

    let skill = make_skill("copy-skill", "Copy Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "copy-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: "/tmp/cursor/copy-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let skills = get_skills_by_agent_impl(&pool, "cursor").await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].link_type, "copy");
    assert!(
        skills[0].symlink_target.is_none(),
        "copy skills have no symlink target"
    );
}

// ── read_file_by_path ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_read_file_by_path_success() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test-skill.md");
    let content = "---\nname: Test\n---\n\n# Test Skill";
    fs::write(&file_path, content).unwrap();

    let result = read_file_by_path_impl(&file_path.to_string_lossy());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
}

#[tokio::test]
async fn test_read_file_by_path_not_found() {
    let result = read_file_by_path_impl("/nonexistent/file.md");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_directory_tree_reads_nested_local_structure() {
    let tmp = TempDir::new().unwrap();
    make_directory_tree_fixture(tmp.path());

    let result = super::files::list_directory_tree_impl(&tmp.path().to_string_lossy()).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "examples");
    assert_eq!(result[0].file_type, "dir");
    assert_eq!(result[0].children.len(), 2);
    assert_eq!(result[1].name, "SKILL.md");
    assert_eq!(result[1].file_type, "file");
}

#[tokio::test]
async fn test_list_directory_tree_rejects_missing_path() {
    let result = super::files::list_directory_tree_impl("/definitely/missing/path");
    assert!(result.is_err());
}

// ── open_in_file_manager ───────────────────────────────────────────────────

#[tokio::test]
async fn test_open_in_file_manager_nonexistent_path() {
    let result = open_in_file_manager_checked_impl("/nonexistent/path/that/does/not/exist");
    assert!(result.is_err());
}
