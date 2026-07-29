# Design: PR #25 cross-platform CI repair

## 1. Boundaries And Invariants

This repair changes no application behavior. It corrects one test fixture so it exercises the intended production path and adds two repository checkout invariants: one for a byte-compared generated artifact and one for raw-byte-locked SQL fixtures.

The production supervisor invariant remains unchanged: after a primary error, the runner terminates the process tree and reaps the direct child; a genuine termination/reap failure returns `RunnerError::TerminationFailed`. The test must create a real stdin write failure while its child is still alive, rather than accepting a cleanup failure as equivalent.

The codegen invariant also remains unchanged: Rust generation and the checked-in TypeScript artifact are byte-identical and end in exactly one newline. Cross-platform checkout normalization belongs in `.gitattributes`, not in semantic normalization inside the checker.

## 2. macOS Fixture Repair

`drop(std::io::stdin())` only drops a handle to the process-global stdin object; it does not close file descriptor/handle inherited from the parent. Add a test-only `close_fixture_stdin` helper with platform branches:

- Unix: construct an owning `OwnedFd` from the inherited fd 0 and drop it, using the standard library's OS-close ownership semantics without adding a dependency or handwritten FFI; do not use stdin again in the fixture process.
- Windows: close the inherited raw stdin handle using the already-approved `windows-sys::Win32::Foundation::CloseHandle`, assert success, and do not use stdin again in the fixture process.

The `close_stdin` fixture calls this helper and then sleeps briefly. The 8 MiB parent write exceeds pipe capacity, so even if the writer starts first it blocks until the child closes the pipe; it then receives a write error while the child/process group is still alive. The existing `WriteStdin` assertion remains the regression test.

Do not change `ProcessRunner::run`, `terminate_and_reap`, `ProcessTreeGuard`, or the error mapping. Those paths already pass timeout, cancellation, output-limit, and descendant cleanup tests; weakening them would hide a real leak risk.

## 3. Generated Artifact Line Endings

Add a path-specific attribute:

```gitattributes
src/lib/ipc/generatedCommandMap.ts text eol=lf
```

This makes the index-to-working-tree conversion deterministic even when Windows Git uses `core.autocrlf=true`. Keep the rule path-specific because the repository contains many ordinary TypeScript files with CRLF/mixed Windows working-tree forms and this task does not authorize a repository-wide normalization.

Do not normalize `expected` and `actual` inside `ipc_codegen.rs`: doing so would weaken the byte-drift/one-final-newline contract and allow a generated artifact to change form after checkout. Do not rewrite the generated file; its index content is already LF and current.

## 4. Frozen Migration Fixture Line Endings

The later Windows `just-ci` failure is a third checkout-boundary defect. The five
`src-tauri/tests/fixtures/db/*.sql` files are intentionally hashed as raw bytes,
but no attribute prevents `core.autocrlf` from rewriting their LF bytes. Add a
path-scoped `text eol=lf` rule for this fixture set. Preserve the raw-byte test
and manifest digests so the lock continues to detect actual fixture edits.

## 5. Regression Signals

The fast local signals are:

1. The exact closed-stdin Rust test returns `WriteStdin`.
2. A temporary `git -c core.autocrlf=true checkout-index` of `generatedCommandMap.ts` contains zero CRLF sequences.
3. `pnpm ipc:codegen:check` passes and leaves the generated file unchanged.
4. A temporary autocrlf checkout of all five SQL fixtures preserves every
   manifest SHA-256 digest, and the focused fixture-lock test passes.

The platform proof is the PR matrix on the new head SHA. macOS is authoritative for the process-group regression; Windows `just-ci` is authoritative for checkout conversion; Ubuntu provides Unix non-macOS regression coverage.

## 6. Documentation And Prevention

Update the process-supervision spec to state that broken-pipe fixtures must close the inherited OS stdin object, not merely drop a standard-library handle. Update the CI quality-gate spec to require explicit LF attributes for byte-compared generated text artifacts. Update the database-migrations spec to keep raw-byte-locked SQL fixtures LF across checkout settings. These are narrow prevention rules derived from the repeated CI failures.

## 7. Rollback And Remote Operations

The initial repair is one atomic commit containing the macOS fixture fix, the generated-artifact LF rule, and their spec updates. The independently discovered SQL fixture defect is a second atomic commit containing only its path-scoped LF rule and database-migration spec update. Either repair can be reverted independently without data migration or runtime state changes. Trellis archive/journal commits remain separate.

The initial repair commit has already been pushed. The only remaining remote mutation is pushing the second repair commit from local `dev` to `origin/dev`, which updates PR #25 and starts a new CI run. Trellis archive/journal bookkeeping remains local unless separately authorized. The task does not edit the PR, merge it, dispatch smoke packaging, or mutate releases. Monitoring must bind to the resulting PR head SHA.
