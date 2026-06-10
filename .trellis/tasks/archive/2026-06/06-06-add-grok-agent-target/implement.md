# Add Grok agent target implementation plan

## Checklist

1. Respect the approved scope decision: Grok is an independent upstream-compatible target, not Universal.
2. Backend seed:
   - Add `grok` to default enabled platform IDs if approved.
   - Add Grok `Agent` in `src-tauri/src/db/seed.rs` after Codex.
   - Leave Universal constants in `src-tauri/src/db/types.rs` unchanged.
3. Backend tests:
   - Extend built-in agent seed tests to assert Grok local and remote paths.
   - Add or extend project install test to prove Grok writes to `.grok/skills`.
4. Frontend platform model:
   - Add Grok to `DEFAULT_ENABLED_PLATFORM_IDS` / sorting in `src/lib/platformVisibility.ts`.
   - Do not add Grok to `src/lib/platformTargetGroups.ts` Universal arrays; rely on standalone visibility.
   - Add `grok` path hint override only if the existing path formatting is insufficient.
5. Frontend icon:
   - Add Grok mapping in `PlatformIcon.tsx`.
   - Extend `PlatformIcon.test.tsx` coverage.
6. Docs:
   - Update `README.md` and `README_CN.md` to list Grok under Coding with `~/.grok/skills`.
   - Mention `.grok/skills` project behavior where the docs already describe project paths.
7. Validation:
   - Run focused Rust tests for database seed and project installation.
   - Run focused Vitest tests for platform visibility / target grouping / icon behavior.
   - Run `pnpm typecheck && pnpm lint`.
   - Run `just ci`.

## Risk Points

- Accidentally adding Grok to Universal constants would change both global and project install semantics.
- Backend and frontend default-enabled lists can drift; update both or neither according to the scope decision.
- Existing databases rely on startup seed upsert; avoid one-off migrations unless tests prove seeding is insufficient.
- Icon imports can increase bundle size or fail if the package does not export a Grok icon. Prefer a tested existing export or a compact local SVG branch.

## Rollback

Revert the Grok seed entry, frontend visibility/icon mappings, docs changes, and focused tests. No data migration is planned, so rollback should not require database schema changes.
