# Renderer 权限最小化与 capability drift check - Implementation Plan

## Preconditions

- Task remains `planning` until the user approves the final planning summary.
- Run `task.py start` only after that approval.
- Preserve all unrelated dirty files and other `07-24-*` task directories.

## Ordered Implementation

1. **Portable-state domain tests and file adapter**
   - Add typed error variants and bounded read/atomic write helpers under `services/portable_state`.
   - Cover `.json` validation, opened-handle regular-file validation, metadata cap, post-metadata growth cap, UTF-8/JSON errors, overwrite success, persist failure, and temp cleanup.
   - Add path-taking command shells and register/type their IPC contracts.

2. **Portability frontend migration**
   - Add store actions for backend save and file-preview commands.
   - Keep dialog open/save in the component but remove all `plugin-fs` imports and mocks.
   - Preserve paste/edit/pretty/raw behavior, progress state, i18n errors, and import results.

3. **Marketplace preview install migration**
   - Make preview entries discriminated as registry-backed or direct-GitHub.
   - Route registry entries to `install_marketplace_skill`.
   - Route direct GitHub entries through a store action using fresh preview + existing `import_github_repo_skills`; carry `sourcePath`, handle disappeared candidates, and preserve loading/toast/rescan behavior.
   - Remove page-level `fetch`, `mkdir`, and `writeTextFile`.

4. **Remove plaintext secret reveal**
   - Delete reveal command/service exports and tests in both PAT and AI domains.
   - Remove handler registrations, IPC allowlist entries, store actions, view bindings, component props, reveal-only state, Eye UI, and stale i18n strings.
   - Replace tests with fixed-mask/write-only assertions and a contract assertion that reveal command names do not occur in production frontend/backend entrypoints.

5. **Capability and dependency minimization**
   - Remove all fs permissions/scopes and replace `shell:default` with `shell:allow-open`.
   - Remove `@tauri-apps/plugin-fs`, `tauri-plugin-fs`, and `tauri_plugin_fs::init()`; refresh both lockfiles using existing package managers.
   - Verify HTTP(S) external links still work and unsafe schemes remain rejected.

6. **Capability inventory and required check**
   - Rewrite the inventory to match live plugin imports and minimized permissions, including a marker-delimited JSON contract plus a marker-delimited human-readable table block.
   - Add the TypeScript-AST/JSON/Cargo-metadata drift checker, a shared deterministic table renderer for check/update modes, and negative contract tests covering stale JSON and stale prose-table state.
   - Add `pnpm capabilitycheck` to `scripts/run-ci.mjs`; update CI contract expectations only where the shared local/remote gate requires it.

7. **Focused validation**
   - Run the new drift checker and its negative tests.
   - Run portability Rust tests and affected GitHub import/Marketplace Rust tests.
   - Run affected Central portability, Marketplace, settings, store, IPC coverage, and external URL Vitest files.
   - Run `pnpm typecheck` and `pnpm lint`.

8. **Full child gate**
   - Run `just ci` and fix all in-scope failures.
   - Run Windows `pnpm tauri build`.
   - Confirm a newly generated installer exists under `src-tauri/target/release/bundle/nsis/` and record its path, size, and timestamp as task evidence.

9. **Review and finish**
   - Dispatch `trellis-check` for spec, regression, and scope review; resolve all findings and rerun affected checks.
   - Run `trellis-update-spec`; update only specs whose executable contract changed.
   - Inspect the complete diff against this task's acceptance criteria and excluded dirty files.
   - Create one or more Chinese emoji local commits only for this child's code/spec/task artifacts; do not push.
   - Archive only `07-24-renderer-capability-min` and record its code/validation/archive commits in the journal.

## Rollback Points

- After step 2: existing text-based portability commands still exist, so file adapter changes can be reverted without data migration.
- After step 3: registry and GitHub import backend services remain unchanged; frontend routing can be reverted independently.
- After step 4: reveal removal is deliberate; rollback restores the security exposure and therefore requires a new user decision, not an automatic fallback.
- Before commit: generated lockfile changes must be limited to removing `plugin-fs` packages that are no longer reachable.

## Start Review Checklist

- [x] PRD records the approved removal of plaintext reveal.
- [x] Design preserves HTTP(S) external links with `shell:allow-open`.
- [x] Marketplace immutable snapshot work remains assigned to its existing child.
- [x] Both context manifests contain real spec/research entries.
- [x] User explicitly approves this final planning summary in a subsequent message.

## Completion Evidence

- Focused Vitest: 8 files, 235 tests passed; the portability file repeated three times with 16/16 passing each run.
- Portable-state Rust tests: 11 passed.
- `just ci` passed on Windows: frontend 1438 passed / 1 skipped; Rust 926 passed / 5 ignored, plus integration groups 3/4/5 passed; typecheck, lint, capabilitycheck, sizecheck, production build, fmt, and all-targets Clippy passed.
- `pnpm tauri build` passed and produced `src-tauri/target/release/bundle/nsis/SkillPort_0.10.14_x64-setup.exe` (15,137,555 bytes, 2026-07-26 18:09:03 Asia/Shanghai).
- Installer SHA-256: `185762B273CF57ADB57E6565FCD9167060E29B0D1D63C84B0D1A17DD5AC00E5`.
