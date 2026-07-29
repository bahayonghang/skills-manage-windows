# Implementation plan: PR #25 cross-platform CI repair

## 1. Activation And Baseline

- [x] Obtain explicit approval of this final planning summary, then run `python ./.trellis/scripts/task.py start 07-28-pr-25-cross-platform-ci-repair`.
- [x] Dispatch implementation with the active task context and load the curated backend/quality specs.
- [x] Reconfirm clean `dev`, PR #25 head, synthetic merge tree parity, and failure run IDs before editing.
- [x] Run and record the pre-fix Windows checkout simulation as a red-capable line-ending signal.

## 2. Focused Repair

- [x] In `src-tauri/src/targets/runner.rs`, replace the ineffective stdin-handle drop with a test-only OS-level close helper for Unix and Windows. Keep the child alive after close and preserve the exact `WriteStdin` assertion.
- [x] Do not change production runner, process-tree, termination precedence, error variants, or caller behavior.
- [x] In `.gitattributes`, add only the path-specific LF rule for `src/lib/ipc/generatedCommandMap.ts`.
- [x] Do not hand-edit or regenerate `generatedCommandMap.ts` unless the checker proves semantic drift independently of line endings.

## 3. Focused Verification

- [x] Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml targets::runner::tests::closed_stdin_is_classified_as_write_failure --locked -- --exact`.
- [x] Run the adjacent ProcessRunner test group to cover timeout, cancellation, stdout/stderr limits, and descendant cleanup.
- [x] Run `git check-attr eol -- src/lib/ipc/generatedCommandMap.ts` and repeat the `core.autocrlf=true` temporary checkout simulation; require zero CRLF sequences.
- [x] Run `pnpm ipc:codegen:check` and prove the generated artifact remains unmodified.

## 4. Full Quality Gate

- [x] Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --locked`.
- [x] Run `just ci`.
- [x] Run `python ./.trellis/scripts/task.py validate`, `git diff --check`, and inspect the final diff/stat for scope.
- [x] Dispatch `trellis-check`; resolve every correctness, cross-platform, standards, and test finding before commit.
- [x] Use `trellis-break-loop` to capture why the earlier macOS fixture and Windows codegen repairs failed and ensure the prevention rules are reflected in existing specs.

## 5. Commit And Remote Verification

- [x] Update the two existing Trellis specs only with the prevention rules learned from this bug.
- [x] Create one atomic initial product/spec repair commit; keep task archive and journal bookkeeping separate.
- [x] Before push, disclose: local `dev` -> `origin/dev`, PR #25 `dev` -> `main`, unchanged title/body, exact commits and current/new head SHA, no merge, and automatic CI rerun side effect.
- [x] Push only `dev`; do not force-push, merge, edit PR metadata, or dispatch package workflows.
- [x] Poll PR #25/check-runs by the exact pushed head SHA until all non-skipped jobs reach terminal state.
- [x] If a check fails, fetch its logs and return to diagnosis; do not rerun blindly or report an older green run.
- [x] When the exact head is green, archive the named Trellis task and record only the two real work commits in the developer journal.

## 6. Approved SQL Fixture Follow-Up

- [x] Diagnose run `30378982381` by its exact head and prove the reported `v0.10.9` digest is the CRLF checkout of the manifest-locked LF fixture.
- [x] Confirm all five fixture manifest digests match LF bytes, drift under CRLF, and the PR head/merge contain the same fixture blob and tree.
- [x] Add only a path-scoped LF rule for `src-tauri/tests/fixtures/db/*.sql`; do not edit fixture SQL, manifest digests, or the raw-byte assertion.
- [x] Update the existing database migration spec with the immutable fixture checkout contract.
- [x] Run the autocrlf hash simulation, focused migration fixture test, relevant Rust gates, `just ci`, Trellis validation, and diff checks.
- [x] Dispatch `trellis-check` and resolve all findings before a second atomic repair commit.
- [x] Disclose and push only the second repair commit to `dev`, then monitor PR #25 by its exact new head SHA until every non-skipped job succeeds.

## 7. Exact-Head Completion Evidence

- PR #25 remained open as `dev` -> `main` with unchanged title/body and exact
  head `e1a2d6f54df73b693f3b9e67f5ecfb61881a85ed`.
- Pull-request run `30415927278` completed `success`: Ubuntu/macOS source
  validation, supply-chain, and Windows `just-ci` all succeeded; the three
  smoke-package jobs were intentionally skipped.
- All seven check-runs reached an allowed terminal conclusion, with zero
  failure annotations. The four warnings were the known `actions/checkout`
  Node.js 20 deprecation warning and were outside this repair's scope.
