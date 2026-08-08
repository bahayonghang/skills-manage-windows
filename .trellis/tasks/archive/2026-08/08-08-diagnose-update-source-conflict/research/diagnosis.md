# Update Center Skills-scope retry conflict diagnosis

## Conclusion

The screenshot contains two separate conditions:

1. `archive-planning` is still tracked locally at
   `skills/developer-tools-integrations/archive-planning`, but upstream removed that skill on
   2026-07-22. Regular mode therefore correctly emits
   `central_updates.skill_source_missing`; there is no same-id replacement to relocate to.
2. The row-level "re-check in incremental mode" action introduced in `b4715bf0` has a merge
   defect for a **Skills-scoped regular inventory**. Repository-backed `updatable` rows in that
   baseline have `repository_id = None`. The retry computes a repository-scoped slice whose
   corresponding rows have a real repository id. The merge retains the `None` baseline rows and
   appends the slice rows, so the same skill is persisted twice under the same
   `(inventory_id, bucket, entity_key)` primary key. SQLite rejects the write, and the legacy IPC
   mapper turns the unique-constraint text into the misleading `resource.conflict` message.

This is an application regression, not corruption in the user's Central skill or SQLite file.

## Runtime evidence

- `PRAGMA quick_check` on the live database returned `ok` under `PRAGMA query_only=ON`.
- The 2026-08-08 20:38:32 regular refresh succeeded and persisted a
  `central_updates.skill_source_missing / decision_required` row whose diagnostic source path is
  the tracked `archive-planning` path.
- The 20:38:56 row retry failed after 3746 ms. Its operation record is
  `action=update_center.retry_repositories`, `status=failed`,
  `error_category=central_updates.db`, `mode_override=sync`.
- The repository's `last_synced_at` and newly discovered pending additions were updated at the
  retry timestamp. Remote computation therefore finished; failure occurred when the merged
  inventory was persisted.
- The stored regular inventory id begins with `skills:regular:`. Sixteen updatable rows in that
  inventory belong to the target repository, and all sixteen have `repository_id IS NULL`.
  A repository-scoped slice would append those same sixteen entity keys.
- The failed inventory replacement is transactional, so the previous regular inventory remains
  readable. Pending-addition upserts and `last_synced_at` occur earlier and are idempotent; they do
  not indicate database damage.

All live-database access used `sqlite3 -readonly` plus `PRAGMA query_only=ON`. No refresh, apply,
migration, Central mutation, or credential read was performed during diagnosis.

## Upstream source evidence

The current upstream tree has no `skills/**/archive-planning` skill. Commit
`38dfa34f8622611f0bb6b4bbc70c3be8efe75dd9` (`chore(skills): remove 6 obsolete skills and repoint
references`) removed its `SKILL.md`, eval, script, test, and generated docs on
2026-07-22T09:56:21Z. The preceding commit still contains the tracked `SKILL.md`.

Repository links:

- https://github.com/bahayonghang/my-claude-code-settings
- https://github.com/bahayonghang/my-claude-code-settings/commit/38dfa34f8622611f0bb6b4bbc70c3be8efe75dd9

## Deterministic feedback loop

A temporary Rust regression used a fresh in-memory SQLite database and temporary skill
directories. Its minimal fixture contained two local skills assigned to one repository:

- `stable`: remote content differs, so it is `updatable`;
- `gone`: tracked path is absent remotely, so regular mode emits `decision_required`.

The exact command was run twice:

```text
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  diagnosis_retry_skills_scope_merges_same_repository_without_conflict -- --nocapture
```

Both runs failed; the cached run completed in 0.05 seconds:

```text
UNIQUE constraint failed:
skill_update_inventory_entries.inventory_id,
skill_update_inventory_entries.bucket,
skill_update_inventory_entries.entity_key
```

Changing only the baseline scope from `Skills` to `Repositories` made the same test pass. The
existing one-missing-skill control also passed:

```text
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  retry_with_sync_override_produces_removal_decisions_for_a_regular_inventory -- --nocapture

cargo test: 1 passed
```

Replacing the row retry with a full `Skills + sync` refresh on the same fixture passed as well.
The temporary diagnostic test was removed after the loop was established.

## Hypotheses and dispositions

| Rank | Hypothesis | Prediction | Result |
| --- | --- | --- | --- |
| 1 | Skills baseline loses repository ownership, causing duplicate merge rows | Fresh fixture fails only with `Skills` base; `Repositories` base passes | Confirmed |
| 2 | Remote additions or pending additions collide | Failure disappears when snapshot has no added skills | Rejected; minimal snapshot still failed |
| 3 | Live DB is corrupt or contains stale duplicate inventory rows | Fresh DB does not reproduce | Rejected; fresh DB reproduced and live quick-check is `ok` |
| 4 | Concurrent refresh/job lease causes conflict | Sequential unit path without jobs passes | Rejected; it failed deterministically |
| 5 | A missing/moved skill alone is unsupported | Existing single-missing retry fails | Rejected; existing control passes |

## Code path

1. `buildUpdateCheckScope` deliberately creates `{ kind: "skills", mode: "regular" }` for a
   regular check.
2. In `compute_skill_update_inventory`, `SkillRefreshScopeKind::Skills` sets
   `repository_ids = []`. `valid_repositories` and the later `repo_by_id` map are therefore empty,
   even though `PreparedSkillUpdate.assignment.repository` still knows each skill's repository.
3. The `UpdateAvailable` branch calls `repository_id_for_state(&repo_by_id, ...)`, producing
   `None` for every repository-backed updatable row in a Skills-scoped inventory.
4. `retry_failed_repositories_impl` computes the retry slice with
   `SkillRefreshScopeKind::Repositories`, so the same updatable rows now carry `Some(repo_id)`.
5. `merge_inventory_for_repositories` removes baseline rows only when their serialized
   `repository_id` is in the retry target set. It intentionally preserves `None`, then appends the
   slice, creating duplicate `updatable` entity keys.
6. `persist_refresh_inventory` serializes both rows and
   `replace_skill_update_inventory` inserts them under primary key
   `(inventory_id, bucket, entity_key)`.
7. `ipc_error::legacy_plain_message` maps any text containing `unique constraint failed` to
   `resource.conflict`, hiding the internal inventory invariant failure behind a user-data
   conflict message.

The retry feature and its `None`-preserving merge rule were introduced together by
`b4715bf01c887338ad21e9f5d7cbdd793966b5d1` on 2026-08-05. The older regular check already
reported a missing source path, but there was no clickable repository-slice retry command. The
current double failure therefore requires both the upstream deletion and the new retry merge.

The database does not retain enough previous-check history to prove whether the user's last
successful check preceded the 2026-07-22 deletion or simply excluded `archive-planning`. It does
prove that the `resource.conflict` retry regression could not occur before the 2026-08-05 retry
feature.

## Immediate safe recovery

Inside Update Center:

1. Do not use the failed row's "re-check in incremental mode" action in the affected build.
2. Change the top mode selector from **Regular** to **Incremental and removal**.
3. Click the top **Refresh** button. This performs a full Skills-scoped sync refresh and does not
   use repository-slice merge; the isolated production-path fixture passed.
4. For `archive-planning`, choose **Keep** if the local Central copy is still wanted. The existing
   keep path preserves the skill directory and detaches the obsolete repository source. Choose
   **Delete** only if the skill is intentionally obsolete, because that removes the Central skill.

No manual SQLite edit, inventory-table deletion, database rebuild, or Central directory deletion
is needed. "Clear inventory" may remove the visible result but does not fix the merge defect.

## Permanent fix

The complete fix needs both a producer invariant and legacy compatibility:

1. **Carry authoritative repository ownership for every scope.** Build the inventory's repository
   map from `PreparedSkillUpdate.assignment.repository`, not only from scope-level
   `valid_repositories`; preferably assign `repository_id` directly from the prepared assignment
   instead of reverse-matching URL and branch. This applies to Skills and Platform scopes as well
   as All/Repositories.
2. **Make retry merge tolerate old persisted inventories.** Before merging, derive the complete
   target skill-id set from the requested repository ids. Remove a baseline actionable row when
   either its `repository_id` targets that repository or its `skill_id` is in the authoritative
   target-member set. This also removes a stale old updatable row when the new slice is now
   up-to-date and therefore emits no replacement row.
3. **Keep persistence strict.** Do not solve the defect with `INSERT OR REPLACE` or blind
   deduplication; that would hide ownership ambiguity and can preserve stale rows. Optionally add
   a typed pre-persistence duplicate-key invariant so an internal bug cannot be mislabeled as
   `resource.conflict`.
4. **Add focused regressions.** Cover:
   - Skills regular base: same repository has one updatable and one missing skill, then sync retry;
   - Platform regular base with the same shape;
   - legacy baseline rows whose `repository_id` is `None`;
   - target row becoming up-to-date on retry, proving the stale base row is removed;
   - unique `(bucket, entity_key)` assertion and persisted reload.

No production fix was implemented in this diagnosis task.
