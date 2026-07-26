# Renderer capability live boundary research

## Scope

Read-only live verification for `07-24-renderer-capability-min` on `dev`, 2026-07-26. This note records repository-answerable facts before the product decision on plaintext secret reveal.

## Confirmed authority surface

| Surface | Live evidence | Required boundary |
|---|---|---|
| Main WebView capability | `src-tauri/capabilities/default.json:10-27` grants `shell:default`, three fs commands, `$HOME/**`, and three common user-directory globs | Remove shell and all fs permissions/scopes from the main capability |
| Portability export | `CentralStatePortabilityDialog.tsx:184-200` selects a path and writes JSON in the renderer | Keep the dialog path selection in the renderer; pass the selected local path to a backend command that writes atomically |
| Portability import | `CentralStatePortabilityDialog.tsx:202-219` selects and reads a file in the renderer before preview | Backend validates a regular `.json` file, rejects oversize input before allocation, reads it through the shared blocking-FS seam, then parses/previews |
| Marketplace repo preview install | `MarketplaceView.tsx:205-227` fetches content and writes `~/.skillsmanage/skills/<name>/SKILL.md` directly | Store action calls a backend service; backend owns network policy, skill-id/path validation, Central mutation coordination, write/replace behavior, and DB refresh semantics |
| External URL open | `externalUrl.ts:1,14-18` imports `plugin-shell` and permits parsed HTTP(S) URLs | Replace `shell:default` with `shell:allow-open`; retain the existing URL parser and shell plugin |
| Plaintext secret reveal | `lib.rs:474,478` registers both reveal commands; settings stores invoke them directly | User approved complete removal on 2026-07-26 |
| Documentation drift | `ipc-capability-inventory.md:19,61-65` incorrectly says there is no shell frontend import and that `shell:default` is absent | A required local/CI check must fail when inventory, plugin imports, dependencies, or capability permissions disagree |

## Existing seams to reuse

- `commands/portable_state.rs` already owns export, preview, import progress, target snapshot resolution, and operation-log behavior. New path-taking commands should delegate into the same domain rather than duplicate manifest logic.
- `services/portable_state::PortableStateError` is the owning domain error. File validation and atomic-write errors belong there; commands stringify only at IPC.
- `fs_util::run_blocking_fs_with` is the only allowed wrapper for new blocking file work. The blocking closure must not own `AppHandle`.
- `commands/marketplace.rs` and `services/marketplace` already own Marketplace installation. The repo-preview path must enter this boundary through a store action instead of page-level `fetch` and `plugin-fs` calls.
- `services/github_import` owns GitHub URL/network policy and Central import staging. The later `github-preview-snapshot` task owns immutable SHA binding; this task must not implement a competing snapshot protocol.
- Local final Central mutations must use the existing `central_mutation` contract at the top-level service boundary and must not acquire the same lock twice.
- Frontend IPC calls belong in stores via `@/lib/ipc`; components/pages do not invoke commands directly.

## Planned validation surface

- Backend unit tests: extension, non-file, missing file, oversize metadata, oversize streamed/read content, invalid JSON, atomic export replacement, and temp-file cleanup on failure.
- Marketplace service tests: invalid skill identity/path rejection, allowlisted download behavior, Local write through the shared Central path, and no DB installed marker on failed write.
- Frontend tests: portability path commands, cancellation/error feedback, Marketplace preview install store routing, and removal of reveal interaction if that option is approved.
- Contract check: parse `default.json`, inspect TypeScript imports with the TypeScript compiler API, verify dependency/runtime registration state, compare the machine-owned inventory JSON contract, and deterministically verify the human-readable table rendered from that contract. Add it to `scripts/run-ci.mjs` so `just ci` and GitHub Actions share the same gate.
- Full child gate: focused Vitest/Rust tests, `pnpm typecheck`, `pnpm lint`, and `just ci`. Because this child changes Tauri capability/plugin configuration, also run Windows `pnpm tauri build` and verify the configured NSIS bundle exists.

## Scope boundaries

- Do not create per-command Tauri permissions for every custom command in this child.
- Do not change CSP beyond removing renderer-side Marketplace download justification unless live implementation proves a directly coupled minimum change.
- Do not implement immutable GitHub preview tokens; preserve that boundary for `07-24-github-preview-snapshot`.
- Do not introduce OS authentication; the user chose to remove plaintext reveal.

## Product decision

Viewing an already-saved GitHub PAT or AI API key in plaintext is no longer a supported product capability. The user approved removal on 2026-07-26.
