#![cfg(test)]
//! Marketplace service tests: registry CRUD, sync caching, candidate identity,
//! and journaled Central installation.

use super::{
    add_registry_impl, install_marketplace_pinned_snapshot, install_marketplace_skill_impl,
    marketplace_candidate_for_id, marketplace_skills_from_candidates, registry_has_cached_skills,
    remove_registry_impl, replace_registry_cache_snapshot,
    resolve_skills_sh_candidate_from_snapshot, search_marketplace_skills_impl,
    skills_sh_file_entries_from_snapshot, source_to_github_url, sync_registry_impl,
    MarketplaceSkill, RegistryCacheMetadata, RegistrySyncStatus, SyncRegistryOptions,
};
use crate::db;
use crate::secrets::{MockSecretStore, SecretStore, GITHUB_PAT_SECRET_KEY};
use crate::services::github_import::{
    GitHubRepoRef, GitHubRepoSnapshot, PinnedGitHubRepoSnapshot, RemoteSkillCandidate,
    ResolvedGitHubRepoSource,
};
use std::collections::{HashMap, HashSet};
use tempfile::{tempdir, TempDir};

async fn setup_test_db() -> (crate::db::DbPool, TempDir) {
    crate::test_support::file_pool().await
}

fn snapshot_skill(registry_id: &str, candidate_id: &str) -> MarketplaceSkill {
    MarketplaceSkill {
        id: format!("{registry_id}::{candidate_id}"),
        registry_id: registry_id.to_string(),
        name: candidate_id.to_string(),
        description: Some(format!("{candidate_id} description")),
        download_url: format!("https://example.invalid/{candidate_id}/SKILL.md"),
        is_installed: false,
        synced_at: "2026-08-03T01:00:00Z".to_string(),
        cache_updated_at: Some("2026-08-03T01:00:00Z".to_string()),
    }
}

#[test]
fn marketplace_skills_from_candidates_supports_namespaced_layouts() {
    let skills = marketplace_skills_from_candidates(
        "openai",
        vec![
            RemoteSkillCandidate {
                source_path: "skills/.curated/openai-docs".to_string(),
                skill_id: "openai-docs".to_string(),
                skill_name: "openai-docs".to_string(),
                description: Some("Docs skill".to_string()),
                plugin_name: None,
                root_directory: "skills/.curated".to_string(),
                skill_directory_name: "openai-docs".to_string(),
                download_url:
                    "https://raw.githubusercontent.com/openai/skills/main/skills/.curated/openai-docs/SKILL.md"
                        .to_string(),
            },
            RemoteSkillCandidate {
                source_path: "skills/.system/skill-creator".to_string(),
                skill_id: "skill-creator".to_string(),
                skill_name: "skill-creator".to_string(),
                description: Some("Create skills".to_string()),
                plugin_name: None,
                root_directory: "skills/.system".to_string(),
                skill_directory_name: "skill-creator".to_string(),
                download_url:
                    "https://raw.githubusercontent.com/openai/skills/main/skills/.system/skill-creator/SKILL.md"
                        .to_string(),
            },
        ],
    )
    .expect("candidate mapping");

    assert_eq!(skills.len(), 2);
    assert_eq!(skills[0].id, "openai::openai-docs");
    assert_eq!(skills[0].name, "openai-docs");
    assert!(skills[0]
        .download_url
        .contains("skills/.curated/openai-docs"));
    assert_eq!(skills[1].id, "openai::skill-creator");
    assert_eq!(skills[1].name, "skill-creator");
    assert!(skills[1]
        .download_url
        .contains("skills/.system/skill-creator"));
}

#[test]
fn marketplace_candidate_mapping_rejects_duplicate_stable_ids_but_keeps_duplicate_names() {
    let mut first = RemoteSkillCandidate {
        source_path: "skills/first".to_string(),
        skill_id: "first".to_string(),
        skill_name: "Shared display name".to_string(),
        description: None,
        plugin_name: None,
        root_directory: "skills".to_string(),
        skill_directory_name: "first".to_string(),
        download_url: "https://example.invalid/first".to_string(),
    };
    let mut second = first.clone();
    second.source_path = "skills/second".to_string();
    second.skill_id = "second".to_string();
    second.skill_directory_name = "second".to_string();

    let mapped = marketplace_skills_from_candidates("registry", vec![first.clone(), second])
        .expect("duplicate display names remain distinct");
    assert_eq!(mapped.len(), 2);

    first.source_path = "skills/duplicate".to_string();
    let error = marketplace_skills_from_candidates(
        "registry",
        vec![
            first.clone(),
            RemoteSkillCandidate {
                source_path: "skills/other-duplicate".to_string(),
                ..first
            },
        ],
    )
    .expect_err("duplicate stable ids fail closed");
    assert!(matches!(error, super::MarketplaceError::CandidateAmbiguous));

    let stale = marketplace_candidate_for_id(
        "registry",
        "registry::missing",
        &[RemoteSkillCandidate {
            source_path: "skills/present".to_string(),
            skill_id: "present".to_string(),
            skill_name: "Present".to_string(),
            description: None,
            plugin_name: None,
            root_directory: "skills".to_string(),
            skill_directory_name: "present".to_string(),
            download_url: "https://example.invalid/present".to_string(),
        }],
    )
    .expect_err("missing stable id is stale");
    assert!(matches!(stale, super::MarketplaceError::CandidateStale));
}

#[tokio::test]
async fn add_registry_persists_cache_metadata() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Custom Repo".to_string(),
        "github".to_string(),
        "https://github.com/example/custom".to_string(),
        Some(RegistryCacheMetadata {
            etag: Some("etag-123".to_string()),
            last_modified: Some("Wed, 01 Jan 2025 00:00:00 GMT".to_string()),
            cache_expires_at: Some("2026-04-16T00:00:00Z".to_string()),
        }),
    )
    .await
    .expect("registry created");

    let row = sqlx::query(
        "SELECT last_sync_status, etag, last_modified, cache_expires_at
         FROM skill_registries WHERE id = ?",
    )
    .bind(&registry.id)
    .fetch_one(&pool)
    .await
    .expect("fetch registry");

    use sqlx::Row;
    assert_eq!(
        row.get::<String, _>("last_sync_status"),
        RegistrySyncStatus::Never.as_str()
    );
    assert_eq!(
        row.get::<Option<String>, _>("etag").as_deref(),
        Some("etag-123")
    );
    assert_eq!(
        row.get::<Option<String>, _>("last_modified").as_deref(),
        Some("Wed, 01 Jan 2025 00:00:00 GMT")
    );
    assert_eq!(
        row.get::<Option<String>, _>("cache_expires_at").as_deref(),
        Some("2026-04-16T00:00:00Z")
    );
}

#[tokio::test]
async fn sync_registry_uses_cached_skills_without_refresh() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Cached Repo".to_string(),
        "github".to_string(),
        "https://github.com/example/invalid".to_string(),
        None,
    )
    .await
    .expect("registry created");

    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at, cache_updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(format!("{}::cached-skill", registry.id))
    .bind(&registry.id)
    .bind("cached-skill")
    .bind("served from cache")
    .bind("https://example.com/SKILL.md")
    .bind("2026-04-16T12:00:00Z")
    .bind("2026-04-16T12:00:00Z")
    .execute(&pool)
    .await
    .expect("insert cached skill");

    let skills = sync_registry_impl(
        &pool,
        &pool,
        &MockSecretStore::default(),
        registry.id.clone(),
        SyncRegistryOptions::default(),
    )
    .await
    .expect("sync succeeds from cache");

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "cached-skill");

    let row = sqlx::query(
        "SELECT last_attempted_sync, last_synced, last_sync_status
         FROM skill_registries WHERE id = ?",
    )
    .bind(&registry.id)
    .fetch_one(&pool)
    .await
    .expect("fetch registry");

    use sqlx::Row;
    assert!(row
        .get::<Option<String>, _>("last_attempted_sync")
        .is_none());
    assert!(row.get::<Option<String>, _>("last_synced").is_none());
    assert_eq!(
        row.get::<String, _>("last_sync_status"),
        RegistrySyncStatus::Never.as_str()
    );
}

#[tokio::test]
async fn force_refresh_failure_preserves_last_good_cached_data() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Broken Repo".to_string(),
        "github".to_string(),
        "not-a-valid-github-url".to_string(),
        None,
    )
    .await
    .expect("registry created");

    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at, cache_updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(format!("{}::last-good", registry.id))
    .bind(&registry.id)
    .bind("last-good")
    .bind("cached before failure")
    .bind("https://example.com/last-good/SKILL.md")
    .bind("2026-04-16T12:00:00Z")
    .bind("2026-04-16T12:00:00Z")
    .execute(&pool)
    .await
    .expect("insert cached skill");

    let skills = sync_registry_impl(
        &pool,
        &pool,
        &MockSecretStore::default(),
        registry.id.clone(),
        SyncRegistryOptions {
            force_refresh: true,
        },
    )
    .await
    .expect("force refresh returns cached data on failure");

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "last-good");

    let row = sqlx::query(
        "SELECT last_sync_status, last_sync_error, last_synced
         FROM skill_registries WHERE id = ?",
    )
    .bind(&registry.id)
    .fetch_one(&pool)
    .await
    .expect("fetch registry");

    use sqlx::Row;
    assert_eq!(
        row.get::<String, _>("last_sync_status"),
        RegistrySyncStatus::Error.as_str()
    );
    let last_sync_error = row
        .get::<Option<String>, _>("last_sync_error")
        .unwrap_or_default();
    assert!(
        last_sync_error.contains("GitHub repository URL") || last_sync_error.contains("github.com"),
        "unexpected sync error: {last_sync_error}"
    );
    assert!(row.get::<Option<String>, _>("last_synced").is_none());

    let cached_skills = search_marketplace_skills_impl(&pool, Some(registry.id.clone()), None)
        .await
        .expect("cached skills still queryable");
    assert_eq!(cached_skills.len(), 1);
    assert_eq!(cached_skills[0].name, "last-good");
}

#[tokio::test]
async fn transactional_marketplace_remove_rolls_back_children_and_preserves_special_cases() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Remove rollback".to_string(),
        "github".to_string(),
        "https://github.com/example/remove-rollback".to_string(),
        None,
    )
    .await
    .unwrap();
    for candidate_id in ["a", "b"] {
        let skill = snapshot_skill(&registry.id, candidate_id);
        sqlx::query(
            "INSERT INTO marketplace_skills
             (id, registry_id, name, description, download_url, is_installed, synced_at, cache_updated_at)
             VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&skill.id)
        .bind(&skill.registry_id)
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(&skill.download_url)
        .bind(&skill.synced_at)
        .bind(&skill.cache_updated_at)
        .execute(&pool)
        .await
        .unwrap();
    }
    let trigger_sql = format!(
        "CREATE TRIGGER fail_registry_parent_delete
         BEFORE DELETE ON skill_registries
         WHEN OLD.id = '{}'
         BEGIN SELECT RAISE(ABORT, 'injected registry delete failure'); END",
        registry.id
    );
    sqlx::query(&trigger_sql).execute(&pool).await.unwrap();

    let error = remove_registry_impl(&pool, registry.id.clone())
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected registry delete failure"));
    let child_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM marketplace_skills WHERE registry_id = ?")
            .bind(&registry.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(child_count, 2);
    let parent_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM skill_registries WHERE id = ?")
            .bind(&registry.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(parent_count, 1);

    sqlx::query("DROP TRIGGER fail_registry_parent_delete")
        .execute(&pool)
        .await
        .unwrap();
    remove_registry_impl(&pool, registry.id.clone())
        .await
        .unwrap();
    remove_registry_impl(&pool, "missing-registry".to_string())
        .await
        .unwrap();

    let builtin_id: String = sqlx::query_scalar(
        "SELECT id FROM skill_registries WHERE is_builtin = 1 ORDER BY id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let error = remove_registry_impl(&pool, builtin_id.clone())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        super::MarketplaceError::BuiltinRegistryRemoval
    ));
    let builtin_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM skill_registries WHERE id = ?")
            .bind(builtin_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(builtin_count, 1);
}

#[tokio::test]
async fn transactional_marketplace_snapshot_replaces_clears_and_rolls_back_failures() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Snapshot rollback".to_string(),
        "github".to_string(),
        "https://github.com/example/snapshot-rollback".to_string(),
        None,
    )
    .await
    .unwrap();
    let old = vec![
        snapshot_skill(&registry.id, "a"),
        snapshot_skill(&registry.id, "b"),
    ];
    replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &old,
        &HashSet::new(),
        "2026-08-03T00:00:00Z",
        "2026-08-03T00:00:01Z",
    )
    .await
    .unwrap();

    let fresh = vec![
        snapshot_skill(&registry.id, "b"),
        snapshot_skill(&registry.id, "c"),
    ];
    let trigger_sql = format!(
        "CREATE TRIGGER fail_second_snapshot_insert
         BEFORE INSERT ON marketplace_skills
         WHEN NEW.id = '{}::c'
         BEGIN SELECT RAISE(ABORT, 'injected second snapshot insert failure'); END",
        registry.id
    );
    sqlx::query(&trigger_sql).execute(&pool).await.unwrap();
    let error = replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &fresh,
        &HashSet::new(),
        "2026-08-03T02:00:00Z",
        "2026-08-03T02:00:01Z",
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected second snapshot insert failure"));
    let ids = sqlx::query_scalar::<_, String>(
        "SELECT id FROM marketplace_skills WHERE registry_id = ? ORDER BY id",
    )
    .bind(&registry.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        ids,
        vec![format!("{}::a", registry.id), format!("{}::b", registry.id)]
    );
    let status: String =
        sqlx::query_scalar("SELECT last_sync_status FROM skill_registries WHERE id = ?")
            .bind(&registry.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, RegistrySyncStatus::Error.as_str());

    sqlx::query("DROP TRIGGER fail_second_snapshot_insert")
        .execute(&pool)
        .await
        .unwrap();
    let installed = HashSet::from([format!("{}::b", registry.id)]);
    replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &fresh,
        &installed,
        "2026-08-03T03:00:00Z",
        "2026-08-03T03:00:01Z",
    )
    .await
    .unwrap();
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT id, is_installed FROM marketplace_skills
         WHERE registry_id = ? ORDER BY id",
    )
    .bind(&registry.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let expected_fresh_rows = vec![
        (format!("{}::b", registry.id), 1),
        (format!("{}::c", registry.id), 0),
    ];
    assert_eq!(rows, expected_fresh_rows);
    let last_synced_before_status_failure: Option<String> =
        sqlx::query_scalar("SELECT last_synced FROM skill_registries WHERE id = ?")
            .bind(&registry.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(
        "CREATE TRIGGER fail_snapshot_success_status
         BEFORE UPDATE OF last_sync_status ON skill_registries
         WHEN NEW.last_sync_status = 'success'
         BEGIN SELECT RAISE(ABORT, 'injected success status failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &[],
        &HashSet::new(),
        "2026-08-03T04:00:00Z",
        "2026-08-03T04:00:01Z",
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected success status failure"));
    let rows_after_status_failure = sqlx::query_as::<_, (String, i64)>(
        "SELECT id, is_installed FROM marketplace_skills
         WHERE registry_id = ? ORDER BY id",
    )
    .bind(&registry.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows_after_status_failure, expected_fresh_rows,
        "status failure must preserve the complete B,C snapshot"
    );
    let registry_after_status_failure = sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT last_synced, last_sync_status FROM skill_registries WHERE id = ?",
    )
    .bind(&registry.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        registry_after_status_failure.0,
        last_synced_before_status_failure
    );
    assert_eq!(
        registry_after_status_failure.1,
        RegistrySyncStatus::Error.as_str(),
        "failed success metadata must never remain visible as success"
    );
    sqlx::query("DROP TRIGGER fail_snapshot_success_status")
        .execute(&pool)
        .await
        .unwrap();
    replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &[],
        &HashSet::new(),
        "2026-08-03T05:00:00Z",
        "2026-08-03T05:00:01Z",
    )
    .await
    .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM marketplace_skills WHERE registry_id = ?")
            .bind(&registry.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn transactional_marketplace_snapshot_rolls_back_a_later_batch() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Large snapshot rollback".to_string(),
        "github".to_string(),
        "https://github.com/example/large-snapshot".to_string(),
        None,
    )
    .await
    .unwrap();
    let old = vec![
        snapshot_skill(&registry.id, "old-a"),
        snapshot_skill(&registry.id, "old-b"),
    ];
    replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &old,
        &HashSet::new(),
        "2026-08-03T00:00:00Z",
        "2026-08-03T00:00:01Z",
    )
    .await
    .unwrap();
    let fresh = (0..113)
        .map(|index| snapshot_skill(&registry.id, &format!("fresh-{index:03}")))
        .collect::<Vec<_>>();
    let trigger_sql = format!(
        "CREATE TRIGGER fail_later_snapshot_batch
         BEFORE INSERT ON marketplace_skills
         WHEN NEW.id = '{}::fresh-112'
         BEGIN SELECT RAISE(ABORT, 'injected later snapshot batch failure'); END",
        registry.id
    );
    sqlx::query(&trigger_sql).execute(&pool).await.unwrap();

    let error = replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &fresh,
        &HashSet::new(),
        "2026-08-03T06:00:00Z",
        "2026-08-03T06:00:01Z",
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("injected later snapshot batch failure"));
    let ids = sqlx::query_scalar::<_, String>(
        "SELECT id FROM marketplace_skills WHERE registry_id = ? ORDER BY id",
    )
    .bind(&registry.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        ids,
        vec![
            format!("{}::old-a", registry.id),
            format!("{}::old-b", registry.id),
        ]
    );
}

#[tokio::test]
async fn transactional_marketplace_snapshot_rolls_back_commit_failure() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Commit rollback".to_string(),
        "github".to_string(),
        "https://github.com/example/commit-rollback".to_string(),
        None,
    )
    .await
    .unwrap();
    let old = vec![snapshot_skill(&registry.id, "old")];
    replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &old,
        &HashSet::new(),
        "2026-08-03T00:00:00Z",
        "2026-08-03T00:00:01Z",
    )
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE deferred_snapshot_commit_guard (
             registry_id TEXT NOT NULL,
             FOREIGN KEY (registry_id) REFERENCES skill_registries(id)
                 DEFERRABLE INITIALLY DEFERRED
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_snapshot_commit
         AFTER UPDATE OF last_sync_status ON skill_registries
         WHEN NEW.last_sync_status = 'success'
         BEGIN
             INSERT INTO deferred_snapshot_commit_guard (registry_id)
             VALUES ('missing-registry');
         END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let attempt_time = "2026-08-03T07:00:00Z";
    let error = replace_registry_cache_snapshot(
        &pool,
        &registry.id,
        &[snapshot_skill(&registry.id, "fresh")],
        &HashSet::new(),
        attempt_time,
        "2026-08-03T07:00:01Z",
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));

    let ids = sqlx::query_scalar::<_, String>(
        "SELECT id FROM marketplace_skills WHERE registry_id = ? ORDER BY id",
    )
    .bind(&registry.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(ids, vec![format!("{}::old", registry.id)]);
    let marker = sqlx::query_as::<_, (Option<String>, String, Option<String>)>(
        "SELECT last_attempted_sync, last_sync_status, last_sync_error
         FROM skill_registries WHERE id = ?",
    )
    .bind(&registry.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(marker.0.as_deref(), Some(attempt_time));
    assert_eq!(marker.1, RegistrySyncStatus::Error.as_str());
    assert!(marker
        .2
        .as_deref()
        .is_some_and(|message| message.contains("FOREIGN KEY constraint failed")));
}

#[tokio::test]
async fn registry_cache_column_migration_is_idempotent() {
    // 豁免 test_support::file_pool：本测试手工搭建 legacy schema 验证迁移，
    // 必须拿到未 init 的裸文件池。
    let dir = tempdir().expect("create tempdir");
    let db_path = dir.path().join("migration.sqlite");
    let db_path = db_path.to_string_lossy().into_owned();
    let pool = db::create_pool(std::path::Path::new(&db_path))
        .await
        .expect("create pool");

    sqlx::query(
        "CREATE TABLE skill_registries (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url TEXT NOT NULL,
            is_builtin BOOLEAN NOT NULL DEFAULT 0,
            is_enabled BOOLEAN NOT NULL DEFAULT 1,
            last_synced TEXT,
            created_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("create legacy skill_registries");
    sqlx::query(
        "CREATE TABLE marketplace_skills (
            id TEXT PRIMARY KEY,
            registry_id TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            download_url TEXT NOT NULL,
            is_installed BOOLEAN NOT NULL DEFAULT 0,
            synced_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("create legacy marketplace_skills");

    db::init_database(&pool).await.expect("migrate once");
    db::init_database(&pool).await.expect("migrate twice");

    let registry_columns = sqlx::query("PRAGMA table_info(skill_registries)")
        .fetch_all(&pool)
        .await
        .expect("pragma registry");
    let skill_columns = sqlx::query("PRAGMA table_info(marketplace_skills)")
        .fetch_all(&pool)
        .await
        .expect("pragma skills");

    use sqlx::Row;
    let registry_names: Vec<String> = registry_columns.iter().map(|row| row.get("name")).collect();
    let skill_names: Vec<String> = skill_columns.iter().map(|row| row.get("name")).collect();

    for expected in [
        "last_attempted_sync",
        "last_sync_status",
        "last_sync_error",
        "cache_updated_at",
        "cache_expires_at",
        "etag",
        "last_modified",
    ] {
        assert!(
            registry_names.iter().any(|name| name == expected),
            "missing registry column {expected}"
        );
    }
    assert!(
        skill_names.iter().any(|name| name == "cache_updated_at"),
        "missing marketplace_skills.cache_updated_at"
    );
}

#[tokio::test]
async fn registry_has_cached_skills_detects_persisted_rows() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Cache Check".to_string(),
        "github".to_string(),
        "https://github.com/example/cache-check".to_string(),
        None,
    )
    .await
    .expect("registry created");

    assert!(!registry_has_cached_skills(&pool, &registry.id)
        .await
        .expect("empty"));

    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at, cache_updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(format!("{}::cached", registry.id))
    .bind(&registry.id)
    .bind("cached")
    .bind("cached row")
    .bind("https://example.com/cached/SKILL.md")
    .bind("2026-04-16T12:00:00Z")
    .bind("2026-04-16T12:00:00Z")
    .execute(&pool)
    .await
    .expect("insert skill");

    assert!(registry_has_cached_skills(&pool, &registry.id)
        .await
        .expect("cached"));
}

#[tokio::test]
async fn installed_state_does_not_match_an_unrelated_same_name_central_skill() {
    let (pool, dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Identity registry".to_string(),
        "github".to_string(),
        "https://github.com/owner/repo".to_string(),
        None,
    )
    .await
    .expect("registry created");
    let marketplace_id = format!("{}::wanted-skill", registry.id);
    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at)
         VALUES (?, ?, 'Shared display name', NULL, 'https://example.invalid/ignored', 1, ?)",
    )
    .bind(&marketplace_id)
    .bind(&registry.id)
    .bind("2026-08-03T00:00:00Z")
    .execute(&pool)
    .await
    .expect("marketplace row");

    let unrelated_dir = dir.path().join("unrelated-skill");
    let unrelated = crate::db::Skill {
        id: "unrelated-skill".to_string(),
        uid: "uid-unrelated-skill".to_string(),
        name: "Shared display name".to_string(),
        description: None,
        file_path: unrelated_dir
            .join("SKILL.md")
            .to_string_lossy()
            .into_owned(),
        canonical_path: Some(unrelated_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some("github:owner/repo".to_string()),
        content: None,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill_with_github_repository(
        &pool,
        &unrelated,
        "owner",
        "repo",
        "main",
        "https://github.com/owner/repo",
        "skills/unrelated-skill",
        Some("0123456789abcdef0123456789abcdef01234567"),
        Some("sha256-v1:unrelated"),
    )
    .await
    .expect("unrelated Central skill with provenance");

    let visible = search_marketplace_skills_impl(&pool, Some(registry.id), None)
        .await
        .expect("Marketplace search");
    assert_eq!(visible.len(), 1);
    assert!(!visible[0].is_installed);
    let cached =
        sqlx::query_scalar::<_, i64>("SELECT is_installed FROM marketplace_skills WHERE id = ?")
            .bind(marketplace_id)
            .fetch_one(&pool)
            .await
            .expect("repaired marker");
    assert_eq!(cached, 0);
}

#[tokio::test]
async fn installed_state_matches_candidate_and_repository_case_insensitively() {
    let (pool, dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Case registry".to_string(),
        "github".to_string(),
        "https://github.com/Owner/Repo".to_string(),
        None,
    )
    .await
    .expect("registry created");
    let marketplace_id = format!("{}::wanted-skill", registry.id);
    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at)
         VALUES (?, ?, 'Current display name', NULL, 'https://example.invalid/ignored', 0, ?)",
    )
    .bind(&marketplace_id)
    .bind(&registry.id)
    .bind("2026-08-03T00:00:00Z")
    .execute(&pool)
    .await
    .expect("marketplace row");

    let central_dir = dir.path().join("wanted-skill");
    let central = crate::db::Skill {
        id: "wanted-skill".to_string(),
        uid: "uid-wanted-skill".to_string(),
        name: "Old display name".to_string(),
        description: None,
        file_path: central_dir.join("SKILL.md").to_string_lossy().into_owned(),
        canonical_path: Some(central_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some("github:owner/repo".to_string()),
        content: None,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill_with_github_repository(
        &pool,
        &central,
        "owner",
        "repo",
        "main",
        "https://github.com/owner/repo",
        "skills/wanted-skill",
        Some("0123456789abcdef0123456789abcdef01234567"),
        Some("sha256-v1:wanted"),
    )
    .await
    .expect("Central skill with provenance");

    let visible = search_marketplace_skills_impl(&pool, Some(registry.id), None)
        .await
        .expect("Marketplace search");
    assert_eq!(visible.len(), 1);
    assert!(visible[0].is_installed);
}

#[tokio::test]
async fn registry_install_rejects_disabled_source_before_acquisition_or_mutation() {
    let (pool, _dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &pool,
        "Disabled registry".to_string(),
        "github".to_string(),
        "https://github.com/owner/repo".to_string(),
        None,
    )
    .await
    .expect("registry created");
    sqlx::query("UPDATE skill_registries SET is_enabled = 0 WHERE id = ?")
        .bind(&registry.id)
        .execute(&pool)
        .await
        .expect("disable registry");
    let marketplace_id = format!("{}::safe-skill", registry.id);
    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at)
         VALUES (?, ?, 'Safe skill', NULL, 'https://attacker.invalid/ignored', 0, ?)",
    )
    .bind(&marketplace_id)
    .bind(&registry.id)
    .bind("2026-08-03T00:00:00Z")
    .execute(&pool)
    .await
    .expect("marketplace row");

    let error = install_marketplace_skill_impl(
        &pool,
        &pool,
        &MockSecretStore::default(),
        crate::targets::ActiveTarget::Local,
        marketplace_id,
    )
    .await
    .expect_err("disabled source fails closed");
    assert!(matches!(error, super::MarketplaceError::RegistryDisabled));
    let operation_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM fs_db_operations")
        .fetch_one(&pool)
        .await
        .expect("operation count");
    assert_eq!(operation_count, 0);
}

#[tokio::test]
async fn registry_install_reads_github_auth_from_the_explicit_auth_pool() {
    let (target_pool, _target_dir) = setup_test_db().await;
    let (auth_pool, _auth_dir) = setup_test_db().await;
    let registry = add_registry_impl(
        &target_pool,
        "Invalid registry".to_string(),
        "github".to_string(),
        "not-a-valid-github-url".to_string(),
        None,
    )
    .await
    .expect("registry created");
    let marketplace_id = format!("{}::safe-skill", registry.id);
    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at)
         VALUES (?, ?, 'Safe skill', NULL, 'https://attacker.invalid/ignored', 0, ?)",
    )
    .bind(&marketplace_id)
    .bind(&registry.id)
    .bind("2026-08-03T00:00:00Z")
    .execute(&target_pool)
    .await
    .expect("marketplace row");
    db::set_setting(&auth_pool, "github_pat", "auth-pool-token")
        .await
        .expect("legacy auth setting");
    let secrets = MockSecretStore::default();

    let error = install_marketplace_skill_impl(
        &target_pool,
        &auth_pool,
        &secrets,
        crate::targets::ActiveTarget::Local,
        marketplace_id,
    )
    .await
    .expect_err("invalid source stops before network acquisition");
    assert!(matches!(error, super::MarketplaceError::GithubImport(_)));
    assert_eq!(
        secrets.get(GITHUB_PAT_SECRET_KEY).expect("secret read"),
        Some("auth-pool-token".to_string())
    );
    assert_eq!(
        db::get_setting(&auth_pool, "github_pat")
            .await
            .expect("legacy auth removed"),
        None
    );
    assert_eq!(
        db::get_setting(&target_pool, "github_pat")
            .await
            .expect("target auth absent"),
        None
    );
}

fn sample_skill(name: &str, description: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n---\n# {name}\n")
}

fn repo_snapshot(files: &[(&str, String)]) -> GitHubRepoSnapshot {
    GitHubRepoSnapshot {
        files: files
            .iter()
            .map(|(path, content)| (path.to_string(), content.as_bytes().to_vec()))
            .collect::<HashMap<_, _>>(),
    }
}

fn repo_ref() -> GitHubRepoRef {
    GitHubRepoRef {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/owner/repo".to_string(),
    }
}

#[test]
fn marketplace_candidate_identity_ignores_adversarial_frontmatter_display_names() {
    for display_name in [
        "../escape",
        "/absolute",
        r"C:\outside",
        r"\\server\share",
        "nested/name",
        r"nested\name",
        ".",
        "..",
        "技能 名称",
    ] {
        let snapshot = repo_snapshot(&[(
            "skills/safe-skill/SKILL.md",
            format!("---\nname: '{display_name}'\ndescription: adversarial display name\n---\n"),
        )]);
        let candidates =
            crate::services::github_import::build_repo_skill_candidates_from_snapshot_at_path(
                &repo_ref(),
                &snapshot,
                None,
            )
            .expect("candidate discovery");
        let candidate =
            marketplace_candidate_for_id("registry", "registry::safe-skill", &candidates)
                .expect("stable candidate identity");

        assert_eq!(candidate.skill_id, "safe-skill", "display={display_name}");
        assert_eq!(candidate.source_path, "skills/safe-skill");
        assert_eq!(candidate.skill_name, display_name);
    }
}

#[tokio::test]
async fn journaled_marketplace_content_upsert_preserves_peers_provenance_and_repairs_marker() {
    let (pool, dir) = setup_test_db().await;
    let central_root = dir.path().join("central");
    crate::test_support::set_agent_dir(&pool, "central", &central_root).await;
    let registry = add_registry_impl(
        &pool,
        "Journaled registry".to_string(),
        "github".to_string(),
        "https://github.com/owner/repo".to_string(),
        None,
    )
    .await
    .expect("registry");
    let marketplace_id = format!("{}::safe-skill", registry.id);
    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at)
         VALUES (?, ?, ?, ?, ?, 0, ?)",
    )
    .bind(&marketplace_id)
    .bind(&registry.id)
    .bind("../escape")
    .bind("adversarial display name")
    .bind("https://attacker.invalid/ignored")
    .bind("2026-08-03T00:00:00Z")
    .execute(&pool)
    .await
    .expect("marketplace row");

    let snapshot = repo_snapshot(&[
        (
            "skills/safe-skill/SKILL.md",
            "---\nname: '../escape'\ndescription: journaled install\n---\n".to_string(),
        ),
        (
            "skills/safe-skill/references/guide.md",
            "# guide\n".to_string(),
        ),
        (
            "skills/safe-skill/scripts/run.ps1",
            "Write-Output safe\n".to_string(),
        ),
        ("skills/safe-skill/assets/data.txt", "asset\n".to_string()),
    ]);
    let candidates =
        crate::services::github_import::build_repo_skill_candidates_from_snapshot_at_path(
            &repo_ref(),
            &snapshot,
            None,
        )
        .expect("candidate discovery");
    let candidate = marketplace_candidate_for_id(&registry.id, &marketplace_id, &candidates)
        .expect("candidate match");
    let content_digest = crate::services::github_import::candidate_content_digest_from_snapshot(
        &snapshot,
        &candidate.source_path,
    )
    .expect("content digest");
    let target_dir = central_root.join(&candidate.skill_id);
    let commit = "0123456789abcdef0123456789abcdef01234567";
    sqlx::query(
        "CREATE TRIGGER fail_marketplace_marker
         BEFORE UPDATE OF is_installed ON marketplace_skills
         BEGIN SELECT RAISE(FAIL, 'marker failure'); END",
    )
    .execute(&pool)
    .await
    .expect("failure trigger");
    install_marketplace_pinned_snapshot(
        &pool,
        crate::targets::ActiveTarget::Local,
        &marketplace_id,
        &registry.id,
        PinnedGitHubRepoSnapshot {
            resolved: ResolvedGitHubRepoSource {
                repo: repo_ref(),
                source_path: None,
            },
            resolved_commit_sha: commit.to_string(),
            snapshot,
            candidates,
        },
    )
    .await
    .expect("Marketplace pinned install");

    for relative in [
        "SKILL.md",
        "references/guide.md",
        "scripts/run.ps1",
        "assets/data.txt",
    ] {
        assert!(target_dir.join(relative).is_file(), "missing {relative}");
    }
    assert!(!central_root.parent().unwrap().join("escape").exists());
    let stored = db::get_skill_by_id(&pool, "safe-skill")
        .await
        .expect("skill query")
        .expect("skill row");
    let original_uid = stored.uid.clone();
    assert_eq!(stored.name, "../escape");
    assert_eq!(
        stored.canonical_path.as_deref(),
        Some(target_dir.to_string_lossy().as_ref())
    );
    let assignment = db::get_skill_repository_assignment(&pool, "safe-skill")
        .await
        .expect("repository assignment");
    assert_eq!(assignment.source_path.as_deref(), Some("skills/safe-skill"));
    let provenance = db::get_skill_repository_provenance(&pool, "safe-skill")
        .await
        .expect("provenance query")
        .expect("provenance row");
    assert_eq!(provenance.0.as_deref(), Some(commit));
    assert_eq!(provenance.1.as_deref(), Some(content_digest.as_str()));
    let journal = sqlx::query(
        "SELECT operation_kind, phase, manifest_json
         FROM fs_db_operations WHERE skill_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind("safe-skill")
    .fetch_one(&pool)
    .await
    .expect("journal row");
    use sqlx::Row;
    assert_eq!(journal.get::<String, _>("operation_kind"), "central_update");
    assert_eq!(journal.get::<String, _>("phase"), "completed");
    let manifest: serde_json::Value =
        serde_json::from_str(&journal.get::<String, _>("manifest_json")).unwrap();
    assert_eq!(manifest["payload"]["hadTarget"], false);

    let cached_before_repair =
        sqlx::query_scalar::<_, i64>("SELECT is_installed FROM marketplace_skills WHERE id = ?")
            .bind(&marketplace_id)
            .fetch_one(&pool)
            .await
            .expect("cached marker after best-effort failure");
    assert_eq!(cached_before_repair, 0);
    let visible = search_marketplace_skills_impl(&pool, Some(registry.id.clone()), None)
        .await
        .expect("live-state search");
    assert!(visible[0].is_installed);
    sqlx::query("DROP TRIGGER fail_marketplace_marker")
        .execute(&pool)
        .await
        .expect("drop trigger");
    let repaired = search_marketplace_skills_impl(&pool, Some(registry.id.clone()), None)
        .await
        .expect("repair search");
    assert!(repaired[0].is_installed);
    let cached =
        sqlx::query_scalar::<_, i64>("SELECT is_installed FROM marketplace_skills WHERE id = ?")
            .bind(&marketplace_id)
            .fetch_one(&pool)
            .await
            .expect("cached marker");
    assert_eq!(cached, 1);

    let updated_snapshot = repo_snapshot(&[
        (
            "skills/safe-skill/SKILL.md",
            "---\nname: '../escape'\ndescription: updated install\n---\n".to_string(),
        ),
        (
            "skills/safe-skill/references/guide.md",
            "# updated guide\n".to_string(),
        ),
        (
            "skills/safe-skill/scripts/run.ps1",
            "Write-Output updated\n".to_string(),
        ),
        (
            "skills/safe-skill/assets/data.txt",
            "updated asset\n".to_string(),
        ),
    ]);
    let updated_candidates =
        crate::services::github_import::build_repo_skill_candidates_from_snapshot_at_path(
            &repo_ref(),
            &updated_snapshot,
            None,
        )
        .expect("updated candidate discovery");
    install_marketplace_pinned_snapshot(
        &pool,
        crate::targets::ActiveTarget::Local,
        &marketplace_id,
        &registry.id,
        PinnedGitHubRepoSnapshot {
            resolved: ResolvedGitHubRepoSource {
                repo: repo_ref(),
                source_path: None,
            },
            resolved_commit_sha: "fedcba9876543210fedcba9876543210fedcba98".to_string(),
            snapshot: updated_snapshot,
            candidates: updated_candidates,
        },
    )
    .await
    .expect("Marketplace overwrite install");
    let overwritten = db::get_skill_by_id(&pool, "safe-skill")
        .await
        .expect("overwritten skill query")
        .expect("overwritten skill row");
    assert_eq!(overwritten.uid, original_uid);
    assert_eq!(
        std::fs::read_to_string(target_dir.join("references/guide.md")).expect("updated peer"),
        "# updated guide\n"
    );
    let latest_manifest = sqlx::query_scalar::<_, String>(
        "SELECT manifest_json FROM fs_db_operations
         WHERE skill_id = 'safe-skill' ORDER BY created_at DESC, rowid DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("latest overwrite journal");
    let latest_manifest: serde_json::Value =
        serde_json::from_str(&latest_manifest).expect("latest manifest JSON");
    assert_eq!(latest_manifest["payload"]["hadTarget"], true);
}

#[tokio::test]
async fn marketplace_content_upsert_db_failure_rolls_back_first_import_and_keeps_marker_false() {
    let (pool, dir) = setup_test_db().await;
    let central_root = dir.path().join("central");
    crate::test_support::set_agent_dir(&pool, "central", &central_root).await;
    let registry = add_registry_impl(
        &pool,
        "Rollback registry".to_string(),
        "github".to_string(),
        "https://github.com/owner/repo".to_string(),
        None,
    )
    .await
    .expect("registry");
    let marketplace_id = format!("{}::db-fail", registry.id);
    sqlx::query(
        "INSERT INTO marketplace_skills
         (id, registry_id, name, description, download_url, is_installed, synced_at)
         VALUES (?, ?, 'DB failure', NULL, 'https://example.invalid/ignored', 0, ?)",
    )
    .bind(&marketplace_id)
    .bind(&registry.id)
    .bind("2026-08-03T00:00:00Z")
    .execute(&pool)
    .await
    .expect("marketplace row");
    sqlx::query(
        "CREATE TRIGGER fail_marketplace_skill_upsert
         BEFORE INSERT ON skills WHEN NEW.id = 'db-fail'
         BEGIN SELECT RAISE(FAIL, 'forced skill upsert failure'); END",
    )
    .execute(&pool)
    .await
    .expect("failure trigger");

    let snapshot = repo_snapshot(&[
        (
            "skills/db-fail/SKILL.md",
            "---\nname: DB failure\n---\n".to_string(),
        ),
        (
            "skills/db-fail/references/guide.md",
            "# must roll back\n".to_string(),
        ),
    ]);
    let candidates =
        crate::services::github_import::build_repo_skill_candidates_from_snapshot_at_path(
            &repo_ref(),
            &snapshot,
            None,
        )
        .expect("candidate discovery");
    let candidate = marketplace_candidate_for_id(&registry.id, &marketplace_id, &candidates)
        .expect("candidate match");
    let target_dir = central_root.join(&candidate.skill_id);
    let result = crate::services::central_updates::journaled_central_content_upsert(
        &pool,
        &crate::targets::ActiveTarget::Local,
        crate::services::central_updates::JournaledCentralContentUpsert {
            skill: crate::db::Skill {
                id: candidate.skill_id.clone(),
                uid: "uid-db-fail".to_string(),
                name: candidate.skill_name.clone(),
                description: candidate.description.clone(),
                file_path: target_dir.join("SKILL.md").to_string_lossy().into_owned(),
                canonical_path: Some(target_dir.to_string_lossy().into_owned()),
                is_central: true,
                source: Some("github:owner/repo".to_string()),
                content: None,
                scanned_at: chrono::Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
            repo: repo_ref(),
            candidate,
            snapshot: &snapshot,
            target_dir: target_dir.clone(),
            resolved_commit_sha: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            content_digest: Some("sha256-v1:db-failure".to_string()),
        },
    )
    .await;

    assert!(result.is_err());
    assert!(!target_dir.exists());
    assert!(db::get_skill_by_id(&pool, "db-fail")
        .await
        .expect("skill query")
        .is_none());
    let marker =
        sqlx::query_scalar::<_, i64>("SELECT is_installed FROM marketplace_skills WHERE id = ?")
            .bind(&marketplace_id)
            .fetch_one(&pool)
            .await
            .expect("installed marker");
    assert_eq!(marker, 0);
    let phase = sqlx::query_scalar::<_, String>(
        "SELECT phase FROM fs_db_operations WHERE skill_id = 'db-fail'",
    )
    .fetch_one(&pool)
    .await
    .expect("journal phase");
    assert_eq!(phase, "rolled_back");
}

#[test]
fn registry_install_has_no_direct_url_or_display_name_writer() {
    let source = include_str!("mod.rs");
    let install = source
        .split("// ─── Install")
        .nth(1)
        .expect("install section");
    assert!(install.contains("journaled_central_content_upsert"));
    for forbidden in [
        "central_skill_dir_for_name",
        "Client::builder",
        ".get(&skill.download_url)",
        "std::fs::write",
        ".write_file(",
    ] {
        assert!(
            !install.contains(forbidden),
            "forbidden direct writer: {forbidden}"
        );
    }
}

#[test]
fn source_to_github_url_accepts_owner_repo_only() {
    assert_eq!(
        source_to_github_url("owner/repo").expect("valid source"),
        "https://github.com/owner/repo"
    );
    assert!(source_to_github_url("../owner/repo").is_err());
    assert!(source_to_github_url("https://github.com/owner/repo").is_err());
}

#[test]
fn resolve_skills_sh_candidate_matches_nested_skill_id() {
    let snapshot = repo_snapshot(&[
        (
            "content/skills/development/code-auditor/SKILL.md",
            sample_skill("code-auditor", "Audit code"),
        ),
        (
            "content/skills/development/code-auditor/references/checklist.md",
            "# checklist\n".to_string(),
        ),
    ]);

    let candidate =
        resolve_skills_sh_candidate_from_snapshot(&repo_ref(), &snapshot, "code-auditor")
            .expect("candidate");

    assert_eq!(candidate.skill_id, "code-auditor");
    assert_eq!(
        candidate.source_path,
        "content/skills/development/code-auditor"
    );
    assert!(candidate
        .download_url
        .ends_with("/owner/repo/main/content/skills/development/code-auditor/SKILL.md"));
}

#[test]
fn skills_sh_file_entries_from_snapshot_returns_full_directory_tree() {
    let snapshot = repo_snapshot(&[
        (
            "skills/demo-skill/SKILL.md",
            sample_skill("demo-skill", "Demo"),
        ),
        (
            "skills/demo-skill/references/guide.md",
            "# guide\n".to_string(),
        ),
        (
            "skills/demo-skill/scripts/run.ps1",
            "Write-Output demo\n".to_string(),
        ),
        (
            "skills/other-skill/SKILL.md",
            sample_skill("other-skill", "Other"),
        ),
    ]);

    let entries = skills_sh_file_entries_from_snapshot(&snapshot, "skills/demo-skill");
    let paths = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry.is_dir))
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![
            ("skills/demo-skill/SKILL.md", false),
            ("skills/demo-skill/references", true),
            ("skills/demo-skill/references/guide.md", false),
            ("skills/demo-skill/scripts", true),
            ("skills/demo-skill/scripts/run.ps1", false),
        ]
    );
}
