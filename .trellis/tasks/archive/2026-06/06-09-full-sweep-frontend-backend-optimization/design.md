# Technical Design

## Scope and Boundaries

The implementation target is the existing single-repo app:

- Frontend: `src/`
- Backend: `src-tauri/`

Generated build output, dependencies, Tauri targets, and unrelated repository
metadata are outside the scan scope. Planning files under this task directory
are task artifacts, not product code.

## Sweep Model

The run follows the Brooks Full Sweep pipeline:

1. Review pass: local code decay findings.
2. Test pass: test quality and missing-coverage findings.
3. Debt pass: repeated or accumulated maintainability issues.
4. Audit pass: module and boundary-level findings.

Each pass may apply only Safe or Extended-Safe fixes. A fix becomes residual if
it changes public contracts, crosses unclear ownership boundaries, lacks a
reliable gate, or would require product judgment.

## Verification Boundary

The baseline verification gate is `just ci`, which already passed before
implementation. For applied fixes, verification is layered:

- Run focused Vitest or cargo tests for touched modules when a focused command is
  obvious.
- Run frontend-only gates for frontend-only changes:
  `pnpm typecheck`, `pnpm lint`, and targeted `pnpm test` where applicable.
- Run Rust-focused tests/clippy for backend changes:
  `cargo test --manifest-path src-tauri/Cargo.toml <filter>` when focused, then
  `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`.
- Finish with `just ci`.

If a change causes a regression, revert the smallest change set from that pass
and record the finding as residual.

## Frontend Contracts

- Components should not introduce direct Tauri `invoke()` calls.
- Shared Central Skills card sizing remains centralized in
  `src/lib/centralSkillGrid.ts`.
- User-visible strings stay in `src/i18n/locales/en.json` and
  `src/i18n/locales/zh.json`.
- Zustand stores remain the frontend boundary for persistent/server-like state.
- UI optimizations should avoid layout or behavior changes unless covered by
  existing tests.

## Backend Contracts

- Commands, services, database helpers, and target abstractions keep their
  current module boundaries unless a residual item recommends a larger change.
- GitHub import skip segment behavior remains centralized in
  `src-tauri/src/services/github_import/types.rs`.
- Centralized skill directory and installation behavior must continue to respect
  existing linker/centralization semantics.
- Windows path handling and PowerShell-oriented workflows remain first-class.

## Rollback Strategy

- Use git diff to isolate each applied fix.
- If focused or aggregate verification fails, revert only the fix from the
  failing dimension before continuing.
- Do not use destructive git reset or checkout operations.
- Keep residuals for high-risk or repeatedly failing findings instead of forcing
  a broad refactor.

## Report Shape

The final report uses mode `Full Sweep` and includes:

- dimension summary;
- iteration history;
- fix log;
- health-score estimate;
- residual items;
- final verification commands and results.
