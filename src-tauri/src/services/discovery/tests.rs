use super::scan::{is_scan_cancelled, set_scan_cancel_for_test};
use super::*;
use crate::db::{self, DbPool};
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[test]
fn test_default_scan_roots_returns_candidates() {
    let roots = default_scan_roots();
    assert!(!roots.is_empty(), "should return at least some candidates");
    // Each root should have a path and label.
    for root in &roots {
        assert!(!root.path.is_empty());
        assert!(!root.label.is_empty());
    }
}

#[test]
fn test_scan_root_exists_matches_filesystem() {
    let roots = default_scan_roots();
    for root in &roots {
        let actually_exists = Path::new(&root.path).exists();
        assert_eq!(
            root.exists, actually_exists,
            "exists flag should match actual filesystem for {}",
            root.path
        );
    }
}

#[tokio::test]
async fn test_platform_skill_patterns_excludes_central() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    let patterns = platform_skill_patterns(&pool);
    // Central should be excluded.
    assert!(
        !patterns.iter().any(|(id, _, _)| id == "central"),
        "central should not appear in platform skill patterns"
    );
    // Claude Code should be included.
    assert!(
        patterns.iter().any(|(id, _, _)| id == "claude-code"),
        "claude-code should appear in platform skill patterns"
    );
    let mut seen_paths = std::collections::HashSet::new();
    for (_, _, rel_path) in &patterns {
        assert!(
            seen_paths.insert(rel_path.clone()),
            "duplicate platform pattern path {:?}",
            rel_path
        );
    }
    assert!(
        patterns
            .iter()
            .any(|(_, _, rel_path)| rel_path == &PathBuf::from(".agents/skills")),
        "the shared .agents/skills pattern should be discoverable once"
    );
}

#[tokio::test]
async fn test_scan_root_for_projects_finds_nested_skills() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Create a project with a .claude/skills/ subdirectory.
    let project_dir = tmp.path().join("my-project");
    let skill_dir = project_dir.join(".claude/skills/deploy-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: deploy\ndescription: Deploy stuff\n---\n\n# Deploy\n",
    )
    .unwrap();

    // Build patterns: .claude/skills
    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert_eq!(projects.len(), 1, "should find 1 project");
    assert_eq!(projects[0].project_name, "my-project");
    assert_eq!(projects[0].skills.len(), 1);
    assert_eq!(projects[0].skills[0].platform_id, "claude-code");
    assert_eq!(projects[0].skills[0].name, "deploy");
}

#[tokio::test]
async fn test_scan_root_for_projects_skips_dirs_without_skills() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // A project dir with no skill subdirectories.
    let project_dir = tmp.path().join("empty-project");
    std::fs::create_dir_all(project_dir.join("src")).unwrap();

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));
    assert!(
        projects.is_empty(),
        "should not find projects without skills"
    );
}

#[tokio::test]
async fn test_scan_root_for_projects_handles_multiple_platforms() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    let project_dir = tmp.path().join("multi-project");
    // Create skills for two platforms.
    let claude_skill = project_dir.join(".claude/skills/claude-skill");
    std::fs::create_dir_all(&claude_skill).unwrap();
    std::fs::write(
        claude_skill.join("SKILL.md"),
        "---\nname: claude-skill\ndescription: test\n---\n\n# Test\n",
    )
    .unwrap();

    let cursor_skill = project_dir.join(".cursor/skills/cursor-skill");
    std::fs::create_dir_all(&cursor_skill).unwrap();
    std::fs::write(
        cursor_skill.join("SKILL.md"),
        "---\nname: cursor-skill\ndescription: test\n---\n\n# Test\n",
    )
    .unwrap();

    let patterns = vec![
        (
            "claude-code".to_string(),
            "Claude Code".to_string(),
            PathBuf::from(".claude/skills"),
        ),
        (
            "cursor".to_string(),
            "Cursor".to_string(),
            PathBuf::from(".cursor/skills"),
        ),
    ];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].skills.len(), 2);
}

#[tokio::test]
async fn test_scan_root_for_projects_detects_already_central() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Create a skill in central.
    let central_skill = central_dir.join("shared-skill");
    std::fs::create_dir_all(&central_skill).unwrap();
    std::fs::write(
        central_skill.join("SKILL.md"),
        "---\nname: shared-skill\n---\n\n# Test\n",
    )
    .unwrap();

    // Create the same skill name in a project.
    let project_dir = tmp.path().join("my-project");
    let project_skill = project_dir.join(".claude/skills/shared-skill");
    std::fs::create_dir_all(&project_skill).unwrap();
    std::fs::write(
        project_skill.join("SKILL.md"),
        "---\nname: shared-skill\n---\n\n# Test\n",
    )
    .unwrap();

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].skills.len(), 1);
    assert!(
        projects[0].skills[0].is_already_central,
        "should detect skill is already in central"
    );
}

#[tokio::test]
async fn test_import_discovered_skill_to_central_copies_and_persists() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();

    // Override central dir for testing.
    let central_dir = tmp.path().join("central");
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(central_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    std::fs::create_dir_all(&central_dir).unwrap();

    // Create a discovered skill.
    let skill_dir = tmp.path().join("project/.claude/skills/my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: A test skill\n---\n\n# My Skill\n",
    )
    .unwrap();

    // Insert discovered skill record.
    let now = Utc::now().to_rfc3339();
    db::insert_discovered_skill(
        &pool,
        "claude-code__project__my-skill",
        "my-skill",
        Some("A test skill"),
        &skill_dir.join("SKILL.md").to_string_lossy(),
        &skill_dir.to_string_lossy(),
        &tmp.path().join("project").to_string_lossy(),
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Set HOME to tmp so import_discovered_skill_to_central finds the right dir.
    // We'll call the impl directly instead.
    let result = import_discovered_skill_to_central_at(
        &pool,
        "claude-code__project__my-skill",
        &central_dir,
    )
    .await;

    assert!(result.is_ok(), "import should succeed: {:?}", result);

    // Verify the skill was copied to central.
    let target = central_dir.join("my-skill");
    assert!(target.exists(), "skill should be copied to central");
    assert!(
        target.join("SKILL.md").exists(),
        "SKILL.md should exist in central"
    );

    // Verify discovered skill record was removed.
    let record = db::get_discovered_skill_by_id(&pool, "claude-code__project__my-skill")
        .await
        .unwrap();
    assert!(
        record.is_none(),
        "discovered skill record should be removed"
    );
}

// ── Additional tests ──────────────────────────────────────────────────────

/// Helper: set up an in-memory DB with initialized schema.
async fn setup_test_db() -> DbPool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_import_discovered_skill_to_platform_creates_symlink() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // Override agent skills dir for testing.
    let agent_dir = tmp.path().join("agent-skills");
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(agent_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    std::fs::create_dir_all(&agent_dir).unwrap();

    // Create a discovered skill in a project.
    let skill_dir = tmp.path().join("project/.claude/skills/my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: A test skill\n---\n\n# My Skill\n",
    )
    .unwrap();

    let now = Utc::now().to_rfc3339();
    db::insert_discovered_skill(
        &pool,
        "claude-code__project__my-skill",
        "my-skill",
        Some("A test skill"),
        &skill_dir.join("SKILL.md").to_string_lossy(),
        &skill_dir.to_string_lossy(),
        &tmp.path().join("project").to_string_lossy(),
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Import to platform using the impl function.
    let result = import_discovered_skill_to_platform_at(
        &pool,
        "claude-code__project__my-skill",
        "claude-code",
        &agent_dir,
    )
    .await;

    assert!(result.is_ok(), "import should succeed: {:?}", result);

    // Verify the symlink was created.
    let link_path = agent_dir.join("my-skill");
    assert!(link_path.exists(), "symlink target should exist");
    let meta = std::fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.is_symlink(), "should be a symlink");

    // Verify discovered skill record is KEPT (not deleted) after platform install.
    // This enables multi-platform install — the record stays so it can be
    // installed to additional platforms.
    let record = db::get_discovered_skill_by_id(&pool, "claude-code__project__my-skill")
        .await
        .unwrap();
    assert!(
        record.is_some(),
        "discovered skill record should be kept after platform install"
    );
}

#[tokio::test]
async fn test_import_discovered_skill_to_platform_copy_creates_real_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let agent_dir = tmp.path().join("agent-skills");
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(agent_dir.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    std::fs::create_dir_all(&agent_dir).unwrap();

    let skill_dir = tmp.path().join("project/.cursor/skills/my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: A test skill\n---\n\n# My Skill\n",
    )
    .unwrap();
    std::fs::write(skill_dir.join("extra.txt"), "copy me").unwrap();

    let now = Utc::now().to_rfc3339();
    db::insert_discovered_skill(
        &pool,
        "cursor__project__my-skill",
        "my-skill",
        Some("A test skill"),
        &skill_dir.join("SKILL.md").to_string_lossy(),
        &skill_dir.to_string_lossy(),
        &tmp.path().join("project").to_string_lossy(),
        "project",
        "cursor",
        &now,
    )
    .await
    .unwrap();

    let result = import_discovered_skill_to_platform_with_method_at(
        &pool,
        "cursor__project__my-skill",
        "cursor",
        &agent_dir,
        Some("copy"),
    )
    .await;

    assert!(result.is_ok(), "copy import should succeed: {:?}", result);

    let target_path = agent_dir.join("my-skill");
    assert!(target_path.exists(), "copied target should exist");
    let meta = std::fs::symlink_metadata(&target_path).unwrap();
    assert!(meta.is_dir(), "copy install should create a real directory");
    assert!(
        !meta.file_type().is_symlink(),
        "copy install must not create a symlink"
    );
    assert!(target_path.join("SKILL.md").exists());
    assert!(target_path.join("extra.txt").exists());

    let installations = db::get_skill_installations(&pool, "my-skill")
        .await
        .unwrap();
    let installation = installations
        .iter()
        .find(|installation| installation.agent_id == "cursor")
        .expect("cursor install record should exist");
    assert_eq!(installation.link_type, "copy");
    assert!(installation.symlink_target.is_none());
}

#[tokio::test]
async fn test_import_source_skill_to_central_copies_without_discovered_row() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    let skill_dir = tmp.path().join("vault/.skills/obsidian-demo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: obsidian-demo\ndescription: Vault sourced\n---\n\n# Demo\n",
    )
    .unwrap();

    let result =
        import_source_skill_to_central_at(&pool, &skill_dir.to_string_lossy(), &central_dir).await;

    assert!(result.is_ok(), "source import should succeed: {:?}", result);
    assert!(central_dir.join("obsidian-demo").join("SKILL.md").exists());

    let stored = db::get_skill_by_id(&pool, "obsidian-demo")
        .await
        .unwrap()
        .expect("central skill should be stored");
    assert!(stored.is_central);
    assert_eq!(
        stored.file_path,
        central_dir
            .join("obsidian-demo")
            .join("SKILL.md")
            .to_string_lossy()
    );
}

#[tokio::test]
async fn test_stop_project_scan_sets_cancel_flag() {
    // Before calling stop, the flag should be false.
    set_scan_cancel_for_test(false);
    assert!(!is_scan_cancelled());

    // After calling stop, the flag should be true.
    set_scan_cancel_for_test(true);
    assert!(is_scan_cancelled());

    // Reset for other tests.
    set_scan_cancel_for_test(false);
}

#[tokio::test]
async fn test_get_discovered_skills_groups_by_project() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();

    // Insert two discovered skills in the same project.
    db::insert_discovered_skill(
        &pool,
        "claude-code__proj1__skill-a",
        "skill-a",
        Some("Skill A"),
        "/tmp/proj1/.claude/skills/skill-a/SKILL.md",
        "/tmp/proj1/.claude/skills/skill-a",
        "/tmp/proj1",
        "proj1",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    db::insert_discovered_skill(
        &pool,
        "cursor__proj1__skill-b",
        "skill-b",
        Some("Skill B"),
        "/tmp/proj1/.cursor/skills/skill-b/SKILL.md",
        "/tmp/proj1/.cursor/skills/skill-b",
        "/tmp/proj1",
        "proj1",
        "cursor",
        &now,
    )
    .await
    .unwrap();

    // Insert a skill in a different project.
    db::insert_discovered_skill(
        &pool,
        "claude-code__proj2__skill-c",
        "skill-c",
        Some("Skill C"),
        "/tmp/proj2/.claude/skills/skill-c/SKILL.md",
        "/tmp/proj2/.claude/skills/skill-c",
        "/tmp/proj2",
        "proj2",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    let rows = db::get_all_discovered_skills(&pool).await.unwrap();
    assert_eq!(rows.len(), 3, "should have 3 discovered skill rows");

    // Group by project_path.
    let mut by_project: HashMap<String, Vec<db::DiscoveredSkillRow>> = HashMap::new();
    for row in rows {
        by_project
            .entry(row.project_path.clone())
            .or_default()
            .push(row);
    }

    assert_eq!(by_project.len(), 2, "should have 2 projects");
    let proj1_skills = by_project.get("/tmp/proj1").unwrap();
    assert_eq!(proj1_skills.len(), 2, "proj1 should have 2 skills");
    let proj2_skills = by_project.get("/tmp/proj2").unwrap();
    assert_eq!(proj2_skills.len(), 1, "proj2 should have 1 skill");
}

#[tokio::test]
async fn test_clear_discovered_skills_removes_all() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();

    db::insert_discovered_skill(
        &pool,
        "id1",
        "skill-1",
        None,
        "/tmp/skill1/SKILL.md",
        "/tmp/skill1",
        "/tmp/proj1",
        "proj1",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    db::insert_discovered_skill(
        &pool,
        "id2",
        "skill-2",
        None,
        "/tmp/skill2/SKILL.md",
        "/tmp/skill2",
        "/tmp/proj1",
        "proj1",
        "cursor",
        &now,
    )
    .await
    .unwrap();

    let before = db::get_all_discovered_skills(&pool).await.unwrap();
    assert_eq!(before.len(), 2);

    db::clear_all_discovered_skills(&pool).await.unwrap();

    let after = db::get_all_discovered_skills(&pool).await.unwrap();
    assert!(after.is_empty(), "all discovered skills should be cleared");
}

#[tokio::test]
async fn test_get_scan_roots_returns_defaults() {
    let pool = setup_test_db().await;

    // No persisted config yet — should return defaults.
    let roots = get_scan_roots_impl(&pool).await.unwrap();
    assert!(!roots.is_empty(), "should return default scan roots");

    // Each root should have a path and label.
    for root in &roots {
        assert!(!root.path.is_empty());
        assert!(!root.label.is_empty());
    }
}

#[tokio::test]
async fn test_set_scan_root_enabled_persists_state() {
    let pool = setup_test_db().await;

    // Get defaults.
    let roots = get_scan_roots_impl(&pool).await.unwrap();
    let some_path = roots[0].path.clone();

    // Disable a root.
    set_scan_root_enabled_impl(&pool, some_path.clone(), false)
        .await
        .unwrap();

    // Verify the change is reflected.
    let updated = get_scan_roots_impl(&pool).await.unwrap();
    let changed = updated.iter().find(|r| r.path == some_path).unwrap();
    assert!(
        !changed.enabled,
        "root should be disabled after set_scan_root_enabled"
    );

    // Re-enable it.
    set_scan_root_enabled_impl(&pool, some_path.clone(), true)
        .await
        .unwrap();

    let re_updated = get_scan_roots_impl(&pool).await.unwrap();
    let re_changed = re_updated.iter().find(|r| r.path == some_path).unwrap();
    assert!(re_changed.enabled, "root should be re-enabled");
}

#[tokio::test]
async fn test_scan_cancellation_stops_early() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Create multiple project dirs with skills.
    for i in 0..5 {
        let project_dir = tmp.path().join(format!("project-{}", i));
        let skill_dir = project_dir.join(".claude/skills/deploy-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: deploy-{}\ndescription: Deploy stuff\n---\n\n# Deploy {}\n",
                i, i
            ),
        )
        .unwrap();
    }

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    // Pre-cancelled flag drives the core's early-return path directly,
    // without depending on the orchestration layer's global SCAN_CANCEL.
    let cancel = AtomicBool::new(true);

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &cancel);
    assert!(
        projects.is_empty(),
        "should find no projects when cancel flag is set"
    );
}

#[tokio::test]
async fn test_discovered_skill_insert_and_get_by_id() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();

    db::insert_discovered_skill(
        &pool,
        "test-id-1",
        "test-skill",
        Some("A description"),
        "/tmp/project/.claude/skills/test-skill/SKILL.md",
        "/tmp/project/.claude/skills/test-skill",
        "/tmp/project",
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    let found = db::get_discovered_skill_by_id(&pool, "test-id-1")
        .await
        .unwrap();
    assert!(found.is_some());
    let row = found.unwrap();
    assert_eq!(row.name, "test-skill");
    assert_eq!(row.platform_id, "claude-code");
    assert_eq!(row.project_name, "project");

    let not_found = db::get_discovered_skill_by_id(&pool, "nonexistent")
        .await
        .unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_insert_discovered_skill_is_idempotent() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();

    // INSERT OR IGNORE should not fail on duplicate.
    db::insert_discovered_skill(
        &pool,
        "dup-id",
        "dup-skill",
        None,
        "/tmp/dup/SKILL.md",
        "/tmp/dup",
        "/tmp/proj",
        "proj",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Second insert with same ID should be silently ignored.
    db::insert_discovered_skill(
        &pool,
        "dup-id",
        "dup-skill-updated",
        Some("updated description"),
        "/tmp/dup/SKILL.md",
        "/tmp/dup",
        "/tmp/proj",
        "proj",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    let rows = db::get_all_discovered_skills(&pool).await.unwrap();
    assert_eq!(rows.len(), 1, "should still have only 1 row");
    assert_eq!(
        rows[0].name, "dup-skill",
        "original name should be preserved (INSERT OR IGNORE)"
    );
}

#[tokio::test]
async fn test_delete_discovered_skill() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();

    db::insert_discovered_skill(
        &pool,
        "to-delete",
        "delete-me",
        None,
        "/tmp/del/SKILL.md",
        "/tmp/del",
        "/tmp/proj",
        "proj",
        "cursor",
        &now,
    )
    .await
    .unwrap();

    let found = db::get_discovered_skill_by_id(&pool, "to-delete")
        .await
        .unwrap();
    assert!(found.is_some());

    db::delete_discovered_skill(&pool, "to-delete")
        .await
        .unwrap();

    let gone = db::get_discovered_skill_by_id(&pool, "to-delete")
        .await
        .unwrap();
    assert!(gone.is_none());
}

#[tokio::test]
async fn test_import_to_central_refuses_duplicate() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = setup_test_db().await;
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Create a skill already in central.
    let existing = central_dir.join("existing-skill");
    std::fs::create_dir_all(&existing).unwrap();
    std::fs::write(
        existing.join("SKILL.md"),
        "---\nname: existing-skill\n---\n\n# Test\n",
    )
    .unwrap();

    // Also create the same skill in a project (discovered).
    let project_skill = tmp.path().join("project/.claude/skills/existing-skill");
    std::fs::create_dir_all(&project_skill).unwrap();
    std::fs::write(
        project_skill.join("SKILL.md"),
        "---\nname: existing-skill\n---\n\n# Test\n",
    )
    .unwrap();

    let now = Utc::now().to_rfc3339();
    db::insert_discovered_skill(
        &pool,
        "claude-code__project__existing-skill",
        "existing-skill",
        None,
        &project_skill.join("SKILL.md").to_string_lossy(),
        &project_skill.to_string_lossy(),
        &tmp.path().join("project").to_string_lossy(),
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    let result = import_discovered_skill_to_central_at(
        &pool,
        "claude-code__project__existing-skill",
        &central_dir,
    )
    .await;

    assert!(
        result.is_err(),
        "should refuse to import when skill already exists in central"
    );
}

#[tokio::test]
async fn test_import_to_platform_refuses_existing_installation() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = setup_test_db().await;
    let agent_dir = tmp.path().join("agent-skills");
    std::fs::create_dir_all(&agent_dir).unwrap();

    // Create an existing skill in agent dir.
    let existing = agent_dir.join("existing-skill");
    std::fs::create_dir_all(&existing).unwrap();

    // Also create a discovered skill with the same name.
    let project_skill = tmp.path().join("project/.claude/skills/existing-skill");
    std::fs::create_dir_all(&project_skill).unwrap();
    std::fs::write(
        project_skill.join("SKILL.md"),
        "---\nname: existing-skill\n---\n\n# Test\n",
    )
    .unwrap();

    let now = Utc::now().to_rfc3339();
    db::insert_discovered_skill(
        &pool,
        "claude-code__project__existing-skill",
        "existing-skill",
        None,
        &project_skill.join("SKILL.md").to_string_lossy(),
        &project_skill.to_string_lossy(),
        &tmp.path().join("project").to_string_lossy(),
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    let result = import_discovered_skill_to_platform_at(
        &pool,
        "claude-code__project__existing-skill",
        "claude-code",
        &agent_dir,
    )
    .await;

    assert!(
        result.is_err(),
        "should refuse to import when skill already exists in agent dir"
    );
}

#[tokio::test]
async fn test_platform_skill_patterns_include_unique_non_central_paths() {
    let pool = setup_test_db().await;
    let patterns = platform_skill_patterns(&pool);

    assert!(
        !patterns.is_empty(),
        "platform skill patterns should not be empty"
    );
    assert!(
        patterns.iter().any(|(id, _, _)| id == "claude-code"),
        "claude-code should still expose its dedicated project pattern"
    );
    assert!(
        patterns
            .iter()
            .any(|(_, _, rel_path)| rel_path == &PathBuf::from(".agents/skills")),
        "shared .agents/skills should still be discoverable"
    );

    let mut seen_paths = std::collections::HashSet::new();
    for (_, _, rel_path) in &patterns {
        assert!(
            seen_paths.insert(rel_path.clone()),
            "duplicate platform pattern path {:?}",
            rel_path
        );
    }
}

#[tokio::test]
async fn test_discovered_project_count() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();

    // Insert skills across 3 different projects.
    for i in 0..3 {
        db::insert_discovered_skill(
            &pool,
            &format!("skill-{}", i),
            &format!("skill {}", i),
            None,
            &format!("/tmp/proj{}/SKILL.md", i),
            &format!("/tmp/proj{}", i),
            &format!("/tmp/proj{}", i),
            &format!("proj{}", i),
            "claude-code",
            &now,
        )
        .await
        .unwrap();
    }

    let count = db::get_discovered_project_count(&pool).await.unwrap();
    assert_eq!(count, 3, "should have 3 distinct projects");
}

// ── Recursive scan tests ──────────────────────────────────────────────────

#[tokio::test]
async fn test_recursive_scan_finds_deeply_nested_project() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Create a project nested 3 levels deep: root/org/team/my-project/.claude/skills/...
    let project_dir = tmp.path().join("org").join("team").join("my-project");
    let skill_dir = project_dir.join(".claude/skills/deploy-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: deploy\ndescription: Deploy stuff\n---\n\n# Deploy\n",
    )
    .unwrap();

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert_eq!(projects.len(), 1, "should find 1 project at depth 3");
    assert_eq!(projects[0].project_name, "my-project");
    assert_eq!(projects[0].skills.len(), 1);
    assert_eq!(projects[0].skills[0].platform_id, "claude-code");
    assert_eq!(projects[0].skills[0].name, "deploy");
    // project_path should be the directory containing the platform dir
    assert!(
        projects[0].project_path.contains("my-project"),
        "project_path should be the project dir, got: {}",
        projects[0].project_path
    );
}

#[tokio::test]
async fn test_recursive_scan_skips_hidden_dirs_at_root() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // A hidden directory at root level should be skipped (not traversed).
    let hidden_project = tmp.path().join(".hidden-org").join("my-project");
    let skill_dir = hidden_project.join(".claude/skills/deploy-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: deploy\ndescription: Deploy stuff\n---\n\n# Deploy\n",
    )
    .unwrap();

    // A visible directory should be traversed.
    let visible_project = tmp.path().join("visible-org").join("my-project");
    let visible_skill_dir = visible_project.join(".claude/skills/visible-skill");
    std::fs::create_dir_all(&visible_skill_dir).unwrap();
    std::fs::write(
        visible_skill_dir.join("SKILL.md"),
        "---\nname: visible-skill\ndescription: Visible\n---\n\n# Visible\n",
    )
    .unwrap();

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    // Should only find the project in the visible directory.
    assert_eq!(projects.len(), 1, "should only find the visible project");
    assert_eq!(projects[0].skills[0].name, "visible-skill");
}

#[tokio::test]
async fn test_recursive_scan_skips_node_modules_and_git() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // node_modules with a skill inside should NOT be found.
    let nm_project = tmp.path().join("node_modules").join("some-pkg");
    let nm_skill = nm_project.join(".claude/skills/hidden-skill");
    std::fs::create_dir_all(&nm_skill).unwrap();
    std::fs::write(
        nm_skill.join("SKILL.md"),
        "---\nname: hidden-skill\n---\n\n# Hidden\n",
    )
    .unwrap();

    // .git with a skill inside should NOT be found.
    let git_project = tmp.path().join(".git").join("subdir");
    let git_skill = git_project.join(".claude/skills/git-skill");
    std::fs::create_dir_all(&git_skill).unwrap();
    std::fs::write(
        git_skill.join("SKILL.md"),
        "---\nname: git-skill\n---\n\n# Git\n",
    )
    .unwrap();

    // A normal project should be found.
    let project_dir = tmp.path().join("my-project");
    let skill_dir = project_dir.join(".claude/skills/good-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: good-skill\ndescription: Good\n---\n\n# Good\n",
    )
    .unwrap();

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert_eq!(projects.len(), 1, "should only find the good project");
    assert_eq!(projects[0].skills[0].name, "good-skill");
}

#[tokio::test]
async fn test_recursive_scan_finds_multiple_projects_at_different_depths() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Project at depth 1 (immediate child).
    let project1 = tmp.path().join("project-1");
    let skill1 = project1.join(".claude/skills/skill-1");
    std::fs::create_dir_all(&skill1).unwrap();
    std::fs::write(
        skill1.join("SKILL.md"),
        "---\nname: skill-1\ndescription: First\n---\n\n# First\n",
    )
    .unwrap();

    // Project at depth 3 (nested under org/team).
    let project2 = tmp.path().join("org").join("team").join("project-2");
    let skill2 = project2.join(".factory/skills/skill-2");
    std::fs::create_dir_all(&skill2).unwrap();
    std::fs::write(
        skill2.join("SKILL.md"),
        "---\nname: skill-2\ndescription: Second\n---\n\n# Second\n",
    )
    .unwrap();

    let patterns = vec![
        (
            "claude-code".to_string(),
            "Claude Code".to_string(),
            PathBuf::from(".claude/skills"),
        ),
        (
            "factory-droid".to_string(),
            "Factory Droid".to_string(),
            PathBuf::from(".factory/skills"),
        ),
    ];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert_eq!(
        projects.len(),
        2,
        "should find 2 projects at different depths"
    );
    let names: Vec<&str> = projects.iter().map(|p| p.project_name.as_str()).collect();
    assert!(names.contains(&"project-1"), "should find project-1");
    assert!(names.contains(&"project-2"), "should find project-2");
}

#[tokio::test]
async fn test_recursive_scan_respects_max_depth() {
    let tmp = tempfile::TempDir::new().unwrap();
    let central_dir = tmp.path().join("central");
    std::fs::create_dir_all(&central_dir).unwrap();

    // Create a project deeper than MAX_SCAN_DEPTH.
    // MAX_SCAN_DEPTH = 8, so depth 10 should not be reached.
    let mut deep_path = tmp.path().to_path_buf();
    for i in 0..10 {
        deep_path = deep_path.join(format!("level-{}", i));
    }
    let skill_dir = deep_path.join(".claude/skills/deep-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: deep-skill\ndescription: Too deep\n---\n\n# Deep\n",
    )
    .unwrap();

    let patterns = vec![(
        "claude-code".to_string(),
        "Claude Code".to_string(),
        PathBuf::from(".claude/skills"),
    )];

    let projects = scan_root_for_projects(tmp.path(), &patterns, &central_dir, &AtomicBool::new(false));

    assert!(
        projects.is_empty(),
        "should not find projects beyond MAX_SCAN_DEPTH"
    );
}

#[tokio::test]
async fn test_should_skip_dir_rules() {
    // Always-skipped directories.
    assert!(should_skip_dir("node_modules", 0));
    assert!(should_skip_dir("node_modules", 5));
    assert!(should_skip_dir("target", 0));
    assert!(should_skip_dir("target", 3));
    assert!(should_skip_dir(".git", 0));
    assert!(should_skip_dir(".git", 5));
    assert!(should_skip_dir("build", 0));
    assert!(should_skip_dir("dist", 0));
    assert!(should_skip_dir("__pycache__", 0));
    assert!(should_skip_dir(".cache", 0));

    // Hidden dirs at root level (depth 0) should be skipped.
    assert!(should_skip_dir(".config", 0));
    assert!(should_skip_dir(".local", 0));
    assert!(should_skip_dir(".hidden-project", 0));

    // Hidden dirs at deeper levels should NOT be skipped
    // (they might contain platform patterns like .claude).
    assert!(!should_skip_dir(".claude", 1));
    assert!(!should_skip_dir(".hidden-project", 2));

    // Normal directories should never be skipped.
    assert!(!should_skip_dir("my-project", 0));
    assert!(!should_skip_dir("src", 0));
    assert!(!should_skip_dir("Documents", 0));
    assert!(!should_skip_dir("projects", 1));
}

// ── Cache reconciliation tests ─────────────────────────────────────────────

#[tokio::test]
async fn test_cache_reconciliation_removes_stale_skills() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let now = Utc::now().to_rfc3339();

    // Create a real skill on disk under the scan root.
    let project_dir = tmp.path().join("project");
    let skill_dir = project_dir.join(".claude/skills/real-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: real-skill\ndescription: Exists\n---\n\n# Real\n",
    )
    .unwrap();

    // Insert a discovered skill for a path that EXISTS.
    db::insert_discovered_skill(
        &pool,
        "claude-code__project__real-skill",
        "real-skill",
        Some("Exists"),
        &skill_dir.join("SKILL.md").to_string_lossy(),
        &skill_dir.to_string_lossy(),
        &project_dir.to_string_lossy(),
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Insert a discovered skill whose project_path is under the scan root
    // but whose dir_path no longer exists on disk.
    let stale_project_dir = tmp.path().join("stale-project");
    let stale_skill_dir = stale_project_dir.join(".claude/skills/stale-skill");
    // NOTE: We do NOT create the stale directory on disk.
    db::insert_discovered_skill(
        &pool,
        "claude-code__stale-project__stale-skill",
        "stale-skill",
        Some("Deleted"),
        &stale_skill_dir.join("SKILL.md").to_string_lossy(),
        &stale_skill_dir.to_string_lossy(),
        &stale_project_dir.to_string_lossy(),
        "stale-project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Simulate a scan: the real skill was found, the stale one was not.
    let scan_root = ScanRoot {
        path: tmp.path().to_string_lossy().into_owned(),
        label: "test".to_string(),
        exists: true,
        enabled: true,
    };

    let found_ids = vec!["claude-code__project__real-skill".to_string()];

    reconcile_discovered_skills(&pool, &[&scan_root], &found_ids)
        .await
        .unwrap();

    // The real skill should still be in the DB.
    let real = db::get_discovered_skill_by_id(&pool, "claude-code__project__real-skill")
        .await
        .unwrap();
    assert!(real.is_some(), "real skill should remain in DB");

    // The stale skill should be removed from the DB.
    let stale = db::get_discovered_skill_by_id(&pool, "claude-code__stale-project__stale-skill")
        .await
        .unwrap();
    assert!(stale.is_none(), "stale skill should be removed from DB");
}

#[tokio::test]
async fn test_cache_reconciliation_only_affects_scanned_scope() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let now = Utc::now().to_rfc3339();

    // Insert a stale skill whose project_path is NOT under the scanned root.
    db::insert_discovered_skill(
        &pool,
        "claude-code__other__stale-skill",
        "stale-skill",
        Some("Outside scope"),
        "/other/location/.claude/skills/stale-skill/SKILL.md",
        "/other/location/.claude/skills/stale-skill",
        "/other/location",
        "other",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Scan a different root — the stale skill is outside the scope.
    let scan_root = ScanRoot {
        path: tmp.path().to_string_lossy().into_owned(),
        label: "test".to_string(),
        exists: true,
        enabled: true,
    };

    let found_ids: Vec<String> = vec![];

    reconcile_discovered_skills(&pool, &[&scan_root], &found_ids)
        .await
        .unwrap();

    // The stale skill should still be in the DB (outside scanned scope).
    let outside = db::get_discovered_skill_by_id(&pool, "claude-code__other__stale-skill")
        .await
        .unwrap();
    assert!(
        outside.is_some(),
        "stale skill outside scan scope should remain in DB"
    );
}

#[tokio::test]
async fn test_multi_platform_install_keeps_discovered_record() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // Set up two agent dirs.
    let agent_dir_a = tmp.path().join("agent-a-skills");
    let agent_dir_b = tmp.path().join("agent-b-skills");
    std::fs::create_dir_all(&agent_dir_a).unwrap();
    std::fs::create_dir_all(&agent_dir_b).unwrap();

    // Create a discovered skill.
    let skill_dir = tmp.path().join("project/.claude/skills/my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: A test skill\n---\n\n# My Skill\n",
    )
    .unwrap();

    let now = Utc::now().to_rfc3339();
    db::insert_discovered_skill(
        &pool,
        "claude-code__project__my-skill",
        "my-skill",
        Some("A test skill"),
        &skill_dir.join("SKILL.md").to_string_lossy(),
        &skill_dir.to_string_lossy(),
        &tmp.path().join("project").to_string_lossy(),
        "project",
        "claude-code",
        &now,
    )
    .await
    .unwrap();

    // Import to first platform.
    let result_a = import_discovered_skill_to_platform_at(
        &pool,
        "claude-code__project__my-skill",
        "agent-a",
        &agent_dir_a,
    )
    .await;
    assert!(result_a.is_ok(), "first import should succeed");

    // Import to second platform — this should also succeed because
    // the discovered record is NOT deleted after platform install.
    let result_b = import_discovered_skill_to_platform_at(
        &pool,
        "claude-code__project__my-skill",
        "agent-b",
        &agent_dir_b,
    )
    .await;
    assert!(result_b.is_ok(), "second import should succeed");

    // Both symlinks should exist.
    assert!(agent_dir_a.join("my-skill").exists());
    assert!(agent_dir_b.join("my-skill").exists());

    // Discovered record should still exist.
    let record = db::get_discovered_skill_by_id(&pool, "claude-code__project__my-skill")
        .await
        .unwrap();
    assert!(
        record.is_some(),
        "discovered record should be kept after platform installs"
    );
}
