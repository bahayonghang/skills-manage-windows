# Data Model

SQLite is the single persistence layer. `~/.skillsmanage/db.sqlite` is opened in WAL mode at startup and migrated incrementally — there is no Diesel-style migration directory.

## Schema Init Order

Tables are created in dependency order so foreign keys land on existing primaries. `src-tauri/src/db/schema/mod.rs::init` runs each domain in turn:

```text
core         skills / skill_installations / agent_skill_observations / agents
 └─ collections    collections / collection_skills
    └─ metadata    repositories / update_states / tags / tag_links / ai_reviews
       └─ discovery   scan_directories
          └─ projects   projects / project_skill_installations
             └─ settings settings / operation_logs (+6 indexes)
                └─ marketplace registries / skills / explanations (+8 ALTERs)
```

Every `CREATE TABLE` is wrapped in `IF NOT EXISTS` and incremental columns are added through `migrations::ensure_column` so the schema is idempotent across versions.

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

## Field Reference

Field details are regenerated from `src-tauri/src/db/schema/*.rs` by `scripts/build-schema-table.mjs` — never edit the generated section by hand.

<!--@include: ./_generated/data-model.md-->

## Migration Contract

- New columns: add to the schema `init.rs` and append a `migrations::ensure_column` call so old DBs upgrade in-place.
- Renames: ship a Rust migration that copies + drops; SQLite cannot rename columns reliably across all builds we ship.
- Drops: never drop a column the UI is still reading. Use a release-cycle deprecation: stop writing → migrate readers → drop in the next version.

Last reviewed: 2026-05-04
