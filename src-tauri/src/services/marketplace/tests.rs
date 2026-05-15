#![cfg(test)]
//! Marketplace service tests: registry CRUD round-trips, sync caching, and
//! install path helpers. AI-explanation tests live in `services::ai_provider::tests`.

use super::{
    add_registry_impl, central_skill_dir_for_name, is_skill_installed_in_central,
    marketplace_skills_from_candidates, registry_has_cached_skills,
    resolve_skills_sh_candidate_from_snapshot, search_marketplace_skills_impl,
    skills_sh_file_entries_from_snapshot, source_to_github_url, sync_registry_impl,
    RegistryCacheMetadata, RegistrySyncStatus, SyncRegistryOptions,
};
use crate::commands::github_import::{GitHubRepoRef, GitHubRepoSnapshot, RemoteSkillCandidate};
use crate::db;
use crate::secrets::MockSecretStore;
use std::collections::HashMap;
use std::path::Path;
use tempfile::{tempdir, TempDir};

async fn setup_test_db() -> (crate::db::DbPool, TempDir) {
    let dir = tempdir().expect("create tempdir");
    let db_path = dir.path().join("marketplace-cache.sqlite");
    let db_path = db_path.to_string_lossy().into_owned();
    let pool = db::create_pool(&db_path).await.expect("create pool");
    db::init_database(&pool).await.expect("init db");
    (pool, dir)
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
                root_directory: "skills/.system".to_string(),
                skill_directory_name: "skill-creator".to_string(),
                download_url:
                    "https://raw.githubusercontent.com/openai/skills/main/skills/.system/skill-creator/SKILL.md"
                        .to_string(),
            },
        ],
    );

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
async fn registry_cache_column_migration_is_idempotent() {
    let dir = tempdir().expect("create tempdir");
    let db_path = dir.path().join("migration.sqlite");
    let db_path = db_path.to_string_lossy().into_owned();
    let pool = db::create_pool(&db_path).await.expect("create pool");

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

#[test]
fn central_skill_dir_for_name_uses_the_given_central_dir() {
    let central_dir = Path::new(r"C:\Users\lyh\.skillsmanage\skills");
    let skill_dir = central_skill_dir_for_name(central_dir, "demo-skill");

    assert_eq!(skill_dir, central_dir.join("demo-skill"));
}

#[test]
fn is_skill_installed_in_central_checks_for_skill_md() {
    let dir = tempdir().expect("create tempdir");
    let skill_dir = dir.path().join("demo-skill");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(skill_dir.join("SKILL.md"), "---\nname: demo-skill\n---\n")
        .expect("write skill md");

    assert!(is_skill_installed_in_central(dir.path(), "demo-skill"));
    assert!(!is_skill_installed_in_central(dir.path(), "missing-skill"));
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
