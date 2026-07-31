# Implementation plan: GitHub import manual branch selection

## 1. Shared branch resolution

- [x] Add the optional branch request/helper in `services/github_import` and
      preserve URL-only wrappers for CLI/current internal callers.
- [x] Validate and reconcile URL/manual branches before GitHub acquisition;
      add typed invalid/conflict errors and IPC codes without logging source
      secrets.
- [x] Pass the chosen branch through Local and SSH/WSL preview entry points and
      snapshot binding; leave commit pinning, retained bytes, mutation locking,
      provenance writes, and Central update behavior unchanged.
- [x] Add focused Rust tests for default, `dev`, same/mismatched tree URL,
      invalid single-segment input, snapshot mismatch, and persistence identity.

## 2. Typed frontend data flow

- [x] Extend the typed IPC command map and marketplace store action/state with
      the optional branch and preview-associated branch identity.
- [x] Extend the shared import-intent store/bindings with `githubBranch`, dirty
      detection, and reset rules. Clear it when opening/consuming a deep-link
      source so URL-encoded branch intent cannot inherit stale manual state.
- [x] Thread controlled branch props through Central and Marketplace workflows,
      preview, re-preview, and confirm without adding direct component `invoke()`
      calls or constructing GitHub URLs in TypeScript.
- [x] Update store, IPC coverage, immutable-snapshot contract, and import-intent
      tests; keep render-scoped mocked Zustand state stable.

## 3. Wizard UI and i18n

- [x] Add the compact optional branch input to the input step with `GitBranch`,
      responsive sizing, accessible label, and blank-default hint.
- [x] Add English/Chinese UI strings and branch-specific backend error strings.
- [x] Extend wizard and Central/Marketplace integration tests for blank default,
      `dev`, URL/manual match, conflict feedback, reset, and re-preview behavior.

## 4. Focused validation

- [x] `pnpm exec vitest run src/test/components/marketplace/GitHubRepoImportWizard.test.tsx src/test/components/import/ImportIntentController.test.tsx src/test/stores/marketplaceStore.test.ts src/test/contracts/ipcCommandCoverage.test.ts src/test/contracts/githubPreviewSnapshotContract.test.ts`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml github_import --lib --locked`
- [x] `pnpm typecheck`
- [x] `pnpm lint`
- [x] `git diff --check`

## 5. Full gate and review

- [x] Run `just ci` and fix all in-scope failures before completion.
- [x] Inspect the final diff for URL-only CLI compatibility, Local/SSH/WSL
      parity, immutable preview behavior, i18n completeness, and unrelated
      Trellis/worktree changes.
- [ ] Manually smoke the Tauri wizard with one default-branch repository and one
      repository containing `dev`; verify the toolbar/result branch and confirm
      no write occurs before confirmation.
- [x] Run the Trellis quality review, update the relevant executable spec if the
      final contract changed, then stop for the repository's commit/closeout
      workflow. Do not push unless separately requested.

GUI smoke remains unverified: the Computer Use native pipe was unavailable on
this Windows host, so the default-branch and `dev` Tauri wizard flows were not
manually exercised. Automated component/store/IPC/Rust coverage and `just ci`
passed; this note must not be treated as a manual GUI pass.

## Risk and rollback points

- Resolver/IPC changes are the primary rollback point. If optional branch data
  cannot be carried without weakening snapshot binding, revert the new field and
  keep URL-only selection rather than adding a renderer URL workaround.
- Do not broaden validation to slash-containing refs inside this task; that
  changes every API/raw/archive URL builder and the SSRF endpoint policy.
- Do not add branch-list fetches, pagination, PAT UI, database migration, or new
  dependencies while implementing this plan.
