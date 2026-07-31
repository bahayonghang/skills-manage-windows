# GitHub import branch selection

## Goal

Allow users to explicitly import skills from a non-default GitHub branch, such
as `dev`, without requiring them to construct a GitHub `/tree/<branch>` URL.
Leaving the branch input empty must preserve the current behavior of resolving
the repository's default branch.

## Background

- The input step currently exposes only the repository URL and preview action
  (`src/components/marketplace/GitHubRepoImportWizardChrome.tsx:191-242`).
- The shared Rust source resolver already accepts a single-segment branch in a
  GitHub tree URL and otherwise reads `default_branch`
  (`src-tauri/src/services/github_import/source.rs:4-67,70-155`).
- `GitHubRepoRef.branch` already crosses the Rust and TypeScript preview/result
  contracts, is shown in the preview toolbar, is persisted as repository
  provenance, and is used by Central updates
  (`src/types/githubImport.ts:1-6,43-51` and
  `src/components/marketplace/GitHubRepoImportWizardChrome.tsx:310-321`).
- Confirmed import consumes an immutable preview snapshot rather than
  re-resolving a moving branch. Snapshot binding already rejects a different
  branch supplied through a tree URL
  (`src-tauri/src/services/github_import/snapshot.rs:268-293` and
  `src-tauri/src/services/github_import/tests.rs:2393-2400`).
- The branch validator rejects `/` and `\\`; `dev` is supported, while
  slash-containing refs require a separate security and URL-encoding design
  (`src-tauri/src/services/github_import/raw_http.rs:166-216`).
- The frontend store and typed IPC currently accept only `repoUrl` for preview
  (`src/stores/marketplaceStore.githubImportSlice.ts:49-120` and
  `src/lib/ipc/commandMap.ts:137`).

## Requirements

### R1. Manual branch input

- Add a localized optional branch input to the GitHub import input step.
- The input is manual, not a static choice list and not a live GitHub branch
  browser. Empty or whitespace-only input means "use the repository default".
- A non-empty value is trimmed on submission and supports the existing safe,
  single-segment branch contract, including `dev`.

### R2. One authoritative source identity

- Carry the optional explicit branch as structured data through the controlled
  wizard state, marketplace store, typed IPC, Tauri command, and shared Rust
  GitHub source resolver. React code must not construct `/tree/<branch>` URLs or
  duplicate the Rust parser.
- If the URL already includes `/tree/<branch>` and the separate branch input is
  empty, keep using the URL branch. If both specify the same branch, accept it.
  If they differ, reject the request with a localized, actionable conflict
  error; neither value may silently override the other.
- Resolve and validate the selected branch in the shared Rust service layer so
  Local, SSH, WSL, desktop, and shared CLI paths retain one branch contract.

### R3. Preview and import lifecycle

- Preview, re-preview, Markdown reads, confirmation, persisted per-skill
  provenance, import results, and later Central updates must identify the same
  resolved branch.
- Preserve the immutable preview-token contract: confirmation imports only the
  bytes retained by the preview, never a newly resolved branch tip.
- Changing the repository URL or explicit branch cannot make the previous
  preview confirmable as the new source. A new preview replaces/discards the
  old snapshot before confirmation.
- Closing/resetting the wizard, changing targets, starting another import, and
  consuming a queued deep-link intent must clear the manual branch state at the
  same lifecycle boundary as the existing GitHub import session. A deep-link
  tree URL continues to carry its own branch and must not inherit an earlier
  manual branch value.

### R4. Compatibility and errors

- Preserve full GitHub URLs, shorthand repository inputs, optional source
  subpaths, existing `/tree/<branch>/<subpath>` URLs, local/SSH/WSL targets, PAT
  handling, mirror fallback, resource budgets, typed IPC coverage, and CLI
  behavior.
- Invalid, conflicting, missing, or inaccessible branches fail before Central
  filesystem or database mutation and produce localized, actionable feedback.
- All new labels, placeholders, hints, and coded backend errors are added to
  both English and Chinese resources.

### R5. Verification

- Add focused Rust tests for explicit/default/conflicting/invalid branch
  resolution and snapshot binding.
- Add focused store/IPC, import-intent lifecycle, wizard interaction, and i18n
  coverage without destabilizing existing preview object identity.
- Run focused checks first, then the required repository-wide `just ci` gate.

## Acceptance Criteria

- [x] AC1 / R1: `https://github.com/owner/repo` plus an empty branch input uses
      GitHub's reported default branch exactly as today.
- [x] AC2 / R1-R3: the same URL plus `dev` previews `dev`; confirmation imports
      the retained `dev` bytes; preview toolbar, result, stored provenance, and
      subsequent update identity all retain `dev`.
- [x] AC3 / R2: URL `/tree/dev/...` plus empty or `dev` branch input succeeds;
      the same URL plus `main` fails with a localized conflict and performs no
      Central write.
- [x] AC4 / R2-R4: whitespace-only, slash-containing, or otherwise invalid
      manual branch input fails through the shared validation boundary without
      issuing branch acquisition or performing a Central write.
- [x] AC5 / R3: changing URL/branch, re-previewing, closing/resetting, changing
      target, starting another import, or consuming a queued deep link cannot
      reuse or inherit the wrong manual branch or stale preview snapshot.
- [x] AC6 / R4: existing tree URLs with source subpaths and Local, SSH, WSL,
      PAT, mirror, resource-budget, CLI, and Central update behavior regressions
      are covered or proven unchanged by the relevant focused tests.
- [x] AC7 / R4: every new user-visible string and branch-specific coded error
      renders in English and Chinese without leaking credentials or snapshot
      details.
- [x] AC8 / R5: `pnpm typecheck`, `pnpm lint`, focused Vitest and Rust tests,
      `git diff --check`, and `just ci` pass before completion.

## Out of Scope

- Fetching or searching the repository's branch list.
- Static hard-coded branch choices such as only `main`, `master`, and `dev`.
- Supporting slash-containing branch names such as `feature/foo`.
- Importing tags or arbitrary commit SHAs through the new field.
- Changing GitHub skill discovery, duplicate resolution, installation, update,
  or persistence semantics beyond carrying the chosen branch consistently.
- Adding production dependencies, database migrations, or packaging changes.
