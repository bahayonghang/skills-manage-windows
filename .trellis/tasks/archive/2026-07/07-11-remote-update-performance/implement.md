# Implementation Plan: SSH/WSL Central Update Reliability And Performance

## Preconditions

- The user reviewed and approved the conditional persistent-SSH scope on 2026-07-11.
- Capture an update-specific before baseline with a fixed fixture before changing production behavior.
- Preserve the clean boundary around unrelated working-tree changes.

## 1. Lock The Feedback Loops

- [x] Keep `research/benchmark_process_startup.ps1` and `research/analyze_operation_logs.py` runnable.
- [x] Add a Windows-only ignored integration harness that runs the real WSL argv/update shape against `/tmp`, never `~/.skillsmanage`.
- [ ] Add FakeRunner call-count tests for refresh and apply formulas.
- [ ] Define a fixed 1/10/25-skill fixture with deterministic file counts and archive bytes.
- [ ] Record Local/WSL/SSH cold and warm p50/p95, phase totals, child-process count, round-trip count, and payload bytes in `research/baseline-before.md`.

Verify:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\.trellis\tasks\07-11-remote-update-performance\research\benchmark_process_startup.ps1
python .\.trellis\tasks\07-11-remote-update-performance\research\analyze_operation_logs.py
```

## 2. Fix WSL Argv And Remove Eager Action Probes

- [x] Write a failing test that expects `ConnectedWslTarget::base_command()` to use `--exec` and preserve script arguments.
- [x] Change the WSL base command and rerun the live `/tmp` probe.
- [x] Split target construction from explicit connectivity probing for SSH and WSL.
- [x] Route business operations through the unprobed open path; keep settings test/create/update probes explicit.
- [x] Verify error variants and user-visible failure details remain unchanged.

Verify:

```powershell
cd src-tauri
cargo test targets
```

Rollback point: WSL argv fix and lazy-open change form one independently reviewable commit.

## 3. Add Update Timing And Count Observability

- [x] Add total operation logs for Update Center refresh/apply/force actions.
- [x] Add narrow tracing spans for open/hash/snapshot/archive/write/copy/persist phases.
- [x] Add counts without sensitive fields: skills, repositories, hash chunks, write chunks, copy chunks, payload bytes.
- [x] Add redaction tests and operation-log duration assertions.

Verify:

```powershell
cd src-tauri
cargo test operation_log
cargo test central_updates
```

## 4. Implement Batched Central Writes

- [x] Add `CentralSkillWrite` and per-skill outcome types.
- [x] Validate all ids, target paths, relative archive paths, and manifest fields before execution.
- [x] Build grouped/chunked archives off the async runtime.
- [x] Implement Local batch hook by reusing the current atomic writer.
- [x] Implement Remote batch staging/extract/swap/rollback script with parseable per-skill results.
- [ ] Test archive corruption, unsafe paths, missing parent, extraction failure, mid-swap failure, rollback, and cleanup.
- [x] Assert remote process count is `ceil(N / 16)` for writes.

Verify:

```powershell
cd src-tauri
cargo test central_updates_fs
```

Rollback point: batch FS hook is added before orchestration switches to it.

## 5. Refactor Update Orchestration

- [x] Split plan, filesystem execution, database persistence, copy refresh, and final state emission.
- [x] Use the batch executor from normal update, force update, and force mirror.
- [x] Preserve per-skill success/failure/skip outputs and progress counters.
- [x] Preserve snapshot/hash reuse and manual refresh bypass behavior.
- [x] Test cancellation between chunks and partial remote failures.

Verify:

```powershell
cd src-tauri
cargo test central_updates
cargo test skill_update_inventory
```

## 6. Batch Copy Refresh

- [x] Collect and deduplicate copy targets across successful writes.
- [x] Implement Local and Remote batch hooks with per-target outcomes.
- [x] Chunk remote requests at 32 copy targets.
- [x] Remove nested per-skill process concurrency after equivalence tests pass.
- [x] Preserve basename guard and current failure reporting.

Verify:

```powershell
cd src-tauri
cargo test central_updates
cargo test installation
```

## 7. Frontend And User-Visible Behavior

- [x] Keep existing IPC payloads unless timing metadata is explicitly added.
- [x] Ensure apply completion does not trigger a full refresh/network check; loading persisted inventory is allowed.
- [x] Preserve per-skill progress, cancellation control, errors, and i18n.
- [x] Update focused Zustand/Vitest expectations only where call behavior intentionally changes.

Verify:

```powershell
pnpm exec vitest run src/test/updateCenterStore.test.ts src/test/centralSkillsStore.test.ts
pnpm typecheck
pnpm lint
```

## 8. Performance Acceptance

- [ ] Rerun the exact before fixture for Local/WSL/SSH.
- [x] Confirm process-count formulas from FakeRunner and runtime metrics.
- [ ] Confirm WSL/SSH 10-skill warm apply p50 improves by at least 60%.
- [ ] Confirm WSL added time is at most 1.5 seconds and LAN SSH added time at most 4 seconds above Local, or return to planning with measured evidence.
- [x] Confirm single-skill actions do not regress.
- [x] Save results to `research/baseline-after.md` with raw command lines and environment notes.

## 9. Conditional SSH Persistent Session

- [x] Create child task `07-11-ssh-persistent-session`; keep it in `planning` until the performance gate is evaluated.
- [ ] If Stage 2 misses the accepted SSH gate, plan and start the child task for current `russh` research and migration.
- [ ] Revalidate dependency versions, licenses, Windows packaging, auth/key formats, known-hosts policy, cancellation, timeouts, and real-host compatibility.
- [x] Preserve the historical decision to avoid ControlMaster unless the user explicitly revises it.

## 10. Final Quality Gate

```powershell
cd src-tauri
cargo test targets
cargo test central_updates_fs
cargo test central_updates
cargo test skill_update_inventory
cargo clippy -- -D warnings
cd ..
pnpm typecheck
pnpm lint
just ci
git diff --check
```

Review risky files before `task.py start`:

- `src-tauri/src/targets/exec.rs`
- `src-tauri/src/targets/askpass.rs`
- `src-tauri/src/targets/remote.rs`
- `src-tauri/src/targets/runner.rs`
- `src-tauri/src/services/central_updates/fs.rs`
- `src-tauri/src/services/central_updates/fs/remote_scripts.rs`
- `src-tauri/src/services/central_updates/core.rs`
- `src-tauri/src/services/central_updates/inventory/force.rs`
- `src-tauri/src/commands/skill_update_inventory.rs`
- `src/stores/updateCenterStore.ts`
