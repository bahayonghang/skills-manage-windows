# Data Model

SQLite is the single persistence layer. `~/.skillsmanage/db.sqlite` and target-cache databases are opened through one path-aware API with WAL mode and per-connection foreign-key enforcement.

## Versioned Initialization

`schema_migrations(version, checksum, applied_at)` records contiguous immutable migrations. Startup validates the descriptor sequence, applied versions, future versions, and SHA-256 checksums before writing. The fixed order is:

```text
open pool with FK enabled
  -> read-only migration preflight
  -> verified whole-database backup when an existing file has pending work
  -> migration 1 legacy baseline
  -> orphan inventory / audit / repair
  -> migration 2 owned-relation FK rebuild
  -> foreign_key_check
  -> built-in seed
```

Migration 1 freezes the legacy `v0.10.9` through `v0.10.14` normalization logic. Migration 2 rebuilds the seven skill-owned relation tables with `FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE`. Observations, project snapshots, call history, and usage-resolution metadata intentionally remain independent.

Before any repair or migration write, an existing database with pending work is snapshotted with bound-path `VACUUM INTO`, integrity-checked, synced, and published as a sibling `*.pre-migration-v<source>-*.sqlite3`. A failed upgrade closes the private pool, quarantines the failed file, restores a copy of the backup, verifies it, and still returns the startup error.

## Repositories

`db/repos/` exposes one repo per logical object:

| Repo | Tables |
| --- | --- |
| `skills_repo` | `skills` |
| `installations_repo` | `skill_installations` |
| `observations_repo` | `agent_skill_observations` |
| `agents_repo` | `agents` |
| `collections_repo` | `collections`, `collection_skills` |
| `repositories_repo` | `skill_repositories`, `skill_repository_members` |
| `update_states_repo` | `skill_update_states` |
| `tags_repo` | `skill_tags`, `skill_tag_links`, `skill_ai_tag_reviews` |
| `projects_repo` | `projects`, `project_skill_installations` |
| `scan_dirs_repo` | `scan_directories` |
| `settings_repo` | `settings` |
| `operation_logs_repo` | `operation_logs` |

Repos hide raw `sqlx::query()` calls. Higher layers (commands / services) take a `&DbPool` and call repo methods.

Batch metadata mutations validate all referenced IDs before writing, use a
shared conservative SQLite bind budget, and keep every chunk in one top-level
transaction. Repository/tag replacements and collection deletion therefore
return either the complete new state or the unchanged old state. Project parent
deletion uses the pool's per-connection foreign-key enforcement for cascade.

## Field Reference

Field details are regenerated from `src-tauri/src/db/schema/*.rs` by `scripts/build-schema-table.mjs` — never edit the generated section by hand.

<!--@include: ./_generated/data-model.md-->

## Migration Contract

- Released migration sources and their checksums are immutable. Add a new contiguous descriptor for every later schema or data change.
- Each migration and its `schema_migrations` row commit in one transaction; table rebuilds include row-count guards and `foreign_key_check`.
- Local desktop, `skillport-cli`, SSH cache, and WSL cache must call `open_database*`; production code must not compose a raw pool with initialization.
- Older binaries reject unknown future versions instead of attempting a downgrade. The retained pre-migration snapshot is the rollback artifact.

Last reviewed: 2026-07-26
