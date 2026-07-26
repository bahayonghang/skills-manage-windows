# Design: Versioned SQLite Migration and FK Enforcement

## 1. Boundary and Public Flow

Production callers stop composing `create_pool` and `init_database*` themselves. The DB module exposes two path-aware entry points:

```rust
pub async fn open_database(path: &Path) -> Result<DbPool, sqlx::Error>;
pub async fn open_database_for_remote_home(
    path: &Path,
    remote_home: &str,
) -> Result<DbPool, sqlx::Error>;
```

Both call one internal orchestrator with a seed profile. Desktop setup, `CliContext::open_default`, and `TargetRegistry::remote_db_for` must use these functions. `create_pool` becomes internal. Tests use explicit shared memory/file helpers so production cannot accidentally initialize a file DB without the path required for backup.

The orchestrator owns the pool until migration succeeds. A remote pool is inserted into the target registry only afterward; the local pool is managed by Tauri only afterward.

## 2. Per-Connection Foreign-Key Contract

`SqlitePoolOptions::after_connect` runs on every connection:

1. Execute `PRAGMA foreign_keys = ON`.
2. Query `PRAGMA foreign_keys` on the same connection.
3. Return `sqlx::Error` unless the value is exactly `1`.

WAL remains a `SqliteConnectOptions` setting. Test pool constructors reuse the same pool-options helper. Legacy fixture setup may use a bare connection only with the existing semantic-exemption comment, and enables FK before exercising the migrated database.

## 3. Migration Registry and Checksums

`db/migrations.rs` becomes the runner and keeps version modules under `db/migrations/versions/`. A descriptor contains a positive contiguous version and immutable checksum material loaded with `include_str!`. SHA-256 of all source files reachable by that migration is the stored checksum, so changing applied migration code causes deterministic drift detection.

```rust
struct MigrationDescriptor {
    version: i64,
    source: &'static str,
}

CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    checksum   TEXT NOT NULL,
    applied_at TEXT NOT NULL
);
```

Startup preflight reads rows in version order and rejects before mutation when:

- versions do not start at 1 or contain a gap;
- an applied version is newer than the binary;
- an applied checksum differs from the current descriptor;
- descriptors themselves are non-contiguous.

When `schema_migrations` is absent, the runner treats the DB as unversioned without writing. Migration 1 creates the table inside its own transaction before inserting version 1; an empty applied set is therefore distinguishable without a bootstrap write.

Each pending migration acquires one connection, begins one transaction, applies all DDL/data steps, inserts its `schema_migrations` row, and commits. A failure rolls back that migration. The outer backup/restore boundary also reverses earlier migrations or repair commits from the same startup attempt.

### Migration 1: Legacy Baseline

Normalize empty and unversioned `v0.10.9`-`v0.10.14` databases to the current pre-FK schema. Existing schema module functions and `ensure_column` are converted to execute on the acquired migration connection so all legacy CREATE/ALTER/DROP/backfill work is transaction-scoped. An empty DB follows this same path.

The migration 1 checksum concatenates its version module plus every schema/helper source it invokes. After release those baseline sources are frozen; later schema work adds migration 3+ instead of editing migration 1 inputs. A checksum-lock test records the expected digest and fails on accidental formatting or logic drift.

### Migration 2: Owned Skill FK Cascade

After the predecessor repair succeeds, rebuild exactly the seven owned tables with:

```sql
FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
```

For each table: create a fixed-name replacement with the exact current columns/PK/defaults, copy rows, assert row counts, drop the old table, rename the replacement, and recreate indexes. Identifiers come from compile-time definitions only. The migration ends with explicit `PRAGMA foreign_key_check`; any row is an error and rolls back all seven rebuilds.

## 4. Backup, Repair, Migration, and Restore Ordering

The file-database sequence is:

```text
open pool with FK enabled
  -> read-only migration preflight
  -> if legacy/pending and source DB existed: create/verify backup
  -> migration 1 when pending
  -> repair_orphan_skill_relations (persistent audit + delete transaction)
  -> migration 2 when pending
  -> global foreign_key_check
  -> seed
  -> return pool
```

Whether the source path existed with a non-empty SQLite file is captured before opening the pool. An empty newly created file skips backup. A current version with valid checksums has no repair/migration work and creates no repeated backup.

Backup uses SQLite `VACUUM INTO ?` with a bound canonical sibling path while the pool is still private; the path is never interpolated into SQL. After a direct read-only connection runs `PRAGMA integrity_check` on the snapshot, blocking filesystem work syncs and publishes a uniquely named `db.sqlite.pre-migration-v<source>-<attempt>.sqlite3`. Each pending attempt creates a fresh snapshot. Only after the new backup is durable may older backups for that DB/source version be removed, leaving the newest last-known-good copy.

If any mutation/validation step after backup fails:

1. Close the private pool.
2. Move the failed DB aside to a unique diagnostic sibling.
3. Restore a copied backup through a temporary sibling rename, leaving the backup intact.
4. Remove stale live `-wal` and `-shm` companions.
5. Verify restored `integrity_check` on a fresh direct connection that cannot invoke migration recursively.
6. Return the original migration/repair/seed error; if restore also fails, return a combined error and keep both diagnostic files.

Startup does not silently retry migration in the same process. This is a narrowly scoped migration recovery protocol, not the general FS+DB operation journal owned by the next child.

Checksum/future-version failures occur before mutation and do not trigger restore. Backup creation or validation failure also blocks before schema/repair writes.

## 5. Runtime Delete Simplification

After migration 2:

- `delete_skill`: delete the parent skill, prune empty repositories, commit.
- `delete_skills_not_in_scope`: delete parent rows absent from the keep set, prune, commit.
- scanner stale cleanup: keep agent-scoped observation/install cleanup, delete stale parent skills, prune, commit.

SQLite cascades the seven owned relations. The relation specs remain the single source for migration/repair/FK-preflight tests; runtime call sites must not retain duplicate manual owned-table loops. Independent observation/project/usage lifecycles remain unchanged.

## 6. Release Fixtures and Fault Tests

Commit readable fixtures under `src-tauri/tests/fixtures/db/`:

- one SQL snapshot per selected tag;
- a manifest containing tag, resolved commit, and SHA-256;
- sentinel parent/owned/independent rows supported by that version.

Tests materialize each SQL snapshot into a real temporary SQLite file, invoke the same production open API, and assert latest versions/checksums, preserved sentinel data, seven cascade FKs, empty `foreign_key_check`, and expected backup.

Fault coverage includes:

- applied checksum drift, version gap, and future version: zero mutations;
- a reserved rebuild-table collision after legacy normalization: migration failure, full backup restore, no repair audit/schema residue;
- corrupted/rejected backup: migration never begins;
- multiple acquired pool connections: `foreign_keys=1` on each;
- parent deletion: seven cascades, independent history retained;
- second open at latest version: idempotent, no new backup.

Migration tests live in a dedicated module instead of expanding `db/tests.rs` further.

## 7. Compatibility, Documentation, and Rollback

- No IPC or frontend payload changes.
- Update English/Chinese data-model and backend docs to describe version files, checksum validation, backups, and FK enforcement.
- Add a backend `database-migrations.md` code-spec and update `skill-deletion-integrity.md` from manual runtime cascade to DB cascade after version 2.
- A code rollback cannot downgrade a DB that contains an unknown future version; the older binary must fail closed. The retained pre-migration backup is the rollback artifact.
- No new production dependency is needed: SQLx/SQLite provide DB operations, `sha2` already exists, and existing blocking-FS helpers cover sync/rename work.

## 8. Expected Files

- `src-tauri/src/db/{pool.rs,migrations.rs,seed.rs,mod.rs}`
- `src-tauri/src/db/migrations/{backup.rs,tests.rs,versions/*.rs}`
- `src-tauri/src/db/schema/*.rs` and `db/repos/{skills_repo.rs,skill_relations_repo.rs}`
- `src-tauri/src/services/scanner/persistence.rs`
- `src-tauri/src/{lib.rs,cli_api/mod.rs,targets/registry.rs,test_support.rs}`
- `src-tauri/tests/fixtures/db/*`
- `docs/architecture/{data-model.md,backend.md}` and Chinese mirrors
- `.trellis/spec/backend/{index.md,database-migrations.md,skill-deletion-integrity.md,test-support.md}`
