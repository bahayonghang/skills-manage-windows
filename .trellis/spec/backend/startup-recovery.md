# Recoverable Desktop Startup Contract

## 1. Scope / Trigger

Use this contract when changing desktop startup, local database opening,
startup-only commands, pre-`AppState` Tauri state, or the renderer startup gate.
It keeps directory, SQLite, and schema failures recoverable without exposing
internal errors or mounting DB-backed UI before the database is ready.

## 2. Signatures

```rust
pub async fn attempt_startup(
    db_path: &Path,
) -> Result<DbPool, StartupAttemptFailure>;

get_startup_status() -> StartupStatus;
retry_startup() -> Result<StartupStatus, String>;
rebuild_startup_database() -> Result<StartupStatus, String>;
exit_startup() -> ();
```

`StartupStatus` is a serde-tagged enum with `phase` equal to `checking`,
`ready`, `recovery_required`, or `fatal`. Recovery payloads contain only the
closed `issue` and `diagnostic` enums plus `canRebuild` and `backupCreated`.

## 3. Contracts

- Tauri manages `StartupCoordinator` before opening the database. Startup
  commands depend on that coordinator and `AppHandle`, never `AppState`.
- Directory creation failure becomes fatal. Pool-open failure becomes
  `database_open_failed`; preflight, migration, FK, and seed failures become
  `schema_initialization_failed`. Classification uses `DatabaseOpenFailure`,
  not error-string matching.
- Retry repeats the normal directory/open/migrate/seed path without moving or
  rewriting the failed database. Retry and rebuild share the coordinator's
  operation mutex.
- Rebuild moves each existing member of `db.sqlite`, `db.sqlite-wal`, and
  `db.sqlite-shm` into one UUID-named sibling directory. It creates a temporary
  directory, moves the full set, syncs the temporary directory on Unix,
  atomically publishes it, and syncs the parent directory before creating a
  clean database.
- Any failure before or after backup publication rolls moved members back from
  the actual current directory. Incomplete rollback is fatal and clean DB
  initialization must not run. Published recovery backups are never pruned
  automatically.
- Cold-start and recovered-start success use the same `install_ready_state`
  path. `AppState` is managed before ready-only secret and legacy-Central jobs
  are spawned; the coordinator publishes `ready` only after installation.
- The renderer's Zustand startup store is the sole startup IPC owner.
  `StartupGate` mounts the normal app only for `ready`, while loading and both
  failure states still show the hidden main window after the first status call.
- IPC, DOM, and startup tracing fields contain stable codes and enum values,
  never raw paths, SQL, database content, credentials, or source errors.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Data directory cannot be created | `fatal / data_directory_unavailable`; retry and exit only |
| SQLite pool cannot open | `recovery_required / database_open_failed` |
| Schema preflight, migration, FK, or seed fails | `recovery_required / schema_initialization_failed` |
| Integrity check returns non-`ok` | Diagnostic `corrupt`; log only stable code/result |
| Integrity check cannot run | Diagnostic `unavailable`; do not log the raw SQLx error |
| Retry fails | Preserve the database and remain on the recovery surface |
| Backup member move fails | Roll all moved members back; publish no recovery directory |
| Error occurs after backup directory publish | Roll back from the published directory, not the old temporary path |
| Backup rollback is incomplete | `fatal / database_recovery_failed`; do not create a new database |
| Clean initialization after backup fails | Retain the recovery backup and report `backupCreated=true` |
| Repeated or concurrent action | Serialize at the coordinator; never install state or spawn jobs twice |

## 5. Good / Base / Bad Cases

- Good: a corrupt DB and its WAL/SHM are published as one retained recovery
  set, a clean DB reaches ready, and the normal app mounts in the same process.
- Base: a healthy cold start opens the DB once, installs `AppState` once, and
  adds only the startup-status IPC before the existing app flow.
- Good: a post-publish durability failure restores all three original files
  and removes the recovery directory before returning failure.
- Bad: catch a raw SQLx string in `lib.rs`, display it in React, or use it to
  decide whether rebuild is allowed.
- Bad: return failure after moving the DB but leave rollback pointing at the
  no-longer-existing temporary recovery directory.

## 6. Tests Required

- Rust fault injection for directory failure, corrupt DB, schema preflight
  failure, retry non-mutation, DB/WAL/SHM preservation, partial move rollback,
  post-publish rollback, and retained backup after clean initialization.
- Serialization coverage asserting exact camelCase recovery fields and absence
  of paths or internal error text.
- Store/component coverage for loading, ready, recovery, fatal, failed actions,
  rebuild success, duplicate-action suppression, hidden `AppShell`, and the
  browser-ready fixture.
- Static checks for removed startup-path `expect`, typed command-map coverage,
  i18n parity, and no raw startup diagnostic fields.
- Minimum gate: focused startup Rust/Vitest tests, `cargo fmt --all -- --check`,
  all-target locked Clippy, locked Rust tests, full frontend tests/build,
  `just ci`, and a Windows `pnpm tauri dev` cold-start window smoke.

## 7. Wrong vs Correct

### Wrong

```rust
std::fs::rename(&temp_dir, &final_dir)?;
sync_parent(parent)?; // failure returns while the original DB is already gone
```

### Correct

```rust
std::fs::rename(&temp_dir, &final_dir)?;
published = true;
if let Err(error) = sync_parent(parent) {
    rollback_database_set(&moved, &final_dir)?;
    return Err(error.into());
}
```

The recovery source is selected from the publication state, so every reported
backup failure either restores the original set or escalates as an explicit
fail-closed rollback failure.
