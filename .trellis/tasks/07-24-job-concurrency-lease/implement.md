# Implementation Plan: Exclusive Job Lease And Migration Coordination

## 1. Freeze Contracts And Tests

- [x] Add registry tests for single-family busy, cross-family independence, exact-ID cancellation, cancel-before-register, stale pending discard, stale cancel after successor start, no-active idempotence, RAII release, invalid IDs, and poison fail-closed behavior.
- [x] Add progress payload/merge tests that require jobId and reject stale update/portability events.
- [x] Add migration contention tests with isolated temp lock/source/target paths before changing production flow.

## 2. Backend Registry And AppState

- [x] Add a shared exclusive registry/error module with RAII lease and stable IPC error codes.
- [x] Replace `central_update_cancel` and `portable_state_cancel` in `AppState` and all test constructors with two registry instances.
- [x] Keep `AiTagJobRegistry` unchanged and add a structural test preventing shared cancel fields from returning.

## 3. Central Update Command Migration

- [x] Add jobId to check skill, check repository, update skill, apply decisions and cancel command arguments.
- [x] Acquire the Central update lease before the first await; pass its flag to existing services and release only by RAII.
- [x] Thread explicit jobId through update/repository-sync progress emission and add it to every payload.
- [x] Preserve inventory refresh `operationId` and leave force update/mirror outside the registry.

## 4. Portability Command Migration

- [x] Add jobId to export, JSON preview, file preview, import and cancel command arguments.
- [x] Remove nested command lease acquisition from file preview by sharing a private established-job helper.
- [x] Thread explicit jobId through manual and service-level portability progress emission.
- [x] Preserve save-export behavior and existing cancellation checkpoints/results.

## 5. Frontend Correlation And Feedback

- [x] Type affected IPC commands in `commandMap.ts` where practical and remove their stale allowlist entries; otherwise update the existing typed argument contracts without widening the allowlist.
- [x] Add jobId to job/progress types, initial states, constructors and merge helpers.
- [x] Generate one ID per store action, pass it to invoke, condition all async state writes on that ID, and send it in cancel calls.
- [x] Prevent same-store duplicate starts from replacing active UI state while retaining backend enforcement across stores.
- [x] Add bilingual backend error keys and route Central update, Update Center and portability visible errors through `formatBackendError`.

## 6. Legacy Migration Coordination

- [x] Refactor recursive migration FS work into one `run_blocking_fs_with` unit with typed join errors.
- [x] Acquire the existing Local mutation guard, recheck the completion marker under the guard, write the marker before release, and prohibit unlocked fallback.
- [x] Preserve summary JSON, source retention, existing-target skip, partial-copy cleanup, startup progress and next-start retry behavior.
- [x] Prove real lock contention/retry and verify no async recursive `std::fs` remains.

## 7. Focused Verification

- [x] Run focused Rust registry, Central update, portability, central migration, operation log and command tests.
- [x] Run focused Vitest store, Task Center, Update Center, portability dialog and backend-error tests repeatedly where async event timing is involved.
- [x] Run `pnpm typecheck` and `pnpm lint`.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`.
      (Final checker: registry 5 passed; migration 3; Central update 91 passed/3 ignored; portability 30; operation log 11; focused Vitest 161.)

## 8. Full Gate, Spec And Closeout

- [x] Run default-concurrency `just ci`, `python ./.trellis/scripts/task.py validate`, and `git diff --check`.
      (`just ci`: frontend 1497 passed/1 skipped; Rust library 1009 passed/6 ignored plus all binary/E2E suites; production build passed.)
- [x] Add backend/frontend lifecycle specs and synchronize mutation-lock/spawn-blocking references.
- [ ] Run Trellis full-scope check and inspect the scoped diff; create one Chinese emoji work commit for this child only.
- [ ] Archive only `07-24-job-concurrency-lease`, journal its work commit, leave the parent active, preserve unrelated dirty files, and do not push.

## Rollback Points

- Do not land backend command-argument changes without the matching frontend invoke/cancel payloads.
- Do not add jobId to only some progress emissions; mixed correlated/uncorrelated events are a release blocker.
- Do not remove shared flags until every listed consumer owns a registry lease.
- Do not move migration FS to blocking without holding the existing Local file lock through marker write.
