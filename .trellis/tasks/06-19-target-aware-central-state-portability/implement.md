# Implementation Plan

## Checklist

1. Add target metadata types to portable state manifest.
   - Extend `ExportedFrom` with optional `target`.
   - Keep old JSON deserializable.
   - Verify with a parse test for old v1 JSON and an export test for target metadata.

2. Route portable state commands through active target and active DB.
   - In `commands/portable_state.rs`, fetch `active_target`, `active_db`, and `target_context_from_active_target`.
   - Use active DB for export, preview, and import.
   - Replace `local_target_context()` in operation logs.
   - Verify with command-layer tests if available, or focused service tests plus existing command patterns.

3. Add remote-aware portable import execution.
   - Reuse `ensure_github_sources`, `build_import_groups`, and `restore_skill_tags`.
   - Local path remains existing behavior.
   - Remote path calls `github_import::import_github_repo_skills_remote_with_auth`.
   - Preserve cancellation result semantics and progress events.
   - Verify with Rust tests that remote branch delegates to remote import or with an extracted import executor seam if direct SSH/WSL integration is too heavy for unit tests.

4. Refresh active-target data after import.
   - Existing frontend already reloads `get_central_skills`, repositories, tags, and update states after import.
   - Confirm those commands are active-target aware; if not, route only the missing ones through active DB as a prerequisite fix.
   - Verify by code inspection and targeted tests.

5. Make the portability dialog target-aware.
   - Surface active target label/kind in `CentralStatePortabilityDialog`.
   - Pass active target from `useTargetStore` or Central view bindings.
   - Add optional origin-target warning after parsing import JSON if metadata differs.
   - Update `en.json` and `zh.json`.
   - Verify with targeted Vitest coverage.

6. Update and add tests.
   - Rust:
     - old v1 manifest remains valid
     - export includes origin target metadata when provided
     - active DB routing/remote import branch is covered
     - existing portable state tests still pass
   - Frontend:
     - dialog displays Local target
     - dialog displays SSH/WSL target labels from store fixture
     - optional mismatch warning appears only when JSON origin target differs

7. Run validation.
   - `cargo test --manifest-path src-tauri/Cargo.toml portable_state`
   - targeted frontend test file(s)
   - `pnpm typecheck`
   - `pnpm lint`
   - `just ci`

## Likely Files

- `src-tauri/src/commands/portable_state.rs`
- `src-tauri/src/services/portable_state/types.rs`
- `src-tauri/src/services/portable_state/export.rs`
- `src-tauri/src/services/portable_state/import.rs`
- `src-tauri/src/services/portable_state/tests.rs`
- `src/components/central/CentralStatePortabilityDialog.tsx`
- `src/components/central/CentralSkillDialogs.tsx`
- `src/pages/CentralSkillsView.tsx`
- `src/pages/centralSkillsViewModel.ts`
- `src/stores/centralSkillsStore.updateSlice.ts`
- `src/stores/targetStore.ts`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`
- relevant `src/test/*.test.tsx`

## Review Gates

- Before coding, load `trellis-before-dev` and relevant backend/frontend specs.
- Do not start implementation until this task is started with `task.py start` after planning review.
- Keep code changes surgical: target routing and UI target disclosure only.

## Validation Notes

`just ci` is required by repo instructions before completion. If it fails for a pre-existing environment issue, record the exact failing command and error in this task before stopping.

## Rollback Points

- If remote import cannot be cleanly unit-tested without large mocking, extract a small executor interface inside portable state import and test branch selection there.
- If optional target metadata causes unexpected serialization compatibility issues, keep UI target disclosure and active target routing, but defer JSON origin metadata behind a separate follow-up.
