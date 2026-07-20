# Validation

Date: 2026-07-20

## Regression evidence

- Initial focused Rust tests failed because the 12-tag seed and classifiable built-in
  prompt candidates did not yet exist.
- Initial focused Vitest failed because usage-aware built-in visibility did not yet
  exist.
- `cargo test db::`: 78 passed.
- `cargo test ai_tagging`: 8 passed.
- `pnpm vitest run src/test/centralTags.test.ts src/test/CentralTopFilters.test.tsx`:
  8 passed.
- `pnpm typecheck`: passed.
- `pnpm lint`: passed.

## Final gate

- `just ci`: passed.
  - Web: 128 test files passed, 1402 tests passed, 1 skipped; production build passed.
  - Rust: Clippy with locked all-targets passed; 876 tests passed, 4 ignored;
    CLI and project E2E targets passed.
  - Size budget passed after moving tag seed ownership to `db/seed/skill_tags.rs`.
