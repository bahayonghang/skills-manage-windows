# Implementation Plan

## Checklist

1. Establish red regression tests
   - Add a `kill-ai-slop`-shaped snapshot containing `skill/SKILL.md` plus unrelated repository files.
   - Assert the current builder fails to return the expected candidate before changing production code.
   - Add root URL/effective `Some("skill")` assertions for identical `source_path`, ID, name, description, and copy boundary metadata.
   - Add frontend classifier or real-wizard tests proving `subpaths` currently renders the PAT hint while real denial messages do render it.

2. Fix repository-level candidate identity
   - Extract or reuse the existing repository skill-ID normalization from the root `SKILL.md` branch.
   - Apply repository identity when `manifest.source_path` is `.` or exactly `skill`.
   - Keep `source_path`, `root_directory`, `skill_directory_name`, download URL, plugin metadata, and frontmatter display fields unchanged.
   - Leave the generic-candidate filter in place for deeper basename-`skill` paths.

3. Lock down backend compatibility
   - Assert `skill/SKILL.md` returns `skillId=kill-ai-slop` and is not filtered.
   - Assert `build_repo_skill_candidates_from_snapshot_at_path(..., Some("skill"))` returns the same candidate.
   - Keep `agent_reach/skill` and `packages/example/skill` filtering tests green.
   - Verify root `SKILL.md`, named nested skills, manifest grouping, and partial-import crafted-selection behavior remain unchanged.
   - Add an import-level assertion if candidate-only tests do not prove that only the `skill/` subtree is written and recorded.

4. Fix PAT guidance classification
   - Replace the broad regex in `looksLikeGitHubAuthGuidance` with explicit backend denial signals.
   - Preserve `looksLikeConfiguredGitHubTokenFailure` behavior.
   - Remove any duplicated stale regex from marketplace test support by reusing the production helper when applicable.
   - Do not add or modify i18n text unless implementation reveals a missing user-facing state.

5. Add frontend regressions
   - Negative: the exact `NoImportableSkills` message containing `subpaths` shows no PAT guidance.
   - Positive: a rate-limit denial still shows the generic PAT settings hint.
   - Positive: a configured-token denial shows the configured-token hint.
   - Negative: a non-auth GitHub URL/path validation error does not show PAT guidance.

6. Update durable contract
   - Add the repository-level singular `skill/` identity rule to `.trellis/spec/backend/github-import-preview-contract.md`.
   - Add the auth-guidance positive/negative matrix and required regression tests.
   - Confirm plugin grouping remains preview-only and additive.

7. Validation and review
   - Run formatting/check-only gates for changed files.
   - Run targeted Rust GitHub import tests.
   - Run targeted real-wizard and marketplace GitHub tests.
   - Run `git diff --check` and full `just ci`.
   - Inspect the final diff for unintended DTO, database, source metadata, or i18n changes.

## Likely Files

- `src-tauri/src/services/github_import/source.rs`
- `src-tauri/src/services/github_import/tests.rs`
- `src/components/marketplace/githubImportWizardUtils.ts`
- `src/test/GitHubRepoImportWizard.test.tsx`
- `src/test/marketplaceViewTestSupport.tsx` (only if needed to remove classifier duplication)
- `.trellis/spec/backend/github-import-preview-contract.md`

## Watch-Only Files

- `src-tauri/src/services/github_import/import.rs`
- `src-tauri/src/services/github_import/preview.rs`
- `src-tauri/src/services/github_import/types.rs`
- `src-tauri/src/services/github_import/plugin_manifest.rs`
- `src/components/marketplace/GitHubRepoImportWizardChrome.tsx`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

## Review Gates

- The exact target repository shape is red before the backend fix and green after it.
- Top-level `skill/` imports as the repository identity, never as generic ID `skill`.
- `sourcePath=skill` remains the selection, copy, and update metadata boundary.
- Deep generic `.../skill/` candidates remain filtered.
- Local and SSH/WSL candidate construction share the same identity rule.
- The exact `subpaths` message does not show PAT guidance.
- Backend rate-limit and access-denial messages still show the intended localized guidance.
- No new schema, DTO field, persisted metadata, or user-visible text is introduced.

## Validation Commands

```powershell
cd src-tauri; cargo test services::github_import::tests::tests
pnpm test -- GitHubRepoImportWizard MarketplaceView.github-ssh-and-result
git diff --check
just ci
```

## Rollback Points

- After backend tests: revert the narrow repository-level ID branch if existing generic-filter or import-boundary tests regress.
- After frontend tests: revert the classifier expression independently if real denial messages lose guidance.
- Before `just ci`: confirm the task still requires no database migration or i18n update.
