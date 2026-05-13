//! Integration tests for super (services::scanner::*). cfg(test) only;
//! does not count toward the 800-line production-code budget.

#![cfg(test)]
#![allow(unused_imports)]

use super::*;
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

// ── Helpers ───────────────────────────────────────────────────────────────

/// Write a SKILL.md with the given content in `dir/<skill_name>/SKILL.md`.
fn create_skill_dir(parent: &Path, dir_name: &str, content: &str) -> std::path::PathBuf {
    let skill_dir = parent.join(dir_name);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    skill_dir
}

fn valid_skill_md(name: &str, description: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {}\n\nContent.\n",
        name, description, name
    )
}

fn skill_md_no_description(name: &str) -> String {
    format!("---\nname: {}\n---\n\n# {}\n", name, name)
}

fn write_claude_plugin_runtime(claude_root: &Path, enabled_plugins: &[(&str, &Path)]) {
    fs::create_dir_all(claude_root.join("plugins")).unwrap();

    let enabled_json = enabled_plugins
        .iter()
        .map(|(plugin_id, _)| (plugin_id.to_string(), serde_json::Value::Bool(true)))
        .collect::<serde_json::Map<_, _>>();
    fs::write(
        claude_root.join("settings.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "enabledPlugins": enabled_json,
        }))
        .unwrap(),
    )
    .unwrap();

    let installed_json = enabled_plugins
        .iter()
        .map(|(plugin_id, install_path)| {
            (
                plugin_id.to_string(),
                serde_json::json!([{
                    "scope": "user",
                    "installPath": install_path.to_string_lossy().to_string(),
                    "version": "test-version",
                    "installedAt": "2026-04-23T00:00:00Z",
                    "lastUpdated": "2026-04-23T00:00:00Z"
                }]),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    fs::write(
        claude_root.join("plugins/installed_plugins.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 2,
            "plugins": installed_json,
        }))
        .unwrap(),
    )
    .unwrap();
}

// ── parse_skill_md ────────────────────────────────────────────────────────

#[test]
fn test_parse_skill_md_valid() {
    let tmp = TempDir::new().unwrap();
    let md_path = tmp.path().join("SKILL.md");
    fs::write(&md_path, valid_skill_md("My Skill", "A great skill")).unwrap();

    let info = parse_skill_md(&md_path).expect("should parse valid SKILL.md");
    assert_eq!(info.name, "My Skill");
    assert_eq!(info.description.as_deref(), Some("A great skill"));
}

#[test]
fn test_parse_skill_md_no_description() {
    let tmp = TempDir::new().unwrap();
    let md_path = tmp.path().join("SKILL.md");
    fs::write(&md_path, skill_md_no_description("Minimal Skill")).unwrap();

    let info = parse_skill_md(&md_path).expect("should parse frontmatter without description");
    assert_eq!(info.name, "Minimal Skill");
    assert!(info.description.is_none());
}

#[test]
fn test_parse_skill_md_missing_name() {
    let tmp = TempDir::new().unwrap();
    let md_path = tmp.path().join("SKILL.md");
    fs::write(
        &md_path,
        "---\ndescription: Has description but no name\n---\n\nContent.",
    )
    .unwrap();

    let result = parse_skill_md(&md_path);
    assert!(result.is_none(), "should return None when name is missing");
}

#[test]
fn test_parse_skill_md_no_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let md_path = tmp.path().join("SKILL.md");
    fs::write(&md_path, "# Just a Markdown file\n\nNo frontmatter here.").unwrap();

    let result = parse_skill_md(&md_path);
    assert!(
        result.is_none(),
        "should return None when frontmatter is absent"
    );
}

#[test]
fn test_parse_skill_md_empty_file() {
    let tmp = TempDir::new().unwrap();
    let md_path = tmp.path().join("SKILL.md");
    fs::write(&md_path, "").unwrap();

    let result = parse_skill_md(&md_path);
    assert!(result.is_none(), "should return None for an empty file");
}

#[test]
fn test_parse_skill_md_file_not_found() {
    let result = parse_skill_md(Path::new("/nonexistent/path/SKILL.md"));
    assert!(result.is_none(), "should return None for a missing file");
}

#[test]
fn test_parse_skill_md_multiline_description() {
    let tmp = TempDir::new().unwrap();
    let md_path = tmp.path().join("SKILL.md");
    // YAML block scalar for multiline strings
    let content = "---\nname: Block Skill\ndescription: \"Line one. Line two.\"\n---\n\nBody.\n";
    fs::write(&md_path, content).unwrap();

    let info = parse_skill_md(&md_path).expect("should parse multiline description");
    assert_eq!(info.name, "Block Skill");
    assert!(info.description.is_some());
}

// ── detect_link_type ──────────────────────────────────────────────────────

#[test]
fn test_detect_link_type_real_dir_platform() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("real-skill");
    fs::create_dir_all(&dir).unwrap();

    let (kind, target) = detect_link_type(&dir, false);
    assert_eq!(
        kind, "copy",
        "real dir in platform context should be 'copy'"
    );
    assert!(target.is_none());
}

#[test]
fn test_detect_link_type_real_dir_central() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("central-skill");
    fs::create_dir_all(&dir).unwrap();

    let (kind, target) = detect_link_type(&dir, true);
    assert_eq!(
        kind, "native",
        "real dir in central context should be 'native'"
    );
    assert!(target.is_none());
}

#[cfg(unix)]
#[test]
fn test_detect_link_type_symlink() {
    let tmp = TempDir::new().unwrap();

    // Create a real target directory
    let target_dir = tmp.path().join("target-skill");
    fs::create_dir_all(&target_dir).unwrap();

    // Create a symlink pointing to it
    let link_path = tmp.path().join("linked-skill");
    symlink(&target_dir, &link_path).expect("failed to create symlink");

    let (kind, sym_target) = detect_link_type(&link_path, false);
    assert_eq!(kind, "symlink");
    assert!(
        sym_target.is_some(),
        "symlink target path should be returned"
    );
}

#[cfg(unix)]
#[test]
fn test_detect_link_type_symlink_is_symlink_regardless_of_is_central() {
    let tmp = TempDir::new().unwrap();
    let target_dir = tmp.path().join("target");
    fs::create_dir_all(&target_dir).unwrap();
    let link_path = tmp.path().join("link");
    symlink(&target_dir, &link_path).unwrap();

    // Even in central context, a symlink is a symlink
    let (kind, _) = detect_link_type(&link_path, true);
    assert_eq!(kind, "symlink");
}

// ── scan_directory ────────────────────────────────────────────────────────

#[test]
fn test_scan_directory_empty() {
    let tmp = TempDir::new().unwrap();
    let result = scan_directory(tmp.path(), false);
    assert!(result.is_empty(), "empty directory should yield no skills");
}

#[test]
fn test_scan_directory_finds_single_skill() {
    let tmp = TempDir::new().unwrap();
    create_skill_dir(
        tmp.path(),
        "cool-skill",
        &valid_skill_md("Cool Skill", "Does cool things"),
    );

    let skills = scan_directory(tmp.path(), false);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "cool-skill");
    assert_eq!(skills[0].name, "Cool Skill");
    assert_eq!(skills[0].description.as_deref(), Some("Does cool things"));
}

#[test]
fn test_scan_directory_finds_multiple_skills() {
    let tmp = TempDir::new().unwrap();
    create_skill_dir(tmp.path(), "skill-a", &valid_skill_md("Skill A", "Alpha"));
    create_skill_dir(tmp.path(), "skill-b", &valid_skill_md("Skill B", "Beta"));
    create_skill_dir(tmp.path(), "skill-c", &valid_skill_md("Skill C", "Gamma"));

    let mut skills = scan_directory(tmp.path(), false);
    skills.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(skills.len(), 3);
    assert_eq!(skills[0].id, "skill-a");
    assert_eq!(skills[1].id, "skill-b");
    assert_eq!(skills[2].id, "skill-c");
}

#[test]
fn test_scan_directory_skips_dirs_without_skill_md() {
    let tmp = TempDir::new().unwrap();
    create_skill_dir(tmp.path(), "valid-skill", &valid_skill_md("Valid", "OK"));

    // A directory without SKILL.md should be ignored
    fs::create_dir_all(tmp.path().join("no-skill-md")).unwrap();

    let skills = scan_directory(tmp.path(), false);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "valid-skill");
}

#[test]
fn test_scan_directory_skips_invalid_frontmatter() {
    let tmp = TempDir::new().unwrap();
    create_skill_dir(tmp.path(), "valid-skill", &valid_skill_md("Valid", "OK"));
    create_skill_dir(
        tmp.path(),
        "invalid-skill",
        "# No frontmatter here\n\nJust content.",
    );

    let skills = scan_directory(tmp.path(), false);
    assert_eq!(
        skills.len(),
        1,
        "skill with invalid frontmatter should be skipped"
    );
    assert_eq!(skills[0].id, "valid-skill");
}

#[test]
fn test_scan_directory_skips_regular_files() {
    let tmp = TempDir::new().unwrap();
    // A plain file at the top level should be ignored
    fs::write(tmp.path().join("README.md"), "# readme").unwrap();
    create_skill_dir(tmp.path(), "real-skill", &valid_skill_md("Real", "desc"));

    let skills = scan_directory(tmp.path(), false);
    assert_eq!(skills.len(), 1);
}

#[test]
fn test_scan_directory_is_not_recursive() {
    let tmp = TempDir::new().unwrap();
    // Create a nested structure (depth 2); only top-level subdirs should be found
    let deep_dir = tmp.path().join("outer").join("inner");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(
        deep_dir.join("SKILL.md"),
        valid_skill_md("Deep Skill", "too deep"),
    )
    .unwrap();

    let skills = scan_directory(tmp.path(), false);
    assert!(
        skills.is_empty(),
        "scan_directory should not descend more than one level"
    );
}

#[test]
fn test_scan_directory_central_dir_marks_native() {
    let tmp = TempDir::new().unwrap();
    create_skill_dir(
        tmp.path(),
        "central-skill",
        &valid_skill_md("Central", "desc"),
    );

    let skills = scan_directory(tmp.path(), true /* is_central */);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].link_type, "native");
    assert!(skills[0].is_central);
}

#[cfg(unix)]
#[test]
fn test_scan_directory_detects_symlinked_skill() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("agent-skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Create a real skill in another location (central-like)
    let central_dir = tmp.path().join("central");
    create_skill_dir(
        &central_dir,
        "my-skill",
        &valid_skill_md("My Skill", "desc"),
    );

    // Symlink it into the agent skills dir
    let link = skills_dir.join("my-skill");
    symlink(central_dir.join("my-skill"), &link).unwrap();

    let skills = scan_directory(&skills_dir, false);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].link_type, "symlink");
    assert!(skills[0].symlink_target.is_some());
}

#[test]
fn test_scan_directory_nonexistent_dir_returns_empty() {
    let result = scan_directory(Path::new("/nonexistent/path/skills"), false);
    assert!(result.is_empty());
}

// ── scan_all_skills_impl ──────────────────────────────────────────────────

async fn setup_test_db() -> DbPool {
    use crate::db;
    use sqlx::SqlitePool;
    let pool = SqlitePool::connect(":memory:").await.expect("in-memory DB");
    db::init_database(&pool).await.expect("init");
    pool
}

#[tokio::test]
async fn test_scan_all_skills_impl_empty_dirs() {
    use sqlx::SqlitePool;

    // Build a pool with tables but no seeded agents so the test is
    // isolated from whatever the user has installed on their machine.
    let pool = SqlitePool::connect(":memory:").await.expect("in-memory DB");
    db::init_database(&pool).await.expect("init");
    // Remove all seeded agents so the test is isolated from whatever the
    // user has installed on their machine.
    sqlx::query("DELETE FROM agents")
        .execute(&pool)
        .await
        .expect("delete agents");
    // Also clear the builtin scan directories that init_database seeds,
    // so the custom-scan-dir loop has nothing to scan either.
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .expect("delete scan_directories");

    // Add one agent whose skills dir definitely does not exist.
    let dummy_agent = db::Agent {
        id: "empty-agent".to_string(),
        display_name: "Empty Agent".to_string(),
        category: "coding".to_string(),
        global_skills_dir: "/nonexistent/path/skills".to_string(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &dummy_agent)
        .await
        .expect("insert dummy agent");

    let result = scan_all_skills_impl(&pool).await;
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.total_skills, 0);
    assert_eq!(r.agents_scanned, 1);
    assert_eq!(r.skills_by_agent.get("empty-agent").copied(), Some(0));
}

#[tokio::test]
async fn test_scan_all_skills_impl_persists_skills() {
    use crate::db;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // Add a custom agent pointing to our temp directory
    let test_agent = db::Agent {
        id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        category: "coding".to_string(),
        global_skills_dir: tmp.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &test_agent).await.unwrap();

    // Create skills in the temp directory
    create_skill_dir(
        tmp.path(),
        "alpha-skill",
        &valid_skill_md("Alpha Skill", "First skill"),
    );
    create_skill_dir(
        tmp.path(),
        "beta-skill",
        &valid_skill_md("Beta Skill", "Second skill"),
    );

    let result = scan_all_skills_impl(&pool).await.unwrap();

    // Test agent should have 2 skills
    assert_eq!(result.skills_by_agent.get("test-agent").copied(), Some(2));

    // Skills should be in the DB
    let skills_in_db = db::get_skills_by_agent(&pool, "test-agent").await.unwrap();
    assert_eq!(skills_in_db.len(), 2);
}

#[tokio::test]
async fn test_scan_all_skills_impl_central_skills_are_marked() {
    use crate::db;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // Override the "central" agent's dir with our temp dir by inserting a
    // custom agent with id "central-test".
    let central_agent = db::Agent {
        id: "central-test".to_string(),
        display_name: "Central Test".to_string(),
        category: "central".to_string(),
        global_skills_dir: tmp.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &central_agent)
        .await
        .unwrap();

    create_skill_dir(
        tmp.path(),
        "canon-skill",
        &valid_skill_md("Canon Skill", "Canonical"),
    );

    scan_all_skills_impl(&pool).await.unwrap();

    let skill = db::get_skill_by_id(&pool, "canon-skill").await.unwrap();
    assert!(skill.is_some());
    assert!(
        skill.unwrap().is_central,
        "skills scanned from a category=central agent must be marked central"
    );

    let central_skills = db::get_central_skills(&pool).await.unwrap();
    assert!(
        central_skills.iter().any(|skill| skill.id == "canon-skill"),
        "scan_all_skills should make the scanned central skill visible to Central Skills queries"
    );
}

#[tokio::test]
async fn test_scan_all_skills_impl_with_custom_scan_directory() {
    use crate::db;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // Add a custom scan directory
    db::add_scan_directory(&pool, tmp.path().to_str().unwrap(), Some("Test Dir"))
        .await
        .unwrap();

    create_skill_dir(
        tmp.path(),
        "custom-dir-skill",
        &valid_skill_md("Custom Dir Skill", "From custom dir"),
    );

    let result = scan_all_skills_impl(&pool).await.unwrap();
    // Skill should be in total count (custom dirs contribute to total)
    assert!(result.total_skills > 0);

    // Skill should be in the DB
    let skill = db::get_skill_by_id(&pool, "custom-dir-skill")
        .await
        .unwrap();
    assert!(skill.is_some());
}

#[tokio::test]
async fn test_scan_all_skills_impl_reuses_shared_directory_for_codex_and_central() {
    use crate::db;

    let shared_dir = TempDir::new().unwrap();
    let pool = setup_test_db().await;
    let shared_path = shared_dir.path().to_string_lossy().into_owned();
    let missing_path = shared_dir
        .path()
        .join("missing")
        .to_string_lossy()
        .into_owned();

    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ?")
        .bind(&missing_path)
        .execute(&pool)
        .await
        .unwrap();

    for agent_id in crate::db::UNIVERSAL_AGENT_IDS
        .iter()
        .copied()
        .chain(std::iter::once("central"))
    {
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = ?")
            .bind(&shared_path)
            .bind(agent_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    create_skill_dir(
        shared_dir.path(),
        "shared-skill",
        &valid_skill_md("Shared Skill", "Shared"),
    );

    let result = scan_all_skills_impl(&pool).await.unwrap();
    assert_eq!(result.total_skills, 1);
    for agent_id in crate::db::UNIVERSAL_AGENT_IDS {
        assert_eq!(result.skills_by_agent.get(agent_id).copied(), Some(1));
    }
    assert_eq!(result.skills_by_agent.get("central").copied(), Some(1));

    let installations = db::get_skill_installations(&pool, "shared-skill")
        .await
        .unwrap();
    assert_eq!(
        installations.len(),
        crate::db::UNIVERSAL_AGENT_IDS.len() + 1,
        "shared skill should be mapped to universal agents and central"
    );
    assert!(
        installations
            .iter()
            .any(|installation| installation.agent_id == "codex"
                && installation.link_type == "native"),
        "codex shared-root installation should be native"
    );
    assert!(
        installations
            .iter()
            .any(|installation| installation.agent_id == "central"),
        "central installation should be present"
    );
}

#[tokio::test]
async fn test_scan_all_skills_impl_returns_per_agent_counts() {
    use crate::db;

    let tmp_a = TempDir::new().unwrap();
    let tmp_b = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let agent_a = db::Agent {
        id: "agent-a".to_string(),
        display_name: "Agent A".to_string(),
        category: "coding".to_string(),
        global_skills_dir: tmp_a.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    let agent_b = db::Agent {
        id: "agent-b".to_string(),
        display_name: "Agent B".to_string(),
        category: "coding".to_string(),
        global_skills_dir: tmp_b.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &agent_a).await.unwrap();
    db::insert_custom_agent(&pool, &agent_b).await.unwrap();

    create_skill_dir(tmp_a.path(), "skill-x", &valid_skill_md("Skill X", "In A"));
    create_skill_dir(
        tmp_a.path(),
        "skill-y",
        &valid_skill_md("Skill Y", "In A too"),
    );
    create_skill_dir(tmp_b.path(), "skill-z", &valid_skill_md("Skill Z", "In B"));

    let result = scan_all_skills_impl(&pool).await.unwrap();

    assert_eq!(result.skills_by_agent.get("agent-a").copied(), Some(2));
    assert_eq!(result.skills_by_agent.get("agent-b").copied(), Some(1));
}

#[tokio::test]
async fn test_scan_all_skills_impl_claude_scans_user_and_multiple_plugin_roots() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents WHERE id != 'claude-code'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let claude_root = tmp.path().join(".claude");
    let user_root = claude_root.join("skills");
    let plugin_a_root = claude_root.join("plugins/cache/publisher-a/plugin-a/1.0.0");
    let plugin_b_root = claude_root.join("plugins/cache/publisher-b/plugin-b/2.0.0");
    let plugin_a_skill_root = plugin_a_root.join("skills");
    let plugin_b_skill_root = plugin_b_root.join(".claude").join("skills");

    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&plugin_a_skill_root).unwrap();
    fs::create_dir_all(&plugin_b_skill_root).unwrap();

    create_skill_dir(
        &user_root,
        "user-skill",
        &valid_skill_md("User Skill", "From ~/.claude/skills"),
    );
    create_skill_dir(
        &plugin_a_skill_root,
        "plugin-a-skill",
        &valid_skill_md("plugin-a:skill", "From plugin A"),
    );
    create_skill_dir(
        &plugin_b_skill_root,
        "plugin-b-skill",
        &valid_skill_md("plugin-b:skill", "From plugin B"),
    );
    write_claude_plugin_runtime(
        &claude_root,
        &[
            ("plugin-a@publisher-a", &plugin_a_root),
            ("plugin-b@publisher-b", &plugin_b_root),
        ],
    );

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(user_root.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    let result = scan_all_skills_impl(&pool).await.unwrap();
    assert_eq!(result.agents_scanned, 1);
    assert_eq!(result.skills_by_agent.get("claude-code").copied(), Some(3));

    let mut skills = db::get_skills_by_agent(&pool, "claude-code").await.unwrap();
    skills.sort_by(|a, b| a.id.cmp(&b.id));
    let ids: Vec<&str> = skills.iter().map(|skill| skill.id.as_str()).collect();
    assert_eq!(ids, vec!["plugin-a-skill", "plugin-b-skill", "user-skill"]);

    let observations = db::get_agent_skill_observations(&pool, "claude-code")
        .await
        .unwrap();
    assert_eq!(observations.len(), 3);

    let plugin_a_rows: Vec<_> = observations
        .iter()
        .filter(|row| row.skill_id == "plugin-a-skill")
        .collect();
    assert_eq!(plugin_a_rows.len(), 1);
    assert_eq!(plugin_a_rows[0].source_kind, "plugin");
    assert_eq!(
        plugin_a_rows[0].dir_path,
        plugin_a_skill_root.join("plugin-a-skill").to_string_lossy()
    );
    assert_eq!(
        plugin_a_rows[0].source_root,
        plugin_a_root.to_string_lossy()
    );

    let plugin_b_rows: Vec<_> = observations
        .iter()
        .filter(|row| row.skill_id == "plugin-b-skill")
        .collect();
    assert_eq!(plugin_b_rows.len(), 1);
    assert_eq!(plugin_b_rows[0].source_kind, "plugin");
    assert_eq!(
        plugin_b_rows[0].dir_path,
        plugin_b_skill_root.join("plugin-b-skill").to_string_lossy()
    );
    assert_eq!(
        plugin_b_rows[0].source_root,
        plugin_b_root.to_string_lossy()
    );

    let plugin_a_installations = db::get_skill_installations(&pool, "plugin-a-skill")
        .await
        .unwrap();
    assert!(
        plugin_a_installations.is_empty(),
        "plugin rows should not create install-state records"
    );
}

#[tokio::test]
async fn test_scan_all_skills_impl_claude_duplicate_rows_stay_distinct_without_install_pollution() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents WHERE id != 'claude-code'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let claude_root = tmp.path().join(".claude");
    let user_root = claude_root.join("skills");
    let plugin_root = claude_root.join("plugins/cache/publisher/shared-plugin/1.0.0");
    let plugin_skill_root = plugin_root.join("skills");
    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&plugin_skill_root).unwrap();

    create_skill_dir(
        &user_root,
        "shared-skill",
        &valid_skill_md("Shared Skill", "User copy"),
    );
    create_skill_dir(
        &plugin_skill_root,
        "shared-skill",
        &valid_skill_md("shared-plugin:shared-skill", "Plugin copy"),
    );
    write_claude_plugin_runtime(&claude_root, &[("shared-plugin@publisher", &plugin_root)]);

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(user_root.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    scan_all_skills_impl(&pool).await.unwrap();

    let rows = db::get_agent_skill_observations(&pool, "claude-code")
        .await
        .unwrap();
    let shared_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.skill_id == "shared-skill")
        .collect();
    assert_eq!(
        shared_rows.len(),
        2,
        "user and plugin copies should remain distinct observation rows"
    );
    assert_ne!(shared_rows[0].row_id, shared_rows[1].row_id);

    let installs = db::get_skill_installations(&pool, "shared-skill")
        .await
        .unwrap();
    assert_eq!(
        installs.len(),
        1,
        "only the user copy should remain manageable"
    );
    assert_eq!(
        installs[0].installed_path,
        user_root.join("shared-skill").to_string_lossy()
    );

    let stored_skill = db::get_skill_by_id(&pool, "shared-skill")
        .await
        .unwrap()
        .expect("user copy should still back the logical skill row");
    assert_eq!(
        stored_skill.file_path,
        user_root
            .join("shared-skill")
            .join("SKILL.md")
            .to_string_lossy()
    );
}

#[tokio::test]
async fn test_scan_all_skills_impl_claude_scans_plugins_even_without_user_root() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents WHERE id != 'claude-code'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let claude_root = tmp.path().join(".claude");
    let user_root = claude_root.join("skills");
    let plugin_a_root = claude_root.join("plugins/cache/publisher-a/plugin-a/1.0.0");
    let plugin_b_root = claude_root.join("plugins/cache/publisher-b/plugin-b/2.0.0");
    let plugin_a_skill_root = plugin_a_root.join("skills");
    let plugin_b_skill_root = plugin_b_root.join(".claude").join("skills");

    fs::create_dir_all(&plugin_a_skill_root).unwrap();
    fs::create_dir_all(&plugin_b_skill_root).unwrap();

    create_skill_dir(
        &plugin_a_skill_root,
        "plugin-a-skill",
        &valid_skill_md("plugin-a:skill", "From plugin A"),
    );
    create_skill_dir(
        &plugin_b_skill_root,
        "plugin-b-skill",
        &valid_skill_md("plugin-b:skill", "From plugin B"),
    );
    write_claude_plugin_runtime(
        &claude_root,
        &[
            ("plugin-a@publisher-a", &plugin_a_root),
            ("plugin-b@publisher-b", &plugin_b_root),
        ],
    );

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(user_root.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    let result = scan_all_skills_impl(&pool).await.unwrap();
    assert_eq!(result.skills_by_agent.get("claude-code").copied(), Some(2));

    let detected = db::get_agent_by_id(&pool, "claude-code")
        .await
        .unwrap()
        .unwrap();
    assert!(
        detected.is_detected,
        "claude-code should remain detected when only plugin roots exist"
    );
}

#[tokio::test]
async fn test_scan_all_skills_impl_non_claude_agents_ignore_claude_plugins() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let claude_like_root = tmp.path().join(".claude");
    let user_root = claude_like_root.join("skills");
    let plugin_a_root = claude_like_root.join("plugins/cache/publisher-a/plugin-a/1.0.0");
    let plugin_b_root = claude_like_root.join("plugins/cache/publisher-b/plugin-b/2.0.0");
    let plugin_a_skill_root = plugin_a_root.join("skills");
    let plugin_b_skill_root = plugin_b_root.join(".claude").join("skills");

    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&plugin_a_skill_root).unwrap();
    fs::create_dir_all(&plugin_b_skill_root).unwrap();

    create_skill_dir(
        &user_root,
        "user-skill",
        &valid_skill_md("User Skill", "From primary root"),
    );
    create_skill_dir(
        &plugin_a_skill_root,
        "plugin-a-skill",
        &valid_skill_md("plugin-a:skill", "From plugin A"),
    );
    create_skill_dir(
        &plugin_b_skill_root,
        "plugin-b-skill",
        &valid_skill_md("plugin-b:skill", "From plugin B"),
    );
    write_claude_plugin_runtime(
        &claude_like_root,
        &[
            ("plugin-a@publisher-a", &plugin_a_root),
            ("plugin-b@publisher-b", &plugin_b_root),
        ],
    );

    let agent = db::Agent {
        id: "not-claude".to_string(),
        display_name: "Not Claude".to_string(),
        category: "coding".to_string(),
        global_skills_dir: user_root.to_string_lossy().to_string(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &agent).await.unwrap();

    let result = scan_all_skills_impl(&pool).await.unwrap();
    assert_eq!(result.agents_scanned, 1);
    assert_eq!(result.skills_by_agent.get("not-claude").copied(), Some(1));

    let skills = db::get_skills_by_agent(&pool, "not-claude").await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "user-skill");
}

#[tokio::test]
#[ignore = "manual isolated-home sanity check"]
async fn test_scan_all_skills_impl_claude_fixture_home_sanity() {
    let fixture_home = Path::new("/tmp/skillport-test-fixtures/claude-multi-source");
    if fixture_home.exists() {
        fs::remove_dir_all(fixture_home).unwrap();
    }
    fs::create_dir_all(fixture_home).unwrap();

    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents WHERE id != 'claude-code'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let user_root = fixture_home.join(".claude/skills");
    let plugin_a_root = fixture_home.join(".claude/plugins/cache/publisher-a/plugin-a/1.0.0");
    let plugin_b_root = fixture_home.join(".claude/plugins/cache/publisher-b/plugin-b/2.0.0");
    let plugin_a_skill_root = plugin_a_root.join("skills");
    let plugin_b_skill_root = plugin_b_root.join(".claude").join("skills");

    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&plugin_a_skill_root).unwrap();
    fs::create_dir_all(&plugin_b_skill_root).unwrap();

    create_skill_dir(
        &user_root,
        "fixture-user-skill",
        &valid_skill_md("Fixture User Skill", "From fixture user root"),
    );
    create_skill_dir(
        &plugin_a_skill_root,
        "fixture-plugin-a-skill",
        &valid_skill_md("plugin-a:fixture", "From fixture plugin A"),
    );
    create_skill_dir(
        &plugin_b_skill_root,
        "fixture-plugin-b-skill",
        &valid_skill_md("plugin-b:fixture", "From fixture plugin B"),
    );
    write_claude_plugin_runtime(
        &fixture_home.join(".claude"),
        &[
            ("plugin-a@publisher-a", &plugin_a_root),
            ("plugin-b@publisher-b", &plugin_b_root),
        ],
    );

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(user_root.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    let result = scan_all_skills_impl(&pool).await.unwrap();
    assert_eq!(result.skills_by_agent.get("claude-code").copied(), Some(3));
}

#[tokio::test]
async fn test_scan_all_skills_impl_claude_rescan_drops_stale_plugin_duplicate_only() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents WHERE id != 'claude-code'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let claude_root = tmp.path().join(".claude");
    let user_root = claude_root.join("skills");
    let plugin_root = claude_root.join("plugins/cache/publisher/shared-plugin/1.0.0");
    let plugin_skill_root = plugin_root.join("skills");
    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&plugin_skill_root).unwrap();

    let plugin_skill_dir = create_skill_dir(
        &plugin_skill_root,
        "shared-skill",
        &valid_skill_md("shared-plugin:shared-skill", "Plugin copy"),
    );
    create_skill_dir(
        &user_root,
        "shared-skill",
        &valid_skill_md("Shared Skill", "User copy"),
    );
    write_claude_plugin_runtime(&claude_root, &[("shared-plugin@publisher", &plugin_root)]);

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(user_root.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    scan_all_skills_impl(&pool).await.unwrap();
    fs::remove_dir_all(&plugin_skill_dir).unwrap();
    scan_all_skills_impl(&pool).await.unwrap();

    let rows = db::get_agent_skill_observations(&pool, "claude-code")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1, "only the user observation should remain");
    assert_eq!(rows[0].source_kind, "user");

    let installs = db::get_skill_installations(&pool, "shared-skill")
        .await
        .unwrap();
    assert_eq!(
        installs.len(),
        1,
        "user install state should survive plugin cleanup"
    );
}

#[tokio::test]
async fn test_scan_all_skills_impl_claude_plugin_survives_when_user_duplicate_is_removed() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("DELETE FROM agents WHERE id != 'claude-code'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scan_directories")
        .execute(&pool)
        .await
        .unwrap();

    let claude_root = tmp.path().join(".claude");
    let user_root = claude_root.join("skills");
    let plugin_root = claude_root.join("plugins/cache/publisher/shared-plugin/1.0.0");
    let plugin_skill_root = plugin_root.join("skills");
    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&plugin_skill_root).unwrap();

    let user_skill_dir = create_skill_dir(
        &user_root,
        "shared-skill",
        &valid_skill_md("Shared Skill", "User copy"),
    );
    create_skill_dir(
        &plugin_skill_root,
        "shared-skill",
        &valid_skill_md("shared-plugin:shared-skill", "Plugin copy"),
    );
    write_claude_plugin_runtime(&claude_root, &[("shared-plugin@publisher", &plugin_root)]);

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(user_root.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    scan_all_skills_impl(&pool).await.unwrap();
    fs::remove_dir_all(&user_skill_dir).unwrap();
    scan_all_skills_impl(&pool).await.unwrap();

    let rows = db::get_agent_skill_observations(&pool, "claude-code")
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "plugin observation should survive even after the user duplicate disappears"
    );
    assert_eq!(rows[0].source_kind, "plugin");

    let installs = db::get_skill_installations(&pool, "shared-skill")
        .await
        .unwrap();
    assert!(
        installs.is_empty(),
        "plugin observations must not keep stale Claude install-state rows alive"
    );

    let skill = db::get_skill_by_id(&pool, "shared-skill").await.unwrap();
    assert!(
        skill.is_none(),
        "plugin observations should not keep a stale manageable skill row alive"
    );
}

// ── Regression: Bug 1 — installed_path must be the skill directory ────────

/// installed_path should point to the skill directory, not to the SKILL.md
/// file inside it.
#[tokio::test]
async fn test_installed_path_is_skill_directory_not_skill_md() {
    use crate::db;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let test_agent = db::Agent {
        id: "path-agent".to_string(),
        display_name: "Path Agent".to_string(),
        category: "coding".to_string(),
        global_skills_dir: tmp.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &test_agent).await.unwrap();

    let skill_dir = create_skill_dir(tmp.path(), "my-skill", &valid_skill_md("My Skill", "desc"));

    scan_all_skills_impl(&pool).await.unwrap();

    let installations = db::get_skill_installations(&pool, "my-skill")
        .await
        .unwrap();
    assert_eq!(
        installations.len(),
        1,
        "Expected exactly one installation record"
    );

    let inst = &installations[0];
    // installed_path must NOT be the SKILL.md file path.
    assert!(
        !inst.installed_path.ends_with("SKILL.md"),
        "installed_path should not point to the SKILL.md file; got: {}",
        inst.installed_path
    );
    // installed_path must equal the skill directory path.
    assert_eq!(
        inst.installed_path,
        skill_dir.to_string_lossy().as_ref(),
        "installed_path should be the skill directory, not the SKILL.md inside it"
    );
}

// ── Regression: Bug 2 — rescan removes stale skills from DB ──────────────

/// After removing a skill from disk and rescanning, the corresponding rows
/// must no longer appear in skills or skill_installations queries.
#[tokio::test]
async fn test_rescan_removes_deleted_skills_from_db() {
    use crate::db;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let test_agent = db::Agent {
        id: "stale-agent".to_string(),
        display_name: "Stale Agent".to_string(),
        category: "coding".to_string(),
        global_skills_dir: tmp.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &test_agent).await.unwrap();

    // Create two skills on disk.
    create_skill_dir(
        tmp.path(),
        "skill-keep",
        &valid_skill_md("Keep Skill", "stays"),
    );
    create_skill_dir(
        tmp.path(),
        "skill-remove",
        &valid_skill_md("Remove Skill", "will be deleted"),
    );

    // First scan — both skills should be persisted.
    scan_all_skills_impl(&pool).await.unwrap();
    let skills_first = db::get_skills_by_agent(&pool, "stale-agent").await.unwrap();
    assert_eq!(
        skills_first.len(),
        2,
        "Both skills should be in DB after first scan"
    );

    // Remove "skill-remove" from disk.
    fs::remove_dir_all(tmp.path().join("skill-remove")).unwrap();

    // Second scan — "skill-remove" must disappear from the DB.
    scan_all_skills_impl(&pool).await.unwrap();

    let skills_after = db::get_skills_by_agent(&pool, "stale-agent").await.unwrap();
    assert_eq!(
        skills_after.len(),
        1,
        "Only one skill should remain after rescan"
    );
    assert_eq!(
        skills_after[0].id, "skill-keep",
        "The surviving skill should be 'skill-keep'"
    );

    // The deleted skill must also be gone from the skills table.
    let stale_skill = db::get_skill_by_id(&pool, "skill-remove").await.unwrap();
    assert!(
        stale_skill.is_none(),
        "skill-remove should be removed from the skills table after rescan"
    );

    // No orphaned installation record should remain.
    let stale_inst = db::get_skill_installations(&pool, "skill-remove")
        .await
        .unwrap();
    assert!(
        stale_inst.is_empty(),
        "skill-remove's installation record should be removed after rescan"
    );
}

// ── Regression: is_central preserved when codex shares the central dir ───

/// When a central-category agent and a coding-category agent (codex) both
/// point to the same directory, skills from that directory must end up with
/// `is_central = true` after scanning — regardless of scan order.
///
/// Historically this failed because:
///  1. The scan used `agent.id == "central"` (not `agent.category`) to set
///     `is_central`, so the codex agent always cleared the flag.
///  2. Even after fixing the flag, the `INSERT OR REPLACE` would overwrite
///     `is_central = true` with `false` when codex was processed last.
#[tokio::test]
async fn test_is_central_preserved_when_shared_with_coding_agent() {
    use crate::db;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // Insert a central-category agent pointing to the shared temp directory.
    // Use "AA Central Test" as the display_name so it sorts BEFORE "ZZ Codex Test"
    // (ORDER BY display_name ASC) ensuring the central scan runs first.
    let central_agent = db::Agent {
        id: "aa-central-test".to_string(),
        display_name: "AA Central Test".to_string(),
        category: "central".to_string(),
        global_skills_dir: tmp.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    // Insert a coding-category agent pointing to the SAME temp directory,
    // sorted AFTER the central agent so it is processed last (worst case).
    let coding_agent = db::Agent {
        id: "zz-codex-test".to_string(),
        display_name: "ZZ Codex Test".to_string(),
        category: "coding".to_string(),
        global_skills_dir: tmp.path().to_string_lossy().into_owned(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    db::insert_custom_agent(&pool, &central_agent)
        .await
        .unwrap();
    db::insert_custom_agent(&pool, &coding_agent).await.unwrap();

    // Place one skill in the shared directory.
    create_skill_dir(
        tmp.path(),
        "shared-skill",
        &valid_skill_md("Shared Skill", "desc"),
    );

    // Run the full scan. The coding agent is processed AFTER the central agent
    // (due to display_name ordering), which is the failure scenario for the bug.
    scan_all_skills_impl(&pool).await.unwrap();

    // The skill must still be marked as central even though the coding agent
    // scanned the same directory afterwards.
    let skill = db::get_skill_by_id(&pool, "shared-skill")
        .await
        .unwrap()
        .expect("shared-skill must be in the DB");
    assert!(
        skill.is_central,
        "skill should remain is_central=true even when a coding agent \
         scans the same directory after the central agent"
    );
}
