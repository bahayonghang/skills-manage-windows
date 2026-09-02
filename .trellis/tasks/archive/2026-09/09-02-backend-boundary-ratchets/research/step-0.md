# Step 0 boundary ratchets

Measured 2026-09-03 from a dirty-tree scan of `src-tauri/src` before product-code edits.
Do **not** freeze the engineering-audit file count of 87; that counted any `crate::db` importer, including types.

## Scan roots

- Services → commands: `src-tauri/src/services`
- Wide repository function calls: `src-tauri/src`
- Function owners: `src-tauri/src/db/repos/*_repo.rs`

## Exclusions

- Test-file allowlist: directory segment `tests`, basename `tests.rs`, suffix `_tests.rs`, prefix `test_`
- Comment and string-literal text is stripped before matching
- `#[cfg(test)]` inline modules in production files are **not** stripped
- Facade/owner files are excluded from the wide-fn scan: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/repos/**`
- Unqualified `db::<repo_fn>` counts even without a local `use crate::db::{self}`, because child modules inherit the alias via `use super::*`

## Match lists

- `step-0-scan-meta.json`
- `step-0-services-commands.json` — **6** hits (the known HTTP identity call sites)
- `step-0-wide-repo-fns-migration-trees.json` — **174** hits in the three migration trees
- `step-0-wide-repo-fns-historical.json` — **388** hits elsewhere (debt that must not grow)

Live recalculation lives in `src/test/contracts/rustBoundaryContract.test.ts`.
