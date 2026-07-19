# Implementation Plan: 更新检查仓库进度反馈

## 1. Backend Progress Contract

- [x] Add repository refresh progress status/payload types and the dedicated event constant.
- [x] Add a testable optional snapshot progress reporter while preserving existing snapshot helper callers.
- [x] Emit start/settled lifecycle events from the real cache/download paths without changing deduplication or 4-way semaphore concurrency.
- [x] Pass `operationId` from `refresh_skill_update_inventory` into the reporter and keep event emission best effort.
- [x] Emit `started` after the deduplicated repository set is known and `finalizing` after snapshot preparation succeeds.

Validation:

```powershell
cd src-tauri
cargo test central_updates::snapshots
cargo test central_updates::inventory
```

Review gate: total/completed invariants are independent of completion order; each started repository settles once; cache hits and failure paths are covered.

## 2. Frontend Store And Types

- [x] Add the event payload and transient store progress types in `src/types/skillUpdateInventory.ts`.
- [x] Extend `updateCenterStore.refresh` to generate `operationId`, await subscription before invoke, filter and idempotently merge matching events, and always unlisten.
- [x] Reset transient progress at the start of every attempt and clear it on success/failure; preserve existing inventory, cache-bypass, and error behavior.
- [x] Add store tests for listener-before-invoke ordering, active repository set updates, out-of-order completion, stale operation filtering, success cleanup, failure cleanup, retry reset, and unlisten.

Validation:

```powershell
pnpm test -- updateCenterStore
pnpm typecheck
```

Review gate: components remain free of IPC/event calls and no listener can outlive its refresh promise.

## 3. Focused Dialog Progress View

- [x] Pass refresh progress from `useCentralUpdateCheckModeController` to `UpdateCheckModeDialog`.
- [x] Replace mode cards/warning with a stable progress layout while submitting; restore selection view on failure.
- [x] Render preparing, determinate checking, and finalizing states; list all active repositories and handle long names without layout overflow.
- [x] Add correct progressbar semantics, live status text, and bilingual i18n strings.
- [x] Preserve current success navigation, failure inline alert/toast, selected mode, retry, and close behavior.
- [x] Extend component and Central view tests for the state transition and 1-4 active repositories.

Validation:

```powershell
pnpm test -- UpdateCheckModeDialog CentralSkillsView.updates-and-search
pnpm typecheck
pnpm lint
```

Review gate: no mode cards remain visible during submission; failure restores them; all active `owner/repo` labels are readable/accessibly available.

## 4. Cross-Layer Verification

- [x] Confirm Rust and TypeScript payload field names/status values match exactly after camelCase serialization.
- [x] Run formatting and focused tests first, then the repository gate.
- [x] Inspect the final diff for unrelated changes, sensitive event fields, accidental command/result changes, and serialized download regressions.
- [x] Manually exercise the Tauri Windows flow with multiple repositories and verify the progress bar, concurrent active list, success transition, and a controlled failure/retry path.

## Verification Evidence

- Windows Tauri：真实检查 7 个仓库，观察到准备态、`6 / 7`、`86%`、活跃仓库
  `pbakaus/impeccable`，随后自动打开 Update Center。
- Windows Tauri：受控失败后恢复原模式选择，同时显示内联错误和 toast；提交按钮恢复，
  再次点击可执行新尝试。临时故障开关已撤销并重新编译正常代码。
- `just ci`：124 个前端测试文件、1362 passed / 1 skipped；Rust 798 passed / 4
  ignored，集成测试 3 + 5 passed；typecheck、lint、sizecheck、entrypoint、Clippy 全部通过。
- `git diff --check` 通过，仅有工作区既有 LF/CRLF 提示。

Validation:

```powershell
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test central_updates::snapshots
cargo test central_updates::inventory
cd ..
pnpm test -- updateCenterStore UpdateCheckModeDialog CentralSkillsView.updates-and-search
just ci
```

Manual Windows acceptance is required because browser fixtures cannot prove Tauri event timing or actual concurrent repository labels.

## Rollback Points

- Backend event/reporter changes are additive and can be reverted without touching inventory persistence.
- Frontend store/types can be reverted together to restore the one-shot refresh call.
- Dialog/controller/i18n changes can be reverted together to restore the existing “检查中...” button-only feedback.
- No migration or generated artifact cleanup is required.
