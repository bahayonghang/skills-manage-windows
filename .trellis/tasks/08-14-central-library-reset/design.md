# Design: unknown_source Central reset for re-import

## 1. Problem Boundary

Reset is a **target-scoped batch delete of Central skills that have no repository membership**, followed by clearing that target's Update Center inventory so the user can GitHub-import those skill ids with provenance.

It does not rebuild SQLite, does not wipe GitHub-backed skills, and does not invent membership.

```text
active TargetContext
  -> list unknown-source Central skill ids (no membership)
  -> existing preview_delete / delete_central_skills
  -> clear current-target inventory + pending additions
  -> renderer refresh Central + Update Center
```

## 2. Decisions

| Area | Decision | Reason |
| --- | --- | --- |
| Candidate set | `is_central = 1` AND no `skill_repository_members` row | Same predicate as `UnsupportedSkillReasonCode::UnknownSource`; ignores stale inventory counts (screenshot 54 vs a later refresh). |
| Target | Current `TargetContext` only: Local, SSH, and WSL | User needs both Local and SSH; WSL already shares the remote delete path. |
| Cross-target | Never write another target's pool or Central root | Local and SSH caches are separate files under `~/.skillsmanage/` vs `targets/<id>/`. |
| Delete machinery | Reuse `preview_delete_central_skills_*` and `delete_central_skills_*` | Already journals FS+DB, cascades owned relations, unlinks symlink installs, optional copy deletes. |
| Copy installs | Default keep; preview checkbox like BatchDelete | Matches existing Central delete UX; npx leftovers on agent copy dirs stay unless chosen. |
| Inventory | After successful delete, `clear_skill_update_inventory_impl(pool, None)` on **that** pool | Unsupported tab would otherwise keep stale rows. |
| Startup rebuild | Out of scope | Healthy DBs must not use `rebuild_startup_database`. |
| Tests | Isolated `mem_pool` / `file_pool` / `FakeRunner` only | Developer Local store is already repaired; tests must not open `~/.skillsmanage/db.sqlite` or live SSH caches. |

## 3. Candidate Selection

Do not trust Update Center inventory as the delete set. Compute ids from the frozen target DB:

```sql
SELECT s.id
FROM skills s
WHERE s.is_central = 1
  AND NOT EXISTS (
    SELECT 1 FROM skill_repository_members m WHERE m.skill_id = s.id
  )
```

Skills with GitHub membership stay. Empty candidate set: preview succeeds with zero ids; apply is a no-op besides optional inventory clear if the user still confirms (prefer: disable confirm when count is 0).

## 4. IPC and Layering

Commands stay thin shells in `commands/skills.rs` (same family as batch delete):

```text
preview_reset_unknown_source_skills()
  -> { skillIds, preview: BatchDeleteCentralSkillPreviewResult }

reset_unknown_source_skills({ removeCopyAgentIds: string[] })
  -> BatchDeleteCentralSkillResult
```

Service (new helpers next to central_skills delete, not a second delete engine):

1. `resolve_target_context()`
2. `list_unknown_source_central_skill_ids(pool)`
3. Preview: existing Local or SSH preview with those ids
4. Apply: existing Local or SSH batch delete with `BatchDeleteCentralSkillRequest { skill_id, remove_agent_ids }` filled from the shared copy-agent selection
5. On full or partial success: `clear_skill_update_inventory_impl(&pool, None)` for that pool only
6. Operation log via existing delete logging + reset-specific counts (target kind, attempted, succeeded, failed). Redact paths.

Typed `CentralSkillsError` if needed for empty-target messaging; IPC uses stable `code:message` already understood by `IpcError`. Prefer a dedicated code such as `central.reset_failed` only for the outer command failure; per-skill failures stay on the existing batch result DTO.

Frontend:

- `centralSkillsStore` owns preview/apply (it already owns `deleteCentralSkills`).
- Update Center `UnsupportedTabPanel` / dialog footer exposes the action when preview count > 0.
- Reuse `BatchDeleteCentralSkillsDialog` (or a thin wrapper) for confirm + copy checkboxes.
- Visible errors go through `formatBackendError`; do not add a new `String(err)` toast path.
- After success: reload Central skills and `clearInventory` / reload Update Center store for the active target.

Register new commands in `ipc_registry`, `commandMap` / generated map, browser fixtures, and `pnpm docs:gen` as required by existing IPC codegen.

## 5. Filesystem Effects

| Target | Deleted | Not deleted |
| --- | --- | --- |
| Local | Candidate dirs under that Local Central root (test fixtures or user-confirmed Local store) | Other targets' caches; GitHub-backed Local skills; secrets |
| SSH/WSL | Candidate dirs under **that remote** `~/.skillsmanage/skills` via existing remote journal | Windows Local Central; other SSH ids; unselected copy installs |

Native installation rows are dropped with the parent skill; native paths outside Central are not added to the remote removable list today. SSH evidence for the broken target shows `file_path` already under remote Central, so those files are in the delete plan.

## 6. Concurrency

Apply uses the same target mutation guard as batch delete. Do not take the Central-update exclusive job lease unless a running refresh must be cancelled; if refresh is in flight, fail with the existing busy/lock error rather than unlocking.

## 7. Tests Required

- Local `file_pool` + TempDir Central: two skills (one with membership, one without); preview only the unknown; apply deletes only that dir and row; membership skill remains; inventory unsupported row for the deleted id is gone.
- Fake SSH: same split; remote script/argv assertions reuse delete tests; Local temp files unchanged.
- Cross-target: two pools; reset on pool A does not mutate pool B.
- Cancel/preview-only: no FS/DB writes.
- Copy-install checkbox: default retain; selected copy removed (reuse delete coverage where possible).
- Frontend: Local vs SSH active target still shows the button; confirm disabled at 0; formatBackendError on rejection; inventory/store refresh after success.
- Grep/contract: production reset entrypoints do not hardcode `app_data_dir()` as the test fixture path.

## 8. Wrong vs Correct

```rust
// Wrong: delete whatever the stale Unsupported tab lists.
delete_central_skills_impl(pool, inventory.unsupported.skill_ids).await?;

// Correct: recompute unknown-source ids from the frozen target DB, then reuse delete.
let ids = list_unknown_source_central_skill_ids(pool).await?;
```

```rust
// Wrong: tests open the developer's Local store.
create_pool(Path::new(r"C:\Users\lyh\.skillsmanage\db.sqlite")).await?;

// Correct: test_support::file_pool() / mem_pool() and FakeRunner.
```
