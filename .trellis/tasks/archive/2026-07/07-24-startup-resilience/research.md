# Startup resilience research

## Current startup boundary

- `src-tauri/src/lib.rs` initializes logging, creates the application data directory, opens/migrates SQLite, manages `AppState`, and then starts secret and Central migration background work inside one `setup` closure.
- Directory creation and `db::open_database` still use `expect`, so a recoverable database failure prevents the webview from presenting any UI.
- Tauri permits commands that do not request `State<AppState>` to remain callable when `AppState` was not managed. This allows a narrow startup command surface while every normal command remains unreachable behind the frontend gate.

## Existing mechanisms to reuse

- `db/migrations.rs` owns preflight, ordered migrations, backup-before-write, restore-on-failure, foreign-key validation, and seed.
- `db/migrations/backup.rs` owns `PRAGMA integrity_check`, validated SQLite backup publication, quarantine and restore. Startup diagnostics and recovery should extend this ownership instead of duplicating SQL or path rules in `lib.rs`.
- `fs_util::run_blocking_fs_with` is the required boundary for coherent blocking filesystem operations and typed join failures.
- `showMainWindowWhenReady` currently runs from `AppShell`; a recovery view before `AppShell` must own this call so startup failures are visible instead of leaving the hidden window hidden.

## Frontend boundary

- `src/lib/ipc/commandMap.ts` is the typed command registry and `src/lib/ipc/invoke.ts` is the only IPC adapter.
- `.trellis/spec/frontend/ipc-adapter.md` requires store-only invoke ownership. A dedicated startup store should load status and expose retry/rebuild/exit actions.
- Browser fixtures are installed before React render. The startup fixture must report `ready`, preserving the existing demo path.
- Tests live under the nested ownership layout. IPC coverage is now `src/test/contracts/ipcCommandCoverage.test.ts`, not the old top-level path recorded in the audit PRD.

## Chosen product decision

The user chose a full frontend recovery page. The page keeps the main window alive, gates all DB-dependent UI, and exposes retry, backup-and-rebuild, and exit. A native-dialog-only implementation is rejected for this task because it cannot provide the required multi-step diagnostic and retry states consistently.

## Risk notes

- Database open and schema initialization must be classified without parsing error strings. Preserve the current `open_database` compatibility surface while adding an internal staged failure representation for startup.
- Backup-and-rebuild is destructive only after a complete, unique backup set is established. Companion files move with the main DB and partial failure must fail closed.
- Recovery commands can race through repeated clicks or StrictMode. Backend serialization is authoritative; frontend disabled states are only ergonomic protection.
- Unit-testable startup filesystem/state logic should not capture `AppHandle`; the Windows test-linking constraint in `spawn-blocking-io.md` remains applicable.
