# Unknown-Source Central Reset Contract

## 1. Scope / Trigger

Use this contract when adding, changing, or calling the target-scoped reset
that deletes Central skills with no repository membership so the user can
re-import them with provenance.

This is **not** `clear_skill_update_inventory`, **not**
`rebuild_startup_database`, and **not** a wipe of GitHub-backed Central skills.
Filesystem + DB deletion reuses journaled batch delete
(`skill-deletion-integrity.md`, `fs-db-operation-journal.md`).

## 2. Signatures

```rust
pub async fn list_unknown_source_central_skill_ids(
    pool: &DbPool,
) -> Result<Vec<String>, CentralSkillsError>;

pub async fn preview_reset_unknown_source_skills_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
) -> Result<ResetUnknownSourceSkillsPreview, CentralSkillsError>;

pub async fn reset_unknown_source_skills_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    confirmed_skill_ids: &[String],
    remove_copy_agent_ids: &[String],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError>;
```

IPC (commands layer, `resolve_target_context()` once):

```text
preview_reset_unknown_source_skills()
  -> ResetUnknownSourceSkillsPreview { skillIds, preview }

reset_unknown_source_skills({ skillIds, removeCopyAgentIds })
  -> BatchDeleteCentralSkillResult
```

Candidate SQL (frozen target pool only):

```sql
SELECT s.id
FROM skills s
WHERE s.is_central = 1
  AND NOT EXISTS (
    SELECT 1 FROM skill_repository_members m WHERE m.skill_id = s.id
  )
ORDER BY s.id
```

## 3. Contracts

- One reset writes only the current `TargetContext` pool and that target's
  Central root. Local must not mutate SSH/WSL cache DBs or remote files, and
  the reverse is also forbidden.
- Preview lists unknown-source ids **without** taking the mutation lock.
- Apply acquires `acquire_target_mutation_guard` once, re-lists unknown-source
  ids, intersects with `skillIds` from the renderer, re-runs delete preview,
  and deletes only ids that appear in `preview.previews`. Delete helpers must
  not acquire the same lock again.
- Renderer confirm must pass `preview.previews[].skill_id`, not inventory
  Unsupported ids and not `preview.failed` ids.
- Empty `skillIds`: no delete. Inventory/pending additions clear only when the
  re-listed unknown-source set is also empty (stale Unsupported rows).
- After any successful delete (`succeeded.len() > 0`), clear that pool's
  `skill_update_inventory_*` and `pending_additions`. All-fail does not clear.
- Copy installs default to keep; `removeCopyAgentIds` intersects per skill.
  Symlink installs follow existing batch delete auto-unlink.
- Outer IPC failure uses `central.reset_failed` except mutation lock, which
  uses `central_skills.mutation_lock_failed`. Per-skill failures stay on the
  batch result DTO (`error_code` + public `error`).
- Operation log action `central.reset_unknown_source` records target kind and
  counts only. No paths, tokens, URLs, or skill bodies.
- Tests use `mem_pool` / `file_pool` / `FakeRunner` / TempDir. Production
  `~/.skillsmanage/db.sqlite` and live `targets/*/db.sqlite` are forbidden
  fixtures.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Preview, zero unknown-source ids | Empty `skillIds` + empty preview; no FS/DB writes |
| Confirm with `preview.previews` ids | Delete those ids if they still have no membership and still preview |
| Confirmed id gained membership before apply | Skipped; not deleted |
| Confirmed id fails delete preview | Skipped; row and files remain |
| GitHub-backed Central skill | Never in candidate set |
| Mutation lock busy/timeout | IPC `central_skills.mutation_lock_failed` |
| Other outer apply/preview failure | IPC `central.reset_failed` |
| Partial item failures | Result `failed[]` with stable `error_code`; inventory still cleared if any succeeded |
| Cross-target apply | Other pool and Central root unchanged |

## 5. Good / Base / Bad Cases

- **Good**: Local `file_pool` with one membership skill and one npx leftover;
  apply deletes only the leftover dir/row and clears that pool's inventory.
- **Base**: Empty confirmed ids on a pool with only GitHub-backed skills;
  no delete; stale Unsupported inventory cleared.
- **Bad**: Deleting `inventory.unsupported` skill ids, calling
  `rebuild_startup_database`, `DELETE FROM skills` without journaled delete,
  listing candidates outside the lock then deleting without re-checking
  membership, or opening `C:\Users\<dev>\.skillsmanage\db.sqlite` in tests.

## 6. Tests Required

- Local: membership skill retained; unknown-source dir/row gone; inventory
  and pending additions cleared on that pool only.
- Fake SSH: remote unknown-source row deleted; local sentinel files unchanged.
- Two `file_pool`s: reset on A does not mutate B.
- Preview-failed / outside-central-root id in `skillIds` is not deleted.
- Confirmed id that already has membership is not deleted.
- After delete, the same skill id can be re-seeded and assigned membership
  (`list_unknown_source_central_skill_ids` no longer returns it).
- Frontend: Local and SSH show the Unsupported reset control; confirm disabled
  at 0; cancel does not invoke apply; preview/apply rejection uses
  `formatBackendError`; partial apply lists `skill_id` + reviewed code and
  keeps the dialog open.

## 7. Wrong vs Correct

### Wrong

```rust
delete_central_skills_impl(pool, inventory.unsupported.skill_ids).await?;
```

```rust
reset_unknown_source_skills_impl(pool, target, &[], &[]).await?;
// empty skillIds must not delete leftover unknown-source skills
```

### Correct

```rust
let _guard = acquire_target_mutation_guard(target, "reset unknown-source Central skills", timeout).await?;
let listed = list_unknown_source_central_skill_ids(pool).await?;
let deletable = listed.into_iter().filter(|id| confirmed.contains(id)).collect::<Vec<_>>();
// re-preview; delete only preview.previews ids via delete_central_skills_under_guard
```
