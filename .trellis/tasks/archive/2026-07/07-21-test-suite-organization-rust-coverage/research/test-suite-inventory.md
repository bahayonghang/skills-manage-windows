# Test Suite Inventory

## Baseline

- `src/test` has 133 files: 127 Vitest files and 6 support/fixture files.
- All 127 Vitest files currently live directly under `src/test`; only the JSON fixture is nested.
- `pnpm exec vitest list --filesOnly` currently discovers all 127 test files.
- `vite.config.ts` uses recursive includes (`src/test/**/*.test.*` and `src/test/**/*.spec.*`), so Vitest itself supports subdirectories.
- `scripts/run-vitest-sequential.mjs` uses a single non-recursive `readdirSync(testDir)` pass. Moving tests into subdirectories without changing this script would make `pnpm test:serial` silently omit them.
- `just ci` runs `pnpm test` and `cargo test --locked` through `scripts/run-ci.mjs`; the hosted `just-ci` job calls the same script.
- `src/test/contracts/ciWorkflowContract.test.ts` protects that shared CI contract and must remain discoverable after the move.

## Frontend Organization Options

### Keep the flat directory

Rejected as the default: 127 test files plus shared helpers already exceed a practical scan surface, and the count continues to grow. Naming prefixes partially group files but do not separate helpers, repository contracts, stores, hooks, components, and page workflows.

### Group only by product domain

Not preferred as the primary rule. It makes feature-wide browsing convenient, but cross-cutting tests such as runtime adapters, typography contracts, shell behavior, shared hooks, and CI scripts do not have a stable single domain. Future contributors would repeatedly decide between domain and technical ownership.

### Mirror primary source ownership

Recommended. Classify a test by the production boundary it primarily exercises:

- `components/<domain>/` mirrors `src/components/<domain>/`.
- `pages/`, `stores/`, `lib/`, `hooks/`, `runtime/`, `fixtures/`, and similar folders mirror `src/` ownership.
- `contracts/` holds repository/static contracts without a single production module, including CI, typography, font, theme-contrast, and locale completeness checks.
- `scripts/` holds repository script tests.
- `support/` holds `setup.ts`, `ipcMock.ts`, `testPlatform.ts`, reusable view harnesses, and test-only fixtures; feature-specific harnesses may stay beside their owning page tests when only that group consumes them.

This rule is deterministic, aligns navigation with the production tree, and avoids inventing a second domain taxonomy.

## Discovery And Path Risks

1. Update `scripts/run-vitest-sequential.mjs` to recursively collect test files with normalized repository-relative paths and deterministic sorting.
2. Add a focused discovery regression test for nested files rather than relying only on the final full suite.
3. Update `vite.config.ts` if `setup.ts` moves under `support/`.
4. Repair relative imports and mocks. Most existing tests import production files through `../`; a directory move changes their depth. Prefer the existing `@/*` alias for `src` imports where it reduces path fragility, while repository-root files may continue to use explicit relative paths or `process.cwd()`.
5. Update live command examples and source comments that point to moved test paths. Historical plan documents do not need mass rewrites.
6. Compare Vitest's discovered file list before and after the move; the post-change count must equal the moved baseline plus genuinely added tests.

## Rust Coverage Boundary

- `src-tauri/src` currently contains 914 unit-test markers across module-local test suites.
- `src-tauri/tests` contains one integration crate, `projects_e2e.rs`, with 5 tests.
- Adding broad integration duplicates for heavily tested internal services would add cost without proving a new boundary.
- `services::local_archive_import` and `services::portable_state` expose their orchestration functions as `pub(crate)`; expanding production visibility only to place tests under `tests/` would weaken encapsulation.
- `cli_api::CliContext` is intentionally public and used across the library/binary boundary. It currently has four module tests, but no external-crate contract suite.

Recommended Rust scope:

- Add `src-tauri/tests/cli_api_e2e.rs` using only public `skillport_lib` APIs.
- Cover list/show identity, dry-run sync planning, ambiguous references, duplicate references, invalid selection/method errors, and stable error codes without network or keyring access.
- Move any exactly duplicated happy-path assertion out of `cli_api` module tests instead of keeping two copies.
- With a second integration crate, introduce `src-tauri/tests/common/mod.rs` for the database/skill fixtures used by both suites; keep the no-op `SecretStore` local to `cli_api_e2e.rs` so unrelated integration crates do not compile unused helpers. Update the backend test-support spec to record this integration-crate harness boundary.

## Required Validation

- Focused recursive discovery test.
- `pnpm exec vitest list --filesOnly` inventory comparison.
- Focused Vitest groups after each move batch.
- `pnpm test:serial -- <nested-test-path>` smoke validation.
- `cargo test --manifest-path src-tauri/Cargo.toml --test cli_api_e2e --locked`.
- `cargo test --manifest-path src-tauri/Cargo.toml --test projects_e2e --locked` after shared fixture extraction.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`.
- Final `just ci`.
