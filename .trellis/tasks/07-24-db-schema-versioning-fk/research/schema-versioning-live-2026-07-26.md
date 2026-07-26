# Schema Versioning Live Evidence (2026-07-26)

## Repository State

- Branch: `dev`
- Evidence base after predecessor closeout: `ac52e67a`
- Predecessor work: `21eb82a9` (`07-24-db-stale-cleanup-fix`)
- Parent progress at research time: `6/16`
- Existing unrelated Trellis runtime/config dirt and other task directories remain excluded.

## Production Entry Points

All production databases eventually use `db::create_pool`, but initialization is currently a separate call:

| Entry | Current path |
| --- | --- |
| Desktop local DB | `lib.rs`: resolve `~/.skillsmanage/db.sqlite`, create pool, call `init_database` |
| `skillport-cli` | `cli_api/mod.rs::CliContext::open_default`: create pool, call `init_database` |
| SSH/WSL cache | `targets/registry.rs::remote_db_for`: resolve target cache path, create pool, call `init_database_for_remote_home` |

The path is no longer available to `init_database*`, so backup cannot be made mandatory without a path-aware open/init API. Remote pools are initialized before insertion into the registry cache, which gives migration exclusive startup ownership for that target DB.

## Current Schema Mechanism

- `db/schema/mod.rs::init` sequentially calls nine schema modules and then returns.
- `db/migrations.rs::ensure_column` probes `PRAGMA table_info` and runs an `ALTER TABLE` when a column is absent.
- DDL executes through the pool, not one acquired connection/transaction.
- No `schema_migrations` table or checksum exists.
- `db/seed.rs::init_database_with_agents` currently runs `schema::init`, then the predecessor's orphan repair, then seed.
- `db/pool.rs::create_pool` enables WAL but has no per-connection `foreign_keys` hook.

## Published Compatibility Window

The user selected every recent tagged Windows release in the 0.10 line:

| Tag | Date | Relevant observation |
| --- | --- | --- |
| `v0.10.9` | 2026-05-27 | Oldest selected Windows release baseline |
| `v0.10.10` | 2026-06-13 | Large schema-module delta from `v0.10.9` |
| `v0.10.12` | 2026-07-07 | No schema-file delta from `v0.10.10` |
| `v0.10.13` | 2026-07-13 | No schema-file delta from `v0.10.12` |
| `v0.10.14` | 2026-07-15 | Added core/usage schema since `v0.10.13` |

`v0.10.14..HEAD` additionally changes metadata schema. The missing `v0.10.11` tag is not a fixture target. Each fixture must record its source tag/commit and freeze that tag's schema rather than calling current initialization code.

## Ownership and FK Boundary

The predecessor established the exact seven owned relations and a reusable compile-time list in `skill_relations_repo.rs`. Live local/target inventory proved `agent_skill_observations` is independent; project snapshots and usage history are likewise excluded. The FK migration must consume the same ownership definition and keep these exclusions.

The predecessor also established a correctness-critical repair transaction: inventory -> stable JSON audit -> orphan delete -> commit. Final startup ordering must place the whole-DB backup before this repair and execute the FK rebuild only after it succeeds.

## Backup Constraints

- SQLite runs in WAL mode; copying only `db.sqlite` while a live writer exists is not a consistent backup.
- Production migration runs before the pool is exposed to commands/registry consumers, so SQLite `VACUUM INTO` can create a consistent snapshot without a new dependency.
- Publish the backup through a sibling temporary path and durable rename. On failure, close the pool before replacing the DB and remove stale `-wal`/`-shm` companions.
- New empty databases have no user state and do not need a pre-migration backup.

## Test and Documentation Gaps

- No tag-specific DB fixtures exist. Current legacy tests hand-create one partial table and are insufficient release evidence.
- `db/tests.rs` is already large; migration fixtures/fault tests should live in a focused migration test module.
- `docs/architecture/data-model.md` and the Chinese mirror explicitly say there is no migration directory and `ensure_column` is the extension contract; both become stale.
- `test_support` requires shared fixtures except documented bare legacy pools. Tag fixtures qualify as semantic legacy exceptions and must be recorded in the spec.
