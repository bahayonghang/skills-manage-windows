# Design: Central Bulk Uninstall From Platforms

## Architecture and Boundaries

This feature should be implemented as Central-page orchestration over existing platform uninstall primitives.

- UI entry: `src/components/central/BulkActionBar.tsx`
- Central page wiring: `src/pages/CentralSkillsView.tsx`
- Action orchestration: `src/pages/centralSkillsActions.ts`
- Reusable uninstall request type: `src/types/platformBatch.ts`
- Existing backend command: `batch_uninstall_skills_from_agent`
- Existing Tauri/Rust service: `src-tauri/src/services/installation/batch.rs`

The initial design does not add a new Rust command. Existing backend behavior already gives the important safety contracts: per-agent batch execution, partial results, copy/symlink-aware removal, row-aware support where needed, read-only protection, and shared Central directory rejection.

## Data Flow

1. User selects Central skills in `CentralSkillsView`.
2. User clicks the new bulk uninstall button.
3. Frontend derives a preview from current `SkillWithLinks[]`:
   - selected skills are looked up by id
   - removable agent ids are `linked_agents - shared_root_agents - central`
   - each removable agent gets a list of `{ skill_id }` requests
   - selected skills with no removable agent ids are counted as skipped/not applicable
   - shared-root links are counted separately as non-removable/always included
4. Confirmation dialog displays:
   - selected skill count
   - removable platform install count
   - affected platform count
   - skipped skill count
   - shared-root/non-removable count when present
5. On confirm, frontend calls `batch_uninstall_skills_from_agent` once per affected agent.
6. Results are flattened into a Central-level summary:
   - `succeeded`: `{ skill_id, agent_id }`
   - `failed`: `{ skill_id, agent_id, error }`
   - `skipped`: local preview-only not-installed/shared-root items
7. Refresh:
   - `refreshCounts()`
   - `loadCentralSkills()`
   - refresh current detail installations if the open detail skill was affected
   - optionally refresh cached platform rows through `useSkillStore.batchUninstallSkillsFromAgent`

## Skip and Edge Semantics

- Not installed on a platform: do not send to backend; report as skipped/not applicable.
- Installed on one platform but not another: send only the installed platform request.
- Shared Central directory platform: do not send to backend; report as non-removable/always included.
- Missing selected skill id after filter refresh: ignore it in preview and let the existing selection pruning effect remove it.
- Duplicate agent ids in `linked_agents`: dedupe before grouping.
- All selected skills no-op: dialog can open with disabled confirm and explanatory copy, or the button can show an info toast. Preferred: open the dialog so the user sees why nothing will be removed.

## UI Shape

Add a distinct bulk action button between Install and Categorize or after Install:

- Icon: use `Unlink` or `CircleSlash` from `lucide-react`, not `Trash2`, to avoid conflating uninstall with deletion.
- Label: use compact wording, `Uninstall` in English and `批量卸载` in Chinese, with dialog title clarifying "from platforms".
- Existing Central delete button remains red/destructive and keeps `Trash2`.

Add a new dialog rather than reusing `BatchDeleteCentralSkillsDialog`, because that dialog is intentionally about Central deletion and copy cleanup.

## Compatibility and Safety

- No Central file deletion paths are introduced.
- Backend shared-root guard remains a defense-in-depth layer, but frontend should avoid calling it for shared-root agents.
- Remote active targets should work through the existing command because `batch_uninstall_skills_from_agent` already handles Local, SSH, and WSL active targets.
- The dialog and toasts must avoid saying "delete skills" for this flow.

## Trade-offs

- Frontend orchestration avoids a new backend command and keeps implementation surgical.
- A Rust aggregate command would centralize preview/apply semantics but would add more API surface and duplicate existing batch uninstall logic.
- Frontend preview depends on `SkillWithLinks.linked_agents`; this is acceptable because the Central page already uses that metadata for install status and filters.

## Risks

- `linked_agents` includes `shared_root_agents`; forgetting to subtract them would produce backend failures and bad UX.
- Reusing `isDeleting` for uninstall loading could blur Central delete and platform uninstall state. Prefer a local `isBatchUninstalling` state or a small Central-store `isUninstalling` field.
- Adding props to `CentralSkillsShell` / `BulkActionBar` can affect many shell tests. Keep prop changes minimal.
- Central files are size-budgeted; avoid expanding `CentralSkillsView.tsx` substantially. Put helper logic in a small `src/lib` or `src/pages` helper if needed.
