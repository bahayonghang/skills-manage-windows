# Architecture Overview

SkillPort is a three-layer desktop application: a React UI talks to a Rust backend over Tauri IPC, the backend persists to a local SQLite file, and a small set of HTTP clients reach out to GitHub and AI providers.

## Layer Diagram

```text
┌──────────────────────────────────────────────────────────────────┐
│ React 19 + TypeScript (src/)                                     │
│   Routes (react-router v7)                                       │
│   Pages (src/pages/*)                                            │
│   Stores (src/stores/* — Zustand)                                │
│   Components (src/components/*)                                  │
└──────────────┬───────────────────────────────────────────────────┘
               │  invoke() / event listen()
┌──────────────▼───────────────────────────────────────────────────┐
│ Rust + Tauri v2 (src-tauri/src/)                                 │
│   commands/*  thin IPC handlers                                  │
│   services/*  business logic (scanner / installation / projects) │
│   targets/*   local + SSH execution adapters                     │
│   db/*        sqlx pool + schema + repos                         │
└──────────────┬─────────────────────────┬─────────────────────────┘
               │                         │
       ┌───────▼──────┐         ┌────────▼────────┐
       │ SQLite (WAL) │         │ HTTP (reqwest)  │
       │ ~/.skillsmanage/db.sqlite │ GitHub + AI │
       └──────────────┘         └─────────────────┘
```

## Module Boundaries

| Layer | Responsibility | Reaches |
| --- | --- | --- |
| `src/pages/` | Route-level views | Stores only |
| `src/stores/` | UI state + IPC calls | `invoke()` and Tauri events |
| `src/components/` | Reusable UI | Stores via hooks |
| `src-tauri/src/commands/` | `#[tauri::command]` handlers | `services/*`, `db/*`, `targets/*` |
| `src-tauri/src/services/` | Pure business logic | `db/repos`, OS, HTTP |
| `src-tauri/src/targets/` | Local + SSH execution | OS, `ssh` binary |
| `src-tauri/src/db/` | Schema + sqlx repos | SQLite pool |

Components never call `invoke()` directly. Stores own the IPC surface so test mocks can hook a single layer.

## Canonical Data Path

```text
[user action] ──► page ──► store action ──► invoke('xxx')
                                         │
                                         ▼
                                 commands::xxx
                                         │
                                         ▼
                                 services::xxx
                                         │
                                  ┌──────┴──────┐
                                  ▼             ▼
                            db::repos      OS / HTTP
```

The two split points keep the backend testable: `commands::xxx` stays thin so `services::xxx` can be exercised without spinning up a Tauri runtime; `services::xxx` borrows `&DbPool` so unit tests use a temporary SQLite file.

## Source Layout

```text
src/
├── pages/             route-level views
├── stores/            Zustand stores, the only IPC entry point
├── components/        reusable UI
├── data/              static catalogs (officialSources / aiProviders)
├── i18n/              en + zh JSON resources
├── lib/               cross-cutting helpers (platformTargetGroups, …)
└── test/              vitest setup and fixtures

src-tauri/src/
├── commands/          IPC handlers grouped by domain
├── services/          business logic split by domain
│   ├── scanner/       SKILL.md discovery on disk
│   ├── installation/  centralize / native / project / remote / batch
│   ├── projects/      project-level skill management (add / scan / install / uninstall)
│   ├── obsidian/      Obsidian vault scan + source-only import
│   ├── github_import/ archive download, preview workspace, raw HTTP
│   ├── marketplace/   registry sync + cache
│   ├── central_skills/ canonical store services
│   └── ai_provider/   Claude / OpenAI-compatible streaming
├── targets/           local + SSH execution adapters
├── db/                schema, migrations, repos, seed
└── lib.rs             Tauri builder with the full invoke_handler list
```

## Cross-cutting Concerns

- **State sync.** The backend emits `system://migration-progress` during startup so the UI can show a banner without polling.
- **Observability.** User actions write structured `operation_logs` rows for the Operation layer, while frontend/backend diagnostics write bounded `skillport-YYYY-MM-DD.log` files for the Runtime layer. The `/logs` console reads both through `commands::logs`; see [Runtime Observability](./runtime-observability.md).
- **Active target.** All commands resolve `AppState::active_db()` first so SSH-bound calls hit the remote SQLite cache instead of the local one.

Last reviewed: 2026-06-03
