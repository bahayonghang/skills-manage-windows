# Backend

The Rust side is a Tauri v2 application split into thin IPC handlers, domain services, target adapters, and a sqlx-backed SQLite layer.

## Crate Layout

```text
src-tauri/src/
├── lib.rs             Tauri builder + invoke_handler list (107 commands)
├── main.rs            Entry point — calls lib::run()
├── path_utils.rs      Cross-platform path helpers
├── paths.rs           Stable application paths (~/.skillsmanage)
├── central_migration.rs  Legacy ~/.agents/skills → private store migration
├── commands/          IPC handlers, grouped by domain
├── services/          Pure business logic
├── targets/           Local + SSH execution
└── db/                Pool + schema + repos
```

## AppState

`AppState` (in `lib.rs`) is shared via `tauri::State<AppState>`:

| Field | Purpose |
| --- | --- |
| `db: DbPool` | Always-local sqlite pool for the user's machine |
| `targets: TargetRegistry` | Active target (Local / SSH) + cache of remote pools |
| `ai_tag_jobs: AiTagJobRegistry` | Cooperative cancel flags for AI tagging tasks |

`AppState::active_db()` returns the pool that matches the current target — local commands work uniformly whether the user is on the local machine or attached to an SSH host.

## Commands Layer

Every `#[tauri::command]` lives under `src-tauri/src/commands/`. Files are domain-scoped; the full registry lives in `lib.rs::run()` inside `tauri::generate_handler!`.

| File | Domain |
| --- | --- |
| `bootstrap.rs` | Cold-start snapshot for the dashboard |
| `targets.rs` | Active target + SSH target CRUD |
| `logs.rs` | Operation log list / get / clear / export |
| `scanner.rs` | On-demand `scan_all_skills` |
| `agents.rs` | 27 built-in + custom agents, enable/disable |
| `linker.rs` | Install / uninstall / batch install |
| `skills.rs` | Skill detail / content / file tree / open in OS |
| `central_metadata.rs` | Repos, tags, AI tag suggestions |
| `central_updates.rs` | Remote update status + bulk apply |
| `collections.rs` | Collection CRUD + import / export |
| `settings.rs` | Key/value + scan directories + GitHub PAT |
| `discover.rs` | Project scan + Obsidian vault scan |
| `github_import.rs` | Repo preview + import + raw fetch |
| `marketplace.rs` | Registries + cache + AI explanation |
| `portable_state.rs` | SkillPort state import / export |

The complete handler list is generated automatically — see [IPC Commands](./ipc-commands.md).

## Services Layer

Services in `src-tauri/src/services/` host business logic. Commands stay thin (parse arguments, call a service, format the response).

```text
services/
├── scanner/             Read SKILL.md frontmatter on disk
├── discovery/           Project + Obsidian source-only scans
├── installation/        centralize / native / project / remote / batch
├── central_skills/      Canonical-store query / delete / file tree
├── github_import/       Archive download, preview workspace, raw HTTP
├── marketplace/         Registry sync + cache
└── ai_provider/         Claude + OpenAI-compatible streaming
```

The split lets each service own its tests under the same module. Larger services (installation, discovery, github_import) further decompose into purpose-named files instead of growing one big mod.rs.

## Targets

`targets/` abstracts execution between the local machine and an SSH host:

| File | Role |
| --- | --- |
| `model.rs` | Persisted target rows |
| `registry.rs` | Active target resolution + remote pool cache |
| `exec.rs` | Run commands locally or via `ssh` |
| `cred.rs` | Encrypted password storage |
| `askpass.rs` | Password helper used by ssh |
| `commands.rs` | IPC commands (re-exported through `commands::targets`) |

The result is that services do not branch on `if remote {}`; they call `targets::exec` and let the registry route the call.

## Persistence

`db/` is split into:

```text
db/
├── pool.rs             create_pool() with WAL mode
├── types.rs            shared structs
├── schema/             init.sql equivalents grouped by domain
├── migrations.rs       ensure_column for incremental ALTERs
├── repos/              one repo per business object
├── seed.rs             agent registry seed
└── tests.rs            integration-style db tests
```

See [Data Model](./data-model.md) for the table layout.

## Logging and Errors

- **Operation logs.** Long-lived, structured rows in `operation_logs`. Inserted by the linker / discover / marketplace services; surfaced in the Logs page.
- **Errors.** All commands return `Result<T, String>`. Services bubble `String` for the IPC boundary; rich error context stays inside services until the boundary collapses it for serialization.

Last reviewed: 2026-05-04
