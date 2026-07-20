# Validation

## Focused checks

- `cargo test db:: --locked`: passed, 85 tests.
  - Covered legacy proposal-column migration, proposal round-trip without tag creation,
    orphan filtering, concurrent same-name creation, derived-id collision fallback,
    same-name multi-skill acceptance, and skip without tag/link residue.
- `cargo test ai_tagging --locked`: passed, 11 tests.
  - Covered response compatibility, proposal resolution/collision downgrade, review-only
    persistence, review counts, and absence of the `uncategorized` fallback link.
- `pnpm vitest run src/test/CentralSkillsView.categorize.test.tsx`: passed,
  11 tests.
- `pnpm typecheck`: passed.
- `pnpm lint`: passed.
- `cargo fmt --all -- --check`: passed.

## Full gate

- `just ci`: passed.
  - Web: 128 test files passed, 1403 tests passed, 1 skipped; production build passed.
  - Rust: locked all-targets Clippy passed with warnings denied; 886 tests passed,
    4 ignored; CLI and project integration suites passed.
