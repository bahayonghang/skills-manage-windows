# Design: recoverable desktop startup

## Architecture

Introduce a startup domain that is alive before the database-backed `AppState`:

```text
Tauri setup
  -> initialize logging
  -> manage StartupCoordinator
  -> attempt_startup(data_dir)
       -> create/validate data directory
       -> staged database open + migration + seed
       -> optional integrity diagnosis on failure
  -> ready: manage AppState + start ready-only background work once
  -> failure: retain only StartupCoordinator and continue window startup

React root
  -> startupStore.loadStatus()
  -> checking: startup loading surface
  -> ready: App routes -> AppShell -> DB-backed stores
  -> recovery_required/fatal: StartupRecoveryView (AppShell never mounts)
```

The normal command surface remains unchanged. Commands requiring `State<AppState>` are never invoked until status is `ready`; startup commands depend only on `StartupCoordinator` and `AppHandle`.

## Backend State And Contracts

`StartupCoordinator` owns a serialized operation lease and the last public `StartupStatus`. The status is a serde tagged enum:

- `checking`
- `ready`
- `recovery_required { issue, diagnostic, canRebuild }`
- `fatal { issue }`

`issue` and `diagnostic` are closed enums with stable snake_case wire values. They contain no raw error, SQL, path or database content. Internal typed errors retain sources for Runtime Log diagnostics; command boundaries expose only the safe status or stable safe error text.

The startup domain exposes three operations:

1. `retry`: re-run directory and staged DB initialization without filesystem mutation.
2. `rebuild`: for DB-related recovery states only, publish a unique backup set for the DB plus WAL/SHM, then initialize a clean DB.
3. `exit`: terminate the desktop process through the app handle.

The coordinator serializes retry/rebuild. A second request while one is active returns the current `checking` state or a stable busy error; it cannot run another DB open or backup. Installing `AppState` and spawning ready-only tasks is guarded once on the backend even if the frontend retries.

## Database Failure Classification

Do not branch on `error.to_string()`. Refactor the internal migration/open path to retain a typed stage:

- pool/open failure -> `database_open_failed`
- preflight, backup, migration, FK validation or seed failure -> `schema_initialization_failed`
- integrity check reports non-`ok` -> diagnostic `corrupt`
- integrity check cannot run -> diagnostic `unavailable`

Keep the existing public `db::open_database` and `open_database_for_remote_home` behavior compatible for current callers and tests. Add a startup-oriented internal entry point or typed wrapper that preserves stage metadata while sharing the same migration implementation.

## Backup And Rebuild Protocol

The startup domain reuses database backup/integrity primitives and the blocking-FS helper. With no live pool:

1. Create a unique sibling recovery directory using timestamp plus UUID; never reuse or overwrite an existing location.
2. Move each existing member of `{db.sqlite, db.sqlite-wal, db.sqlite-shm}` into the recovery directory in one blocking unit.
3. If a move fails, roll previously moved members back. If rollback also fails, return a typed fail-closed error and do not attempt new DB initialization.
4. Sync/publish the recovery directory before opening a new database.
5. Run the exact normal staged open/migrate/seed path. If it fails, preserve the backup and report recovery required; never delete the backup automatically.

The public status reports only that a backup was created, not its absolute path. Runtime logs record stable stage codes and diagnostic sources under the existing local redaction policy.

## Ready-State Installation

Extract the existing post-open setup from `lib.rs` into one helper that:

- constructs and manages the existing `AppState`;
- performs Local Central operation recovery and target-config recovery;
- starts secret migration and legacy Central migration jobs;
- emits existing migration progress events.

Both cold-start success and recovery success call this helper. It must be idempotently guarded so jobs run once. Failure to install an already-present `AppState` is handled as already-ready only when the coordinator confirms ready; inconsistent state fails closed.

## Frontend State And UI

Add a dedicated Zustand startup store, which is the sole owner of startup IPC. It tracks public status, initial loading, active action and action error. Each retry/rebuild clears stale errors, disables both mutation buttons while active, and rethrows nothing into React.

Wrap the routed app with `StartupGate` in the React root:

- call `showMainWindowWhenReady` after the first status resolves, including failure states;
- render the existing app only for `ready`;
- never mount `AppShell` while checking or failed;
- browser fixture returns `ready` before the gate renders.

`StartupRecoveryView` is a full-window, unframed recovery surface using existing tokens and Lucide icons. Recovery states show retry, backup-and-rebuild and exit; fatal directory failures show retry and exit. All visible strings live in both locale files. Raw backend error text is not rendered.

## Compatibility

- No production dependency is added.
- Existing `AppState`, normal command signatures and `Result<T, String>` command boundary remain unchanged.
- Existing database migration backups remain authoritative; startup backup names are a separate recovery family and are never pruned automatically.
- Browser demo startup remains ready through a typed fixture.

## Rollback

Code rollback removes the startup gate/commands/coordinator and restores the old setup path. User recovery backup directories are retained because deleting them would destroy recovery evidence. The database migration format is unchanged, so rollback does not require a schema down-migration.

## Risks

- Conditional `AppState` management is safe only if the frontend gate prevents all normal IPC; component and store tests must prove `AppShell` is absent on failures.
- Partial companion-file movement is the highest data-integrity risk; inject each move failure and assert rollback/fail-closed behavior.
- StrictMode and repeated clicks can duplicate requests; backend serialization and one-time ready installation are mandatory.
- Moving window-show ownership can regress hidden-window startup; ready and failure tests must both assert the show call.
