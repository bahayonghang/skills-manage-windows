# Implementation Plan: main protection and test workflow

## Preconditions

- [x] Confirm the task remains in `planning` until this plan is approved.
- [x] Re-read current git status and preserve unrelated changes.
- [x] Load `trellis-before-dev` and the relevant frontend/testing guidance before editing implementation files.
- [x] Re-read remote `main` protection immediately before the write; stop if another actor has introduced a conflicting policy.

## 1. Add The Regression Test First

- [x] Add `yaml` as a direct development dependency using pnpm.
- [x] Add `src/test/ciWorkflowContract.test.ts` that parses `.github/workflows/ci.yml` and initially exposes the missing PR/push triggers and package guards.
- [x] Run the focused Vitest and record the expected failure before implementation.

Validation:

```powershell
pnpm vitest run src/test/ciWorkflowContract.test.ts
```

## 2. Complete The Local Gate

- [x] Add the frontend production build to the web chain in `scripts/run-ci.mjs`.
- [x] Add Rust formatting, all-target locked Clippy, and locked Rust tests to the Rust chain.
- [x] Preserve the existing parallel fail-fast process management.
- [x] Update the `justfile` comments to match the executable contract.

Focused validation:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
pnpm build
```

## 3. Activate CI For Change Events

- [x] Add `pull_request` for `main`, `push` for `main` and `dev`, and `workflow_dispatch` while retaining `release.published`.
- [x] Keep job/check name `just-ci` stable and unguarded.
- [x] Install both `clippy` and `rustfmt` in the Rust toolchain step.
- [x] Guard all cross-platform package jobs so only release/manual events run them.
- [x] Make the YAML contract test pass.

Validation:

```powershell
pnpm vitest run src/test/ciWorkflowContract.test.ts
git diff --check
```

## 4. Reconcile Documentation

- [x] Update `AGENTS.md` and `CONTRIBUTING.md` to point contributors to the complete `just ci` gate.
- [x] Update the gate summaries in `README.md` and `README_CN.md`.
- [x] Update `docs/reference/cli-just.md` and `docs/zh/reference/cli-just.md` with every executed step and the CI event/package matrix.
- [x] Search for stale descriptions of `just ci` and resolve only directly related mismatches.

## 5. Run Local Quality Gates

- [x] Run the focused CI contract test.
- [x] Run the complete local gate.
- [x] Run the Windows Tauri build required for workflow/packaging changes.
- [x] Confirm the expected Windows installer/bundle exists.
- [x] Inspect generated metadata and revert nothing unless the change was created by this task and is demonstrably incidental.

Validation:

```powershell
pnpm vitest run src/test/ciWorkflowContract.test.ts
just ci
pnpm tauri build
Get-ChildItem src-tauri/target/release/bundle -Recurse -File
```

## 6. Enable And Verify Main Protection

- [x] Clear only the process-local `GH_TOKEN` override so `gh` uses the authenticated keyring identity; do not modify persisted credentials.
- [x] Confirm `main` is still unprotected and no repository ruleset now conflicts.
- [x] Apply classic branch protection with strict `just-ci` bound to app id `15368`, admin enforcement, PR requirement with zero approvals, conversation resolution, and force-push/deletion disabled.
- [x] Read the full protection object back and verify every enabled and intentionally disabled rule against R5.
- [x] Confirm the branch endpoint now reports `protected: true`.

Rollback point:

- If GitHub rejects the update, make no partial substitute policy; diagnose the exact API/permission/schema error.
- If the update succeeds but readback differs materially, stop and report the mismatch before any additional remote mutation.
- The recovery endpoint is `DELETE /repos/bahayonghang/skills-manage-windows/branches/main/protection`; invoke it only with explicit authorization or when immediately reversing this task's demonstrably broken update.

## 7. Final Review And Trellis Closeout

- [x] Load `trellis-check` and run the final scope/spec/quality review.
- [x] Run `git diff --check`, inspect the complete diff, and confirm no unrelated files are included.
- [x] Record validation evidence and remote protection readback in the task artifacts or journal.
- [x] Follow Trellis spec-update, commit, archive, and session-record steps; do not push, open a PR, merge, or release.
