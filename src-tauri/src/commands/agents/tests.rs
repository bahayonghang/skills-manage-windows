use super::*;
use crate::db;
use sqlx::SqlitePool;
use std::fs;
use tempfile::TempDir;

const BUILTIN_AGENT_COUNT: usize = 35;

async fn setup_test_db() -> DbPool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

#[test]
fn test_is_detected_existing_dir() {
    let tmp = TempDir::new().unwrap();
    assert!(
        is_agent_detected(tmp.path().to_str().unwrap()),
        "existing directory should be detected"
    );
}

#[test]
fn test_is_detected_existing_parent() {
    let tmp = TempDir::new().unwrap();
    let nonexistent_skills = tmp.path().join("skills");
    assert!(
        is_agent_detected(nonexistent_skills.to_str().unwrap()),
        "should be detected when parent dir exists"
    );
}

#[test]
fn test_is_detected_nonexistent_path() {
    assert!(
        !is_agent_detected("/nonexistent/path/that/does/not/exist/skills"),
        "should not be detected when parent does not exist"
    );
}

#[tokio::test]
async fn test_list_platform_paths_resolves_local_paths() {
    let pool = setup_test_db().await;
    let tmp = TempDir::new().unwrap();
    let global_dir = tmp.path().join("custom-agent").join("skills");

    let config = CustomAgentConfig {
        id: Some("path-agent".to_string()),
        display_name: "Path Agent".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: global_dir.to_string_lossy().into_owned(),
    };
    add_custom_agent_impl(&pool, config).await.unwrap();

    let paths = list_platform_paths_impl(&pool, None).await.unwrap();
    let custom = paths.get("path-agent").unwrap();
    assert_eq!(custom.global_skills_dir, global_dir.to_string_lossy());
    assert_eq!(custom.project_skills_dir, None);

    let claude = paths.get("claude-code").unwrap();
    assert!(
        claude.global_skills_dir.ends_with(".claude\\skills")
            || claude.global_skills_dir.ends_with(".claude/skills")
    );
    assert_eq!(claude.project_skills_dir.as_deref(), Some(".claude/skills"));
}

#[tokio::test]
async fn test_list_platform_paths_resolves_remote_paths() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database_for_remote_home(&pool, "/home/alice")
        .await
        .unwrap();

    let paths = list_platform_paths_impl(&pool, Some("/home/alice"))
        .await
        .unwrap();
    let claude = paths.get("claude-code").unwrap();

    assert_eq!(claude.global_skills_dir, "/home/alice/.claude/skills");
    assert_eq!(claude.project_skills_dir.as_deref(), Some(".claude/skills"));
    let antigravity_cli = paths.get("antigravity-cli").unwrap();
    assert_eq!(
        antigravity_cli.global_skills_dir,
        "/home/alice/.gemini/antigravity-cli/skills"
    );
    assert_eq!(
        antigravity_cli.project_skills_dir.as_deref(),
        Some(".agents/skills")
    );
}

#[tokio::test]
async fn test_get_agents_returns_all_builtin() {
    let pool = setup_test_db().await;
    let agents = get_agents_impl(&pool).await.unwrap();
    assert_eq!(
        agents.len(),
        BUILTIN_AGENT_COUNT,
        "should return all built-in agents"
    );
}

#[tokio::test]
async fn test_get_agents_detected_flag_reflects_fs() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(tmp.path().to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let agents = get_agents_impl(&pool).await.unwrap();
    let claude = agents.iter().find(|a| a.id == "claude-code").unwrap();
    assert!(
        claude.is_detected,
        "claude-code should be detected when its dir exists"
    );
}

#[tokio::test]
async fn test_get_agents_not_detected_when_dir_missing() {
    let pool = setup_test_db().await;

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind("/nonexistent/deep/path/skills")
        .execute(&pool)
        .await
        .unwrap();

    let agents = get_agents_impl(&pool).await.unwrap();
    let claude = agents.iter().find(|a| a.id == "claude-code").unwrap();
    assert!(
        !claude.is_detected,
        "claude-code should not be detected when dir and parent both missing"
    );
}

#[tokio::test]
async fn test_get_agents_cached_uses_persisted_detection_state() {
    let pool = setup_test_db().await;

    sqlx::query(
        "UPDATE agents SET global_skills_dir = ?, is_detected = 1 WHERE id = 'claude-code'",
    )
    .bind("/nonexistent/deep/path/skills")
    .execute(&pool)
    .await
    .unwrap();

    let agents = get_agents_cached_impl(&pool).await.unwrap();
    let claude = agents.iter().find(|a| a.id == "claude-code").unwrap();
    assert!(
        claude.is_detected,
        "cached agents should not re-check remote paths on the local filesystem"
    );
}

#[tokio::test]
async fn test_detect_agents_updates_db() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(tmp.path().to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let before = db::get_agent_by_id(&pool, "claude-code")
        .await
        .unwrap()
        .unwrap();
    assert!(!before.is_detected);

    detect_agents_impl(&pool).await.unwrap();

    let after = db::get_agent_by_id(&pool, "claude-code")
        .await
        .unwrap()
        .unwrap();
    assert!(
        after.is_detected,
        "DB should reflect detected status after detect_agents"
    );
}

#[tokio::test]
async fn test_detect_agents_returns_all_agents() {
    let pool = setup_test_db().await;
    let agents = detect_agents_impl(&pool).await.unwrap();
    assert_eq!(agents.len(), BUILTIN_AGENT_COUNT);
}

#[tokio::test]
async fn test_add_custom_agent_appears_in_list() {
    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: Some("my-custom".to_string()),
        display_name: "My Custom Agent".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: "/tmp/my-custom/skills".to_string(),
    };

    add_custom_agent_impl(&pool, config).await.unwrap();

    let agents = get_agents_impl(&pool).await.unwrap();
    assert_eq!(
        agents.len(),
        BUILTIN_AGENT_COUNT + 1,
        "should have all built-ins + 1 custom"
    );

    let custom = agents.iter().find(|a| a.id == "my-custom").unwrap();
    assert_eq!(custom.display_name, "My Custom Agent");
    assert!(!custom.is_builtin);
}

#[tokio::test]
async fn test_add_custom_agent_auto_generates_id() {
    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: None,
        display_name: "Auto Named".to_string(),
        category: None,
        global_skills_dir: "/tmp/auto/skills".to_string(),
    };

    let agent = add_custom_agent_impl(&pool, config).await.unwrap();
    assert!(
        !agent.id.is_empty(),
        "auto-generated ID should not be empty"
    );
    assert!(
        agent.id.starts_with("custom-"),
        "auto-generated ID should start with 'custom-'"
    );
}

#[tokio::test]
async fn test_add_custom_agent_with_detected_dir() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: Some("detected-agent".to_string()),
        display_name: "Detected Agent".to_string(),
        category: None,
        global_skills_dir: skills_dir.to_string_lossy().into_owned(),
    };

    let agent = add_custom_agent_impl(&pool, config).await.unwrap();
    assert!(
        agent.is_detected,
        "agent should be detected when skills dir exists"
    );
}

#[tokio::test]
async fn test_add_custom_agent_duplicate_id_fails() {
    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: Some("unique-id".to_string()),
        display_name: "First".to_string(),
        category: None,
        global_skills_dir: "/tmp/first/skills".to_string(),
    };
    add_custom_agent_impl(&pool, config).await.unwrap();

    let config2 = CustomAgentConfig {
        id: Some("unique-id".to_string()),
        display_name: "Second".to_string(),
        category: None,
        global_skills_dir: "/tmp/second/skills".to_string(),
    };
    let result = add_custom_agent_impl(&pool, config2).await;
    assert!(result.is_err(), "duplicate agent ID should fail");
}

#[tokio::test]
async fn test_add_custom_agent_default_category() {
    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: Some("no-category".to_string()),
        display_name: "No Category".to_string(),
        category: None,
        global_skills_dir: "/tmp/nc/skills".to_string(),
    };

    let agent = add_custom_agent_impl(&pool, config).await.unwrap();
    assert_eq!(
        agent.category, "other",
        "default category should be 'other'"
    );
}

#[tokio::test]
async fn test_add_custom_agent_expands_tilde_path() {
    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: Some("tilde-agent".to_string()),
        display_name: "Tilde Agent".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: "~/.tilde-agent/skills".to_string(),
    };

    let agent = add_custom_agent_impl(&pool, config).await.unwrap();
    assert!(
        !agent.global_skills_dir.starts_with('~'),
        "tilde paths must be expanded before persistence"
    );
    assert!(agent.global_skills_dir.contains(".tilde-agent"));
}

#[tokio::test]
async fn test_add_custom_agent_expands_tilde_path_with_remote_home() {
    let pool = setup_test_db().await;

    let config = CustomAgentConfig {
        id: Some("remote-tilde-agent".to_string()),
        display_name: "Remote Tilde Agent".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: "~/.remote-agent/skills".to_string(),
    };

    let agent = add_custom_agent_impl_for_home(&pool, config, Some("/home/alice"))
        .await
        .unwrap();
    assert_eq!(agent.global_skills_dir, "/home/alice/.remote-agent/skills");
}

async fn add_test_custom_agent(pool: &DbPool, id: &str) {
    let config = CustomAgentConfig {
        id: Some(id.to_string()),
        display_name: format!("Agent {}", id),
        category: Some("other".to_string()),
        global_skills_dir: format!("/tmp/{}/skills", id),
    };
    add_custom_agent_impl(pool, config).await.unwrap();
}

#[tokio::test]
async fn test_update_custom_agent_changes_fields() {
    let pool = setup_test_db().await;
    add_test_custom_agent(&pool, "update-me").await;

    let config = UpdateCustomAgentConfig {
        display_name: "Updated Name".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: "/tmp/updated/skills".to_string(),
    };

    let updated = update_custom_agent_impl(&pool, "update-me", config)
        .await
        .unwrap();
    assert_eq!(updated.display_name, "Updated Name");
    assert_eq!(updated.category, "coding");
    assert_eq!(updated.global_skills_dir, "/tmp/updated/skills");
    assert!(!updated.is_builtin);
}

#[tokio::test]
async fn test_update_custom_agent_default_category() {
    let pool = setup_test_db().await;
    add_test_custom_agent(&pool, "cat-default").await;

    let config = UpdateCustomAgentConfig {
        display_name: "Cat Default".to_string(),
        category: None,
        global_skills_dir: "/tmp/cat-default/skills".to_string(),
    };

    let updated = update_custom_agent_impl(&pool, "cat-default", config)
        .await
        .unwrap();
    assert_eq!(
        updated.category, "other",
        "default category should be 'other'"
    );
}

#[tokio::test]
async fn test_update_custom_agent_expands_tilde_path() {
    let pool = setup_test_db().await;
    add_test_custom_agent(&pool, "tilde-update").await;

    let config = UpdateCustomAgentConfig {
        display_name: "Tilde Update".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: "~/.tilde-update/skills".to_string(),
    };

    let updated = update_custom_agent_impl(&pool, "tilde-update", config)
        .await
        .unwrap();
    assert!(
        !updated.global_skills_dir.starts_with('~'),
        "tilde paths must be expanded before persistence"
    );
    assert!(updated.global_skills_dir.contains(".tilde-update"));
}

#[tokio::test]
async fn test_update_custom_agent_expands_tilde_path_with_remote_home() {
    let pool = setup_test_db().await;
    add_test_custom_agent(&pool, "remote-tilde-update").await;

    let config = UpdateCustomAgentConfig {
        display_name: "Remote Tilde Update".to_string(),
        category: Some("coding".to_string()),
        global_skills_dir: "~/.remote-update/skills".to_string(),
    };

    let updated = update_custom_agent_impl_for_home(
        &pool,
        "remote-tilde-update",
        config,
        Some("/home/alice"),
    )
    .await
    .unwrap();
    assert_eq!(
        updated.global_skills_dir,
        "/home/alice/.remote-update/skills"
    );
}

#[tokio::test]
async fn test_update_custom_agent_not_found_fails() {
    let pool = setup_test_db().await;

    let config = UpdateCustomAgentConfig {
        display_name: "Ghost".to_string(),
        category: None,
        global_skills_dir: "/tmp/ghost/skills".to_string(),
    };

    let result = update_custom_agent_impl(&pool, "nonexistent-agent", config).await;
    assert!(result.is_err(), "Updating a nonexistent agent should fail");
}

#[tokio::test]
async fn test_update_builtin_agent_fails() {
    let pool = setup_test_db().await;

    let config = UpdateCustomAgentConfig {
        display_name: "Hacked Name".to_string(),
        category: None,
        global_skills_dir: "/tmp/hacked/skills".to_string(),
    };

    let result = update_custom_agent_impl(&pool, "claude-code", config).await;
    assert!(result.is_err(), "Updating a built-in agent should fail");
}

#[tokio::test]
async fn test_update_custom_agent_empty_display_name_fails() {
    let pool = setup_test_db().await;
    add_test_custom_agent(&pool, "empty-name").await;

    let config = UpdateCustomAgentConfig {
        display_name: "   ".to_string(),
        category: None,
        global_skills_dir: "/tmp/empty-name/skills".to_string(),
    };

    let result = update_custom_agent_impl(&pool, "empty-name", config).await;
    assert!(result.is_err(), "Empty display name should fail validation");
}

#[tokio::test]
async fn test_remove_custom_agent_success() {
    let pool = setup_test_db().await;
    add_test_custom_agent(&pool, "removable").await;

    remove_custom_agent_impl(&pool, "removable").await.unwrap();

    let agents = get_agents_impl(&pool).await.unwrap();
    assert!(
        agents.iter().all(|a| a.id != "removable"),
        "Removed agent should no longer appear in agent list"
    );
}

#[tokio::test]
async fn test_remove_custom_agent_not_found_fails() {
    let pool = setup_test_db().await;
    let result = remove_custom_agent_impl(&pool, "ghost-agent").await;
    assert!(result.is_err(), "Removing a nonexistent agent should fail");
}

#[tokio::test]
async fn test_remove_builtin_agent_fails() {
    let pool = setup_test_db().await;
    let result = remove_custom_agent_impl(&pool, "cursor").await;
    assert!(result.is_err(), "Removing a built-in agent should fail");
}

#[tokio::test]
async fn test_set_agent_enabled_updates_builtin_agent() {
    let pool = setup_test_db().await;

    let disabled = set_agent_enabled_impl(&pool, "claude-code", false)
        .await
        .unwrap();
    assert!(!disabled.is_enabled);

    let reenabled = set_agent_enabled_impl(&pool, "claude-code", true)
        .await
        .unwrap();
    assert!(reenabled.is_enabled);
}
