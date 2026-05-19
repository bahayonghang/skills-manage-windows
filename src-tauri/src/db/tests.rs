//! Cross-repo integration tests for the db layer — Phase 2d.
//!
//! Tests previously lived inside `legacy.rs::tests`. Moved to a dedicated
//! file so legacy.rs could be retired while keeping the broad e2e coverage
//! that exercises init_database + seed + repos CRUD interactions.

#![cfg(test)]
#![allow(unused_imports)]

use super::*;
use chrono::Utc;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;

/// Create an in-memory SQLite pool and initialize the schema.
async fn setup_test_db() -> DbPool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");
    init_database(&pool)
        .await
        .expect("Failed to initialize test database");
    pool
}

// ── Init ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_init_creates_all_tables() {
    let pool = setup_test_db().await;

    // Verify all core tables exist by counting rows (empty is fine)
    let tables = [
        "skills",
        "skill_installations",
        "agent_skill_observations",
        "agents",
        "collections",
        "collection_skills",
        "skill_repositories",
        "skill_repository_members",
        "skill_repository_sync_skips",
        "skill_update_states",
        "skill_tags",
        "skill_tag_links",
        "skill_ai_tag_reviews",
        "skill_explanations",
        "scan_directories",
        "settings",
        "operation_logs",
    ];
    for table in &tables {
        let result = sqlx::query(&format!("SELECT COUNT(*) as cnt FROM {}", table))
            .fetch_one(&pool)
            .await;
        assert!(result.is_ok(), "Table '{}' should exist", table);
    }
}

#[tokio::test]
async fn test_init_is_idempotent() {
    let pool = setup_test_db().await;
    // Calling init_database again should not fail
    let result = init_database(&pool).await;
    assert!(result.is_ok(), "Second init should be idempotent");
}

#[tokio::test]
async fn test_init_adds_performance_timestamp_columns_and_indexes() {
    let pool = setup_test_db().await;

    let skill_columns = table_columns(&pool, "skills").await;
    assert!(skill_columns.contains(&"fs_created_at".to_string()));
    assert!(skill_columns.contains(&"fs_updated_at".to_string()));

    let observation_columns = table_columns(&pool, "agent_skill_observations").await;
    assert!(observation_columns.contains(&"fs_created_at".to_string()));
    assert!(observation_columns.contains(&"fs_updated_at".to_string()));

    let observation_indexes = table_indexes(&pool, "agent_skill_observations").await;
    assert!(
        observation_indexes.contains(&"idx_agent_skill_observations_agent_name_dir".to_string())
    );

    let update_state_indexes = table_indexes(&pool, "skill_update_states").await;
    assert!(update_state_indexes.contains(&"idx_skill_update_states_status_skill".to_string()));
}

async fn table_columns(pool: &DbPool, table: &str) -> Vec<String> {
    sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<String, _>("name").unwrap())
        .collect()
}

async fn table_indexes(pool: &DbPool, table: &str) -> Vec<String> {
    sqlx::query(&format!("PRAGMA index_list({table})"))
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<String, _>("name").unwrap())
        .collect()
}

fn test_operation_log_entry(
    action: &str,
    level: &str,
    status: &str,
    target_kind: &str,
    target_id: &str,
    summary: &str,
) -> NewOperationLogEntry {
    NewOperationLogEntry {
        level: level.to_string(),
        target_kind: target_kind.to_string(),
        target_id: target_id.to_string(),
        target_label: Some(if target_kind == "ssh" {
            "Remote VPS".to_string()
        } else {
            "Local".to_string()
        }),
        category: action.split('.').next().unwrap_or("general").to_string(),
        action: action.to_string(),
        status: status.to_string(),
        subject_type: Some("skill".to_string()),
        subject_id: Some("skill-one".to_string()),
        subject_label: Some("Skill One".to_string()),
        summary: summary.to_string(),
        error_summary: (status != "succeeded").then(|| "Operation failed".to_string()),
        details_json: Some(r#"{"safe":true}"#.to_string()),
        duration_ms: Some(42),
        batch_id: None,
    }
}

#[tokio::test]
async fn operation_logs_can_be_inserted_listed_and_filtered() {
    let pool = setup_test_db().await;

    let failed = insert_operation_log(
        &pool,
        test_operation_log_entry(
            "skill.import",
            "error",
            "failed",
            "ssh",
            "ssh-1",
            "Imported Repo One failed",
        ),
    )
    .await
    .unwrap();
    let succeeded = insert_operation_log(
        &pool,
        test_operation_log_entry(
            "scan.all",
            "info",
            "succeeded",
            "local",
            "local",
            "Scan completed",
        ),
    )
    .await
    .unwrap();

    sqlx::query("UPDATE operation_logs SET created_at = ? WHERE id = ?")
        .bind("2026-04-27T09:00:00Z")
        .bind(&failed.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE operation_logs SET created_at = ? WHERE id = ?")
        .bind("2026-04-27T10:00:00Z")
        .bind(&succeeded.id)
        .execute(&pool)
        .await
        .unwrap();

    let page = list_operation_logs(&pool, OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].id, succeeded.id);
    assert_eq!(page.limit, DEFAULT_OPERATION_LOG_LIMIT);

    let error_page = list_operation_logs(
        &pool,
        OperationLogFilter {
            level: Some("error".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(error_page.total, 1);
    assert_eq!(error_page.entries[0].id, failed.id);

    let search_page = list_operation_logs(
        &pool,
        OperationLogFilter {
            query: Some("Repo One".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(search_page.total, 1);
    assert_eq!(search_page.entries[0].id, failed.id);

    let target_page = list_operation_logs(
        &pool,
        OperationLogFilter {
            target_id: Some("ssh-1".to_string()),
            status: Some("failed".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(target_page.total, 1);
    assert_eq!(target_page.entries[0].target_kind, "ssh");

    let time_page = list_operation_logs(
        &pool,
        OperationLogFilter {
            created_after: Some("2026-04-27T09:30:00Z".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(time_page.total, 1);
    assert_eq!(time_page.entries[0].id, succeeded.id);

    let capped_page = list_operation_logs(
        &pool,
        OperationLogFilter {
            limit: Some(5_000),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(capped_page.limit, MAX_OPERATION_LOG_LIMIT);
}

#[tokio::test]
async fn operation_log_clear_is_scoped_to_logs() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO skills (id, name, file_path, is_central, scanned_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind("skill-one")
    .bind("Skill One")
    .bind("/tmp/skill-one/SKILL.md")
    .bind(true)
    .bind("2026-04-27T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();

    insert_operation_log(
        &pool,
        test_operation_log_entry("scan.all", "info", "succeeded", "local", "local", "Scan"),
    )
    .await
    .unwrap();
    insert_operation_log(
        &pool,
        test_operation_log_entry(
            "skill.import",
            "error",
            "failed",
            "ssh",
            "ssh-1",
            "Import failed",
        ),
    )
    .await
    .unwrap();

    let deleted = clear_operation_logs(
        &pool,
        OperationLogFilter {
            level: Some("error".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(deleted, 1);

    let remaining = list_operation_logs(&pool, OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(remaining.total, 1);

    let skill_count: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM skills")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("cnt");
    assert_eq!(skill_count, 1);
}

#[tokio::test]
async fn operation_log_export_contains_metadata_and_entries() {
    let pool = setup_test_db().await;
    insert_operation_log(
        &pool,
        test_operation_log_entry(
            "scan.all",
            "info",
            "succeeded",
            "local",
            "local",
            "Scan completed",
        ),
    )
    .await
    .unwrap();

    let exported = export_operation_logs_json(
        &pool,
        OperationLogFilter {
            action: Some("scan.all".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&exported).unwrap();

    assert!(value["exportedAt"].as_str().is_some());
    assert_eq!(value["filter"]["action"], "scan.all");
    assert_eq!(value["total"], 1);
    assert_eq!(value["entries"].as_array().unwrap().len(), 1);
    assert_eq!(value["entries"][0]["action"], "scan.all");
}

#[tokio::test]
async fn test_builtin_agents_seeded() {
    let pool = setup_test_db().await;
    let agents = get_all_agents(&pool).await.unwrap();
    assert_eq!(agents.len(), 33, "Should have exactly 33 built-in agents");

    let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
    // Coding platforms
    assert!(ids.contains(&"claude-code"));
    assert!(ids.contains(&"codex"));
    assert!(ids.contains(&"cursor"));
    assert!(ids.contains(&"gemini-cli"));
    assert!(ids.contains(&"trae"));
    assert!(ids.contains(&"factory-droid"));
    assert!(ids.contains(&"junie"));
    assert!(ids.contains(&"qwen"));
    assert!(ids.contains(&"trae-cn"));
    assert!(ids.contains(&"windsurf"));
    assert!(ids.contains(&"qoder"));
    assert!(ids.contains(&"augment"));
    assert!(ids.contains(&"opencode"));
    assert!(ids.contains(&"kilocode"));
    assert!(ids.contains(&"ob1"));
    assert!(ids.contains(&"amp"));
    assert!(ids.contains(&"antigravity"));
    assert!(ids.contains(&"cline"));
    assert!(ids.contains(&"deep-agents"));
    assert!(ids.contains(&"firebender"));
    assert!(ids.contains(&"kiro"));
    assert!(ids.contains(&"kimi-code-cli"));
    assert!(ids.contains(&"codebuddy"));
    assert!(ids.contains(&"hermes"));
    assert!(ids.contains(&"copilot"));
    assert!(ids.contains(&"warp"));
    assert!(ids.contains(&"aider"));
    // Lobster platforms
    assert!(ids.contains(&"openclaw"));
    assert!(ids.contains(&"qclaw"));
    assert!(ids.contains(&"easyclaw"));
    assert!(ids.contains(&"autoclaw"));
    assert!(ids.contains(&"workbuddy"));
    // Central
    assert!(ids.contains(&"central"));
}

#[tokio::test]
async fn test_builtin_agents_are_marked_builtin() {
    let pool = setup_test_db().await;
    let agents = get_all_agents(&pool).await.unwrap();
    for agent in &agents {
        assert!(agent.is_builtin, "All seeded agents should be builtin");
    }
}

#[tokio::test]
async fn test_universal_agents_share_universal_skills_dir() {
    let pool = setup_test_db().await;
    let agents = get_all_agents(&pool).await.unwrap();
    let central = agents
        .iter()
        .find(|agent| agent.id == "central")
        .expect("central agent should exist");
    let universal_dir = crate::paths::universal_skills_dir();

    assert!(
        crate::paths::paths_equivalent(
            Path::new(&central.global_skills_dir),
            &crate::paths::central_skills_dir()
        ),
        "central should use the private SkillPort skills directory"
    );
    assert!(
        !crate::paths::paths_equivalent(Path::new(&central.global_skills_dir), &universal_dir),
        "central should not share the Universal Agents directory"
    );

    for agent_id in UNIVERSAL_AGENT_IDS {
        let agent = agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .unwrap_or_else(|| panic!("missing universal agent {agent_id}"));
        assert!(
            crate::paths::paths_equivalent(Path::new(&agent.global_skills_dir), &universal_dir),
            "{agent_id} should use the Universal Agents skills directory"
        );
    }
}

#[tokio::test]
async fn test_builtin_agents_seed_default_enabled_subset() {
    let pool = setup_test_db().await;
    let agents = get_all_agents(&pool).await.unwrap();

    let enabled_ids: std::collections::HashSet<&str> = agents
        .iter()
        .filter(|agent| agent.is_enabled)
        .map(|agent| agent.id.as_str())
        .collect();

    let expected_enabled_ids = std::collections::HashSet::from([
        "claude-code",
        "codex",
        "gemini-cli",
        "opencode",
        "kiro",
        "central",
    ]);

    assert_eq!(enabled_ids, expected_enabled_ids);
}

#[tokio::test]
async fn test_init_does_not_duplicate_agents_on_reinit() {
    let pool = setup_test_db().await;
    init_database(&pool).await.unwrap(); // Call a second time
    let agents = get_all_agents(&pool).await.unwrap();
    assert_eq!(agents.len(), 33, "Reinit must not duplicate agents");
}

// ── Skills ────────────────────────────────────────────────────────────────

fn make_skill(id: &str, name: &str, is_central: bool) -> Skill {
    Skill {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(format!("Description for {}", name)),
        file_path: format!("/tmp/{}/SKILL.md", id),
        canonical_path: if is_central {
            Some(format!("/tmp/.skillsmanage/skills/{}", id))
        } else {
            None
        },
        is_central,
        source: None,
        content: Some("# Test Skill\n\nContent here.".to_string()),
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

#[tokio::test]
async fn test_upsert_skill_insert() {
    let pool = setup_test_db().await;
    let skill = make_skill("test-skill", "Test Skill", false);
    upsert_skill(&pool, &skill).await.unwrap();

    let retrieved = get_skill_by_id(&pool, "test-skill").await.unwrap();
    assert!(retrieved.is_some());
    let s = retrieved.unwrap();
    assert_eq!(s.name, "Test Skill");
    assert!(!s.is_central);
}

#[tokio::test]
async fn test_upsert_skill_update() {
    let pool = setup_test_db().await;
    let mut skill = make_skill("skill-1", "Original Name", false);
    upsert_skill(&pool, &skill).await.unwrap();

    skill.name = "Updated Name".to_string();
    upsert_skill(&pool, &skill).await.unwrap();

    let retrieved = get_skill_by_id(&pool, "skill-1").await.unwrap().unwrap();
    assert_eq!(retrieved.name, "Updated Name");
}

#[tokio::test]
async fn test_upsert_skill_preserves_central_record_when_platform_copy_is_seen_later() {
    let pool = setup_test_db().await;
    let mut central = make_skill("shared-skill", "Central Truth", true);
    central.file_path = "/tmp/.skillsmanage/skills/shared-skill/SKILL.md".to_string();
    central.canonical_path = Some("/tmp/.skillsmanage/skills/shared-skill".to_string());
    central.source = Some("native".to_string());
    upsert_skill(&pool, &central).await.unwrap();

    let mut platform = make_skill("shared-skill", "Platform Copy", false);
    platform.file_path = "/tmp/.agents/skills/shared-skill/SKILL.md".to_string();
    platform.canonical_path = Some("/tmp/.agents/skills/shared-skill".to_string());
    platform.source = Some("copy".to_string());
    upsert_skill(&pool, &platform).await.unwrap();

    let retrieved = get_skill_by_id(&pool, "shared-skill")
        .await
        .unwrap()
        .unwrap();
    assert!(retrieved.is_central);
    assert_eq!(retrieved.name, "Central Truth");
    assert_eq!(
        retrieved.file_path,
        "/tmp/.skillsmanage/skills/shared-skill/SKILL.md"
    );
    assert_eq!(
        retrieved.canonical_path.as_deref(),
        Some("/tmp/.skillsmanage/skills/shared-skill")
    );
    assert_eq!(retrieved.source.as_deref(), Some("native"));
}

#[tokio::test]
async fn test_get_skill_by_id_not_found() {
    let pool = setup_test_db().await;
    let result = get_skill_by_id(&pool, "nonexistent").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_central_skills() {
    let pool = setup_test_db().await;
    upsert_skill(&pool, &make_skill("central-1", "Central One", true))
        .await
        .unwrap();
    upsert_skill(&pool, &make_skill("non-central", "Not Central", false))
        .await
        .unwrap();

    let central = get_central_skills(&pool).await.unwrap();
    assert_eq!(central.len(), 1);
    assert_eq!(central[0].id, "central-1");
}

#[tokio::test]
async fn test_upsert_skill_update_state() {
    let pool = setup_test_db().await;
    let state = SkillUpdateState {
        skill_id: "central-1".to_string(),
        source_type: "github".to_string(),
        source_url: Some("https://github.com/example/skills".to_string()),
        ref_name: Some("main".to_string()),
        source_path: Some("skills/central-1".to_string()),
        last_remote_hash: Some("fnv1a64:old".to_string()),
        latest_remote_hash: Some("fnv1a64:new".to_string()),
        last_checked_at: Some("2026-04-25T00:00:00Z".to_string()),
        last_updated_at: None,
        status: "update_available".to_string(),
        error: None,
    };

    upsert_skill_update_state(&pool, &state).await.unwrap();
    let states = get_skill_update_states_for_skills(&pool, &["central-1".to_string()])
        .await
        .unwrap();

    assert_eq!(states.len(), 1);
    assert_eq!(states[0].skill_id, "central-1");
    assert_eq!(states[0].status, "update_available");
    assert_eq!(states[0].latest_remote_hash.as_deref(), Some("fnv1a64:new"));
}

#[tokio::test]
async fn test_delete_skill() {
    let pool = setup_test_db().await;
    let skill = make_skill("to-delete", "Delete Me", false);
    upsert_skill(&pool, &skill).await.unwrap();

    let repository = create_or_update_skill_repository(
        &pool,
        Some("repo-delete-test"),
        "Delete Test Repo",
        "github",
        Some("owner"),
        Some("repo"),
        Some("main"),
        Some("https://example.com/owner/repo"),
        false,
    )
    .await
    .unwrap();
    assign_skills_to_repository(
        &pool,
        &repository.id,
        &["to-delete".to_string()],
        Some("skills/to-delete"),
    )
    .await
    .unwrap();

    let tag = create_skill_tag(&pool, "Delete Tag", None, None)
        .await
        .unwrap();
    assign_skill_tags(
        &pool,
        &["to-delete".to_string()],
        std::slice::from_ref(&tag.id),
        "manual",
        Some(1.0),
        Some("delete test"),
    )
    .await
    .unwrap();
    replace_pending_ai_tag_reviews(
        &pool,
        "to-delete",
        &[(tag.id.clone(), 0.42, "review".to_string())],
    )
    .await
    .unwrap();
    upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "to-delete".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/to-delete".to_string()),
            last_remote_hash: Some("fnv1a64:old".to_string()),
            latest_remote_hash: Some("fnv1a64:new".to_string()),
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: "update_available".to_string(),
            error: None,
        },
    )
    .await
    .unwrap();

    let collection = create_collection(&pool, "Delete Collection", None)
        .await
        .unwrap();
    add_skill_to_collection(&pool, &collection.id, "to-delete")
        .await
        .unwrap();
    upsert_skill_installation(&pool, &make_installation("to-delete", "cursor", "copy"))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO skill_explanations (skill_id, explanation, lang, model, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("to-delete")
    .bind("Explanation")
    .bind("zh")
    .bind("test-model")
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    delete_skill(&pool, "to-delete").await.unwrap();
    let result = get_skill_by_id(&pool, "to-delete").await.unwrap();
    assert!(result.is_none());

    for table in [
        "skill_repository_members",
        "skill_update_states",
        "skill_tag_links",
        "skill_ai_tag_reviews",
        "skill_explanations",
        "collection_skills",
        "skill_installations",
    ] {
        let query = format!("SELECT COUNT(*) FROM {table} WHERE skill_id = ?");
        let count = sqlx::query_scalar::<_, i64>(&query)
            .bind("to-delete")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} rows must be deleted");
    }
}

// ── Skill Installations ───────────────────────────────────────────────────

fn make_installation(skill_id: &str, agent_id: &str, link_type: &str) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: format!("/tmp/{}/{}", agent_id, skill_id),
        link_type: link_type.to_string(),
        symlink_target: if link_type == "symlink" {
            Some(format!("/tmp/.agents/skills/{}", skill_id))
        } else {
            None
        },
        created_at: Utc::now().to_rfc3339(),
    }
}

#[tokio::test]
async fn test_upsert_and_get_skill_installation() {
    let pool = setup_test_db().await;
    let skill = make_skill("my-skill", "My Skill", false);
    upsert_skill(&pool, &skill).await.unwrap();

    let inst = make_installation("my-skill", "claude-code", "symlink");
    upsert_skill_installation(&pool, &inst).await.unwrap();

    let installations = get_skill_installations(&pool, "my-skill").await.unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0].agent_id, "claude-code");
    assert_eq!(installations[0].link_type, "symlink");
}

#[tokio::test]
async fn test_delete_skill_installation() {
    let pool = setup_test_db().await;
    let skill = make_skill("del-skill", "Del Skill", false);
    upsert_skill(&pool, &skill).await.unwrap();
    upsert_skill_installation(&pool, &make_installation("del-skill", "cursor", "copy"))
        .await
        .unwrap();

    delete_skill_installation(&pool, "del-skill", "cursor")
        .await
        .unwrap();

    let installations = get_skill_installations(&pool, "del-skill").await.unwrap();
    assert!(installations.is_empty());
}

#[tokio::test]
async fn test_get_skills_by_agent() {
    let pool = setup_test_db().await;
    let skill_a = make_skill("skill-a", "Skill A", false);
    let skill_b = make_skill("skill-b", "Skill B", false);
    upsert_skill(&pool, &skill_a).await.unwrap();
    upsert_skill(&pool, &skill_b).await.unwrap();

    upsert_skill_installation(
        &pool,
        &make_installation("skill-a", "claude-code", "symlink"),
    )
    .await
    .unwrap();
    upsert_skill_installation(&pool, &make_installation("skill-b", "cursor", "copy"))
        .await
        .unwrap();

    let claude_skills = get_skills_by_agent(&pool, "claude-code").await.unwrap();
    assert_eq!(claude_skills.len(), 1);
    assert_eq!(claude_skills[0].id, "skill-a");

    let cursor_skills = get_skills_by_agent(&pool, "cursor").await.unwrap();
    assert_eq!(cursor_skills.len(), 1);
    assert_eq!(cursor_skills[0].id, "skill-b");

    let empty = get_skills_by_agent(&pool, "codex").await.unwrap();
    assert!(empty.is_empty());
}

// ── Agents ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_agent_by_id() {
    let pool = setup_test_db().await;
    let agent = get_agent_by_id(&pool, "claude-code").await.unwrap();
    assert!(agent.is_some());
    let a = agent.unwrap();
    assert_eq!(a.display_name, "Claude Code");
    assert_eq!(a.category, "coding");
    assert!(a.is_builtin);
}

#[tokio::test]
async fn test_get_agent_by_id_not_found() {
    let pool = setup_test_db().await;
    let agent = get_agent_by_id(&pool, "nonexistent-agent").await.unwrap();
    assert!(agent.is_none());
}

#[tokio::test]
async fn test_update_agent_detected() {
    let pool = setup_test_db().await;
    update_agent_detected(&pool, "cursor", true).await.unwrap();
    let agent = get_agent_by_id(&pool, "cursor").await.unwrap().unwrap();
    assert!(agent.is_detected);

    update_agent_detected(&pool, "cursor", false).await.unwrap();
    let agent = get_agent_by_id(&pool, "cursor").await.unwrap().unwrap();
    assert!(!agent.is_detected);
}

#[tokio::test]
async fn test_update_agent_enabled() {
    let pool = setup_test_db().await;

    let updated = update_agent_enabled(&pool, "claude-code", false)
        .await
        .unwrap();
    assert!(!updated.is_enabled);

    let persisted = get_agent_by_id(&pool, "claude-code")
        .await
        .unwrap()
        .unwrap();
    assert!(!persisted.is_enabled);

    let reenabled = update_agent_enabled(&pool, "claude-code", true)
        .await
        .unwrap();
    assert!(reenabled.is_enabled);
}

#[tokio::test]
async fn test_insert_custom_agent() {
    let pool = setup_test_db().await;
    let custom = Agent {
        id: "my-custom-agent".to_string(),
        display_name: "My Custom Agent".to_string(),
        category: "other".to_string(),
        global_skills_dir: "/tmp/custom/skills".to_string(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    insert_custom_agent(&pool, &custom).await.unwrap();

    let all = get_all_agents(&pool).await.unwrap();
    assert_eq!(all.len(), 34, "Should have 33 builtins + 1 custom");

    let retrieved = get_agent_by_id(&pool, "my-custom-agent")
        .await
        .unwrap()
        .unwrap();
    assert!(!retrieved.is_builtin);
    assert_eq!(retrieved.display_name, "My Custom Agent");
}

#[tokio::test]
async fn test_delete_custom_agent() {
    let pool = setup_test_db().await;
    let custom = Agent {
        id: "deletable-agent".to_string(),
        display_name: "Deletable".to_string(),
        category: "other".to_string(),
        global_skills_dir: "/tmp/deletable/skills".to_string(),
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };
    insert_custom_agent(&pool, &custom).await.unwrap();
    delete_custom_agent(&pool, "deletable-agent").await.unwrap();

    let retrieved = get_agent_by_id(&pool, "deletable-agent").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_cannot_delete_builtin_agent() {
    let pool = setup_test_db().await;
    let result = delete_custom_agent(&pool, "claude-code").await;
    assert!(
        result.is_err(),
        "Should not be able to delete built-in agent"
    );
}

#[tokio::test]
async fn test_workbuddy_scans_correct_directory() {
    let pool = setup_test_db().await;
    let wb = get_agent_by_id(&pool, "workbuddy")
        .await
        .unwrap()
        .expect("WorkBuddy agent should exist");
    assert_eq!(wb.display_name, "WorkBuddy");
    let expected_suffix = Path::new(".workbuddy")
        .join("skills-marketplace")
        .join("skills");
    assert!(
        Path::new(&wb.global_skills_dir).ends_with(&expected_suffix),
        "WorkBuddy should scan ~/.workbuddy/skills-marketplace/skills, got: {}",
        wb.global_skills_dir
    );
}

#[tokio::test]
async fn test_autoclaw_is_separate_from_workbuddy() {
    let pool = setup_test_db().await;
    let ac = get_agent_by_id(&pool, "autoclaw")
        .await
        .unwrap()
        .expect("AutoClaw agent should exist");
    assert_eq!(ac.display_name, "AutoClaw");
    assert_eq!(ac.category, "lobster");
    let expected_suffix = Path::new(".openclaw-autoclaw").join("skills");
    assert!(
        Path::new(&ac.global_skills_dir).ends_with(&expected_suffix),
        "AutoClaw should scan ~/.openclaw-autoclaw/skills, got: {}",
        ac.global_skills_dir
    );
    // Verify AutoClaw and WorkBuddy are distinct entries
    assert_ne!(ac.id, "workbuddy");
    assert_ne!(
        ac.global_skills_dir,
        get_agent_by_id(&pool, "workbuddy")
            .await
            .unwrap()
            .unwrap()
            .global_skills_dir
    );
}

// ── Collections ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_collection() {
    let pool = setup_test_db().await;
    let col = create_collection(&pool, "My Collection", Some("A test collection"))
        .await
        .unwrap();
    assert!(!col.id.is_empty());
    assert_eq!(col.name, "My Collection");
    assert_eq!(col.description.as_deref(), Some("A test collection"));
}

#[tokio::test]
async fn test_get_all_collections() {
    let pool = setup_test_db().await;
    create_collection(&pool, "Collection A", None)
        .await
        .unwrap();
    create_collection(&pool, "Collection B", Some("Desc"))
        .await
        .unwrap();

    let all = get_all_collections(&pool).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_update_collection() {
    let pool = setup_test_db().await;
    let col = create_collection(&pool, "Old Name", None).await.unwrap();
    update_collection(&pool, &col.id, "New Name", Some("New desc"))
        .await
        .unwrap();

    let updated = get_collection_by_id(&pool, &col.id).await.unwrap().unwrap();
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.description.as_deref(), Some("New desc"));
}

#[tokio::test]
async fn test_delete_collection() {
    let pool = setup_test_db().await;
    let col = create_collection(&pool, "To Delete", None).await.unwrap();
    delete_collection(&pool, &col.id).await.unwrap();

    let retrieved = get_collection_by_id(&pool, &col.id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_add_and_remove_skill_from_collection() {
    let pool = setup_test_db().await;
    let skill = make_skill("collection-skill", "Collection Skill", false);
    upsert_skill(&pool, &skill).await.unwrap();
    let col = create_collection(&pool, "Test Col", None).await.unwrap();

    add_skill_to_collection(&pool, &col.id, "collection-skill")
        .await
        .unwrap();

    let skills = get_collection_skills(&pool, &col.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "collection-skill");

    remove_skill_from_collection(&pool, &col.id, "collection-skill")
        .await
        .unwrap();

    let skills_after = get_collection_skills(&pool, &col.id).await.unwrap();
    assert!(skills_after.is_empty());
}

#[tokio::test]
async fn test_add_skill_to_collection_is_idempotent() {
    let pool = setup_test_db().await;
    let skill = make_skill("idem-skill", "Idem Skill", false);
    upsert_skill(&pool, &skill).await.unwrap();
    let col = create_collection(&pool, "Idem Col", None).await.unwrap();

    add_skill_to_collection(&pool, &col.id, "idem-skill")
        .await
        .unwrap();
    add_skill_to_collection(&pool, &col.id, "idem-skill")
        .await
        .unwrap();

    let skills = get_collection_skills(&pool, &col.id).await.unwrap();
    assert_eq!(skills.len(), 1, "Duplicate add should be a no-op");
}

#[tokio::test]
async fn test_delete_collection_also_removes_skill_memberships() {
    let pool = setup_test_db().await;
    let skill = make_skill("cascade-skill", "Cascade Skill", false);
    upsert_skill(&pool, &skill).await.unwrap();
    let col = create_collection(&pool, "Cascade Col", None).await.unwrap();
    add_skill_to_collection(&pool, &col.id, "cascade-skill")
        .await
        .unwrap();

    delete_collection(&pool, &col.id).await.unwrap();

    // The collection_skills row should also be gone
    let rows: Vec<_> = sqlx::query("SELECT * FROM collection_skills WHERE collection_id = ?")
        .bind(&col.id)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(rows.is_empty(), "Memberships should be cascade-deleted");
}

// ── Skill Repositories and Tags ─────────────────────────────────────────

#[tokio::test]
async fn test_builtin_skill_metadata_seeded_and_idempotent() {
    let pool = setup_test_db().await;
    init_database(&pool).await.unwrap();

    let repositories = get_skill_repositories_with_stats(&pool).await.unwrap();
    assert!(repositories
        .iter()
        .any(|entry| entry.repository.id == LOCAL_UNKNOWN_REPOSITORY_ID));

    let tags = get_skill_tags(&pool).await.unwrap();
    assert!(
        tags.iter().any(|tag| tag.id == UNCATEGORIZED_TAG_ID),
        "uncategorized tag should be seeded"
    );
    assert_eq!(
        tags.iter().filter(|tag| tag.is_builtin).count(),
        builtin_skill_tags().len()
    );
}

#[tokio::test]
async fn test_assign_github_repository_to_skill_records_source_path() {
    let pool = setup_test_db().await;
    let skill = make_skill("github-skill", "GitHub Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();

    assign_github_repository_to_skill(
        &pool,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "github-skill",
        "skills/.curated/github-skill",
    )
    .await
    .unwrap();

    let assignment = get_skill_repository_assignment(&pool, "github-skill")
        .await
        .unwrap();
    assert_eq!(assignment.repository.source_type, "github");
    assert_eq!(assignment.repository.owner.as_deref(), Some("openai"));
    assert_eq!(
        assignment.source_path.as_deref(),
        Some("skills/.curated/github-skill")
    );
    assert!(!assignment.is_source_unknown);
}

#[tokio::test]
async fn test_skill_repository_sync_skip_upsert_list_and_delete() {
    let pool = setup_test_db().await;
    let repository = create_or_update_skill_repository(
        &pool,
        Some("github-openai-skills-main"),
        "openai/skills",
        "github",
        Some("openai"),
        Some("skills"),
        Some("main"),
        Some("https://github.com/openai/skills"),
        false,
    )
    .await
    .unwrap();

    let created = upsert_skill_repository_sync_skip(
        &pool,
        &repository.id,
        "skills/planning-with-files-ar",
        "planning-with-files-ar",
        "Planning with Files AR",
    )
    .await
    .unwrap();
    assert_eq!(created.repository_id, repository.id);
    assert_eq!(created.source_path, "skills/planning-with-files-ar");

    let updated = upsert_skill_repository_sync_skip(
        &pool,
        &repository.id,
        "skills/planning-with-files-ar",
        "planning-files-ar",
        "Planning Files Arabic",
    )
    .await
    .unwrap();
    assert_eq!(updated.created_at, created.created_at);
    assert_eq!(updated.skill_id, "planning-files-ar");
    assert_eq!(updated.skill_name, "Planning Files Arabic");

    let skips = get_skill_repository_sync_skips(&pool, std::slice::from_ref(&repository.id))
        .await
        .unwrap();
    assert_eq!(skips.len(), 1);
    assert_eq!(skips[0].source_path, "skills/planning-with-files-ar");

    assert!(delete_skill_repository_sync_skip(
        &pool,
        &repository.id,
        "skills/planning-with-files-ar"
    )
    .await
    .unwrap());
    assert!(
        get_skill_repository_sync_skips(&pool, std::slice::from_ref(&repository.id))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn test_skill_repository_pinned_defaults_false_and_can_be_updated() {
    let pool = setup_test_db().await;
    let skill = make_skill("github-pin-skill", "GitHub Pin Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();

    let repository = assign_github_repository_to_skill(
        &pool,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "github-pin-skill",
        "skills/github-pin-skill",
    )
    .await
    .unwrap();
    assert!(!repository.pinned);

    let updated = set_skill_repository_pinned(&pool, &repository.id, true)
        .await
        .unwrap();
    assert!(updated.pinned);

    let repositories = get_skill_repositories_with_stats(&pool).await.unwrap();
    let listed = repositories
        .iter()
        .find(|entry| entry.repository.id == repository.id)
        .unwrap();
    assert!(listed.repository.pinned);
}

#[tokio::test]
async fn test_create_or_update_skill_repository_preserves_pinned_state() {
    let pool = setup_test_db().await;
    let repository = create_or_update_skill_repository(
        &pool,
        Some("github-openai-skills-main"),
        "openai/skills",
        "github",
        Some("openai"),
        Some("skills"),
        Some("main"),
        Some("https://github.com/openai/skills"),
        false,
    )
    .await
    .unwrap();

    set_skill_repository_pinned(&pool, &repository.id, true)
        .await
        .unwrap();
    let refreshed = create_or_update_skill_repository(
        &pool,
        Some("github-openai-skills-main"),
        "openai/skills-renamed",
        "github",
        Some("openai"),
        Some("skills"),
        Some("main"),
        Some("https://github.com/openai/skills"),
        false,
    )
    .await
    .unwrap();

    assert_eq!(refreshed.name, "openai/skills-renamed");
    assert!(refreshed.pinned);
}

#[tokio::test]
async fn test_set_skill_repository_pinned_rejects_unknown_repository() {
    let pool = setup_test_db().await;

    let error = set_skill_repository_pinned(&pool, LOCAL_UNKNOWN_REPOSITORY_ID, true)
        .await
        .unwrap_err();

    assert!(error.contains("cannot be pinned"));
}

#[tokio::test]
async fn test_delete_last_repository_skill_prunes_repository() {
    let pool = setup_test_db().await;
    let skill = make_skill("github-prune-skill", "GitHub Prune Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let repository = assign_github_repository_to_skill(
        &pool,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "github-prune-skill",
        "skills/github-prune-skill",
    )
    .await
    .unwrap();

    delete_skill(&pool, "github-prune-skill").await.unwrap();

    assert!(get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_none());
    assert!(
        get_skill_repository_by_id(&pool, LOCAL_UNKNOWN_REPOSITORY_ID)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_delete_skills_not_in_scope_prunes_empty_repositories() {
    let pool = setup_test_db().await;
    let skill = make_skill("stale-github-skill", "Stale GitHub Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let repository = assign_github_repository_to_skill(
        &pool,
        "example",
        "stale",
        "main",
        "https://github.com/example/stale",
        "stale-github-skill",
        "skills/stale-github-skill",
    )
    .await
    .unwrap();

    delete_skills_not_in_scope(&pool, &[]).await.unwrap();

    assert!(get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_none());
    assert!(
        get_skill_repository_by_id(&pool, LOCAL_UNKNOWN_REPOSITORY_ID)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_delete_empty_skill_repository_rejects_unknown_repository() {
    let pool = setup_test_db().await;

    let error = delete_empty_skill_repository(&pool, LOCAL_UNKNOWN_REPOSITORY_ID)
        .await
        .unwrap_err();

    assert!(error.contains("cannot be deleted"));
    assert!(
        get_skill_repository_by_id(&pool, LOCAL_UNKNOWN_REPOSITORY_ID)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_unassigned_central_skill_uses_unknown_repository() {
    let pool = setup_test_db().await;
    let skill = make_skill("unknown-skill", "Unknown Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();

    let assignment = get_skill_repository_assignment(&pool, "unknown-skill")
        .await
        .unwrap();
    assert_eq!(assignment.repository.id, LOCAL_UNKNOWN_REPOSITORY_ID);
    assert!(assignment.is_source_unknown);

    let repositories = get_skill_repositories_with_stats(&pool).await.unwrap();
    let unknown = repositories
        .iter()
        .find(|entry| entry.repository.id == LOCAL_UNKNOWN_REPOSITORY_ID)
        .unwrap();
    assert_eq!(unknown.unknown_skill_count, 1);
}

#[tokio::test]
async fn test_assign_skill_tags_supports_multi_tag_binding() {
    let pool = setup_test_db().await;
    let skill = make_skill("tagged-skill", "Tagged Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let custom = create_skill_tag(&pool, "自定义标签", Some("custom"), Some("#111111"))
        .await
        .unwrap();

    assign_skill_tags(
        &pool,
        &["tagged-skill".to_string()],
        &[custom.id.clone(), UNCATEGORIZED_TAG_ID.to_string()],
        "manual",
        None,
        None,
    )
    .await
    .unwrap();

    let tags = get_skill_tags_for_skill(&pool, "tagged-skill")
        .await
        .unwrap();
    let ids = tags.iter().map(|tag| tag.id.as_str()).collect::<Vec<_>>();
    assert!(ids.contains(&custom.id.as_str()));
    assert!(ids.contains(&UNCATEGORIZED_TAG_ID));
}

#[tokio::test]
async fn test_replace_skill_ai_tags_does_not_remove_manual_tags() {
    let pool = setup_test_db().await;
    let skill = make_skill("ai-tagged-skill", "AI Tagged Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let manual = create_skill_tag(&pool, "人工标签", None, None)
        .await
        .unwrap();

    assign_skill_tags(
        &pool,
        &["ai-tagged-skill".to_string()],
        std::slice::from_ref(&manual.id),
        "manual",
        None,
        None,
    )
    .await
    .unwrap();
    replace_skill_ai_tags(
        &pool,
        "ai-tagged-skill",
        &[(UNCATEGORIZED_TAG_ID.to_string(), 0.7, "AI 建议".to_string())],
    )
    .await
    .unwrap();

    let tags = get_skill_tags_for_skill(&pool, "ai-tagged-skill")
        .await
        .unwrap();
    let ids = tags.iter().map(|tag| tag.id.as_str()).collect::<Vec<_>>();
    assert!(ids.contains(&manual.id.as_str()));
    assert!(ids.contains(&UNCATEGORIZED_TAG_ID));
}

// ── Scan Directories ──────────────────────────────────────────────────────

/// Returns the number of *unique* global_skills_dir paths across all
/// built-in agents. This is the number of rows that seed_builtin_scan_directories
/// inserts, with Universal agents sharing ~/.agents/skills and Central using
/// ~/.skillsmanage/skills.
fn expected_builtin_scan_dir_count() -> usize {
    let mut paths = std::collections::HashSet::new();
    for agent in builtin_agents() {
        paths.insert(agent.global_skills_dir);
    }
    paths.len()
}

#[tokio::test]
async fn test_builtin_scan_dirs_seeded() {
    let pool = setup_test_db().await;
    let dirs = get_scan_directories(&pool).await.unwrap();
    let builtin_count = expected_builtin_scan_dir_count();

    // Expect exactly one row per unique global_skills_dir across built-in agents.
    assert_eq!(
        dirs.len(),
        builtin_count,
        "Should have {} built-in scan directories after init (got {})",
        builtin_count,
        dirs.len()
    );

    // Every seeded row must be marked as built-in and active.
    for dir in &dirs {
        assert!(
            dir.is_builtin,
            "Seeded scan directory '{}' must have is_builtin=true",
            dir.path
        );
        assert!(
            dir.is_active,
            "Seeded scan directory '{}' must be active by default",
            dir.path
        );
    }

    // The paths must match the unique global_skills_dir values.
    let seeded_paths: std::collections::HashSet<&str> =
        dirs.iter().map(|d| d.path.as_str()).collect();
    for agent in builtin_agents() {
        assert!(
            seeded_paths.contains(agent.global_skills_dir.as_str()),
            "Built-in agent '{}' global_skills_dir '{}' must be in scan_directories",
            agent.id,
            agent.global_skills_dir
        );
    }
}

#[tokio::test]
async fn test_builtin_scan_dirs_seeded_is_idempotent() {
    let pool = setup_test_db().await;
    // Second call to init_database must not create duplicate rows.
    init_database(&pool).await.unwrap();
    let dirs = get_scan_directories(&pool).await.unwrap();
    let builtin_count = expected_builtin_scan_dir_count();
    assert_eq!(
        dirs.len(),
        builtin_count,
        "Repeated init must not create duplicate scan directory rows"
    );
}

#[tokio::test]
async fn test_reinit_updates_stale_builtin_agent_paths() {
    let pool = setup_test_db().await;
    sqlx::query("UPDATE agents SET global_skills_dir = '/tmp/.agents/skills' WHERE id = 'central'")
        .execute(&pool)
        .await
        .unwrap();

    init_database(&pool).await.unwrap();

    let central = get_agent_by_id(&pool, "central")
        .await
        .unwrap()
        .expect("central agent should exist");
    assert_eq!(
        central.global_skills_dir,
        crate::paths::central_skills_dir().to_string_lossy()
    );
}

#[tokio::test]
async fn test_reinit_preserves_existing_builtin_agent_enabled_flags() {
    let pool = setup_test_db().await;

    sqlx::query(
        "UPDATE agents
         SET is_enabled = CASE id
           WHEN 'claude-code' THEN 0
           WHEN 'cursor' THEN 1
           ELSE is_enabled
         END",
    )
    .execute(&pool)
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let claude_code = get_agent_by_id(&pool, "claude-code")
        .await
        .unwrap()
        .expect("claude-code should exist");
    let cursor = get_agent_by_id(&pool, "cursor")
        .await
        .unwrap()
        .expect("cursor should exist");

    assert!(!claude_code.is_enabled);
    assert!(cursor.is_enabled);
}

#[tokio::test]
async fn test_reinit_replaces_stale_builtin_scan_directory_paths() {
    let pool = setup_test_db().await;
    sqlx::query("DELETE FROM scan_directories WHERE is_builtin = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO scan_directories (path, label, is_active, is_builtin, added_at)
         VALUES ('/tmp/.agents/skills', 'Central Skills', 1, 1, ?)",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let dirs = get_scan_directories(&pool).await.unwrap();
    let central_path = crate::paths::central_skills_dir()
        .to_string_lossy()
        .into_owned();
    assert!(
        dirs.iter().any(|dir| dir.path == central_path),
        "reinit should seed the resolved central skills path"
    );
    assert!(
        !dirs.iter().any(|dir| dir.path == "/tmp/.agents/skills"),
        "stale /tmp builtin scan directory should be removed"
    );
}

#[tokio::test]
async fn test_add_scan_directory() {
    let pool = setup_test_db().await;
    let dir = add_scan_directory(&pool, "/tmp/my-project", Some("My Project"))
        .await
        .unwrap();
    assert_eq!(dir.path, "/tmp/my-project");
    assert_eq!(dir.label.as_deref(), Some("My Project"));
    assert!(dir.is_active);
    assert!(!dir.is_builtin);
}

#[tokio::test]
async fn test_get_scan_directories() {
    let pool = setup_test_db().await;
    add_scan_directory(&pool, "/tmp/dir-a", None).await.unwrap();
    add_scan_directory(&pool, "/tmp/dir-b", Some("Dir B"))
        .await
        .unwrap();

    let dirs = get_scan_directories(&pool).await.unwrap();
    // There are N built-in dirs (seeded on init) plus the 2 we just added.
    let builtin_count = expected_builtin_scan_dir_count();
    assert_eq!(dirs.len(), builtin_count + 2);

    // Verify the custom ones are present.
    let paths: Vec<&str> = dirs.iter().map(|d| d.path.as_str()).collect();
    assert!(paths.contains(&"/tmp/dir-a"));
    assert!(paths.contains(&"/tmp/dir-b"));
}

#[tokio::test]
async fn test_remove_scan_directory() {
    let pool = setup_test_db().await;
    add_scan_directory(&pool, "/tmp/removable", None)
        .await
        .unwrap();
    remove_scan_directory(&pool, "/tmp/removable")
        .await
        .unwrap();

    let dirs = get_scan_directories(&pool).await.unwrap();
    // Built-in dirs remain; only the custom one is removed.
    let builtin_count = expected_builtin_scan_dir_count();
    assert_eq!(dirs.len(), builtin_count);
    assert!(!dirs.iter().any(|d| d.path == "/tmp/removable"));
}

#[tokio::test]
async fn test_cannot_remove_builtin_scan_directory() {
    let pool = setup_test_db().await;
    // Manually insert a builtin directory
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO scan_directories (path, is_active, is_builtin, added_at)
         VALUES ('/builtin/path', 1, 1, ?)",
    )
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    let result = remove_scan_directory(&pool, "/builtin/path").await;
    assert!(
        result.is_err(),
        "Should not remove a builtin scan directory"
    );
}

#[tokio::test]
async fn test_remove_nonexistent_scan_directory_returns_error() {
    let pool = setup_test_db().await;
    let result = remove_scan_directory(&pool, "/nonexistent/path").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_toggle_scan_directory() {
    let pool = setup_test_db().await;
    add_scan_directory(&pool, "/tmp/toggle-dir", None)
        .await
        .unwrap();
    toggle_scan_directory(&pool, "/tmp/toggle-dir", false)
        .await
        .unwrap();

    let dirs = get_scan_directories(&pool).await.unwrap();
    let dir = dirs.iter().find(|d| d.path == "/tmp/toggle-dir").unwrap();
    assert!(!dir.is_active);
}

// ── Settings ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_set_and_get_setting() {
    let pool = setup_test_db().await;
    set_setting(&pool, "theme", "dark").await.unwrap();
    let value = get_setting(&pool, "theme").await.unwrap();
    assert_eq!(value.as_deref(), Some("dark"));
}

#[tokio::test]
async fn test_get_missing_setting_returns_none() {
    let pool = setup_test_db().await;
    let value = get_setting(&pool, "nonexistent_key").await.unwrap();
    assert!(value.is_none());
}

#[tokio::test]
async fn test_set_setting_upserts() {
    let pool = setup_test_db().await;
    set_setting(&pool, "lang", "en").await.unwrap();
    set_setting(&pool, "lang", "zh").await.unwrap();
    let value = get_setting(&pool, "lang").await.unwrap();
    assert_eq!(value.as_deref(), Some("zh"));
}

#[tokio::test]
async fn test_multiple_settings() {
    let pool = setup_test_db().await;
    set_setting(&pool, "key1", "val1").await.unwrap();
    set_setting(&pool, "key2", "val2").await.unwrap();
    set_setting(&pool, "key3", "val3").await.unwrap();

    assert_eq!(
        get_setting(&pool, "key1").await.unwrap().as_deref(),
        Some("val1")
    );
    assert_eq!(
        get_setting(&pool, "key2").await.unwrap().as_deref(),
        Some("val2")
    );
    assert_eq!(
        get_setting(&pool, "key3").await.unwrap().as_deref(),
        Some("val3")
    );
}

#[tokio::test]
async fn test_batch_settings_preserves_missing_keys() {
    let pool = setup_test_db().await;
    let mut values = HashMap::new();
    values.insert("ai_provider".to_string(), "glm".to_string());
    values.insert("ai_model".to_string(), "glm-5".to_string());
    set_settings(&pool, &values).await.unwrap();

    let loaded = get_settings(
        &pool,
        &[
            "ai_provider".to_string(),
            "ai_model".to_string(),
            "ai_api_key".to_string(),
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        loaded.get("ai_provider").and_then(|value| value.as_deref()),
        Some("glm")
    );
    assert_eq!(
        loaded.get("ai_model").and_then(|value| value.as_deref()),
        Some("glm-5")
    );
    assert!(loaded.get("ai_api_key").unwrap().is_none());
}

// ── Migration: created_at ─────────────────────────────────────────────────

/// Verifies that `init_database` adds the `created_at` column to an existing
/// `skill_installations` table that was created with the old schema (before
/// the column was introduced), and that existing rows are backfilled.
#[tokio::test]
async fn test_migration_adds_created_at_to_skill_installations() {
    // Create a fresh in-memory pool WITHOUT calling init_database first.
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");

    // Build the OLD skill_installations schema — no created_at column.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_installations (
            skill_id       TEXT NOT NULL,
            agent_id       TEXT NOT NULL,
            installed_path TEXT NOT NULL,
            link_type      TEXT NOT NULL,
            symlink_target TEXT,
            PRIMARY KEY (skill_id, agent_id)
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create old skill_installations table");

    // Create the skills table so the FK-style relationship is consistent.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skills (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            canonical_path TEXT,
            is_central     BOOLEAN NOT NULL DEFAULT 0,
            source         TEXT,
            content        TEXT,
            scanned_at     TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create skills table");

    // Insert a skill row (needed before the installation row references it).
    sqlx::query(
        "INSERT INTO skills (id, name, file_path, is_central, scanned_at)
         VALUES ('legacy-skill', 'Legacy Skill', '/tmp/legacy-skill/SKILL.md', 0, '2024-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("Failed to insert legacy skill");

    // Insert an installation row using the OLD schema (no created_at column).
    sqlx::query(
        "INSERT INTO skill_installations (skill_id, agent_id, installed_path, link_type)
         VALUES ('legacy-skill', 'claude-code', '/tmp/claude/legacy-skill', 'symlink')",
    )
    .execute(&pool)
    .await
    .expect("Failed to insert legacy skill_installations row");

    // Run init_database — should detect the missing created_at column and add it.
    init_database(&pool)
        .await
        .expect("init_database should succeed and apply the created_at migration");

    // Confirm the column now exists in PRAGMA table_info.
    let columns = sqlx::query("PRAGMA table_info(skill_installations)")
        .fetch_all(&pool)
        .await
        .expect("PRAGMA table_info should succeed");

    let has_created_at = columns.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == "created_at")
            .unwrap_or(false)
    });
    assert!(
        has_created_at,
        "created_at column must exist in skill_installations after migration"
    );

    // Confirm that the pre-existing row has a non-empty created_at value
    // (backfilled by the DEFAULT (datetime('now')) expression).
    let row = sqlx::query(
        "SELECT created_at FROM skill_installations \
         WHERE skill_id = 'legacy-skill' AND agent_id = 'claude-code'",
    )
    .fetch_one(&pool)
    .await
    .expect("Pre-existing installation row should still be queryable after migration");

    let created_at: String = row
        .try_get("created_at")
        .expect("created_at should be readable from the pre-existing row");
    assert!(
        !created_at.is_empty(),
        "Pre-existing rows must have a non-empty created_at value after migration (got: '{}')",
        created_at
    );
}

/// Verifies that calling `init_database` on a fresh database (one that already
/// includes created_at in the CREATE TABLE) does NOT trigger the ALTER TABLE
/// migration path — i.e., the second `init_database` call is fully idempotent
/// and does not fail.
#[tokio::test]
async fn test_migration_skipped_when_created_at_already_exists() {
    // setup_test_db calls init_database, which creates the table WITH created_at.
    let pool = setup_test_db().await;

    // A second call to init_database must succeed without error (idempotent).
    let result = init_database(&pool).await;
    assert!(
        result.is_ok(),
        "Second init_database should be idempotent when created_at already exists"
    );

    // Confirm created_at is still present and there's exactly one occurrence.
    let columns = sqlx::query("PRAGMA table_info(skill_installations)")
        .fetch_all(&pool)
        .await
        .unwrap();
    let created_at_count = columns
        .iter()
        .filter(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "created_at")
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        created_at_count, 1,
        "created_at column should appear exactly once after repeated init"
    );
}
