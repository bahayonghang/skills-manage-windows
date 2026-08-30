# Implementation Plan: Release Desktop macOS overflow fixture repair

## 1. Activate And Protect Scope

- [x] After explicit approval of the latest planning summary, start `08-10-release-desktop-macos-validation`.
- [x] Create/switch to `task/release-desktop-macos-validation` from clean `dev`; do not modify or publish `v1.0.1` refs/releases.
- [x] Load the Trellis implement context with the backend process-supervision and quality-gate specs.

## 2. Make Overflow Fixtures Deterministic

- [x] In the test-only `supervised_process_fixture`, keep `large_stdout` alive after writing/flushing oversized output.
- [x] Apply the same lifecycle rule to `large_stderr` so both symmetric paths are protected.
- [x] Do not modify production runner, process-tree guard, termination/reap ordering, error mapping, workflow YAML, or release metadata.

## 3. Capture The Prevention Contract

- [x] Update `.trellis/spec/backend/process-supervision.md` to require active-termination fixtures to stay alive until killed and to distinguish them from natural-exit race tests.
- [x] Confirm no new dependency, public API, generated documentation, i18n, version, or packaging change is introduced.

## 4. Validate From Narrow To Broad

- [x] Run the exact stdout and stderr overflow tests once and confirm each executes one test.
- [x] Repeat both focused tests at least 25 times; stop on the first failure.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --locked targets::runner::tests -- --nocapture` and confirm the filter executes the expected non-zero test set.
- [x] Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --locked`.
- [x] Run `just ci` and require success.
- [x] Run Trellis structural validation, `git diff --check`, final diff inspection, and final status inspection.

## 5. Review And Closeout

- [x] Use `trellis-check` for correctness, scope, tests, and spec compliance.
- [x] Record Windows/local evidence separately from deferred macOS hosted evidence.
- [x] Commit the approved local repair and Trellis records as `9b149888`; no push, PR, Actions rerun, or republish was performed.

## Rollback Points

- Fixture change: revert only the two test-only linger additions.
- Spec change: revert the matching fixture-lifecycle rule if the code change is reverted.
- No database, credential, release, tag, installer, or runtime migration rollback is required.
