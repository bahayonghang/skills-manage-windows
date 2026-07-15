# Implementation Plan

## Success Criteria

Root `sourcePath = "."` imports and updates the complete repository snapshot, nested source paths remain subtree-scoped, and an already truncated root skill becomes detectable and repairable through the existing update workflow.

## Ordered Steps

1. Add red tests for the source-path boundary.
   - Add a root snapshot fixture with `SKILL.md`, top-level metadata and nested `references/`, `scripts/`, and `assets/` files.
   - Prove the current import collector drops nested root files.
   - Add the neighboring `skills/agent-browser` control proving unrelated repository paths stay excluded.
   - Add a Central update collector/hash test for `source_path = "."`.

2. Introduce one shared path mapping helper.
   - Place the helper with GitHub repository source-path semantics in `src-tauri/src/services/github_import/source.rs` or the nearest existing source-path module.
   - Re-export it crate-locally from `github_import/mod.rs` for Central updates.
   - Map root `.` to the unchanged repository path; map nested sources by stripping the exact normalized prefix.
   - Do not add repository-specific branches, ignore patterns or a new snapshot abstraction.

3. Fix initial snapshot imports.
   - Replace the inline branch in `github_import/progress.rs::collect_snapshot_source_files` with the shared helper.
   - Preserve deterministic sorting, empty-source errors, byte totals, progress events and write-time safety checks.
   - Add an end-to-end import test that asserts nested files in the target directory and `source_path = "."` in repository membership.
   - Keep rename/overwrite rollback tests green and add nested-file assertions where they improve coverage without duplicating the full fixture.

4. Fix Central update collection and hashing.
   - Replace the duplicate branch in `central_updates/fs.rs::collect_remote_skill_files` with the shared helper.
   - Prove root descendant files affect `remote_hash` and are written by the existing atomic local path.
   - Add an inventory/apply regression for a Central root skill whose top-level files match but nested resources are missing.
   - Assert the repaired directory includes remote descendants, removes a stale local file, retains repository assignment `.`, and refreshes managed copies when enabled.

5. Verify transport parity and compatibility controls.
   - Keep the SSH/WSL direct import `cp -a` behavior unchanged.
   - Assert `remote_skill_source_dir(repo, ".")` remains the repository root.
   - Keep nested `agent-browser`, `skill/` container, named multi-skill, plugin grouping and deep generic-filter tests green.
   - Confirm no database, IPC, frontend or i18n diff is introduced.

6. Update the durable contract.
   - Extend `.trellis/spec/backend/github-import-preview-contract.md` with a root repository content-boundary scenario.
   - Record that root descendants participate in import, hash and update, while nested candidates remain subtree-scoped.
   - Record the required root-vs-nested regression pair and existing-install repair behavior.

7. Run validation and inspect the final diff.
   - Run formatting and the smallest targeted Rust tests after each production slice.
   - Run the full affected GitHub import and Central update suites.
   - Run `git diff --check` and final `just ci`.
   - Confirm only the planned backend, tests, task artifacts and spec files changed; preserve the user's unrelated package/Tauri dirty work.

## Likely Files

- `src-tauri/src/services/github_import/source.rs`
- `src-tauri/src/services/github_import/mod.rs`
- `src-tauri/src/services/github_import/progress.rs`
- `src-tauri/src/services/github_import/tests.rs`
- `src-tauri/src/services/central_updates/fs.rs`
- `src-tauri/src/services/central_updates/fs/tests.rs`
- `src-tauri/src/services/central_updates/inventory/tests.rs`
- `.trellis/spec/backend/github-import-preview-contract.md`

## Watch-Only Files

- `src-tauri/src/services/github_import/import.rs`
- `src-tauri/src/services/github_import/remote.rs`
- `src-tauri/src/services/github_import/archive.rs`
- `src-tauri/src/services/central_updates/core.rs`
- `src-tauri/src/services/central_updates/inventory/force.rs`
- `src-tauri/src/services/central_updates/fs/batch.rs`
- `src-tauri/src/db/repos/repositories_repo.rs`
- `src/components/marketplace/*`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

Promote a watch-only file to a changed file only when a failing acceptance test proves the existing shared path cannot satisfy the contract.

## Validation Commands

```powershell
cd src-tauri
cargo fmt --check
cargo test services::github_import::tests::tests
cargo test services::central_updates::fs::tests
cargo test services::central_updates::inventory::tests
cargo clippy -- -D warnings
cd ..
git diff --check
just ci
```

`pnpm tauri build` is not required unless implementation unexpectedly changes packaging, Tauri resources or bundle configuration.

## Review Gates

- Root means every snapshot file, not only files without `/`.
- Nested means exact selected subtree, not the repository root.
- Import and update call the same path mapping helper.
- Root descendants affect progress totals, remote hash and atomic writes.
- Existing incomplete root installations become repairable without a schema migration.
- Remote direct import remains recursive and Central batching remains compound.
- No repository-name special case, ignore list, DTO field or UI change appears.

## Rollback Points

- After the pure helper tests: revert if root and nested mapping cannot be expressed without weakening path safety.
- After import tests: revert the import call site independently if staging/progress semantics regress, but do not start update work with divergent contracts.
- After update tests: revert the shared change as one unit if batching, atomic replacement or copy refresh behavior changes unexpectedly.
- Before `just ci`: verify the unrelated dirty `package.json` and Tauri dependency/config files remain untouched by this task.
