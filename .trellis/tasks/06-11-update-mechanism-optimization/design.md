# 更新机制优化重构设计

## Decision Summary

Use content-tree hash as the source of truth for update detection. Split state into three separate responsibilities:

1. Snapshot cache: network optimization only.
2. Update inventory: persisted pending check results.
3. Installed baseline: successfully applied source state.

The current Update Center direction is retained. The refactor aligns the implementation with the documented refresh/apply contract instead of replacing the feature.

Add one explicit rescue path:

4. Force update: bypass detection state and forcibly align selected tracked skills or whole repositories with their assigned GitHub source.

## Current Problem

The repository already documents that refresh is read-only and writes inventory, while apply performs mutations. The implementation currently violates that boundary:

- `refresh_skill_update_inventory_impl` writes `skill_update_states`.
- `get_skill_update_inventory_impl_scoped` treats `skill_update_states` as inventory for updatable and remote missing rows.
- `clear_skill_update_inventory_impl` only clears pending remote additions, so it cannot clear all persisted inventory results.
- `prepare_snapshots_for_repo_refs` uses a 10-minute in-memory snapshot cache with no explicit force-refresh path.
- `update_central_skills` can already overwrite Central skill files from remote content, but it skips when remote and local hashes match.
- `update_one_skill` writes via `write_skill_dir_atomic`, updates the skill row/source assignment, and refreshes copy installations.
- GitHub import overwrite has staging/backup/restore behavior, but it targets import decisions rather than repair of existing tracked skills.
- Repository sync apply already composes delete/import decisions; force mirror can reuse its deletion and import kernels while replacing per-row user decisions with a repository-level confirmation.
- Central deletion removes symlink/native installations automatically; copy installations are deleted only when listed in `remove_agent_ids`.
- Frontend Update Center and older update/sync flows coexist, so user-visible behavior can diverge.

## Target State Model

### Snapshot cache

Purpose:

- Avoid duplicate network downloads inside a single app process.
- Never represent the user's authoritative view of remote state.

Rules:

- User-triggered refresh defaults to bypass cache.
- Passive inventory load and apply follow-up reads do not hit network.
- Optional cached refresh can be supported for background or low-cost reloads, but the UI must label it.

Recommended contract:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillRefreshCachePolicy {
    UseFresh,
    Bypass,
}

pub struct SkillRefreshScope {
    pub kind: SkillRefreshScopeKind,
    pub mode: Option<SkillRefreshMode>,
    pub cache_policy: Option<SkillRefreshCachePolicy>,
    pub skill_ids: Option<Vec<String>>,
    pub repository_ids: Option<Vec<String>>,
    pub agent_ids: Option<Vec<String>>,
}
```

Frontend default:

- Toolbar/manual refresh sends `cachePolicy: "bypass"`.
- Reopen dialog/load inventory does not call refresh.
- Any future background refresh can choose `use_fresh`.

### Installed baseline

Existing table:

- `skill_update_states`

Purpose after refactor:

- Store source metadata and last successfully applied remote hash for installed Central skills.
- Record last successful update/application time where relevant.
- Support update comparison baseline, not pending UI inventory.

Rules:

- Refresh must not upsert baseline rows.
- Apply update/import/keep/delete outcomes may update or remove baseline rows as part of successful decisions.
- Error rows from refresh should not be stored here.

### Update inventory

New persistence owns refresh results. Recommended new table:

```sql
CREATE TABLE IF NOT EXISTS skill_update_inventory_entries (
    inventory_id       TEXT NOT NULL,
    bucket             TEXT NOT NULL,
    entity_key         TEXT NOT NULL,
    skill_id           TEXT,
    skill_name         TEXT,
    repository_id      TEXT,
    source_type        TEXT,
    source_url         TEXT,
    ref_name           TEXT,
    source_path        TEXT,
    agent_id           TEXT,
    local_hash         TEXT,
    baseline_hash      TEXT,
    remote_hash        TEXT,
    local_version      TEXT,
    remote_version     TEXT,
    cache_policy       TEXT NOT NULL,
    cache_hit          INTEGER NOT NULL DEFAULT 0,
    snapshot_fetched_at TEXT,
    generated_at       TEXT NOT NULL,
    payload_json       TEXT NOT NULL,
    error              TEXT,
    PRIMARY KEY (inventory_id, bucket, entity_key)
)
```

Recommended companion table:

```sql
CREATE TABLE IF NOT EXISTS skill_update_inventory_runs (
    inventory_id       TEXT PRIMARY KEY,
    scope_kind         TEXT NOT NULL,
    mode               TEXT NOT NULL,
    skill_ids_json     TEXT,
    repository_ids_json TEXT,
    agent_ids_json     TEXT,
    cache_policy       TEXT NOT NULL,
    generated_at       TEXT NOT NULL
)
```

Scope handling:

- `All`: clear/read the latest all-scope inventory run and all its entries.
- `Repositories`: key by repository id list.
- `Skills`: key by skill id list.
- `Platform`: key by agent id list.

Implementation can start with a deterministic `inventory_id` derived from normalized scope and mode, then later support history if needed. The product need is current persisted inventory, not historical audits.

Buckets:

- `updatable`
- `remote_added`
- `remote_missing`
- `platform_duplicates`
- `deleted_platform_copies`
- `orphans`
- `failed_repository`

Remote additions can either migrate from `skill_repository_pending_additions` into the unified table or keep that table temporarily while adding existing-skill inventory rows. Preferred direction is one unified inventory model because clear/read/apply semantics become easier to reason about.

## Data Flow

### Refresh

```text
User clicks refresh
  -> updateCenterStore.refresh(scope with cachePolicy=bypass)
  -> refresh_skill_update_inventory
  -> prepare snapshots using cache policy
  -> compare remote content hash with installed baseline/local hash
  -> write inventory run + entries
  -> return SkillUpdateInventory with diagnostics
```

Refresh must not:

- write skill files
- import new skills
- delete local/platform copies
- call `upsert_skill_update_state`
- hide cache hits from diagnostics

### Load inventory

```text
Dialog opens / apply completes
  -> get_skill_update_inventory(scope)
  -> read persisted inventory entries only
  -> scan live platform duplicate/deleted-copy buckets if intentionally kept live
  -> return SkillUpdateInventory
```

If platform duplicate/deleted-copy scans remain live rather than persisted, the API should document this exception. Preferred direction for consistency is to persist scan results during refresh and clear them with the same inventory clear.

### Clear inventory

```text
User clicks clear
  -> clear_skill_update_inventory(scope)
  -> delete inventory entries/runs for normalized scope
  -> return empty pending checklist
```

Clear must not:

- delete skills
- delete platform copies
- mutate `skill_update_states`
- mutate repository assignments
- clear successful baseline history

### Apply

```text
User selects decisions
  -> apply_skill_update_decisions(decisions)
  -> execute ordered steps
  -> for successful update/import/keep/delete decisions, update baseline state
  -> remove or update corresponding inventory entries
  -> return partial success/failure result
  -> frontend reloads inventory
```

Existing partial-success semantics should be preserved: a failure in one step reports `failures` but does not roll back successful independent steps.

### Force Update And Force Mirror

Force update is a rescue action for cases where inventory detection or baseline state is suspected to be wrong. It is not a replacement for normal refresh/apply.

There are two related but distinct operations:

- Skill force overwrite: repair selected tracked GitHub skills by overwriting their Central directories from remote.
- Repository force mirror: align selected GitHub repositories by overwriting tracked skills, importing remote-added skills, and deleting local tracked skills whose source paths disappeared from remote.

Skill force overwrite behavior:

```text
User selects Force update for skill(s)
  -> confirmation explains overwrite semantics
  -> force_update_central_skills(scope)
  -> resolve tracked GitHub repository assignments
  -> download repo snapshots with cachePolicy=bypass
  -> for each selected tracked skill:
       - validate source path still contains an importable skill and SKILL.md
       - collect remote files
       - atomically overwrite the Central skill directory even if hashes match
       - update skill metadata, source assignment, and baseline state
       - refresh copy installations
       - clear or refresh corresponding inventory entries
  -> return per-skill result
```

Repository force mirror behavior:

```text
User selects Force mirror for repository/repositories
  -> preview counts overwrite/import/delete using fresh remote snapshots
  -> confirmation shows repository names and destructive delete count
  -> force_mirror_central_repositories(repositoryIds)
  -> download repo snapshots with cachePolicy=bypass
  -> inspect remote skill candidates in each repository
  -> compute:
       tracked_overwrites = local tracked source paths that still exist remotely
       remote_additions = valid remote source paths not tracked locally
       remote_missing = local tracked source paths absent remotely
  -> overwrite tracked_overwrites even if hashes match
  -> import remote_additions with overwrite resolution for same-id conflicts
  -> delete remote_missing local tracked Central skills
  -> update baseline/inventory after each successful item
  -> return partial-success result
```

Force operations must not:

- run for unknown/non-GitHub source assignments
- mutate read-only plugin copies directly
- hide per-skill failures behind one aggregate success
- run from passive refresh, startup, or ordinary apply
- delete skills assigned to a different repository or unknown/local-only source

Recommended scopes:

- Skill-level force update: overwrite selected tracked GitHub Central skills.
- Repository-level force mirror: fetch each selected repository snapshot once, then overwrite/import/delete according to the remote repository tree.

Recommended non-goals for the first implementation:

- `git clone` / `git pull` transport.
- Automatic background force updates.

Force update should reuse the normal update write kernel, but add an option to bypass the “already up to date” branch:

```rust
pub enum CentralSkillUpdateMode {
    Normal,
    ForceOverwrite,
}
```

`Normal` keeps current behavior: skip when `remote_hash == local_hash`.

`ForceOverwrite` always calls the atomic write path when remote content is valid. It still records before/after hashes so the result can say whether the overwrite changed bytes or only repaired metadata/platform copies.

Repository mirror should not rely on inventory being correct. It should recompute the remote tree directly from a bypass-cache snapshot and compute additions/removals from repository assignments.

## Public Contracts

### Rust command contract

Keep command names:

- `refresh_skill_update_inventory`
- `get_skill_update_inventory`
- `clear_skill_update_inventory`
- `apply_skill_update_decisions`

Add force commands or extend apply decisions with clearly separate fields. Recommended commands:

- `force_update_central_skills`
- `force_mirror_central_repositories`

Extend payloads:

- `SkillRefreshScope.cache_policy?: "use_fresh" | "bypass"`
- `SkillUpdateInventory.diagnostics?`
- `UpdatableSkill.diagnostics?`
- `RemoteMissingSkill.diagnostics?`
- `FailedRepository` may include snapshot/cache metadata if useful.

Diagnostics shape should be small and stable:

```rust
pub struct SkillUpdateDiagnostic {
    pub source_url: Option<String>,
    pub ref_name: Option<String>,
    pub source_path: Option<String>,
    pub local_hash: Option<String>,
    pub baseline_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub cache_policy: SkillRefreshCachePolicy,
    pub cache_hit: bool,
    pub snapshot_fetched_at: Option<String>,
}
```

Recommended force-update payload:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateRequest {
    pub scope: ForceSkillUpdateScope,
    #[serde(default)]
    pub refresh_copy_installations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ForceSkillUpdateScope {
    Skills { skill_ids: Vec<String> },
    Repositories { repository_ids: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceSkillUpdateResult {
    pub overwritten: Vec<ForceSkillUpdateSuccess>,
    pub skipped: Vec<ForceSkillUpdateSkip>,
    pub failed: Vec<ForceSkillUpdateFailure>,
}
```

The result should include:

- `skillId`
- repository id/source path when available
- local hash before overwrite
- remote hash
- whether bytes changed
- whether copy installations were refreshed
- failure reason for unsupported, remote missing, invalid manifest, or IO errors

Recommended force mirror payload:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceRepositoryMirrorRequest {
    pub repository_ids: Vec<String>,
    #[serde(default)]
    pub delete_missing: bool,
    #[serde(default)]
    pub import_added: bool,
    #[serde(default)]
    pub overwrite_tracked: bool,
    #[serde(default)]
    pub remove_copy_installations_for_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceRepositoryMirrorResult {
    pub overwritten: Vec<ForceSkillUpdateSuccess>,
    pub imported: Vec<ImportedGitHubSkillSummary>,
    pub deleted: BatchDeleteCentralSkillResult,
    pub skipped: Vec<ForceSkillUpdateSkip>,
    pub failed_repositories: Vec<FailedRepository>,
    pub failed_items: Vec<ForceSkillUpdateFailure>,
}
```

The three booleans allow the UI confirmation to pass exactly what the user confirmed. For the full force mirror action, all three should be true. Backend should reject a request where all three are false.

### Frontend contract

Update `src/types/skillUpdateInventory.ts` to match backend payload changes.

Update Center UI should show diagnostics without turning every row into a dense debug panel:

- row-level source metadata stays compact via existing `SourceMeta`;
- add an inspect/copy diagnostics affordance for hash/cache details;
- label `version` as metadata, not update detector.

Expose force update as an explicit rescue affordance:

- Row action on tracked GitHub rows: “Force update”.
- Repository action when scope is repository: “Force mirror repository”.
- Skill force confirmation explains that Central skill directories and copy installs may be overwritten from remote.
- Repository force mirror confirmation explains overwrite/import/delete counts and requires explicit confirmation before deleting missing skills.
- Confirmation dialog lists the target count and source repository/path summary.
- The action is visually secondary/danger-adjacent, not the primary refresh button.

All user-facing text must go through `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`.

## Compatibility and Migration

Database migration:

- Add new inventory tables with `CREATE TABLE IF NOT EXISTS`.
- Do not delete existing `skill_update_states`.
- Do not attempt to infer historical inventory from `skill_update_states`; on first post-migration launch, inventory can be empty until refresh.
- Optional cleanup: if existing `skill_update_states.status` contains `update_available` from old refreshes, keep baseline fields but do not surface them as inventory after the refactor.

Legacy command strategy:

- Keep deprecated backend commands available for compatibility.
- Route user-visible frontend entry points through Update Center.
- Do not remove `CentralUpdateConfirmDialog`, `CentralRepositorySyncDialog`, or `RemoteMissingSkillsDialog` until a separate removal task unless this task explicitly expands scope.

Force update compatibility:

- It should support Local and existing remote targets through `CentralFs`, matching normal updates.
- It should keep `skill_update_states` compatible by writing the same baseline shape normal successful update writes.
- Existing inventory can be stale after force update; the command should either clear affected inventory entries or refresh those entries with the force result.
- Force mirror deletion should call existing Central deletion helpers so DB cleanup, symlink/native removal, and optional copy removal behave like normal delete.

## Trade-offs

### New inventory table vs reusing `skill_update_states`

Chosen: new inventory persistence.

Reason:

- `skill_update_states` has installed-baseline semantics and is keyed only by `skill_id`.
- Inventory is scoped, refresh-generated, clearable, may include repository/platform failures, and may include multiple buckets for the same skill.
- Reusing the old table caused the current clear/refresh ambiguity.

### Content hash vs version

Chosen: content hash remains primary.

Reason:

- Many skills do not have version fields.
- Content hash detects multi-file skill changes.
- Version can drift from actual content and should be diagnostic metadata only.

### Manual refresh bypasses cache vs always use cache

Chosen: manual refresh bypasses cache.

Reason:

- This directly addresses the reported confusion.
- Cache remains useful for passive/in-process reuse.
- Risk is higher GitHub API usage; mitigated by limiting bypass to explicit user action.

### Force update scope

Chosen by user: repository-level force mode includes overwrite, import additions, and delete missing.

Reason:

- The user wants a fallback when update detection is wrong, including cases where additions/removals are missed.
- The operation is explicit and confirmable, so destructive behavior is acceptable when the user chooses force mirror.
- It should still be scoped to selected GitHub repositories to avoid touching unrelated skills.

Alternative:

- A safer force overwrite-only mode would avoid import/delete, but the user rejected that as insufficient.

### Delete copy installations during force mirror

Chosen by user: yes. When force mirror deletes a remote-missing Central skill, it also removes all copy installations for that skill.

Reason:

- Force mirror's purpose is to align local state with the remote repository.
- Leaving copy installs creates orphaned platform copies that can continue to shadow the removed Central skill.
- Existing delete preview can enumerate copy installs, and delete requests already support `remove_agent_ids`.

Trade-off:

- Removing copies is more destructive. The confirmation must clearly show that copy installations will be removed, including affected platform/agent counts, before the user applies repository force mirror.

## Rollback

If the refactor causes problems:

- Backend can stop reading inventory tables and return empty inventory while preserving existing skills.
- `skill_update_states` remains intact because the migration is additive.
- Frontend can temporarily hide diagnostics and keep the Update Center dialog reading the previous payload subset.
- No skill files are mutated by refresh, so rollback does not require filesystem repair.
- Force update uses existing atomic write/backup behavior; failed single-skill overwrites should leave the prior skill directory intact where the filesystem layer can restore it.
