# Implementation Plan: Versioned SQLite Migration and FK Enforcement

## 1. Preconditions and Fixture Freeze

- [x] Confirm predecessor archive/work commit and seven-table ownership list.
- [x] Resolve the five selected tags to commits and freeze readable SQL fixtures plus a checksum manifest.
- [x] Add tag-schema assertions so fixtures cannot silently drift to current schema.
- [x] Preserve unrelated Trellis/runtime and other child-task dirt.

## 2. Connection and Open Boundary

- [x] Add shared SQLx pool options with `after_connect` FK enable + readback verification.
- [x] Introduce path-aware local/remote open APIs that own backup, migration, validation, seed, and pool return.
- [x] Route desktop, CLI, SSH cache, and WSL cache through the new APIs; remove production create+init composition.
- [x] Update shared test fixtures and document only genuine bare legacy-pool exceptions.

## 3. Versioned Migration Runner

- [x] Create `schema_migrations(version, checksum, applied_at)` and contiguous descriptors.
- [x] Hash every immutable source file reachable from a migration with existing SHA-256 support; lock released digests in tests.
- [x] Implement preflight for descriptor gaps, DB gaps, future versions, and checksum mismatch before mutation.
- [x] Convert schema initialization/`ensure_column` to one acquired connection so migration 1 and its metadata row commit atomically.
- [x] Add migration 1 for empty DB and all selected unversioned legacy schemas.

## 4. Backup and Recovery

- [x] Detect existing DB plus pending work before any repair/migration write.
- [x] Capture source-file existence before open; create a consistent bound-path `VACUUM INTO` snapshot, validate through a direct read-only connection, sync, and publish a unique source-version backup.
- [x] Skip backup for empty/current DB; never reuse a stale backup merely because it passes integrity check, and prune older same-version copies only after the new snapshot is durable.
- [x] On post-backup failure, close pool, quarantine failed DB, restore without consuming backup, clean WAL/SHM, verify, and return failure.
- [x] Fault-test backup refusal, restore after an earlier migration commit, and restore-error reporting.

## 5. FK Migration and Runtime Deletes

- [x] Keep startup order backup -> legacy baseline -> orphan repair/audit -> FK rebuild -> FK check -> seed.
- [x] Rebuild all seven owned relations with skill-parent `ON DELETE CASCADE`, exact columns/PK/defaults/indexes, row-count guards, and one migration transaction.
- [x] Run explicit `foreign_key_check` before migration commit and after the full migration sequence.
- [x] Remove runtime manual owned-relation deletes from single, batch, and scanner paths; retain the compile-time list for repair/migration/tests.
- [x] Prove observations, project snapshots, and usage history stay independent.

## 6. Verification and Documentation

- [x] Add focused migration tests for five tag fixtures, checksum/version rejection, per-connection FK, backup/restore, seven cascades, and idempotent reopen.
- [x] Run `cd src-tauri; cargo test db::migrations --locked`.
- [x] Run `cd src-tauri; cargo test db:: --locked` and relevant target/CLI integration tests.
- [x] Run `cd src-tauri; cargo fmt --all -- --check`.
- [x] Run `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`.
- [x] Run `cd src-tauri; cargo test --locked`.
- [x] Run `just ci`.
- [x] Update English/Chinese architecture docs and executable backend specs.

## 7. Review and Closeout

- [x] Run full-scope `trellis-check`; reject any production initialization bypass or duplicated relation list.
- [x] Run `trellis-update-spec` and verify seven-section database migration contract.
- [x] Inspect final diff for FK/schema/backup scope only; do not absorb operation-journal work.
- [x] Create one local Chinese emoji work commit for code, fixtures, docs, specs, and this task's artifacts.
- [ ] Archive only this child and record journal with the work commit; do not push.

## Rollback Points

- Connection/open API may land only with all production call sites migrated in the same atomic commit.
- Do not leave migration 1 without checksum/preflight or migration 2 without predecessor repair ordering.
- A failing restore test blocks completion even if per-migration transaction tests pass.
