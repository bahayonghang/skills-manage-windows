# Repair PR 25 cross-platform CI failures

## Goal

Repair the three diagnosed cross-platform failures on PR #25 (`dev` -> `main`) without weakening production process-supervision, generated-contract checks, or frozen-fixture checks, then push the focused repairs and verify the PR on its exact new head SHA until every non-intentionally-skipped CI job succeeds.

## Background And Confirmed Facts

- `C:\Users\lyh\.grok\last-copy.txt` contains only the literal text `~/.grok/last-copy.txt)` and is not an error log. The authoritative evidence is GitHub Actions run `30371282332` for PR #25 head `83c4ae4ab303785504a493ffdab731bd88f60e97`.
- The PR synthetic merge commit `9a25e31164cac77ecdf59f256617951bbf58047a` and the head commit have the same tree SHA, `84917dfe8e75266713715e7bfc2cf0c05622b042`; the failures are present in the submitted tree rather than caused by merge-only content.
- macOS job `90316266343` fails only `targets::runner::tests::closed_stdin_is_classified_as_write_failure`: the fixture drops a temporary `std::io::Stdin` handle instead of closing the inherited OS stdin descriptor. The writer consequently observes failure only after the child exits, and macOS process-group cleanup returns `EPERM`, surfacing `TerminationFailed` instead of the intended `Io { phase: WriteStdin }`.
- Windows job `90318748481` fails `pnpm ipc:codegen:check`: `src/lib/ipc/generatedCommandMap.ts` has no `eol` attribute, so a Windows checkout may contain CRLF while the Rust exporter emits LF. The byte comparison reports the artifact as stale even though the generated contract content is current. The same command passes in the current LF working tree.
- After those two repairs reached head `0c49dcf6d733e83b93328243a5e5ee85ca884d4d`, run `30378982381` passed both source-validation jobs and supply-chain but exposed a later Windows `just-ci` failure in `fixture_manifest_and_files_are_locked`. The frozen SQL fixtures have no `eol` attribute: the canonical LF `v0.10.9` digest is `0e8e8cfc...`, while an autocrlf checkout deterministically produces the reported `6cafbb96...`. All five manifest digests match LF bytes and drift under CRLF; the PR head and synthetic merge contain the same fixture blob and tree.
- Ubuntu source validation and the supply-chain job already pass. The three smoke-package jobs are intentionally skipped for `pull_request` events by workflow policy.
- The `actions/checkout` Node 20 deprecation annotation is a warning from the pinned action revision, not a failing PR check. Updating action majors across workflows is an independent supply-chain change.

## Requirements

1. The closed-stdin fixture must close the inherited stdin object at the OS boundary and remain alive long enough for the parent writer to deterministically observe the broken pipe while the supervised process group still exists.
2. The fix must preserve the production rule that a real tree-termination or reap failure overrides the primary timeout, cancellation, output-limit, or I/O error. It must not relax `TerminationFailed` handling or merely broaden the test assertion.
3. `src/lib/ipc/generatedCommandMap.ts` must be checked out as LF on Windows and Unix so the existing byte-exact codegen check has one platform-independent artifact. The generated file itself must not be hand-edited.
4. Changes must remain limited to the three diagnosed defects, their focused regression contracts, and Trellis documentation needed to prevent recurrence. Unrelated PR content and working-tree changes must remain untouched.
5. Local verification must include focused ProcessRunner and codegen checks, Rust formatting/lint/tests as required by the project, `just ci`, Trellis validation, whitespace checks, and final diff inspection.
6. Before the remote write, disclose the source/target branches, unchanged PR title/body, commit to push, expected head SHA change, absence of merge, and CI rerun side effects.
7. After pushing to `dev`, monitor PR #25 by the exact new head SHA. Do not report success from an older run, a synthetic merge SHA with a different tree, or a locally passing Windows-only check.
8. Frozen migration SQL fixtures must retain their manifest-locked LF bytes on every checkout. Fix this with a path-scoped `.gitattributes` rule; do not normalize the raw-byte assertion, rewrite fixture SQL, or refresh manifest digests for CRLF-only drift.

## Acceptance Criteria

- [x] `cargo test --manifest-path src-tauri/Cargo.toml targets::runner::tests::closed_stdin_is_classified_as_write_failure --locked -- --exact` passes on the local platform, and the same test passes in the PR macOS source-validation job.
- [x] The existing assertion still requires `RunnerError::Io { phase: RunnerPhase::WriteStdin, .. }`; it does not accept `TerminationFailed` as an alternative.
- [x] `git check-attr eol -- src/lib/ipc/generatedCommandMap.ts` reports `eol: lf`, and a `core.autocrlf=true` checkout simulation contains no CRLF bytes in that artifact.
- [x] `pnpm ipc:codegen:check` passes without modifying `src/lib/ipc/generatedCommandMap.ts`.
- [x] `cargo fmt --all -- --check`, the relevant locked Rust checks, and `just ci` pass locally.
- [x] `git check-attr eol -- src-tauri/tests/fixtures/db/0_10_9.sql` reports `eol: lf`; an autocrlf checkout preserves all five manifest SHA-256 values, and `fixture_manifest_and_files_are_locked` passes.
- [x] `python ./.trellis/scripts/task.py validate`, `git diff --check`, and final scoped diff inspection pass.
- [x] The repair is committed and pushed only to `dev`; PR #25 remains `dev` -> `main`, is not merged, and its title/body are not changed.
- [x] On the exact pushed head SHA, `source-validation (ubuntu-22.04)`, `source-validation (macos-14)`, `supply-chain`, and `just-ci` all conclude `success`; no check run or annotation concludes/reports `failure`.
- [x] Intentionally skipped smoke-package jobs remain acceptable for the PR event. Existing non-failing Node runtime deprecation warnings are recorded but do not count as this task's failure condition.

## Out Of Scope

- Upgrading `actions/checkout` or other pinned Actions solely to remove Node 20 deprecation warnings.
- Changing production process-tree termination precedence, timeout/cancellation behavior, or public error contracts.
- Regenerating IPC types when their semantic content has not changed.
- Running smoke packaging jobs that the PR workflow intentionally skips.
- Merging or closing PR #25, changing its title/body, or modifying branch protection.
- Repairing any further independent failure outside the three diagnosed root causes; each new failure requires fresh diagnosis and an explicit scope decision.
