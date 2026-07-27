# Implementation plan

## 1. Backend startup domain

- [x] Add typed startup status, issue, diagnostic and error types in a focused startup service module.
- [x] Refactor the internal DB open/migration path to preserve failure stage without changing existing public callers or migration order.
- [x] Expose/reuse the existing integrity-check primitive for startup diagnosis; log internal diagnostics while keeping IPC payloads safe.
- [x] Implement the serialized `StartupCoordinator` and pure/testable retry state transitions.

## 2. Safe backup and rebuild

- [x] Implement a coherent blocking operation that moves DB/WAL/SHM into a unique recovery set and never overwrites prior backups.
- [x] Add rollback/fail-closed handling for partial move failures and preserve backup evidence when clean initialization fails.
- [x] Add focused Rust tests for corrupt DB, directory failure, schema failure, companion preservation, injected partial move failure, retry non-mutation and concurrent action serialization.

## 3. Tauri integration

- [x] Extract one ready-state installation helper from `lib.rs` so cold-start and recovered-start paths share `AppState` construction and background startup.
- [x] Replace startup `expect` calls with status capture while allowing the main window to continue loading on failure.
- [x] Add typed `get_startup_status`, `retry_startup`, `rebuild_startup_database` and `exit_startup` commands that do not depend on `AppState`.
- [x] Register the commands and prove healthy initialization/background work occurs once.

## 4. Frontend startup gate

- [x] Add public TypeScript status types and typed command-map entries.
- [x] Add `startupStore` as the only startup invoke owner, including loading/action/error and retry/rebuild/exit actions.
- [x] Add the browser `ready` fixture before React render.
- [x] Add `StartupGate` and a full-window `StartupRecoveryView`; move main-window show ownership above `AppShell`.
- [x] Add complete Chinese and English i18n for loading, classified failures, diagnostics, actions and action errors.

## 5. Verification

- [x] Run focused Rust startup/migration tests and inspect injected-failure assertions for real backup and non-mutation evidence.
- [x] Run focused startup store/component/app tests, IPC coverage and i18n contract tests; repeat async UI tests to detect timing failures.
- [x] Run `pnpm typecheck`, `pnpm lint`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`.
- [x] Run `just ci` and inspect the complete final diff for unrelated files and raw error/path leakage.
- [x] Run a Windows `pnpm tauri dev` cold-start smoke; confirm the window appears and the healthy path reaches the app without duplicate initialization.

## 6. Finish

- [x] Add a concise backend startup-recovery spec and index entry if implementation confirms this new durable contract.
- [ ] Run `trellis-check`, make one scoped local Chinese emoji commit, archive only `07-24-startup-resilience`, and record the session journal without pushing.

## Rollback points

- DB stage typing must preserve the current public `open_database` API and existing migration tests.
- Do not proceed to frontend integration until corrupt/schema/directory classification and backup rollback tests pass.
- Do not archive if the Tauri window remains hidden in either ready or failure state, or if any recovery payload/DOM exposes raw paths or SQLx errors.
