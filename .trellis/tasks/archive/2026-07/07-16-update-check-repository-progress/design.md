# Design: 更新检查仓库进度反馈

## Scope And Boundaries

本任务为 Update Center 库存刷新增加仓库快照阶段的真实进度。实现跨越 Rust snapshot/service、Tauri event、Zustand store、controller/dialog 与 i18n，但不改变检查范围、更新判断、库存持久化或应用决策语义。

主要改动面：

- `src-tauri/src/services/central_updates/snapshots.rs`
- `src-tauri/src/services/central_updates/inventory/{mod.rs,types.rs}`
- `src-tauri/src/commands/skill_update_inventory.rs`
- `src/types/skillUpdateInventory.ts`
- `src/stores/updateCenterStore.ts`
- `src/pages/centralUpdateCheckModeController.tsx`
- `src/components/central/UpdateCheckModeDialog.tsx`
- `src/i18n/locales/{en,zh}.json`
- 对应 Rust、store、component 和 view/controller 测试

## Current Data Flow

```text
UpdateCheckModeDialog
  -> controller.handleConfirm(mode)
  -> updateCenterStore.refresh(scope)
  -> invoke refresh_skill_update_inventory
  -> inventory service resolves scope and deduplicates GitHubRepoRef values
  -> snapshots helper downloads missing snapshots with concurrency = 4
  -> inventory service compares/persists results
  -> one final SkillUpdateInventory response
  -> controller closes mode dialog and opens Update Center
```

前端目前只看到 `isRefreshing`，无法区分准备、活跃仓库、仓库完成或结果整理阶段。

## Proposed Data Flow

```text
store creates operationId
  -> await listen("central://skill-update-inventory-progress")
  -> invoke refresh_skill_update_inventory(scope, operationId)
  -> inventory snapshot reporter emits scoped events
       started(total)
       repository_started(repoKey, owner/repo)
       repository_completed | repository_failed(repoKey, completed)
       finalizing(total, completed)
  -> store filters operationId and derives activeRepositories
  -> controller passes progress to dialog
  -> dialog replaces mode selection with progress view while submitting
  -> success/error finally unlisten and clear transient progress
```

The listener must be installed before `invoke`. The command response remains the authority for success/failure and inventory data; progress events are transient presentation data only.

## Event Contract

Use a new event name, `central://skill-update-inventory-progress`, rather than extending `central://skill-update-progress`, whose payload and consumers are skill-oriented.

Payload fields use camelCase over Tauri serialization:

```text
operationId: string
status: "started" | "repository_started" | "repository_completed"
      | "repository_failed" | "finalizing"
total: number
completed: number
repositoryKey?: string     # owner/repo/branch; stable identity for set updates
repositoryName?: string    # owner/repo; user-facing label
```

Invariants:

- `total` is the count of deduplicated `GitHubRepoRef` cache keys used by this refresh.
- `completed` is monotonic and counts settled repositories, including failures and cache hits.
- A network download emits `repository_started` only after acquiring the existing semaphore permit, so the active list reflects actual work and never exceeds the concurrency limit.
- Every started repository emits exactly one completed/failed event.
- Cache hits count as completed without becoming active.
- `finalizing` is emitted after all snapshot work succeeds and before comparison/persistence finishes; the UI shows no stale active repository during this phase.
- Repository failures still reject the existing command. Events do not replace the current inline error + toast behavior.

## Backend Design

Keep the existing public snapshot helpers compatible. Add a progress-capable internal variant or optional reporter used only by inventory refresh. The reporter is a small `Arc<dyn Fn(...) + Send + Sync>`-style seam so snapshot concurrency can emit testable lifecycle events without making the snapshot module own Tauri UI concerns.

`refresh_skill_update_inventory` accepts an `operation_id` argument and an `AppHandle`. The command builds an event reporter that attaches the operation id and emits the new payload. The inventory service supplies that reporter to the snapshot helper; the helper emits `started` after deduplication, and the inventory service emits `finalizing` after snapshot preparation succeeds.

Do not serialize downloads to produce ordered progress. Existing `SNAPSHOT_DOWNLOAD_CONCURRENCY = 4`, cache behavior, deduplication key, error propagation, and returned snapshot map remain unchanged.

Tests should inject a recording reporter and assert lifecycle invariants without relying on Tauri runtime event capture or repository completion order.

## Frontend State

Add a transient refresh progress model to `updateCenterStore`, separate from persisted inventory:

```text
operationId: string
phase: "preparing" | "checking" | "finalizing"
total: number
completed: number
activeRepositories: Array<{ key: string; name: string }>
```

`refresh(scope)` performs this sequence:

1. Generate a unique operation id and reset progress to `preparing`.
2. Await event subscription.
3. Filter payloads by operation id and merge them idempotently.
4. Invoke the existing command with `scope` plus `operationId`.
5. On success or failure, run the existing inventory/error behavior.
6. In `finally`, unlisten and clear the transient progress.

The merge function treats repository keys as a set: start adds/replaces; completed/failed removes. This tolerates duplicate delivery and arbitrary concurrent completion order. A stale event with another operation id is ignored.

Browser fixture behavior remains immediate and uses no event subscription; the store may expose `preparing` only for the short promise lifetime, then clear it.

## Dialog State Machine

```text
selection
  -> submit
preparing/checking/finalizing
  -> success: close dialog, open Update Center
  -> failure: restore selection, show inline error + toast, allow retry
```

While `isSubmitting` is true, the dialog body replaces mode cards and the warning with a stable-height progress view:

- title remains the existing update-check title;
- status line shows preparing, `completed / total`, or finalizing copy;
- a full-width progress bar uses actual completed/total values;
- all active `owner/repo` names are listed with a small activity icon;
- long names use `min-w-0`, truncation, and an accessible full label/title;
- an unknown or zero total uses an indeterminate semantic progressbar (no `aria-valuenow`) rather than a fake percentage;
- no cancellation capability is added; current dialog close behavior is preserved.

On failure, `isSubmitting` becomes false, so the existing selection view and local error return. The selected mode remains unchanged for direct retry.

## Compatibility And Failure Handling

- Command result and inventory payload are unchanged; the added operation id is internal to the sole frontend caller.
- Existing skill update progress event and `centralSkillsStore.updateJob` remain untouched.
- A listener setup failure rejects the refresh before invoking the backend and follows the current visible error path.
- An emit failure is best effort and must not fail repository checking; the final command response remains authoritative.
- Progress state is not persisted and must not survive success, failure, close/reopen, target changes, or retry.
- No credentials, tokens, local paths, or full URLs are included in events or visible labels.

## Rejected Alternatives

- Serial repository downloads: produces simple linear progress but regresses established network performance.
- Timer-driven percentage: visually active but not truthful and can finish before or after real work.
- Reusing skill update progress: mixes repository and skill identities and risks changing existing top-line/update job behavior.
- Showing only the latest active repository: compact, but rejected by the user because concurrent work would be hidden.
- Persisting progress in the inventory database: transient UI state does not justify schema or migration cost.

## Rollback

The feature is additive: remove the new reporter/event/types/store field and restore the submitting branch of `UpdateCheckModeDialog`. No database migration, cache format, or persisted setting needs rollback.
