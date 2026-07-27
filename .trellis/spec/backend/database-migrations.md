# Versioned SQLite Migration Contract

## 1. Scope / Trigger

Use this contract for every SQLite schema or data migration, database open path,
whole-database migration backup, restore path, FK change, or production DB pool
constructor. It covers the local desktop database, `skillport-cli`, and SSH/WSL
target-cache databases.

## 2. Signatures

```rust
pub async fn open_database(path: &Path) -> Result<DbPool, sqlx::Error>;
pub async fn open_database_for_remote_home(
    path: &Path,
    remote_home: &str,
) -> Result<DbPool, sqlx::Error>;
```

Production callers use these path-aware functions. Raw pool construction and
pool-only initialization are test-only or documented legacy-fixture seams.

## 3. Contracts

- Every pooled connection executes `PRAGMA foreign_keys = ON` and reads back
  exactly `1` in `after_connect`; WAL remains a connect option.
- `schema_migrations(version, checksum, applied_at)` versions start at 1 and
  are contiguous. Released migration sources and locked SHA-256 digests are
  immutable; later changes add a descriptor.
- A migration checksum includes only dedicated immutable migration material.
  Shared compile-time contracts such as `skill_relations_spec.rs` must be
  isolated from mutable runtime repair/repository code before their source is
  added with `include_str!`; otherwise an unrelated runtime fix would make
  already-migrated databases fail checksum preflight.
- Startup preflight is read-only and rejects descriptor gaps, applied gaps,
  unknown future versions, and checksum mismatch before backup or mutation.
- File startup order is `open private pool -> preflight -> backup when pending
  -> legacy baseline -> orphan repair/audit -> FK migration -> global
  foreign_key_check -> seed -> publish pool`.
- Existing non-empty files with pending work receive a fresh bound-path
  `VACUUM INTO` snapshot. The snapshot is integrity-checked and synced before
  publish. Only then may older same-source-version backups be pruned.
- A post-backup failure explicitly rolls back the active migration, closes and
  drops the pool, quarantines the failed DB, preserves the backup, restores a
  copy through a temporary sibling, removes stale WAL/SHM, verifies integrity,
  and returns failure without retrying migration.
- Migration 1 and its metadata row share one transaction. Its baseline sources
  normalize empty DBs and the frozen `v0.10.9`, `v0.10.10`, `v0.10.12`,
  `v0.10.13`, and `v0.10.14` fixtures.
- Migration 2 consumes the compile-time relation specs, rebuilds all seven
  owned tables with `ON DELETE CASCADE`, guards row counts, recreates indexes,
  and runs `foreign_key_check` before commit.
- Migration 3 adds the independent `fs_db_operations` recovery journal,
  target/operation/phase checks, lookup indexes, and a partial unique index
  allowing only one nonterminal operation per target and skill. Its manifest
  contents are not part of operation-log export.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Descriptor or DB version gap | Fail preflight; no backup or write |
| Future version or checksum mismatch | Fail preflight; no backup or write |
| Existing non-empty DB with pending migration | Create and validate a new source-version backup |
| Empty/new or checksum-current DB | Do not create a backup |
| Backup creation or validation fails | Block before repair/migration |
| Migration or FK validation fails after backup | Restore original DB, retain backup and quarantine, return failure |
| Restore also fails | Return combined error and retain backup plus quarantine |
| Any pooled connection has `foreign_keys != 1` | Reject that connection |
| Table rebuild row count changes | Roll back migration 2 |
| Migration 3 active-operation uniqueness or schema check fails | Roll back migration 3 and restore the pre-migration database backup |

All errors below command boundaries remain `sqlx::Error`; backup, metadata,
repair audit, restore, and FK validation are never best effort.

## 5. Good / Base / Bad Cases

- Good: a `v0.10.9` file is snapshotted, normalized, repaired, FK-rebuilt, and
  reopened at version 2 with its sentinel row intact.
- Base: a current version-2 DB passes checks, seeds idempotently, and creates no
  additional backup.
- Good: a reserved rebuild-table collision restores the exact unversioned
  source and returns the migration error.
- Bad: add a column to released migration-1 schema source, update its checksum,
  and silently redefine history.
- Bad: checksum an entire runtime repository module just because migration code
  consumes one constant from it; later repair-only edits would rewrite history.
- Bad: call `create_pool` then `init_database` from a production entry point;
  the path required for mandatory backup has already been lost.

## 6. Tests Required

- Manifest/checksum-locked readable SQL fixtures for all five selected tags.
- A locked digest for each descriptor, including migration 3; changing runtime repair code alone must
  not change an already-released migration digest.
- Fixture pre-schema assertions, three contiguous migration rows, preserved
  sentinel data, seven cascade FKs, empty `foreign_key_check`, and idempotent
  current reopen with no extra backup.
- Multiple simultaneous pool connections each report `foreign_keys=1`.
- Checksum drift, applied gap, and future version reject without backup/write.
- Backup refusal prevents schema writes; injected migration failure restores;
  injected restore failure reports both errors and keeps recovery artifacts.
- Parent deletion cascades the seven owned relations while observations,
  project snapshots, and usage metadata remain.
- Minimum gate: focused `cargo test db::migrations --locked`, full
  `cargo test db:: --locked`, Rust fmt/Clippy/tests, and `just ci`.

## 7. Wrong vs Correct

### Wrong

```rust
let pool = db::create_pool(&path).await?;
db::init_database(&pool).await?;
publish(pool);
```

### Correct

```rust
let pool = db::open_database(&path).await?;
publish(pool);
```

The correct boundary retains the path and private pool ownership until backup,
migration, recovery validation, FK checks, and seed all succeed.
