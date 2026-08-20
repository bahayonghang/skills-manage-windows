use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{Connection, Row, SqliteConnection};
use tempfile::TempDir;

use super::{apply_migration, backup, descriptor_checksum, versions};
use crate::db;

const FIXTURE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/db");
static DATABASE_OPEN_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureManifest {
    format_version: u32,
    fixtures: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureEntry {
    tag: String,
    commit: String,
    source_schema_sha256: String,
    file: String,
    fixture_sha256: String,
    table_count: i64,
    absent_tables: Vec<String>,
}

fn fixture_manifest() -> FixtureManifest {
    serde_json::from_str(include_str!("../../../tests/fixtures/db/manifest.json"))
        .expect("parse fixture manifest")
}

fn fixture_path(entry: &FixtureEntry) -> PathBuf {
    Path::new(FIXTURE_ROOT).join(&entry.file)
}

async fn materialize_fixture(entry: &FixtureEntry, path: &Path) {
    let sql = std::fs::read_to_string(fixture_path(entry)).expect("read SQL fixture");
    let pool = crate::db::pool::create_pool(path)
        .await
        .expect("open raw fixture database");
    sqlx::raw_sql(&sql)
        .execute(&pool)
        .await
        .expect("materialize tag fixture");

    let table_count = sqlx::query(
        "SELECT COUNT(*) AS count FROM sqlite_master
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get::<i64, _>("count")
    .unwrap();
    assert_eq!(table_count, entry.table_count, "{} table drift", entry.tag);
    for table in &entry.absent_tables {
        let present = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?) AS present",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap()
        .try_get::<i64, _>("present")
        .unwrap();
        assert_eq!(present, 0, "{} unexpectedly contains {table}", entry.tag);
    }
    pool.close().await;
}

fn sibling_files(path: &Path, marker: &str) -> Vec<PathBuf> {
    let parent = path.parent().unwrap();
    let database_name = path.file_name().unwrap().to_string_lossy();
    std::fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|candidate| {
            let name = candidate.file_name().unwrap().to_string_lossy();
            name.starts_with(database_name.as_ref()) && name.contains(marker)
        })
        .collect()
}

async fn assert_owned_foreign_keys(pool: &db::DbPool) {
    for relation in db::repos::skill_relations_spec::owned_skill_relations() {
        let sql = format!("PRAGMA foreign_key_list({})", relation.table);
        let rows = sqlx::query(&sql).fetch_all(pool).await.unwrap();
        assert!(
            rows.iter().any(|row| {
                row.try_get::<String, _>("table")
                    .is_ok_and(|value| value == "skills")
                    && row
                        .try_get::<String, _>("from")
                        .is_ok_and(|value| value == relation.skill_column)
                    && row
                        .try_get::<String, _>("on_delete")
                        .is_ok_and(|value| value == "CASCADE")
            }),
            "{} is missing the skill-parent cascade",
            relation.table
        );
    }
}

async fn insert_independent_history(pool: &db::DbPool) {
    sqlx::query(
        "INSERT INTO agent_skill_observations
         (row_id, agent_id, skill_id, name, file_path, dir_path, source_kind,
          source_root, link_type, is_read_only, scanned_at)
         VALUES ('fixture-observation', 'codex', 'fixture-skill', 'Fixture Skill',
                 '/fixtures/skill.md', '/fixtures', 'global', '/fixtures',
                 'native', 0, '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO projects (id, path, name, added_at)
         VALUES ('fixture-project', '/fixtures/project', 'Fixture Project', '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO project_skill_installations
         (project_id, skill_id, agent_id, installed_path, link_type, created_at)
         VALUES ('fixture-project', 'fixture-skill', 'codex', '/fixtures/project/skill',
                 'native', '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO skill_usage_metadata
         (target_id, skill, match_status, resolved_skill_id, scanned_at_ms)
         VALUES ('local', 'fixture-skill', 'matched', 'fixture-skill', 1)",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[test]
fn fixture_manifest_and_files_are_locked() {
    let manifest = fixture_manifest();
    assert_eq!(manifest.format_version, 1);
    assert_eq!(manifest.fixtures.len(), 5);
    for fixture in manifest.fixtures {
        assert_eq!(
            fixture.commit.len(),
            40,
            "{} commit is not pinned",
            fixture.tag
        );
        assert_eq!(
            fixture.source_schema_sha256.len(),
            64,
            "{} source schema checksum is not pinned",
            fixture.tag
        );
        let bytes = std::fs::read(fixture_path(&fixture)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            fixture.fixture_sha256,
            "{} fixture checksum drift",
            fixture.tag
        );
    }
}

#[test]
fn migration_sources_are_checksum_locked() {
    let checksums = [
        descriptor_checksum(1).unwrap(),
        descriptor_checksum(2).unwrap(),
        descriptor_checksum(3).unwrap(),
        descriptor_checksum(4).unwrap(),
        descriptor_checksum(5).unwrap(),
        descriptor_checksum(6).unwrap(),
    ];
    assert_eq!(
        checksums,
        [
            "173296a19419edf197e3baa3b22de1f33184a1d8631141549751fbf1cfc24f7f",
            "92aeea552f562f4142946460635a6d7d2d75c89f8899ea2063a80c213bcf14aa",
            "ad1c327066e8bd7a0f5d5aca5ccd6666247d92fc2dfbee5d9c37c6a60ae948a8",
            "f1225528d87dccc2b06e0553a241fd3f0e56463cf69faafb18976401da111188",
            "1d7efcd9c5d218f4ccf4f3c4d59a286129e8e4090e6786a4fa7075313bec2ba3",
            "1144ab4443ad86e574690ad1c7ebe2e15b9b546031a63ddf596a5e9aba6c502d",
        ]
    );
    assert_eq!(checksums.len(), versions::MIGRATIONS.len());
}

#[tokio::test]
async fn migration_six_preserves_legacy_pending_additions_and_accepts_new_identity() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("migration-six.sqlite");
    let pool = crate::db::pool::create_pool(&database_path).await.unwrap();
    for version in 1..=5 {
        apply_migration(&pool, version).await.unwrap();
    }

    sqlx::query(
        "INSERT INTO skill_repository_pending_additions
         (repository_id, source_path, skill_id, skill_name,
          conflict_existing_skill_id, discovered_at)
         VALUES ('github:owner-repo-main', 'skills/legacy', 'legacy',
                 'Legacy', NULL, '2026-08-19T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    apply_migration(&pool, 6).await.unwrap();
    let migrated_columns = sqlx::query("PRAGMA table_info(skill_repository_pending_additions)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<String, _>("name").unwrap())
        .collect::<Vec<_>>();
    assert!(
        migrated_columns.contains(&"resolved_commit_sha".to_string()),
        "{migrated_columns:?}"
    );
    assert!(
        migrated_columns.contains(&"snapshot_digest".to_string()),
        "{migrated_columns:?}"
    );
    let legacy = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].resolved_commit_sha, None);
    assert_eq!(legacy[0].snapshot_digest, None);

    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-repo-main".to_string(),
            source_path: "skills/legacy".to_string(),
            skill_id: "legacy".to_string(),
            skill_name: "Legacy".to_string(),
            conflict_existing_skill_id: None,
            resolved_commit_sha: Some("a".repeat(40)),
            snapshot_digest: Some("sha256-v1:repository".to_string()),
            discovered_at: "2026-08-20T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();
    let refreshed = db::list_pending_additions(&pool).await.unwrap();
    let expected_commit_sha = "a".repeat(40);
    assert_eq!(
        refreshed[0].resolved_commit_sha.as_deref(),
        Some(expected_commit_sha.as_str())
    );
    assert_eq!(
        refreshed[0].snapshot_digest.as_deref(),
        Some("sha256-v1:repository")
    );
}

#[tokio::test]
async fn selected_release_fixtures_upgrade_with_backup_and_cascades() {
    let _guard = DATABASE_OPEN_TEST_LOCK.lock().await;
    for fixture in fixture_manifest().fixtures {
        let directory = TempDir::new().unwrap();
        let database_path = directory.path().join("db.sqlite");
        materialize_fixture(&fixture, &database_path).await;
        std::fs::copy(
            &database_path,
            directory
                .path()
                .join("db.sqlite.pre-migration-v0-stale.sqlite3"),
        )
        .unwrap();

        let pool = db::open_database(&database_path)
            .await
            .unwrap_or_else(|error| panic!("{} upgrade failed: {error}", fixture.tag));
        let versions =
            sqlx::query("SELECT version, checksum FROM schema_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(versions.len(), 6);
        for (index, row) in versions.iter().enumerate() {
            let version = i64::try_from(index + 1).unwrap();
            assert_eq!(row.try_get::<i64, _>("version").unwrap(), version);
            assert_eq!(
                row.try_get::<String, _>("checksum").unwrap(),
                descriptor_checksum(version).unwrap()
            );
        }
        let sentinel = sqlx::query("SELECT name, uid FROM skills WHERE id = 'fixture-skill'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            sentinel.try_get::<String, _>("name").unwrap(),
            "Fixture Skill"
        );
        assert!(!sentinel.try_get::<String, _>("uid").unwrap().is_empty());
        assert_owned_foreign_keys(&pool).await;
        assert!(file_has_table(&database_path, "fs_db_operations").await);
        // Migration 5 adds the incremental usage-scan file cache.
        assert!(file_has_table(&database_path, "skill_call_file_cache").await);
        let pending_columns = sqlx::query("PRAGMA table_info(skill_repository_pending_additions)")
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.try_get::<String, _>("name").unwrap())
            .collect::<Vec<_>>();
        assert!(pending_columns.contains(&"resolved_commit_sha".to_string()));
        assert!(pending_columns.contains(&"snapshot_digest".to_string()));
        // Migration 4 adds nullable per-skill provenance; pre-existing rows stay
        // NULL and are read as "provenance unknown".
        let provenance = db::get_skill_repository_provenance(&pool, "fixture-skill")
            .await
            .unwrap()
            .expect("fixture membership row survived migration 4");
        assert_eq!(provenance, (None, None), "{} provenance drift", fixture.tag);
        assert_eq!(
            sqlx::query("SELECT source_path FROM skill_repository_members WHERE skill_id = ?")
                .bind("fixture-skill")
                .fetch_one(&pool)
                .await
                .unwrap()
                .try_get::<String, _>("source_path")
                .unwrap(),
            "skills/fixture-skill"
        );
        assert!(sqlx::query("PRAGMA foreign_key_check")
            .fetch_optional(&pool)
            .await
            .unwrap()
            .is_none());

        let mut connections = Vec::new();
        for _ in 0..3 {
            connections.push(pool.acquire().await.unwrap());
        }
        for connection in &mut connections {
            let enabled = sqlx::query("PRAGMA foreign_keys")
                .fetch_one(&mut **connection)
                .await
                .unwrap()
                .try_get::<i64, _>(0)
                .unwrap();
            assert_eq!(enabled, 1);
        }
        drop(connections);

        insert_independent_history(&pool).await;
        db::delete_skill(&pool, "fixture-skill").await.unwrap();
        for relation in db::repos::skill_relations_spec::owned_skill_relations() {
            let sql = format!(
                "SELECT COUNT(*) AS count FROM {} WHERE {} = 'fixture-skill'",
                relation.table, relation.skill_column
            );
            let count = sqlx::query(&sql)
                .fetch_one(&pool)
                .await
                .unwrap()
                .try_get::<i64, _>("count")
                .unwrap();
            assert_eq!(count, 0, "{} did not cascade", relation.table);
        }
        for table in [
            "agent_skill_observations",
            "project_skill_installations",
            "skill_usage_metadata",
        ] {
            let sql = format!("SELECT COUNT(*) AS count FROM {table}");
            let count = sqlx::query(&sql)
                .fetch_one(&pool)
                .await
                .unwrap()
                .try_get::<i64, _>("count")
                .unwrap();
            assert_eq!(count, 1, "{table} history should remain independent");
        }

        let backup_count = sibling_files(&database_path, ".pre-migration-v0-").len();
        assert_eq!(backup_count, 1, "{} must retain one backup", fixture.tag);
        pool.close().await;
        let reopened = db::open_database(&database_path).await.unwrap();
        assert_eq!(
            sibling_files(&database_path, ".pre-migration-v0-").len(),
            backup_count,
            "{} current reopen created another backup",
            fixture.tag
        );
        reopened.close().await;
    }
}

async fn assert_preflight_rejection(mutation_sql: &str, expected: &str) {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("db.sqlite");
    let pool = db::open_database(&database_path).await.unwrap();
    pool.close().await;

    let raw_pool = crate::db::pool::create_pool(&database_path).await.unwrap();
    sqlx::query(mutation_sql).execute(&raw_pool).await.unwrap();
    raw_pool.close().await;

    let error = db::open_database(&database_path).await.unwrap_err();
    assert!(
        error.to_string().contains(expected),
        "unexpected error: {error}"
    );
    assert!(sibling_files(&database_path, ".pre-migration-").is_empty());
}

#[tokio::test]
async fn preflight_accepts_the_published_windows_v1_checksum_without_rewriting_it() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("db.sqlite");
    let pool = db::open_database(&database_path).await.unwrap();
    sqlx::query(
        "INSERT INTO skills
         (id, uid, name, description, file_path, canonical_path, is_central, source,
          content, scanned_at, fs_created_at, fs_updated_at)
         VALUES ('legacy-checksum-sentinel', 'legacy-checksum-uid', 'Legacy checksum sentinel',
                 '', '/fixture/SKILL.md', '/fixture/SKILL.md', 1, 'native', '',
                 '2026-01-01T00:00:00Z', NULL, NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE schema_migrations SET checksum = ? WHERE version = 1")
        .bind(versions::PUBLISHED_WINDOWS_V1_CHECKSUM)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let reopened = db::open_database(&database_path).await.unwrap();
    let stored_checksum = sqlx::query("SELECT checksum FROM schema_migrations WHERE version = 1")
        .fetch_one(&reopened)
        .await
        .unwrap()
        .try_get::<String, _>("checksum")
        .unwrap();
    assert_eq!(stored_checksum, versions::PUBLISHED_WINDOWS_V1_CHECKSUM);
    assert_eq!(
        sqlx::query("SELECT COUNT(*) AS count FROM skills WHERE id = 'legacy-checksum-sentinel'")
            .fetch_one(&reopened)
            .await
            .unwrap()
            .try_get::<i64, _>("count")
            .unwrap(),
        1
    );
    assert!(sibling_files(&database_path, ".pre-migration-").is_empty());
    reopened.close().await;
}

#[tokio::test]
async fn preflight_rejects_checksum_gap_and_future_versions_without_backup() {
    assert_preflight_rejection(
        "UPDATE schema_migrations SET checksum = 'tampered' WHERE version = 1",
        "checksum mismatch",
    )
    .await;
    let uppercase_alias = versions::PUBLISHED_WINDOWS_V1_CHECKSUM.to_ascii_uppercase();
    let uppercase_mutation =
        format!("UPDATE schema_migrations SET checksum = '{uppercase_alias}' WHERE version = 1");
    assert_preflight_rejection(&uppercase_mutation, "checksum mismatch").await;
    let prefixed_mutation = format!(
        "UPDATE schema_migrations SET checksum = 'legacy-{}' WHERE version = 1",
        versions::PUBLISHED_WINDOWS_V1_CHECKSUM
    );
    assert_preflight_rejection(&prefixed_mutation, "checksum mismatch").await;
    let cross_version_mutation = format!(
        "UPDATE schema_migrations SET checksum = '{}' WHERE version = 2",
        versions::PUBLISHED_WINDOWS_V1_CHECKSUM
    );
    assert_preflight_rejection(&cross_version_mutation, "checksum mismatch").await;
    assert_preflight_rejection(
        "DELETE FROM schema_migrations WHERE version = 1",
        "not contiguous",
    )
    .await;
    assert_preflight_rejection(
        "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (7, 'future', 'now')",
        "newer than supported",
    )
    .await;
}

async fn create_rebuild_collision_fixture(path: &Path) {
    let mut manifest = fixture_manifest();
    let fixture = manifest.fixtures.remove(0);
    materialize_fixture(&fixture, path).await;
    let pool = crate::db::pool::create_pool(path).await.unwrap();
    sqlx::query("CREATE TABLE __skill_relation_rebuild (id TEXT PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

async fn file_has_table(path: &Path, table: &str) -> bool {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .read_only(true);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    let present =
        sqlx::query("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)")
            .bind(table)
            .fetch_one(&mut connection)
            .await
            .unwrap()
            .try_get::<i64, _>(0)
            .unwrap()
            == 1;
    connection.close().await.unwrap();
    present
}

#[tokio::test]
async fn failed_fk_migration_restores_the_original_database() {
    let _guard = DATABASE_OPEN_TEST_LOCK.lock().await;
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("db.sqlite");
    create_rebuild_collision_fixture(&database_path).await;

    let error = db::open_database(&database_path).await.unwrap_err();
    assert!(error.to_string().contains("already exists"));
    backup::validate_sqlite_file(&database_path).await.unwrap();
    let backup_files = sibling_files(&database_path, ".pre-migration-v0-");
    assert_eq!(backup_files.len(), 1);
    assert!(
        !file_has_table(&backup_files[0], "schema_migrations").await,
        "backup unexpectedly contains migration metadata"
    );
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&database_path)
        .read_only(true);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    let migration_table = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap()
    .try_get::<i64, _>(0)
    .unwrap();
    assert_eq!(
        migration_table, 0,
        "migration metadata survived restore; open error was: {error}"
    );
    connection.close().await.unwrap();
    assert_eq!(sibling_files(&database_path, ".pre-migration-v0-").len(), 1);
    assert_eq!(sibling_files(&database_path, ".failed-migration-").len(), 1);
}

#[tokio::test]
async fn restore_failure_is_reported_and_keeps_backup_and_quarantine() {
    let _guard = DATABASE_OPEN_TEST_LOCK.lock().await;
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("db.sqlite");
    create_rebuild_collision_fixture(&database_path).await;
    backup::fail_next_restore_after_quarantine();

    let error = db::open_database(&database_path).await.unwrap_err();
    let message = error.to_string();
    assert!(message.contains("Database migration failed"));
    assert!(message.contains("backup restore also failed"));
    assert!(
        message.contains("injected restore failure"),
        "unexpected combined error: {message}"
    );
    assert_eq!(sibling_files(&database_path, ".pre-migration-v0-").len(), 1);
    assert_eq!(sibling_files(&database_path, ".failed-migration-").len(), 1);
}

#[tokio::test]
async fn rejected_backup_blocks_migration_before_any_schema_write() {
    let _guard = DATABASE_OPEN_TEST_LOCK.lock().await;
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("db.sqlite");
    let mut manifest = fixture_manifest();
    let fixture = manifest.fixtures.remove(0);
    materialize_fixture(&fixture, &database_path).await;
    backup::fail_next_backup_validation();

    let error = db::open_database(&database_path).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("injected backup validation failure"));
    assert!(!file_has_table(&database_path, "schema_migrations").await);
    assert!(sibling_files(&database_path, ".pre-migration-").is_empty());
}
