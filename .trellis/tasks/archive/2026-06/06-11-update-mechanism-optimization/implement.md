# 更新机制优化重构实施计划

## Preconditions

- User approves this planning package.
- Task is moved to `in_progress` with `python ./.trellis/scripts/task.py start .trellis/tasks/06-11-update-mechanism-optimization`.
- Before editing code, load `trellis-before-dev` for relevant package/layer guidance.

## Implementation Checklist

### 1. Backend cache policy

- Add `SkillRefreshCachePolicy` to `src-tauri/src/commands/skill_update_inventory/types.rs`.
- Extend `SkillRefreshScope` with optional `cache_policy`.
- Extend snapshot preparation in `src-tauri/src/commands/central_updates.rs` to accept a bypass/use-fresh policy.
- Set Update Center manual refresh to bypass cache by default.
- Keep existing cache behavior for callers that do not pass a policy, unless routed through Update Center manual refresh.
- Add diagnostics for cache policy, cache hit, and snapshot fetched time.

Validation:

- Unit test that bypass policy ignores a fresh cached snapshot and uses the newer fetched/mock snapshot.
- Unit test that use-fresh policy can still reuse cache.

### 2. Inventory persistence schema

- Add inventory tables in `src-tauri/src/db/schema/metadata.rs` using idempotent `CREATE TABLE IF NOT EXISTS`.
- Add Rust row structs in `src-tauri/src/db/types.rs`.
- Add CRUD functions in a new `src-tauri/src/db/repos/update_inventory_repo.rs` or a focused addition under existing repos.
- Export repo functions from `src-tauri/src/db/repos/mod.rs` and `src-tauri/src/db/mod.rs`.

Validation:

- Test insert/list/clear inventory run entries by normalized scope.
- Test clear does not delete skills or `skill_update_states`.

### 3. Refresh writes inventory, not baseline

- Update `refresh_skill_update_inventory_impl` in `src-tauri/src/commands/skill_update_inventory.rs`.
- Remove refresh-time `db::upsert_skill_update_state`.
- Build inventory entries for:
  - updatable
  - remote missing
  - remote added
  - failed repositories
  - platform duplicates
  - deleted platform copies
  - orphans when supported
- Return `SkillUpdateInventory` from the persisted entries.
- Keep content-hash comparison as the update detector.

Validation:

- Replace or update existing tests that currently assert refresh persists non-actionable state to `skill_update_states`.
- Add test: refresh with updatable result leaves `skill_update_states` untouched.
- Add test: no-version skill with changed content hash appears as updatable.
- Add test: same content hash with changed version metadata does not appear as updatable.

### 4. Inventory read and clear semantics

- Update `get_skill_update_inventory_impl_scoped` to read inventory persistence for updatable/remote_missing/remote_added/failures.
- Update `clear_skill_update_inventory_impl` to clear all inventory entries for the normalized scope.
- Decide whether platform duplicate/deleted-copy scan results are persisted during refresh or remain live scans; if they remain live, document and test that clear behavior explicitly.

Validation:

- Test clear all empties all buckets.
- Test clear repositories scope only clears matching repository inventory.
- Test clear skills scope clears matching skill inventory.
- Test stale `skill_update_states.update_available` does not surface in inventory.

### 5. Apply updates baseline only after success

- Update `apply_skill_update_decisions` and apply steps so successful updates/imports/keep/delete decisions update or prune baseline state as appropriate.
- Remove applied inventory entries after successful decisions.
- Preserve partial-success result semantics.

Validation:

- Test successful update updates `skill_update_states.last_remote_hash`.
- Test failed update does not update baseline and leaves/report inventory for retry.
- Test import/skip/unskip still removes or updates relevant inventory rows.

### 6. Force update and force mirror rescue mode

- Add a force update request/result type in `src-tauri/src/commands/skill_update_inventory/types.rs` or a focused central update type module.
- Add `force_update_central_skills` for selected tracked skills.
- Add `force_mirror_central_repositories` for repository-level overwrite/import/delete.
- Refactor `update_central_skills` internals to accept an update mode:
  - `Normal`: skip when remote hash equals local hash.
  - `ForceOverwrite`: overwrite even when hashes match.
- Force update must call snapshot preparation with cache bypass.
- Repository-level force mirror should fetch each repository snapshot once and compute overwrite/import/delete from live remote candidates and local repository assignments.
- Skill-level force update should only process selected skill ids.
- Unsupported/non-GitHub/unknown-source skills should be skipped or failed with explicit reasons.
- Successful force update should update `skill_update_states` baseline and refresh copy installations.
- Clear or update affected inventory entries after successful force update.
- For remote-added candidates, reuse GitHub import with overwrite resolution for same-id conflicts unless the candidate is invalid.
- For remote-missing tracked skills, reuse `delete_central_skills_impl` / `delete_central_skills_remote_impl`.
- For delete requests, build `remove_agent_ids` from delete preview according to the accepted copy-installation deletion policy.

Validation:

- Test force update overwrites when hashes match.
- Test force update bypasses a stale fresh cache.
- Test repository force mirror overwrites tracked local skills.
- Test repository force mirror imports remote-added valid skills.
- Test repository force mirror deletes tracked local skills missing from remote.
- Test repository force mirror deletes all copy installations for deleted remote-missing tracked skills.
- Test repository force mirror does not touch skills assigned to other repositories.
- Test repository force mirror reports invalid remote candidates without aborting unrelated valid candidates.
- Test successful force update refreshes copy installations.

### 7. Frontend types/store/UI

- Update `src/types/skillUpdateInventory.ts` for `cachePolicy` and diagnostics.
- Add frontend types/store methods for skill force update and repository force mirror request/result.
- Update `src/stores/updateCenterStore.ts` so user-triggered refresh sends `cachePolicy: "bypass"`.
- Keep `loadInventory` read-only and non-networked.
- Route visible Central update/check entry points through Update Center store where currently still using legacy calls.
- Add compact diagnostic display to Update Center rows, reusing `SourceMeta` where possible.
- Add a confirmable force update action for tracked GitHub rows and repository scope.
- Add a repository force mirror confirmation that displays overwrite/import/delete counts and destructive warning copy.
- Show force actions as rescue/repair actions, not the default primary update button.
- Update i18n in `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`.

Validation:

- Update `src/test/updateCenterDecisionAggregation.test.ts` or add a store test for default bypass policy.
- Update `UpdateCenterToolbar` / SourceMeta tests for diagnostics copy.
- Add a component/store test that force update opens confirmation and invokes the force command only after confirmation.
- Add a component/store test that repository force mirror sends overwrite/import/delete choices only after confirmation.
- Ensure app shell tests still pass with partial mocked stores.

### 8. Docs and generated references

- Update `docs/guide/update-center.md`.
- Update `docs/zh/guide/update-center.md`.
- If IPC/schema generated docs are expected to change, run `pnpm docs:gen` or note if generated docs are intentionally left to release docs step.

Validation:

- Check docs describe:
  - manual refresh cache bypass
  - inventory clear semantics
  - content hash vs version semantics
  - skill force overwrite semantics
  - repository force mirror overwrite/import/delete semantics and warnings
  - legacy command compatibility status

### 9. Verification gates

Run targeted checks first:

```powershell
cd src-tauri
cargo test skill_update_inventory
cargo test central_updates
cd ..
pnpm test -- updateCenterDecisionAggregation UpdateCenterToolbar UpdateCenterSourceMeta
pnpm typecheck
pnpm lint
```

Final gate:

```powershell
just ci
```

If `just ci` fails, fix the root cause rather than waiving the gate.

## Risky Files

- `src-tauri/src/commands/skill_update_inventory.rs`
- `src-tauri/src/commands/central_updates.rs`
- `src-tauri/src/db/schema/metadata.rs`
- `src-tauri/src/db/repos/update_states_repo.rs`
- new inventory repo file under `src-tauri/src/db/repos/`
- `src-tauri/src/db/types.rs`
- `src/types/skillUpdateInventory.ts`
- `src/stores/updateCenterStore.ts`
- `src/stores/centralSkillsStore.updateSlice.ts`
- `src/components/central/updateCenter/*`
- new or existing force update confirmation component under `src/components/central/`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

## Rollback Points

- After schema addition: rollback by leaving unused additive tables in place; no destructive migration is planned.
- After refresh rewrite: rollback by restoring `get_skill_update_inventory` to old read path, but do not delete user data.
- After force commands: rollback by hiding frontend force actions; normal refresh/apply remains available. Already-applied mirror deletions/imports are real user-confirmed mutations and are not automatically reversible unless backed up separately.
- After frontend routing: rollback by restoring old visible entry points while keeping backend compatibility.

## Review Before Start

- Manual refresh cache bypass is accepted.
- Repository force mirror overwrite/import/delete is accepted.
- Force mirror deletion removes all copy installations for deleted remote-missing tracked skills.

Planning decisions are now resolved. Do not run implementation until the user reviews this planning package and asks to start implementation.
