# Implementation Plan

## Checklist

1. Backend filter helper
   - Add a narrow helper in the GitHub import source/discovery layer for generic candidate rejection.
   - Apply it after skill id normalization so `skill_id == "skill"` is filtered consistently.
   - Keep root repository `SKILL.md` behavior intact.

2. Update Center inventory behavior
   - Ensure `collect_remote_added_skills` does not persist pending additions for filtered generic candidates.
   - Ensure quiet-filtered candidates do not become noisy failed repository entries.
   - Check apply/force mirror paths so stale or crafted selections cannot import the filtered candidate.

3. Frontend metadata styles
   - Update `SourceMeta` with per-row style variants for repository/path/url/hash/cache.
   - Preserve wrapping, title tooltips, labels, and shared use by Updatable, Added, and Removed tabs.
   - Add minimal test hooks only if needed for robust RTL assertions.

4. Tests
   - Update or replace the existing `recursive_fallback_skips_large_generated_directories` expectation that currently accepts `skill_id == "skill"`.
   - Add Rust coverage for `agent_reach/skill/SKILL.md` / `packages/example/skill/SKILL.md` being filtered from remote additions and pending additions.
   - Add or extend React coverage in `src/test/UpdateCenterSourceMeta.test.tsx` for differentiated source metadata row styling.

5. Verification
   - Run focused Rust tests for `github_import` and `skill_update_inventory`.
   - Run focused Vitest tests for Update Center source metadata.
   - Run frontend `pnpm typecheck` and `pnpm lint`.
   - Run final `just ci`.

## Likely Files

- `src-tauri/src/services/github_import/source.rs`
- `src-tauri/src/services/github_import/types.rs`
- `src-tauri/src/services/github_import/tests.rs`
- `src-tauri/src/commands/central_updates/repository_sync.rs`
- `src-tauri/src/commands/central_updates/tests.rs`
- `src-tauri/src/commands/skill_update_inventory/tests.rs`
- `src/components/central/updateCenter/SourceMeta.tsx`
- `src/test/UpdateCenterSourceMeta.test.tsx`

## Validation Commands

```powershell
pnpm test -- src/test/UpdateCenterSourceMeta.test.tsx
cd src-tauri; cargo test github_import --lib
cd src-tauri; cargo test skill_update_inventory --lib
pnpm typecheck
pnpm lint
just ci
```

## Review Gates Before Start

- Confirm this task should stay exact-id based: block only normalized `skill_id == "skill"`, not every path segment named `skill`.
- Confirm no automatic deletion of an already-existing Central `skill` row is required.
