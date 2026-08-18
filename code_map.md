# Code Map

Use this file for navigation. It is not a replacement for behavior rules in `AGENTS.md`.

## Top-Level Routes

| Path | Role | Search anchors |
|---|---|---|
| `src/` | React + TypeScript renderer | `src/i18n/`, `src/stores/`, `src/components/` |
| `src-tauri/src/` | Tauri/Rust backend | `commands/`, `services/`, `db/`, `linker.rs`, `paths.rs` |
| `scripts/` | Version, CI, docs, build, and release helpers | `check/run-ci.mjs`, `check/sync-version.mjs`, `build/build.mjs` |
| `.github/workflows/` | CI, docs, and desktop release workflows | `just-ci`, `release`, `docs` |
| `docs/` | User, architecture, and agent-facing documentation | `docs/agents/`, `docs/architecture/` |
| `.trellis/` | Workflow, specs, tasks, and journals | `.trellis/workflow.md`, `.trellis/spec/` |

## Backend Routes

- `src-tauri/src/commands/` is the IPC shell: argument translation, operation logging, and error
  stringification. Domain behavior belongs in `src-tauri/src/services/` and repositories.
- `src-tauri/src/services/installation/` owns Centralization and install/uninstall transport.
- `src-tauri/src/db/` owns SQLite schema and repositories; read the backend spec index before
  changing migrations or persistence.
- `src-tauri/src/paths.rs` is the path-policy source for Central, Universal Agents, database, and
  target-cache paths.

## Frontend Routes

- `src/stores/` contains domain Zustand stores and their Tauri IPC adapters.
- `src/components/skill/UnifiedSkillCard.tsx` is the shared skill-card implementation.
- `src/i18n/` contains English and Chinese user-visible strings.
- `src/test/` contains Vitest setup, fixtures, and renderer tests.

## Search Workflow

1. Start with `rg --files` and this map to identify the owning layer.
2. Search the symbol or persisted field across commands, services, repositories, stores, and tests
   before changing it.
3. For generated docs, inspect the source command/schema and run the documented generator rather
   than editing `_generated/` by hand.
