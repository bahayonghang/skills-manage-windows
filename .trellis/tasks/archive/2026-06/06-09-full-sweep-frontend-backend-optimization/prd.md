# Full Sweep frontend and backend optimization

## Goal

Run a Brooks Full Sweep across the current repository's frontend (`src/`) and
backend (`src-tauri/`) code, then apply only low-risk optimizations that can be
verified by the existing project gates.

## User Value

- Improve maintainability across the React/TypeScript frontend and Rust/Tauri
  backend without speculative rewrites.
- Preserve the Windows-first build and verification contract for this fork.
- Produce a clear residual report for larger architecture or public-contract
  changes that should not be applied automatically.

## Confirmed Facts

- Scope requested by the user: frontend and backend code.
- Detected code scope: `src/` and `src-tauri/`.
- Files in requested scope: 671 tracked files.
- Test files in requested scope: 123 tracked test files.
- No `.brooks-lint.yaml` is present, so no Brooks config overrides are applied.
- Baseline `just ci` passed before implementation on 2026-06-09.
- `just ci` runs version sync, frontend typecheck/lint/sizecheck/test, and Rust
  clippy/test.
- Frontend stack: React, TypeScript, Tailwind CSS, Zustand, Vitest.
- Backend stack: Rust, Tauri, sqlx/sqlite, Tokio, cargo tests and clippy.
- Project guidance requires at least `just ci` before completion.

## Requirements

- Scan the requested scope in Brooks Full Sweep order:
  1. PR code decay review.
  2. Test quality review.
  3. Tech debt review.
  4. Architecture review.
- Apply only changes that are safe under the Full Sweep contract:
  - single-file local changes that do not alter exported/public contracts;
  - multi-file changes only when existing tests cover the behavior, public
    interfaces remain unchanged, and the pass touches no more than five files.
- Keep high-risk changes as residual items instead of editing them:
  - public API or IPC contract changes;
  - cross-module architecture moves;
  - changes without a reliable test gate;
  - ambiguous product or UX behavior changes.
- Preserve existing project patterns:
  - frontend Tauri calls stay behind stores/services rather than being added in
    components;
  - user-visible text remains in i18n resources;
  - Central Skills card-grid sizing uses `src/lib/centralSkillGrid.ts`;
  - backend GitHub import skip segments remain centralized in
    `src-tauri/src/services/github_import/types.rs`;
  - Windows behavior remains first-class.
- Keep edits surgical and directly tied to detected findings.
- Record applied fixes, residual items, and verification results in the final
  Full Sweep report.

## Acceptance Criteria

- [ ] The sweep report includes dimension summaries for review, test, debt, and
      audit.
- [ ] Every applied change is classified as Safe or Extended-Safe.
- [ ] Public interfaces, IPC command names, database schema behavior, and i18n
      keys are not changed unless a separate explicit approval is obtained.
- [ ] Residual items are listed with Symptom, Source, Consequence, Remedy, and
      the reason they were not applied.
- [ ] Relevant focused tests are run for touched areas when available.
- [ ] `just ci` passes after all applied changes.
- [ ] The task remains local; no commit, push, or amend is performed unless the
      user explicitly requests it.

## Out of Scope

- Broad UI redesign.
- Tauri installer or release pipeline changes.
- Database migrations or persisted data format changes.
- New feature work unrelated to maintainability or verification.
- Deleting unrelated existing code solely because it looks unused.

## Open Questions

- Awaiting the required Brooks Full Sweep pre-flight approval before starting
  autonomous scan and auto-fix execution.
