# AGENTS.md

SkillPort is a React + TypeScript + Tauri desktop application fork whose primary local delivery
target is a stable Windows installer. Use `pnpm` and `just` from the repository root.

The pinned local toolchain is Node 26, pnpm 10.34.5, and Rust 1.98.0. The standard local entry
points are `just doctor`, `just check`, `just ci`, and `just audit`.

## Start Here

- Use [`code_map.md`](code_map.md) for repository navigation and search anchors.
- Read the relevant `.trellis/spec/<layer>/index.md` before editing backend, frontend, or quality
  code; follow the project Trellis workflow for multi-step work.
- Read [`docs/agents/build-and-test.md`](docs/agents/build-and-test.md) for commands and gates.
- Read [`docs/agents/git-and-release.md`](docs/agents/git-and-release.md) for branch, PR, and
  release operations.
- Read [`docs/agents/harness-guide.md`](docs/agents/harness-guide.md) for five-tool rule discovery,
  evidence layers, and task handoff.
- Read [`docs/agents/security-and-shared-state.md`](docs/agents/security-and-shared-state.md)
  before changing Central, persistence, credentials, or updater behavior.

## Durable Project Rules

- Treat the Windows x64 Tauri bundle as a first-class acceptance surface. A frontend-only build
  does not establish a packaging or release result.
- Route all user-visible text through `src/i18n/` and keep `README.md` / `README_CN.md` aligned
  when public wording changes.
- Keep component state in the domain Zustand stores; components do not call Tauri `invoke()`
  directly. Reuse `UnifiedSkillCard` for skill-card scenarios.
- Put shared Central behavior in the Rust service/repository boundary. Preserve persisted `uid`
  semantics, use the existing Central mutation lock and `ensure_centralized` path, and retain
  target-only skills during Central migration.
- Keep credentials behind `SecretStore`; do not place PATs, API keys, passwords, or private keys
  in SQLite, logs, errors, telemetry, or portable exports.
- Treat generated architecture documents as build outputs: after changing Tauri commands or
  `src-tauri/src/db/schema/`, run `pnpm docs:gen` and include the generated files; keep
  `docs:gen:check` and `docs:build` read-only.
- For Windows updater delivery, preserve the Authenticode -> updater signature -> metadata order
  and keep NSIS, `.sig`, `latest.json`, MSI, and ZIP artifacts consistent. See the release guide.

## Completion Gate

Run the smallest relevant checks while iterating and run `just ci` before declaring repository
work complete. Report skipped checks and unverified external, provider, hardware, or production
evidence explicitly.

Task PRs target `dev` and use squash merge. Promotion to `main` uses a merge commit, after which
`dev` is fast-forwarded to the promotion merge SHA; see the Git and release guide for the full
remote-write and read-back rules.

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
