# Implementation Plan: Recoverable FS + DB Operations

## 1. Preconditions And Contract Freeze

- [x] Reconfirm both archived dependencies, migration digest/version 2, current delete/update entrypoints, and the parent `7/16` state.
- [x] Freeze the phase enum, manifest version, terminal/nonterminal sets, recovery decision table, redaction fields, and target-scoped availability decision in tests.
- [x] Preserve unrelated Trellis runtime/config, parent/other child tasks, `.gitattributes`, and the audit report.

## 2. Migration 3 And Journal Repository

- [x] Add immutable migration 3 for `fs_db_operations`, indexes, phase constraints, and active-operation uniqueness.
- [x] Add typed DB rows, manifest encode/decode validation, insert/list/get/transition APIs, and transaction-aware business-marker updates.
- [x] Prove five tag fixtures and version-2 current DB upgrade to version 3, locked digest, backup behavior, current reopen idempotency, and future/checksum rejection.
- [x] Ensure terminal cleanup cannot delete pending rows and journal failures are never best effort.

## 3. Per-Target Mutation Lease

- [x] Extend the existing `fs2` guard to accept a validated lock path while retaining `central-mutation.lock` for Local compatibility.
- [x] Derive SSH/WSL lock filenames from SHA-256 target ID digests under the existing locks directory.
- [x] Add same-target independent-process contention/crash-release tests and different-target non-contention tests.
- [x] Move only delete/update top-level mutation boundaries in this task; leave unrelated Central mutation call sites on the compatible Local API.

## 4. Shared Operation Workspace And Recovery

- [x] Implement versioned delete/update manifests and strict phase transition validation.
- [x] Implement Local sibling staging/backup/swap/restore/finalize/fingerprint primitives through `run_blocking_fs_with`.
- [x] Implement equivalent supervised SSH/WSL compound scripts with operation markers and no automatic deletion of durable backups.
- [x] Implement the recovery decision table, fingerprint/marker collision handling, idempotent repeat recovery, bounded errors, and retention rules.
- [x] Add startup inventory/local recovery and current-target list/retry service APIs without remote polling from read-only commands.

## 5. Delete Saga Integration

- [x] Refactor Local and remote single delete to share plan -> journal -> backup rename -> DB transaction + marker -> finalize.
- [x] Keep `db::delete_skill` parent-only FK cascade semantics and retained copy behavior.
- [x] Route batch delete through the single-skill Saga while preserving ordered partial results and duplicate request coalescing.
- [ ] Add DB failure, missing path, symlink/copy/native, retained copy, remote transport, crash, rollback, commit-unknown, collision, and idempotent retry tests.

## 6. Update Saga Integration

- [x] Keep `update_skills_batch` as the only normal/force/mirror write orchestrator and retain 16-write/32-copy remote chunk limits.
- [x] Split current atomic write into durable stage/swap/finalize operations and stop deleting backups before DB persistence.
- [x] Add transaction-aware skill upsert + repository assignment and write `db_committed` in that same transaction.
- [x] Persist exact copy projection plans, transition to `copies_pending`, retry only incomplete copies, then finalize backup and complete.
- [x] Preserve ordered per-skill outcomes, snapshot semantics, duplicate collapse, and cancellation before/after the destructive boundary.

## 7. Recovery Commands And Operation Logs UI

- [x] Add thin active-target commands to list pending operations and retry recovery with target-ID equality checks.
- [x] Record redacted recovery summaries in always-local Operation Logs; exclude manifests/full paths from list/detail/export and diagnostic text.
- [x] Add typed IPC map entries, a focused Zustand recovery slice, browser fixtures, and bilingual i18n.
- [x] Add a compact current-target pending band to `OperationLogsView` with refresh and retry states; keep cached reads and other targets usable when remote recovery is offline.
- [x] Test loading, empty, pending, retry success/failure, offline failure, target switch, stale responses, and no-overlap responsive structure.

## 8. Crash And Integration Verification

- [x] Add a subprocess crash helper that pauses at prepared, staged, swapped, DB apply/commit, copies pending, and pre-completion markers; parent kills it, reopens DB/FS, and asserts old/new convergence.
- [x] Add fake-runner SSH/WSL phase-loss matrices plus ignored Windows WSL smoke coverage for real rename/restore scripts.
- [x] Run focused migration, central operation, central skills, central updates, targets, operation log, and command tests.
- [x] Run frontend store/page tests, `pnpm typecheck`, and `pnpm lint`.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`.
- [x] Run default-concurrency `just ci`; this task does not change packaging/Tauri config, so no bundle build is required.

## 9. Spec, Review, And Closeout

- [x] Add an executable FS+DB operation journal spec and update backend index.
- [x] Update central mutation lock, central update batching, database migration, deletion integrity, target context, redaction, and test-support contracts only where behavior changed.
- [x] Run full-scope `trellis-check`; reject any unjournaled delete/update path, remote unlocked fallback, best-effort phase write, export leak, or duplicate update orchestrator.
- [ ] Run `trellis-update-spec`, inspect the scoped diff, and create one Chinese emoji work commit for this child only.
- [ ] Archive only `07-24-fs-db-operation-journal`, journal the work commit, and do not push.

## Rollback Points

- Migration 3 must land before any production row can be created; released migration 1/2 sources and digests never change.
- Local and remote delete cannot land separately if either retains direct destructive deletion without a durable row.
- Update stage/swap changes cannot land without normal/force/mirror all routing through the same Saga batch.
- Recovery UI cannot claim completion while a durable row is nonterminal or a backup/copy projection remains unresolved.
- Collision handling always preserves evidence and blocks mutation; it never chooses overwrite as a fallback.

## Implementation Evidence (2026-07-27)

- Migration evidence: `cargo test db::migrations --locked` passed 7 tests, including five frozen release fixtures, checksum/future-version rejection, backup blocking, restore, and migration 3 digest lock.
- Lease evidence: the locked full Rust suite passed the independent-process same-target contention/crash-release test and different-target non-contention test. Local retains `central-mutation.lock`; remote paths use target-ID digests.
- Crash evidence: `subprocess_kill_phase_matrix_converges_to_old_or_new_state` launches a real child test process, kills it at prepared, staged, swapped, DB commit, copies pending, and pre-completion, then reopens DB/FS and verifies terminal convergence plus artifact cleanup. This is subprocess-kill evidence, not an in-process failure simulation.
- Remote script evidence: SSH and WSL `FakeRunner` tests cover delete stage/restore and update stage/swap/rollback protocol parity. Durable update staging proves 33 writes use three gzip archive uploads, and copy refresh retains 32-target chunks. These are FakeRunner protocol tests, not live WSL proof.
- Startup/read-only evidence: startup Local recovery rolls back pending Local rows; separate remote inventory tests prove pending SSH/WSL rows can be listed without opening transport.
- Redaction evidence: pending summaries and exported Operation Logs exclude recovery manifests and full paths; retry logging stores bounded codes/messages only. Live UI/store tests cover loading, empty, pending display, retry success/failure, offline failure, target switch, stale-response discard, target clearing, and narrow-screen wrapping while keeping cached logs usable.
- Final implementation gate: `just ci` passed at default concurrency with 1441 frontend tests passed / 1 skipped, 960 Rust unit tests passed / 6 ignored, all integration suites, typecheck, lint, capabilitycheck, sizecheck, Rust entrypoint/fmt/Clippy, and production build.
- Full-scope Trellis check made rollback phase-aware so partial Local staging can be removed safely, fixed SSH/WSL recovery when a failed swap left only the old backup, removed raw recovery-error tracing, propagated delete/update compensation and marker-cleanup failures, validated retry IPC identities before logging, and completed the Operation Logs recovery-state matrix. Regression tests also prove cancellation observed after durable staging finishes instead of returning a false cancellation, and remote delete/update finalize remains idempotent after markers are already cleaned. Production search still finds no unjournaled Central delete/update caller and the legacy atomic writer remains `#[cfg(test)]` only.
- Final review gate: default-concurrency `just ci` passed with 1446 frontend tests passed / 1 skipped and 967 Rust library tests passed / 6 ignored, plus all integration suites, typecheck, lint, capabilitycheck, sizecheck, Rust entrypoint/fmt/Clippy, and production build. `task.py validate` and `git diff --check` passed; the latter emitted only existing LF-to-CRLF notices.
- Spec update added the seven-section `fs-db-operation-journal.md` contract and synchronized the target lease, update batching, migration 3, deletion transaction, target identity, recovery redaction, critical-write, and crash/FakeRunner contracts without changing unrelated specs.
- A repeated default-concurrency gate exposed three coupled test failures: parallel Local Saga tests exhausted the shared production file-lock timeout before creating a journal row. A `cfg(test)` in-process guard now serializes only real default-path test entrypoints; isolated `_at` process tests still exercise the OS lock. The next full Rust run passed 967 library tests / 6 ignored plus all 3/4/5 integration suites.
- Not claimed: the ignored live Windows WSL smoke was not run because `SKILLPORT_TEST_WSL_DISTRO` was not supplied. The scoped work commit, archive, and journal remain open.
