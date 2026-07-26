# Live DB integrity research - 2026-07-26

## Code evidence

- `skills_repo::delete_skill` deletes 7 relation tables through the pool without a transaction.
- Both branches of `delete_skills_not_in_scope` omit `collection_skills`, `skill_ai_tag_reviews`, and `skill_explanations`.
- `scanner::delete_scan_stale_rows` runs inside `persist_scan_batch`'s transaction but repeats the incomplete ownership list.
- File tracing is initialized before `db::init_database` in the Tauri setup path.
- `agent_skill_observations` uses an independent `row_id` and touched-agent keep-set. Consumers can read it without joining `skills`.

## Read-only database inventory

Queried `~/.skillsmanage/db.sqlite` and four `~/.skillsmanage/targets/*/db.sqlite` files with explicit `LEFT JOIN skills` predicates. No data was modified.

| Database set | Owned relation orphan rows | Independent observation rows without parent |
|---|---:|---:|
| Local | 1 (`skill_explanations`) | 33 (`agent_skill_observations`) |
| Four target caches | 0 | 0 |

The 33 observations are not evidence for deletion. They demonstrate that observation lifetime is not equivalent to `skills` parent ownership, so parent absence alone cannot classify them as invalid.

## Planning consequence

The centralized owned-relation list contains 7 tables, not 8. Startup repair and the next FK task must exclude `agent_skill_observations`. A tracing audit with IDs/counts is diagnostic evidence, not a recoverable backup of deleted user-authored collection/tag rows.

`skillport-cli` initializes the database without installing the desktop file-tracing subscriber. The selected audit-then-delete policy therefore persists the JSON in `operation_logs` inside the repair transaction; tracing is supplemental only.
