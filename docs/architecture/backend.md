# Backend

The Rust side is a Tauri v2 application split into thin IPC handlers, domain services, target adapters, and a sqlx-backed SQLite layer.

## Crate Layout

```text
src-tauri/src/
├── lib.rs             Tauri builder + invoke_handler list
├── main.rs            Entry point — calls lib::run()
├── path_utils.rs      Cross-platform path helpers
├── paths.rs           Stable application paths (~/.skillsmanage)
├── central_migration.rs  Legacy ~/.agents/skills → private store migration
├── commands/          IPC handlers, grouped by domain
├── services/          Pure business logic
├── targets/           Local + SSH execution
└── db/                Pool + versioned migrations + schema + repos
```

## AppState

`AppState` (in `lib.rs`) is shared via `tauri::State<AppState>`:

| Field | Purpose |
| --- | --- |
| `db: DbPool` | Always-local sqlite pool for the user's machine |
| `targets: TargetRegistry` | Active target (Local / SSH / WSL) + cache of remote pools |
| `ai_tag_jobs: AiTagJobRegistry` | Cooperative cancel flags for AI tagging tasks |

Commands that need target-scoped state begin with `AppState::resolve_target_context()`. The returned owned `TargetContext` binds one `ActiveTarget` to its matching SQLite pool, so switching the active target only affects later commands; an in-flight operation keeps its original target, DB, remote-resource identity, and operation-log identity. The legacy `active_db()` and `active_target()` helpers remain for DB-only or identity-only migration paths, but a command must never combine them.

## Commands Layer

Every `#[tauri::command]` lives under `src-tauri/src/commands/`. Files are domain-scoped; the full registry lives in `lib.rs::run()` inside `tauri::generate_handler!`.

| File | Domain |
| --- | --- |
| `bootstrap.rs` | Cold-start snapshot for the dashboard |
| `targets.rs` | Active target + SSH target CRUD |
| `logs.rs` | Operation Log list / get / clear / export + Runtime Log diagnostics |
| `scanner.rs` | On-demand `scan_all_skills` |
| `agents.rs` | 27 built-in + custom agents, enable/disable |
| `linker.rs` | Install / uninstall / batch install |
| `skills.rs` | Skill detail / content / file tree / open in OS |
| `central_metadata.rs` | Repos, tags, AI tag suggestions |
| `central_updates.rs` | Remote update status + bulk apply |
| `collections.rs` | Collection CRUD + import / export |
| `settings.rs` | Key/value + scan directories + GitHub PAT |
| `discover.rs` | (removed in 0.10.x — replaced by `projects.rs` + `obsidian.rs`) |
| `projects.rs` | Project add / list / rename / pin / scan / install / uninstall / remove |
| `obsidian.rs` | Obsidian vault scan + source-only import |
| `github_import.rs` | Repo preview + import + raw fetch |
| `marketplace.rs` | Registries + cache + AI explanation |
| `portable_state.rs` | SkillPort state import / export |

The complete handler list is generated automatically — see [IPC Commands](./ipc-commands.md).

## Services Layer

Services in `src-tauri/src/services/` host business logic. Commands stay thin (parse arguments, call a service, format the response).

```text
services/
├── scanner/             Read SKILL.md frontmatter on disk
├── projects/            Project-level skill management (add / scan / install / uninstall)
├── obsidian/            Obsidian vault scan + source-only import
├── installation/        centralize / native / project / remote / batch
├── central_skills/      Canonical-store query / delete / file tree
├── github_import/       Archive download, preview workspace, raw HTTP
├── marketplace/         Registry sync + cache
└── ai_provider/         Claude + OpenAI-compatible streaming
```

The split lets each service own its tests under the same module. Larger services (installation, github_import) further decompose into purpose-named files instead of growing one big mod.rs.

## Targets

`targets/` abstracts execution between the local machine, SSH hosts, and WSL distributions:

| File | Role |
| --- | --- |
| `model.rs` | Persisted target rows + owned request-scoped `TargetContext` |
| `registry.rs` | Atomic target/DB context resolution + remote pool cache |
| `exec.rs` | Run commands locally or via `ssh` |
| `cred.rs` | Encrypted password storage |
| `askpass.rs` | Password helper used by ssh |
| `commands.rs` | IPC commands (re-exported through `commands::targets`) |

Command handlers freeze one `TargetContext` before asynchronous work and pass its explicit target/DB into services. Services do not reread ambient `AppState`; transport-specific execution is constructed from the frozen target.

## Persistence

`db/` is split into:

```text
db/
├── pool.rs             private pool factory: WAL + per-connection FK verification
├── types.rs            shared structs
├── schema/             frozen migration-1 legacy baseline grouped by domain
├── migrations.rs       path-aware open, preflight, orchestration
├── migrations/         backup/restore, version sources, focused tests
├── repos/              one repo per business object
├── seed.rs             agent registry seed
└── tests.rs            integration-style db tests
```

See [Data Model](./data-model.md) for the table layout.

Desktop setup, `CliContext::open_default`, and `TargetRegistry::remote_db_for` all call the same `open_database*` boundary. A pool is not exposed to Tauri state or the target cache until migration, FK validation, and seed complete.

## Logging and Errors

- **Operation logs.** Long-lived, structured rows in `operation_logs`. Inserted by the linker / projects / marketplace services; surfaced in the Operation layer of the Logs page.
- **Runtime logs.** Short-lived daily files named `skillport-YYYY-MM-DD.log`. Written by backend tracing and frontend diagnostics, read/exported through whitelisted IPC helpers, and cleaned after the retention window.
- **Errors.** All commands return `Result<T, String>`. Services bubble `String` for the IPC boundary; rich error context stays inside services until the boundary collapses it for serialization.

Last reviewed: 2026-07-26
