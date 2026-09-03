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

use crate::test_support::mem_pool as setup_test_db;

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
        "skill_calls",
        "skill_call_providers",
        "skill_call_scan_state",
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

#[tokio::test]
async fn test_init_upgrades_ai_review_proposal_columns() {
    // 豁免 test_support::mem_pool：本测试手工搭建 legacy schema 验证迁移。
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE skill_ai_tag_reviews (
            skill_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            confidence REAL NOT NULL,
            reason TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            suggested_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let columns = table_columns(&pool, "skill_ai_tag_reviews").await;
    assert!(columns.contains(&"proposed_name".to_string()));
    assert!(columns.contains(&"proposed_description".to_string()));
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

#[derive(Debug, PartialEq, Eq)]
struct PragmaColumn {
    type_name: String,
    not_null: i64,
    default_expr: Option<String>,
}

async fn pragma_column(pool: &DbPool, table: &str, column: &str) -> PragmaColumn {
    let row = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.try_get::<String, _>("name").unwrap() == column)
        .unwrap_or_else(|| panic!("{table}.{column} must exist"));
    PragmaColumn {
        type_name: row.try_get::<String, _>("type").unwrap(),
        not_null: row.try_get::<i64, _>("notnull").unwrap(),
        default_expr: row.try_get::<Option<String>, _>("dflt_value").unwrap(),
    }
}

async fn index_is_unique(pool: &DbPool, table: &str, index: &str) -> bool {
    sqlx::query(&format!("PRAGMA index_list({table})"))
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.try_get::<String, _>("name").unwrap() == index)
        .map(|row| row.try_get::<i64, _>("unique").unwrap() == 1)
        .unwrap_or(false)
}

const FINAL_SCHEMA_TEXT_COLUMNS: [(&str, &str); 4] = [
    ("skill_tags", "group_id"),
    ("skill_ai_tag_reviews", "proposed_name"),
    ("skill_ai_tag_reviews", "proposed_description"),
    ("skill_repositories", "last_synced_at"),
];

fn expected_nullable_text_column() -> PragmaColumn {
    PragmaColumn {
        type_name: "TEXT".to_string(),
        not_null: 0,
        default_expr: None,
    }
}

#[tokio::test]
async fn test_init_creates_final_schema_columns_and_unique_uid_index() {
    let pool = setup_test_db().await;
    let expected = expected_nullable_text_column();

    for (table, column) in FINAL_SCHEMA_TEXT_COLUMNS {
        assert_eq!(
            pragma_column(&pool, table, column).await,
            expected,
            "{table}.{column} on a new database must match the base DDL"
        );
    }

    assert!(table_indexes(&pool, "skills")
        .await
        .contains(&"idx_skills_uid".to_string()));
    assert!(
        index_is_unique(&pool, "skills", "idx_skills_uid").await,
        "idx_skills_uid must be unique on a new database"
    );
}

#[tokio::test]
async fn test_init_upgrades_missing_final_schema_columns() {
    // 豁免 test_support::mem_pool：本测试手工搭建缺列旧库验证 ensure_column 升级。
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE skill_repositories (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            owner TEXT,
            repo TEXT,
            branch TEXT,
            url TEXT,
            pinned BOOLEAN NOT NULL DEFAULT 0,
            is_unknown BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE skill_tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            color TEXT,
            is_builtin BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE skill_ai_tag_reviews (
            skill_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            confidence REAL NOT NULL,
            reason TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            suggested_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let expected = expected_nullable_text_column();
    for (table, column) in FINAL_SCHEMA_TEXT_COLUMNS {
        assert_eq!(
            pragma_column(&pool, table, column).await,
            expected,
            "{table}.{column} after upgrading an old database must match the base DDL"
        );
    }
}

const OWNED_SKILL_RELATION_TABLES: [&str; 7] = [
    "skill_update_states",
    "skill_repository_members",
    "collection_skills",
    "skill_tag_links",
    "skill_ai_tag_reviews",
    "skill_explanations",
    "skill_installations",
];

async fn insert_owned_skill_relation_rows(pool: &DbPool, skill_id: &str) {
    let now = Utc::now().to_rfc3339();
    let repository_id = format!("repo-{skill_id}");

    sqlx::query(
        "INSERT INTO skill_update_states (skill_id, source_type, status) VALUES (?, 'github', 'up_to_date')",
    )
    .bind(skill_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, pinned, is_unknown, created_at, updated_at)
         VALUES (?, ?, 'github', 0, 0, ?, ?)",
    )
    .bind(&repository_id)
    .bind(&repository_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_repository_members
         (skill_id, repository_id, source_path, added_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(skill_id)
    .bind(&repository_id)
    .bind(format!("skills/{skill_id}"))
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO collection_skills (collection_id, skill_id, added_at) VALUES (?, ?, ?)",
    )
    .bind(format!("collection-{skill_id}"))
    .bind(skill_id)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_tag_links
         (skill_id, tag_id, source, added_at) VALUES (?, 'uncategorized', 'manual', ?)",
    )
    .bind(skill_id)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_ai_tag_reviews
         (skill_id, tag_id, confidence, status, suggested_at, updated_at)
         VALUES (?, 'uncategorized', 0.5, 'pending', ?, ?)",
    )
    .bind(skill_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_explanations
         (skill_id, explanation, lang, model, created_at, updated_at)
         VALUES (?, 'fixture', 'en', 'fixture-model', ?, ?)",
    )
    .bind(skill_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_installations
         (skill_id, agent_id, installed_path, link_type, created_at)
         VALUES (?, 'fixture-agent', ?, 'copy', ?)",
    )
    .bind(skill_id)
    .bind(format!("/tmp/{skill_id}"))
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_owned_skill_relation_counts(pool: &DbPool, skill_id: &str, expected: i64) {
    for table in OWNED_SKILL_RELATION_TABLES {
        let count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} WHERE skill_id = ?"
        ))
        .bind(skill_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(count, expected, "unexpected {table} count for {skill_id}");
    }
}

async fn assert_no_owned_skill_relation_orphans(pool: &DbPool) {
    for table in OWNED_SKILL_RELATION_TABLES {
        let orphan_count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*)
             FROM {table} relation
             LEFT JOIN skills ON skills.id = relation.skill_id
             WHERE skills.id IS NULL"
        ))
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            orphan_count, 0,
            "{table} must pass the skill-parent FK preflight predicate"
        );
    }
}

async fn insert_independent_skill_history_rows(pool: &DbPool, skill_id: &str) {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agent_skill_observations
         (row_id, agent_id, skill_id, name, file_path, dir_path, source_kind, source_root,
          link_type, is_read_only, scanned_at)
         VALUES (?, 'history-agent', ?, ?, ?, ?, 'global', '/tmp', 'copy', 0, ?)",
    )
    .bind(format!("observation-{skill_id}"))
    .bind(skill_id)
    .bind(skill_id)
    .bind(format!("/tmp/{skill_id}/SKILL.md"))
    .bind(format!("/tmp/{skill_id}"))
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO projects (id, path, name, added_at) VALUES (?, ?, ?, ?)")
        .bind(format!("project-{skill_id}"))
        .bind(format!("/tmp/project-{skill_id}"))
        .bind(format!("Project {skill_id}"))
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO project_skill_installations
         (project_id, skill_id, name, file_path, agent_id, installed_path, link_type, created_at)
         VALUES (?, ?, ?, ?, 'history-agent', ?, 'copy', ?)",
    )
    .bind(format!("project-{skill_id}"))
    .bind(skill_id)
    .bind(skill_id)
    .bind(format!("/tmp/project-{skill_id}/SKILL.md"))
    .bind(format!("/tmp/project-{skill_id}"))
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_calls
         (target_id, skill, timestamp_ms, project, session_id, source)
         VALUES ('local', ?, 1, 'fixture', ?, 'fixture')",
    )
    .bind(skill_id)
    .bind(format!("session-{skill_id}"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_usage_metadata
         (target_id, skill, match_status, resolved_skill_id, scanned_at_ms)
         VALUES ('local', ?, 'matched', ?, 1)",
    )
    .bind(skill_id)
    .bind(skill_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_independent_skill_history_rows(pool: &DbPool, skill_id: &str) {
    for (table, column) in [
        ("agent_skill_observations", "skill_id"),
        ("project_skill_installations", "skill_id"),
        ("skill_calls", "skill"),
        ("skill_usage_metadata", "resolved_skill_id"),
    ] {
        let count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} WHERE {column} = ?"
        ))
        .bind(skill_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "{table} must retain independent history");
    }
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
    assert_eq!(agents.len(), 37, "Should have exactly 37 built-in agents");

    let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
    // Coding platforms
    assert!(ids.contains(&"claude-code"));
    assert!(ids.contains(&"codex"));
    assert!(ids.contains(&"grok"));
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
    assert!(ids.contains(&"antigravity-cli"));
    assert!(ids.contains(&"zed"));
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
    assert!(ids.contains(&"reasonix"));
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

    let antigravity = agents
        .iter()
        .find(|agent| agent.id == "antigravity")
        .expect("antigravity agent should exist");
    assert_eq!(antigravity.display_name, "Antigravity");
    assert!(
        !crate::paths::paths_equivalent(Path::new(&antigravity.global_skills_dir), &universal_dir),
        "antigravity global skills should stay separate from ~/.agents/skills"
    );
    assert!(
        antigravity
            .global_skills_dir
            .replace('\\', "/")
            .ends_with(".gemini/antigravity/skills"),
        "antigravity should use ~/.gemini/antigravity/skills"
    );
    assert_eq!(
        antigravity.project_skills_dir.as_deref(),
        Some(UNIVERSAL_PROJECT_SKILLS_DIR)
    );

    let grok = agents
        .iter()
        .find(|agent| agent.id == "grok")
        .expect("grok agent should exist");
    assert_eq!(grok.display_name, "Grok");
    assert_eq!(grok.category, "coding");
    assert_eq!(grok.icon_name.as_deref(), Some("grok"));
    assert!(
        !crate::paths::paths_equivalent(Path::new(&grok.global_skills_dir), &universal_dir),
        "grok global skills should stay separate from ~/.agents/skills"
    );
    assert!(
        grok.global_skills_dir
            .replace('\\', "/")
            .ends_with(".grok/skills"),
        "grok should use ~/.grok/skills"
    );
    assert_eq!(grok.project_skills_dir.as_deref(), Some(".grok/skills"));

    let antigravity_cli = agents
        .iter()
        .find(|agent| agent.id == "antigravity-cli")
        .expect("antigravity-cli agent should exist");
    assert_eq!(antigravity_cli.display_name, "Antigravity CLI");
    assert_ne!(antigravity_cli.id, antigravity.id);
    assert_ne!(
        antigravity_cli.global_skills_dir,
        antigravity.global_skills_dir
    );
    assert!(
        !crate::paths::paths_equivalent(
            Path::new(&antigravity_cli.global_skills_dir),
            &universal_dir
        ),
        "antigravity-cli global skills should stay separate from ~/.agents/skills"
    );
    assert!(
        antigravity_cli
            .global_skills_dir
            .replace('\\', "/")
            .ends_with(".gemini/antigravity-cli/skills"),
        "antigravity-cli should use ~/.gemini/antigravity-cli/skills"
    );
    assert_eq!(
        antigravity_cli.project_skills_dir.as_deref(),
        Some(UNIVERSAL_PROJECT_SKILLS_DIR)
    );

    let zed = agents
        .iter()
        .find(|agent| agent.id == "zed")
        .expect("zed agent should exist");
    assert_eq!(zed.display_name, "Zed");
    assert!(
        !crate::paths::paths_equivalent(Path::new(&zed.global_skills_dir), &universal_dir),
        "zed should use its community-compatible skills directory, not ~/.agents/skills"
    );
    assert!(
        zed.global_skills_dir
            .replace('\\', "/")
            .ends_with(".config/zed/skills"),
        "zed should use ~/.config/zed/skills"
    );
    assert_eq!(zed.project_skills_dir.as_deref(), None);

    let gemini_cli = agents
        .iter()
        .find(|agent| agent.id == "gemini-cli")
        .expect("gemini-cli agent should exist");
    assert_eq!(gemini_cli.display_name, "Gemini CLI (legacy)");
    assert!(
        !crate::paths::paths_equivalent(Path::new(&gemini_cli.global_skills_dir), &universal_dir),
        "gemini-cli should carry the legacy/shared Google target, not ~/.agents/skills"
    );
    assert!(
        gemini_cli
            .global_skills_dir
            .replace('\\', "/")
            .ends_with(".gemini/skills"),
        "gemini-cli should use ~/.gemini/skills"
    );
}

#[test]
fn test_remote_builtin_agents_rewrite_google_platform_paths() {
    let agents = builtin_agents_for_posix_home("/home/alice");

    let grok = agents
        .iter()
        .find(|agent| agent.id == "grok")
        .expect("grok agent should exist");
    let antigravity = agents
        .iter()
        .find(|agent| agent.id == "antigravity")
        .expect("antigravity agent should exist");
    let antigravity_cli = agents
        .iter()
        .find(|agent| agent.id == "antigravity-cli")
        .expect("antigravity-cli agent should exist");
    let gemini_cli = agents
        .iter()
        .find(|agent| agent.id == "gemini-cli")
        .expect("gemini-cli agent should exist");

    assert_eq!(grok.global_skills_dir, "/home/alice/.grok/skills");
    assert_eq!(grok.project_skills_dir.as_deref(), Some(".grok/skills"));
    assert_eq!(
        antigravity.global_skills_dir,
        "/home/alice/.gemini/antigravity/skills"
    );
    assert_eq!(
        antigravity_cli.global_skills_dir,
        "/home/alice/.gemini/antigravity-cli/skills"
    );
    assert_eq!(gemini_cli.global_skills_dir, "/home/alice/.gemini/skills");
    assert_eq!(
        antigravity_cli.project_skills_dir.as_deref(),
        Some(UNIVERSAL_PROJECT_SKILLS_DIR)
    );
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
        "grok",
        "antigravity",
        "antigravity-cli",
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
    assert_eq!(agents.len(), 37, "Reinit must not duplicate agents");
}

// ── Skills ────────────────────────────────────────────────────────────────

fn make_skill(id: &str, name: &str, is_central: bool) -> Skill {
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
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

    let original_uid = skill.uid.clone();
    skill.uid = "replacement-uid-must-not-win".to_string();
    skill.name = "Updated Name".to_string();
    upsert_skill(&pool, &skill).await.unwrap();

    let retrieved = get_skill_by_id(&pool, "skill-1").await.unwrap().unwrap();
    assert_eq!(retrieved.name, "Updated Name");
    assert_eq!(retrieved.uid, original_uid);
}

#[tokio::test]
async fn test_recreated_skill_slug_gets_a_new_uid() {
    let pool = setup_test_db().await;
    let first = make_skill("recreated", "First", true);
    upsert_skill(&pool, &first).await.unwrap();
    delete_skill(&pool, &first.id).await.unwrap();

    let mut recreated = make_skill("recreated", "Second", true);
    recreated.uid = "recreated-second-uid".to_string();
    upsert_skill(&pool, &recreated).await.unwrap();

    let stored = get_skill_by_id(&pool, "recreated").await.unwrap().unwrap();
    assert_eq!(stored.uid, recreated.uid);
    assert_ne!(stored.uid, first.uid);
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
    assert_eq!(retrieved.uid, central.uid);
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
    upsert_skill(&pool, &make_skill("central-1", "Central One", true))
        .await
        .unwrap();
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
        status: SkillUpdateStatus::UpdateAvailable,
        error: None,
    };

    upsert_skill_update_state(&pool, &state).await.unwrap();
    let states = get_skill_update_states_for_skills(&pool, &["central-1".to_string()])
        .await
        .unwrap();

    assert_eq!(states.len(), 1);
    assert_eq!(states[0].skill_id, "central-1");
    assert_eq!(states[0].status, SkillUpdateStatus::UpdateAvailable);
    assert_eq!(states[0].latest_remote_hash.as_deref(), Some("fnv1a64:new"));
}

#[tokio::test]
async fn test_skill_update_status_decodes_existing_values_and_rejects_unknown() {
    let pool = setup_test_db().await;
    let cases = [
        ("up_to_date", SkillUpdateStatus::UpToDate),
        ("update_available", SkillUpdateStatus::UpdateAvailable),
        ("unsupported", SkillUpdateStatus::Unsupported),
        ("remote_missing", SkillUpdateStatus::RemoteMissing),
        ("error", SkillUpdateStatus::Error),
        ("cancelled", SkillUpdateStatus::Cancelled),
    ];

    for (index, (persisted, _)) in cases.iter().enumerate() {
        let skill_id = format!("status-{index}");
        upsert_skill(&pool, &make_skill(&skill_id, persisted, true))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO skill_update_states (skill_id, source_type, status) VALUES (?, 'github', ?)",
        )
        .bind(skill_id)
        .bind(persisted)
        .execute(&pool)
        .await
        .unwrap();
    }

    let states = get_skill_update_states(&pool).await.unwrap();
    let statuses = states
        .into_iter()
        .map(|state| (state.skill_id, state.status))
        .collect::<HashMap<_, _>>();
    for (index, (_, expected)) in cases.iter().enumerate() {
        assert_eq!(statuses.get(&format!("status-{index}")), Some(expected));
    }

    upsert_skill(&pool, &make_skill("status-invalid", "Invalid", true))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO skill_update_states (skill_id, source_type, status) VALUES ('status-invalid', 'github', 'future_status')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = get_skill_update_states_for_skills(&pool, &["status-invalid".to_string()])
        .await
        .unwrap_err();
    assert!(error.to_string().contains("future_status"));
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
        &[PendingAiTagReviewInput {
            tag_id: tag.id.clone(),
            confidence: 0.42,
            reason: "review".to_string(),
            proposed_name: None,
            proposed_description: None,
        }],
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
            status: SkillUpdateStatus::UpdateAvailable,
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
    insert_independent_skill_history_rows(&pool, "to-delete").await;

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
    assert_independent_skill_history_rows(&pool, "to-delete").await;
}

#[tokio::test]
async fn delete_skill_rolls_back_when_owned_relation_delete_fails() {
    let pool = setup_test_db().await;
    upsert_skill(&pool, &make_skill("rollback-skill", "Rollback Skill", true))
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "rollback-skill").await;

    sqlx::query(
        "CREATE TRIGGER fail_ai_review_delete
         BEFORE DELETE ON skill_ai_tag_reviews
         WHEN OLD.skill_id = 'rollback-skill'
         BEGIN SELECT RAISE(ABORT, 'injected relation delete failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = delete_skill(&pool, "rollback-skill").await.unwrap_err();
    assert!(error
        .to_string()
        .contains("injected relation delete failure"));
    assert!(get_skill_by_id(&pool, "rollback-skill")
        .await
        .unwrap()
        .is_some());
    assert_owned_skill_relation_counts(&pool, "rollback-skill", 1).await;
    assert!(get_skill_repository_by_id(&pool, "repo-rollback-skill")
        .await
        .unwrap()
        .is_some());

    sqlx::query("DROP TRIGGER fail_ai_review_delete")
        .execute(&pool)
        .await
        .unwrap();
    delete_skill(&pool, "rollback-skill").await.unwrap();
    assert_owned_skill_relation_counts(&pool, "rollback-skill", 0).await;
    assert!(get_skill_repository_by_id(&pool, "repo-rollback-skill")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn delete_skill_then_reuse_id_does_not_restore_owned_metadata() {
    let pool = setup_test_db().await;
    upsert_skill(&pool, &make_skill("reused-skill", "Original", true))
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "reused-skill").await;
    insert_independent_skill_history_rows(&pool, "reused-skill").await;

    delete_skill(&pool, "reused-skill").await.unwrap();
    upsert_skill(&pool, &make_skill("reused-skill", "Replacement", true))
        .await
        .unwrap();

    assert_owned_skill_relation_counts(&pool, "reused-skill", 0).await;
    assert_independent_skill_history_rows(&pool, "reused-skill").await;
    assert_eq!(
        get_skill_by_id(&pool, "reused-skill")
            .await
            .unwrap()
            .unwrap()
            .name,
        "Replacement"
    );
}

#[tokio::test]
async fn delete_skills_not_in_scope_cleans_all_owned_relations_for_nonempty_keep_set() {
    let pool = setup_test_db().await;
    for skill_id in ["keep-skill", "stale-skill"] {
        upsert_skill(&pool, &make_skill(skill_id, skill_id, true))
            .await
            .unwrap();
        insert_owned_skill_relation_rows(&pool, skill_id).await;
    }
    insert_independent_skill_history_rows(&pool, "stale-skill").await;

    delete_skills_not_in_scope(&pool, &["keep-skill".to_string()])
        .await
        .unwrap();

    assert_owned_skill_relation_counts(&pool, "keep-skill", 1).await;
    assert_owned_skill_relation_counts(&pool, "stale-skill", 0).await;
    assert!(get_skill_by_id(&pool, "keep-skill")
        .await
        .unwrap()
        .is_some());
    assert!(get_skill_by_id(&pool, "stale-skill")
        .await
        .unwrap()
        .is_none());
    assert_independent_skill_history_rows(&pool, "stale-skill").await;
}

#[tokio::test]
async fn delete_skills_not_in_scope_cleans_all_owned_relations_for_empty_keep_set() {
    let pool = setup_test_db().await;
    upsert_skill(&pool, &make_skill("only-skill", "Only Skill", true))
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "only-skill").await;

    delete_skills_not_in_scope(&pool, &[]).await.unwrap();

    assert_owned_skill_relation_counts(&pool, "only-skill", 0).await;
    assert!(get_skill_by_id(&pool, "only-skill")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn orphan_repair_reports_audits_and_cleans_all_owned_relations() {
    let pool = crate::test_support::mem_pool_single_conn().await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "orphan-z").await;
    insert_owned_skill_relation_rows(&pool, "orphan-a").await;

    let report = repair_orphan_skill_relations(&pool).await.unwrap();

    assert_eq!(report.total_rows, 14);
    assert_eq!(report.relations.len(), OWNED_SKILL_RELATION_TABLES.len());
    for (relation, expected_table) in report.relations.iter().zip(OWNED_SKILL_RELATION_TABLES) {
        assert_eq!(relation.table, expected_table);
        assert_eq!(relation.skill_ids, ["orphan-a", "orphan-z"]);
        assert_eq!(relation.row_count, 2);
    }
    assert_owned_skill_relation_counts(&pool, "orphan-a", 0).await;
    assert_owned_skill_relation_counts(&pool, "orphan-z", 0).await;
    assert_no_owned_skill_relation_orphans(&pool).await;

    let logs = list_operation_logs(
        &pool,
        OperationLogFilter {
            action: Some("orphan_repair".to_string()),
            ..OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(logs.total, 1);
    assert_eq!(logs.entries[0].category, "database");
    let expected_json = concat!(
        r#"{"relations":[{"table":"skill_update_states","skillIds":["orphan-a","orphan-z"],"rowCount":2},"#,
        r#"{"table":"skill_repository_members","skillIds":["orphan-a","orphan-z"],"rowCount":2},"#,
        r#"{"table":"collection_skills","skillIds":["orphan-a","orphan-z"],"rowCount":2},"#,
        r#"{"table":"skill_tag_links","skillIds":["orphan-a","orphan-z"],"rowCount":2},"#,
        r#"{"table":"skill_ai_tag_reviews","skillIds":["orphan-a","orphan-z"],"rowCount":2},"#,
        r#"{"table":"skill_explanations","skillIds":["orphan-a","orphan-z"],"rowCount":2},"#,
        r#"{"table":"skill_installations","skillIds":["orphan-a","orphan-z"],"rowCount":2}],"totalRows":14}"#,
    );
    assert_eq!(logs.entries[0].details_json.as_deref(), Some(expected_json));
    let audited_report: OrphanRepairReport =
        serde_json::from_str(logs.entries[0].details_json.as_deref().unwrap()).unwrap();
    assert_eq!(audited_report, report);

    let second_report = repair_orphan_skill_relations(&pool).await.unwrap();
    assert_eq!(
        second_report,
        OrphanRepairReport {
            relations: Vec::new(),
            total_rows: 0,
        }
    );
    let log_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM operation_logs WHERE action = 'orphan_repair'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(log_count, 1, "zero-row repair must not create audit noise");
}

#[tokio::test]
async fn orphan_repair_rolls_back_when_audit_insert_fails() {
    let pool = crate::test_support::mem_pool_single_conn().await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "audit-failure-orphan").await;
    sqlx::query(
        "CREATE TRIGGER fail_orphan_repair_audit
         BEFORE INSERT ON operation_logs
         WHEN NEW.action = 'orphan_repair'
         BEGIN SELECT RAISE(ABORT, 'injected audit failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = repair_orphan_skill_relations(&pool).await.unwrap_err();
    assert!(error.to_string().contains("injected audit failure"));
    assert_owned_skill_relation_counts(&pool, "audit-failure-orphan", 1).await;
    let log_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM operation_logs WHERE action = 'orphan_repair'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(log_count, 0);

    sqlx::query("DROP TRIGGER fail_orphan_repair_audit")
        .execute(&pool)
        .await
        .unwrap();
    repair_orphan_skill_relations(&pool).await.unwrap();
    assert_owned_skill_relation_counts(&pool, "audit-failure-orphan", 0).await;
    let log_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM operation_logs WHERE action = 'orphan_repair'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(log_count, 1);
}

#[tokio::test]
async fn orphan_repair_rolls_back_audit_when_relation_delete_fails() {
    let pool = crate::test_support::mem_pool_single_conn().await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "delete-failure-orphan").await;
    sqlx::query(
        "CREATE TRIGGER fail_orphan_relation_delete
         BEFORE DELETE ON skill_ai_tag_reviews
         WHEN OLD.skill_id = 'delete-failure-orphan'
         BEGIN SELECT RAISE(ABORT, 'injected orphan delete failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = repair_orphan_skill_relations(&pool).await.unwrap_err();
    assert!(error.to_string().contains("injected orphan delete failure"));
    assert_owned_skill_relation_counts(&pool, "delete-failure-orphan", 1).await;
    let log_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM operation_logs WHERE action = 'orphan_repair'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        log_count, 0,
        "audit insert must roll back with orphan deletes"
    );

    sqlx::query("DROP TRIGGER fail_orphan_relation_delete")
        .execute(&pool)
        .await
        .unwrap();
    repair_orphan_skill_relations(&pool).await.unwrap();
    assert_owned_skill_relation_counts(&pool, "delete-failure-orphan", 0).await;
}

#[tokio::test]
async fn init_database_rejects_orphans_after_fk_migration() {
    let pool = crate::test_support::mem_pool_single_conn().await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .unwrap();
    insert_owned_skill_relation_rows(&pool, "startup-orphan").await;

    let error = init_database(&pool).await.unwrap_err();
    assert!(error.to_string().contains("foreign key validation failed"));

    assert_owned_skill_relation_counts(&pool, "startup-orphan", 1).await;
    let log_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM operation_logs WHERE action = 'orphan_repair'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(log_count, 0);
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
async fn test_upsert_skill_installation_rejects_invalid_link_type() {
    let pool = setup_test_db().await;
    let skill = make_skill("bad-link", "Bad Link", false);
    upsert_skill(&pool, &skill).await.unwrap();

    let err = upsert_skill_installation(&pool, &make_installation("bad-link", "cursor", "weird"))
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Unsupported link_type"));
}

#[tokio::test]
async fn test_upsert_agent_skill_observation_rejects_invalid_link_type() {
    let pool = setup_test_db().await;

    let err = upsert_agent_skill_observation(
        &pool,
        &AgentSkillObservation {
            row_id: "row-1".to_string(),
            agent_id: "cursor".to_string(),
            skill_id: "bad-link".to_string(),
            name: "Bad Link".to_string(),
            description: None,
            file_path: "/tmp/cursor/bad-link/SKILL.md".to_string(),
            dir_path: "/tmp/cursor/bad-link".to_string(),
            source_kind: "user".to_string(),
            source_root: "/tmp/cursor".to_string(),
            link_type: "broken".to_string(),
            symlink_target: None,
            is_read_only: false,
            scanned_at: Utc::now().to_rfc3339(),
            fs_created_at: None,
            fs_updated_at: None,
        },
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("Unsupported link_type"));
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
    assert_eq!(all.len(), 38, "Should have 37 builtins + 1 custom");

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
    assert!(tags
        .iter()
        .any(|tag| tag.id == ACADEMIC_RESEARCH_WRITING_TAG_ID));
    assert!(!tags
        .iter()
        .any(|tag| tag.id == "programming-agent-engineering"));

    let expected_builtin_ids = [
        ACADEMIC_RESEARCH_WRITING_TAG_ID,
        "frontend-development",
        "backend-development",
        "devops-deployment",
        "testing-quality",
        "docs-writing",
        "data-analysis",
        "design-ui",
        "ai-prompt-engineering",
        "productivity-tools",
        "office-documents",
        UNCATEGORIZED_TAG_ID,
    ];
    for id in expected_builtin_ids {
        assert!(
            tags.iter().any(|tag| tag.id == id && tag.is_builtin),
            "expected built-in tag {id}"
        );
    }

    sqlx::query(
        "UPDATE skill_tags
         SET name = '旧后端分类', description = 'stale', color = '#000000'
         WHERE id = 'backend-development'",
    )
    .execute(&pool)
    .await
    .unwrap();
    init_database(&pool).await.unwrap();
    let refreshed = get_skill_tags(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|tag| tag.id == "backend-development")
        .unwrap();
    assert_eq!(refreshed.name, "后端开发");
    assert_eq!(
        refreshed.description.as_deref(),
        Some("Server-side APIs, databases, business logic, system services.")
    );
    assert_eq!(refreshed.color.as_deref(), Some("#8b5cf6"));
}

#[tokio::test]
async fn test_init_preserves_custom_tag_when_builtin_id_conflicts() {
    let pool = setup_test_db().await;
    let skill = make_skill("custom-id-conflict", "Custom Id Conflict", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let now = Utc::now().to_rfc3339();

    sqlx::query("DELETE FROM skill_tags WHERE id = ?")
        .bind("frontend-development")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO skill_tags
         (id, name, description, color, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, ?, ?)",
    )
    .bind("frontend-development")
    .bind("我的前端分类")
    .bind("User-owned description")
    .bind("#123456")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();
    assign_skill_tags(
        &pool,
        std::slice::from_ref(&skill.id),
        &["frontend-development".to_string()],
        "manual",
        None,
        None,
    )
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let tags = get_skill_tags(&pool).await.unwrap();
    let tag = tags
        .iter()
        .find(|tag| tag.id == "frontend-development")
        .unwrap();
    assert!(!tag.is_builtin);
    assert_eq!(tag.name, "我的前端分类");
    assert_eq!(tag.description.as_deref(), Some("User-owned description"));
    let linked = get_skill_tags_for_skill(&pool, &skill.id).await.unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, "frontend-development");
}

#[tokio::test]
async fn test_init_preserves_custom_tag_when_builtin_name_conflicts() {
    let pool = setup_test_db().await;
    sqlx::query("DELETE FROM skill_tags WHERE id = ?")
        .bind("frontend-development")
        .execute(&pool)
        .await
        .unwrap();
    let custom = create_skill_tag(&pool, "前端开发", Some("用户自己的分类"), None)
        .await
        .unwrap();

    init_database(&pool).await.unwrap();

    let tags = get_skill_tags(&pool).await.unwrap();
    let preserved = tags.iter().find(|tag| tag.id == custom.id).unwrap();
    assert!(!preserved.is_builtin);
    assert_eq!(preserved.name, "前端开发");
    assert_eq!(preserved.description.as_deref(), Some("用户自己的分类"));
    assert!(!tags.iter().any(|tag| tag.id == "frontend-development"));
}

#[tokio::test]
async fn test_init_prunes_obsolete_builtin_skill_tags_only() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();
    let skill = make_skill("obsolete-tag-skill", "Obsolete Tag Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let custom = create_skill_tag(&pool, "用户自定义", None, None)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO skill_tags
         (id, name, description, color, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 1, ?, ?)",
    )
    .bind("programming-agent-engineering")
    .bind("编程与 Agent 工程")
    .bind("Retired default tag")
    .bind("#7c3aed")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();
    assign_skill_tags(
        &pool,
        std::slice::from_ref(&skill.id),
        &[
            "programming-agent-engineering".to_string(),
            custom.id.clone(),
        ],
        "manual",
        None,
        None,
    )
    .await
    .unwrap();
    replace_pending_ai_tag_reviews(
        &pool,
        &skill.id,
        &[
            PendingAiTagReviewInput {
                tag_id: "programming-agent-engineering".to_string(),
                confidence: 0.8,
                reason: "旧默认分类".to_string(),
                proposed_name: None,
                proposed_description: None,
            },
            PendingAiTagReviewInput {
                tag_id: custom.id.clone(),
                confidence: 0.8,
                reason: "自定义分类".to_string(),
                proposed_name: None,
                proposed_description: None,
            },
        ],
    )
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let tags = get_skill_tags(&pool).await.unwrap();
    assert!(!tags
        .iter()
        .any(|tag| tag.id == "programming-agent-engineering"));
    assert!(tags
        .iter()
        .any(|tag| tag.id == "frontend-development" && tag.is_builtin));
    assert!(tags.iter().any(|tag| tag.id == custom.id));
    let linked_tags = get_skill_tags_for_skill(&pool, &skill.id).await.unwrap();
    assert_eq!(linked_tags.len(), 1);
    assert_eq!(linked_tags[0].id, custom.id);
    let reviews = get_pending_ai_tag_reviews(&pool).await.unwrap();
    assert!(reviews
        .iter()
        .all(|review| review.tag.id != "programming-agent-engineering"));
    assert!(reviews.iter().any(|review| review.tag.id == custom.id));
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

/// Per-skill GitHub provenance is written in the skill/repository transaction
/// and is never downgraded by a later writer that has no confirmed snapshot
/// (Central update, CLI, portable state), so a skipped or non-preview write
/// cannot erase a known commit/digest pair.
#[tokio::test]
async fn test_github_provenance_is_written_once_and_preserved_by_later_writers() {
    let pool = setup_test_db().await;
    let skill = make_skill("provenance-skill", "Provenance Skill", true);

    // Before any provenance-aware import the row is absent: provenance unknown.
    assert!(get_skill_repository_provenance(&pool, "provenance-skill")
        .await
        .unwrap()
        .is_none());

    upsert_skill_with_github_repository(
        &pool,
        &skill,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "skills/provenance-skill",
        Some("1234567890abcdef1234567890abcdef12345678"),
        Some("sha256-v1:aa"),
    )
    .await
    .unwrap();

    assert_eq!(
        get_skill_repository_provenance(&pool, "provenance-skill")
            .await
            .unwrap(),
        Some((
            Some("1234567890abcdef1234567890abcdef12345678".to_string()),
            Some("sha256-v1:aa".to_string()),
        ))
    );

    // A later writer without a confirmed snapshot must not clear provenance.
    upsert_skill_with_github_repository(
        &pool,
        &skill,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "skills/provenance-skill",
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        get_skill_repository_provenance(&pool, "provenance-skill")
            .await
            .unwrap(),
        Some((
            Some("1234567890abcdef1234567890abcdef12345678".to_string()),
            Some("sha256-v1:aa".to_string()),
        ))
    );

    // A skill that was never imported through a preview snapshot stays unknown.
    let other = make_skill("unknown-provenance-skill", "Unknown", true);
    upsert_skill(&pool, &other).await.unwrap();
    assign_github_repository_to_skill(
        &pool,
        "openai",
        "skills",
        "main",
        "https://github.com/openai/skills",
        "unknown-provenance-skill",
        "skills/unknown-provenance-skill",
    )
    .await
    .unwrap();
    assert_eq!(
        get_skill_repository_provenance(&pool, "unknown-provenance-skill")
            .await
            .unwrap(),
        Some((None, None))
    );
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

    assert!(error.to_string().contains("cannot be pinned"));
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

    assert!(error.to_string().contains("cannot be deleted"));
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
async fn test_create_skill_tag_is_atomic_for_concurrent_same_name() {
    let (pool, _dir) = crate::test_support::file_pool().await;
    let (left, right) = tokio::join!(
        create_skill_tag(&pool, "并发分类", Some("left"), None),
        create_skill_tag(&pool, "并发分类", Some("right"), None),
    );
    let left = left.unwrap();
    let right = right.unwrap();

    assert_eq!(left.id, right.id);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_tags WHERE name = ?")
        .bind("并发分类")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_create_skill_tag_falls_back_when_normalized_id_is_taken() {
    let pool = setup_test_db().await;
    let existing = create_skill_tag(&pool, "collision-name", None, None)
        .await
        .unwrap();

    let created = create_skill_tag(&pool, "collision name", None, None)
        .await
        .unwrap();

    assert_eq!(existing.id, "collision-name");
    assert_ne!(created.id, existing.id);
    assert_eq!(created.name, "collision name");
}

#[tokio::test]
async fn test_proposal_review_round_trip_does_not_create_tag() {
    let pool = setup_test_db().await;
    let skill = make_skill("proposal-skill", "Proposal Skill", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let proposal_id = derive_skill_tag_id("新领域");

    replace_pending_ai_tag_reviews(
        &pool,
        &skill.id,
        &[PendingAiTagReviewInput {
            tag_id: proposal_id.clone(),
            confidence: 0.91,
            reason: "缺少现有分类".to_string(),
            proposed_name: Some("新领域".to_string()),
            proposed_description: Some("A new workflow category.".to_string()),
        }],
    )
    .await
    .unwrap();

    assert!(get_skill_tag_by_name(&pool, "新领域")
        .await
        .unwrap()
        .is_none());
    let reviews = get_pending_ai_tag_reviews(&pool).await.unwrap();
    assert_eq!(reviews.len(), 1);
    assert!(reviews[0].is_proposal);
    assert_eq!(reviews[0].tag.id, proposal_id);
    assert_eq!(reviews[0].tag.name, "新领域");
    assert_eq!(
        reviews[0].tag.description.as_deref(),
        Some("A new workflow category.")
    );
}

#[tokio::test]
async fn test_pending_reviews_filter_orphans_without_proposal_metadata() {
    let pool = crate::test_support::mem_pool_single_conn().await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .unwrap();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO skill_ai_tag_reviews
         (skill_id, tag_id, confidence, reason, status, suggested_at, updated_at)
         VALUES ('orphan-skill', 'missing-tag', 0.5, 'orphan', 'pending', ?, ?)",
    )
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    assert!(get_pending_ai_tag_reviews(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_accepting_same_name_proposals_reuses_one_tag_for_multiple_skills() {
    let pool = setup_test_db().await;
    let proposal_id = derive_skill_tag_id("安全审计");
    for skill_id in ["proposal-a", "proposal-b"] {
        upsert_skill(&pool, &make_skill(skill_id, skill_id, true))
            .await
            .unwrap();
        replace_pending_ai_tag_reviews(
            &pool,
            skill_id,
            &[PendingAiTagReviewInput {
                tag_id: proposal_id.clone(),
                confidence: 0.95,
                reason: "安全工作流".to_string(),
                proposed_name: Some("安全审计".to_string()),
                proposed_description: Some("Security auditing workflows.".to_string()),
            }],
        )
        .await
        .unwrap();
    }

    accept_ai_tag_reviews(&pool, "proposal-a", std::slice::from_ref(&proposal_id))
        .await
        .unwrap();
    accept_ai_tag_reviews(&pool, "proposal-b", std::slice::from_ref(&proposal_id))
        .await
        .unwrap();

    let created = get_skill_tag_by_name(&pool, "安全审计")
        .await
        .unwrap()
        .unwrap();
    assert!(!created.is_builtin);
    assert_eq!(created.id, proposal_id);
    for skill_id in ["proposal-a", "proposal-b"] {
        let tags = get_skill_tags_for_skill(&pool, skill_id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].id, created.id);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill_tags WHERE name = '安全审计'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_skipping_proposal_leaves_no_tag_or_link() {
    let pool = setup_test_db().await;
    let skill = make_skill("proposal-skip", "Proposal Skip", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let proposal_id = derive_skill_tag_id("临时分类");
    replace_pending_ai_tag_reviews(
        &pool,
        &skill.id,
        &[PendingAiTagReviewInput {
            tag_id: proposal_id,
            confidence: 0.88,
            reason: "待确认".to_string(),
            proposed_name: Some("临时分类".to_string()),
            proposed_description: Some("Temporary workflows.".to_string()),
        }],
    )
    .await
    .unwrap();

    skip_ai_tag_reviews(&pool, &skill.id).await.unwrap();

    assert!(get_skill_tag_by_name(&pool, "临时分类")
        .await
        .unwrap()
        .is_none());
    assert!(get_skill_tags_for_skill(&pool, &skill.id)
        .await
        .unwrap()
        .is_empty());
    let status: String = sqlx::query_scalar(
        "SELECT status FROM skill_ai_tag_reviews WHERE skill_id = 'proposal-skip'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "skipped");
}

#[tokio::test]
async fn test_unassign_skill_tags_removes_only_target_links() {
    let pool = setup_test_db().await;
    let skill = make_skill("skill-a", "Skill A", true);
    upsert_skill(&pool, &skill).await.unwrap();
    let tag_keep = create_skill_tag(&pool, "keep", None, None).await.unwrap();
    let tag_drop = create_skill_tag(&pool, "drop", None, None).await.unwrap();

    assign_skill_tags(
        &pool,
        &["skill-a".to_string()],
        &[tag_keep.id.clone(), tag_drop.id.clone()],
        "manual",
        None,
        None,
    )
    .await
    .unwrap();

    unassign_skill_tags(&pool, "skill-a", std::slice::from_ref(&tag_drop.id))
        .await
        .unwrap();

    let tags = get_skill_tags_for_skill(&pool, "skill-a").await.unwrap();
    let ids: Vec<String> = tags.into_iter().map(|t| t.id).collect();
    assert!(ids.contains(&tag_keep.id), "kept tag must remain");
    assert!(!ids.contains(&tag_drop.id), "dropped tag must be removed");
}

#[tokio::test]
async fn test_unassign_skill_tags_empty_is_noop() {
    let pool = setup_test_db().await;
    let skill = make_skill("skill-b", "Skill B", true);
    upsert_skill(&pool, &skill).await.unwrap();
    // 空 tag_ids 不应报错、不应影响其它行
    unassign_skill_tags(&pool, "skill-b", &[]).await.unwrap();
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

#[tokio::test]
async fn transactional_detach_remote_source_rolls_back_all_metadata() {
    let pool = setup_test_db().await;
    upsert_skill(
        &pool,
        &make_skill("detach-rollback", "Detach Rollback", true),
    )
    .await
    .unwrap();
    let repository = create_or_update_skill_repository(
        &pool,
        Some("repo-detach-rollback"),
        "Detach Rollback",
        "github",
        None,
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    assign_skills_to_repository(
        &pool,
        &repository.id,
        &["detach-rollback".to_string()],
        None,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_update_states (skill_id, source_type, status)
         VALUES ('detach-rollback', 'github', 'up_to_date')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_detach_member_delete
         BEFORE DELETE ON skill_repository_members
         WHEN OLD.skill_id = 'detach-rollback'
         BEGIN SELECT RAISE(ABORT, 'injected detach failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = detach_skill_remote_source(&pool, "detach-rollback")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("injected detach failure"));
    for table in ["skill_update_states", "skill_repository_members"] {
        let count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} WHERE skill_id = 'detach-rollback'"
        ))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "{table} must roll back");
    }
    assert!(get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_some());

    sqlx::query("DROP TRIGGER fail_detach_member_delete")
        .execute(&pool)
        .await
        .unwrap();
    detach_skill_remote_source(&pool, "detach-rollback")
        .await
        .unwrap();
    assert!(get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn transactional_repository_assignment_validates_first_and_rolls_back_later_chunk() {
    let pool = setup_test_db().await;
    let repository = create_or_update_skill_repository(
        &pool,
        Some("repo-batched-assignment"),
        "Batched Assignment",
        "github",
        None,
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    upsert_skill(&pool, &make_skill("assign-valid", "Assign Valid", true))
        .await
        .unwrap();
    let error = assign_skills_to_repository(
        &pool,
        &repository.id,
        &["assign-valid".to_string(), "assign-missing".to_string()],
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "Skill 'assign-missing' not found");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM skill_repository_members WHERE repository_id = ?")
            .bind(&repository.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);

    let mut skill_ids = Vec::new();
    for index in 0..181 {
        let id = format!("assign-batch-{index:03}");
        upsert_skill(&pool, &make_skill(&id, &id, true))
            .await
            .unwrap();
        skill_ids.push(id);
    }
    sqlx::query(
        "CREATE TRIGGER fail_later_repository_assignment_chunk
         BEFORE INSERT ON skill_repository_members
         WHEN NEW.skill_id = 'assign-batch-180'
         BEGIN SELECT RAISE(ABORT, 'injected later assignment failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = assign_skills_to_repository(&pool, &repository.id, &skill_ids, None)
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected later assignment failure"));
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM skill_repository_members WHERE repository_id = ?")
            .bind(&repository.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "earlier assignment chunk must roll back");
}

#[tokio::test]
async fn transactional_assign_skill_tags_validates_first_and_rolls_back_trigger() {
    let pool = setup_test_db().await;
    upsert_skill(&pool, &make_skill("tag-batch-skill", "Tag Batch", true))
        .await
        .unwrap();
    let first = create_skill_tag(&pool, "transaction-first", None, None)
        .await
        .unwrap();
    let second = create_skill_tag(&pool, "transaction-second", None, None)
        .await
        .unwrap();

    let error = assign_skill_tags(
        &pool,
        &[
            "tag-batch-skill".to_string(),
            "tag-missing-skill".to_string(),
        ],
        std::slice::from_ref(&first.id),
        "manual",
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "Skill 'tag-missing-skill' not found");
    sqlx::query(
        "CREATE TRIGGER fail_second_tag_assignment
         BEFORE INSERT ON skill_tag_links
         WHEN NEW.tag_id = 'transaction-second'
         BEGIN SELECT RAISE(ABORT, 'injected tag assignment failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = assign_skill_tags(
        &pool,
        &["tag-batch-skill".to_string()],
        &[first.id.clone(), second.id.clone()],
        "manual",
        None,
        None,
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected tag assignment failure"));
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill_tag_links WHERE skill_id = 'tag-batch-skill'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn transactional_replace_ai_tags_preserves_manual_and_retries_after_failure() {
    let pool = setup_test_db().await;
    upsert_skill(&pool, &make_skill("replace-ai-skill", "Replace AI", true))
        .await
        .unwrap();
    let manual = create_skill_tag(&pool, "replace-manual", None, None)
        .await
        .unwrap();
    let old_ai = create_skill_tag(&pool, "replace-old-ai", None, None)
        .await
        .unwrap();
    let first = create_skill_tag(&pool, "replace-new-first", None, None)
        .await
        .unwrap();
    let second = create_skill_tag(&pool, "replace-new-second", None, None)
        .await
        .unwrap();
    assign_skill_tags(
        &pool,
        &["replace-ai-skill".to_string()],
        std::slice::from_ref(&manual.id),
        "manual",
        None,
        None,
    )
    .await
    .unwrap();
    assign_skill_tags(
        &pool,
        &["replace-ai-skill".to_string()],
        std::slice::from_ref(&old_ai.id),
        "ai",
        Some(0.2),
        Some("old"),
    )
    .await
    .unwrap();

    let invalid = replace_skill_ai_tags(
        &pool,
        "replace-ai-skill",
        &[("replace-missing".to_string(), 0.5, "missing".to_string())],
    )
    .await
    .unwrap_err();
    assert_eq!(invalid.to_string(), "Tag 'replace-missing' not found");
    sqlx::query(
        "CREATE TRIGGER fail_second_ai_tag_insert
         BEFORE INSERT ON skill_tag_links
         WHEN NEW.tag_id = 'replace-new-second' AND NEW.source = 'ai'
         BEGIN SELECT RAISE(ABORT, 'injected AI tag failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let suggestions = vec![
        (manual.id.clone(), 0.95, "already manual".to_string()),
        (first.id.clone(), 0.8, "first".to_string()),
        (second.id.clone(), 0.9, "second".to_string()),
    ];
    let error = replace_skill_ai_tags(&pool, "replace-ai-skill", &suggestions)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("injected AI tag failure"));
    let before_retry = sqlx::query_as::<_, (String, String)>(
        "SELECT tag_id, source FROM skill_tag_links
         WHERE skill_id = 'replace-ai-skill' ORDER BY tag_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        before_retry,
        vec![
            (manual.id.clone(), "manual".to_string()),
            (old_ai.id.clone(), "ai".to_string())
        ]
    );

    sqlx::query("DROP TRIGGER fail_second_ai_tag_insert")
        .execute(&pool)
        .await
        .unwrap();
    replace_skill_ai_tags(&pool, "replace-ai-skill", &suggestions)
        .await
        .unwrap();
    let after_retry = sqlx::query_as::<_, (String, String)>(
        "SELECT tag_id, source FROM skill_tag_links
         WHERE skill_id = 'replace-ai-skill' ORDER BY tag_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        after_retry,
        vec![
            (manual.id, "manual".to_string()),
            (first.id, "ai".to_string()),
            (second.id, "ai".to_string()),
        ]
    );
}

#[tokio::test]
async fn transactional_replace_pending_reviews_restores_old_set_and_retries() {
    let pool = setup_test_db().await;
    upsert_skill(
        &pool,
        &make_skill("review-rollback", "Review Rollback", true),
    )
    .await
    .unwrap();
    let old = create_skill_tag(&pool, "review-old", None, None)
        .await
        .unwrap();
    let first = create_skill_tag(&pool, "review-first", None, None)
        .await
        .unwrap();
    let second = create_skill_tag(&pool, "review-second", None, None)
        .await
        .unwrap();
    let review = |tag_id: String| PendingAiTagReviewInput {
        tag_id,
        confidence: 0.7,
        reason: "review".to_string(),
        proposed_name: None,
        proposed_description: None,
    };
    replace_pending_ai_tag_reviews(&pool, "review-rollback", &[review(old.id.clone())])
        .await
        .unwrap();
    let invalid = replace_pending_ai_tag_reviews(
        &pool,
        "review-rollback",
        &[review("review-missing".to_string())],
    )
    .await
    .unwrap_err();
    assert_eq!(invalid.to_string(), "Tag 'review-missing' not found");
    sqlx::query(
        "CREATE TRIGGER fail_second_pending_review
         BEFORE INSERT ON skill_ai_tag_reviews
         WHEN NEW.tag_id = 'review-second'
         BEGIN SELECT RAISE(ABORT, 'injected pending review failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let replacements = vec![review(first.id.clone()), review(second.id.clone())];
    let error = replace_pending_ai_tag_reviews(&pool, "review-rollback", &replacements)
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected pending review failure"));
    let pending = sqlx::query_scalar::<_, String>(
        "SELECT tag_id FROM skill_ai_tag_reviews
         WHERE skill_id = 'review-rollback' AND status = 'pending'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(pending, vec![old.id]);

    sqlx::query("DROP TRIGGER fail_second_pending_review")
        .execute(&pool)
        .await
        .unwrap();
    replace_pending_ai_tag_reviews(&pool, "review-rollback", &replacements)
        .await
        .unwrap();
    let mut pending = sqlx::query_scalar::<_, String>(
        "SELECT tag_id FROM skill_ai_tag_reviews
         WHERE skill_id = 'review-rollback' AND status = 'pending' ORDER BY tag_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let mut expected = vec![first.id, second.id];
    pending.sort();
    expected.sort();
    assert_eq!(pending, expected);
}

#[tokio::test]
async fn transactional_collection_delete_restores_parent_and_child_on_trigger_failure() {
    let pool = setup_test_db().await;
    upsert_skill(
        &pool,
        &make_skill("collection-rollback-skill", "Collection Rollback", true),
    )
    .await
    .unwrap();
    let collection = create_collection(&pool, "Collection Rollback", None)
        .await
        .unwrap();
    add_skill_to_collection(&pool, &collection.id, "collection-rollback-skill")
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_collection_parent_delete
         BEFORE DELETE ON collections
         BEGIN SELECT RAISE(ABORT, 'injected collection failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = delete_collection(&pool, &collection.id).await.unwrap_err();
    assert!(error.to_string().contains("injected collection failure"));
    assert!(get_collection_by_id(&pool, &collection.id)
        .await
        .unwrap()
        .is_some());
    let child_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM collection_skills WHERE collection_id = ?")
            .bind(&collection.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(child_count, 1);
}

#[tokio::test]
async fn transactional_project_delete_uses_per_connection_fk_and_cascade() {
    let (pool, _dir) = crate::test_support::file_pool().await;
    let mut connections = Vec::new();
    for _ in 0..3 {
        connections.push(pool.acquire().await.unwrap());
    }
    for connection in &mut connections {
        let enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut **connection)
            .await
            .unwrap();
        assert_eq!(enabled, 1);
    }
    drop(connections);

    let project = Project {
        id: "project-transactional-delete".to_string(),
        path: "C:/project-transactional-delete".to_string(),
        name: "Transactional Delete".to_string(),
        pinned: false,
        added_at: Utc::now().to_rfc3339(),
        last_scanned_at: None,
    };
    insert_project(&pool, &project).await.unwrap();
    upsert_project_skill_installation(
        &pool,
        &ProjectSkillInstallation {
            project_id: project.id.clone(),
            skill_id: "project-child".to_string(),
            name: "Project Child".to_string(),
            description: None,
            file_path: "C:/project/SKILL.md".to_string(),
            source_origin: "project".to_string(),
            agent_id: "codex".to_string(),
            installed_path: "C:/project/.agents/skills/project-child".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_project_parent_delete
         BEFORE DELETE ON projects
         WHEN OLD.id = 'project-transactional-delete'
         BEGIN SELECT RAISE(ABORT, 'injected project failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = delete_project(&pool, &project.id).await.unwrap_err();
    assert!(error.to_string().contains("injected project failure"));
    assert!(get_project_by_id(&pool, &project.id)
        .await
        .unwrap()
        .is_some());
    let child_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_skill_installations WHERE project_id = ?")
            .bind(&project.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(child_count, 1);

    sqlx::query("DROP TRIGGER fail_project_parent_delete")
        .execute(&pool)
        .await
        .unwrap();
    delete_project(&pool, &project.id).await.unwrap();
    let child_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_skill_installations WHERE project_id = ?")
            .bind(&project.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(child_count, 0);
}

// ── Scan Directories ──────────────────────────────────────────────────────

/// Returns the number of *unique* global_skills_dir paths across all
/// built-in agents. This is the number of rows that seed_builtin_scan_directories
/// inserts, with global Universal agents sharing ~/.agents/skills,
/// Antigravity using ~/.gemini/antigravity/skills, and Central using
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
async fn central_store_location_reinit_preserves_custom_central_agent_path() {
    let pool = setup_test_db().await;
    sqlx::query(
        "UPDATE agents SET global_skills_dir = '/tmp/custom-central-skills' WHERE id = 'central'",
    )
    .execute(&pool)
    .await
    .unwrap();

    init_database(&pool).await.unwrap();

    let central = get_agent_by_id(&pool, "central")
        .await
        .unwrap()
        .expect("central agent should exist");
    assert_eq!(central.global_skills_dir, "/tmp/custom-central-skills");
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
async fn central_store_location_reinit_seeds_scan_dirs_from_custom_central_agent_path() {
    let pool = setup_test_db().await;
    sqlx::query(
        "UPDATE agents SET global_skills_dir = '/tmp/custom-central-skills' WHERE id = 'central'",
    )
    .execute(&pool)
    .await
    .unwrap();
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
    assert!(
        dirs.iter()
            .any(|dir| dir.path == "/tmp/custom-central-skills"),
        "reinit should seed the DB central skills path"
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
    // （豁免 test_support::mem_pool：本测试手工搭建 legacy schema 验证迁移。）
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

    let migrated_uid: String =
        sqlx::query_scalar("SELECT uid FROM skills WHERE id = 'legacy-skill'")
            .fetch_one(&pool)
            .await
            .expect("legacy skill should receive a uid");
    assert!(!migrated_uid.is_empty());
    assert!(uuid::Uuid::parse_str(&migrated_uid).is_ok());

    init_database(&pool)
        .await
        .expect("uid migration should be idempotent");
    let uid_after_second_init: String =
        sqlx::query_scalar("SELECT uid FROM skills WHERE id = 'legacy-skill'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(uid_after_second_init, migrated_uid);

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
