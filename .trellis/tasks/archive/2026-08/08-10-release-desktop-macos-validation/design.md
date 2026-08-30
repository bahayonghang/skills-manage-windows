# Design: Release Desktop macOS overflow fixture repair

## 1. Failure Mechanism

`ProcessRunner::run` concurrently writes stdin, reads bounded stdout/stderr, and waits for the direct child. Once a bounded reader returns `OutputLimitExceeded`, `try_join!` cancels the sibling futures and the runner calls `terminate_and_reap` so the entire process tree is killed before the primary error is returned.

The production contract is intentionally fail closed: if tree termination or child reaping fails, `TerminationFailed` overrides the primary error because the caller cannot safely claim cancellation, timeout, or output limiting completed.

The current overflow fixtures violate the premise of their own assertions. They write eight 8 KiB chunks and then return. When the reader crosses the 1024-byte cap, closing the pipe can make the writer finish or observe a broken pipe and exit immediately. On macOS, process-group termination can then race the unreaped exit and return `EPERM`. The test expects a successful active termination without keeping a termination target alive.

## 2. Minimal Repair

Change only the `large_stdout` and `large_stderr` branches in the test-only `supervised_process_fixture`:

1. Keep the existing bounded chunk writes and flush.
2. After producing output, sleep for 30 seconds.
3. Let the parent supervisor terminate the fixture as soon as it observes the 1024-byte limit.

The sleep is deliberately longer than the 3-second stdout and 5-second stderr test deadlines. Therefore:

- the expected path observes overflow and terminates immediately;
- a broken-pipe write still transitions into the sleep instead of natural exit;
- a regression that fails to observe overflow remains bounded by the existing deadline and fails with the wrong error;
- no production behavior or public API changes.

Do not fix this in `ProcessTreeGuard::terminate` by accepting `EPERM`. Permission denial can mean descendants were not terminated, so ignoring it would weaken the safety contract. Do not broaden the test assertion to accept `TerminationFailed`; that would turn an indeterminate cleanup state into a false pass.

## 3. Regression And Verification

The existing stdout/stderr tests are the correct seam once the fixture lifecycle is deterministic. They execute the real `ProcessRunner`, real OS process tree guard, real bounded pipes, and exact error variants.

Verification order:

1. Repeat both focused overflow tests at least 25 times.
2. Run all `targets::runner::tests` to cover adjacent lifecycle paths.
3. Run Rust format, Clippy, and the complete locked Rust suite.
4. Run repository `just ci` as the required aggregate gate.
5. Inspect the final diff and working tree for scope.

Windows proves the fixture remains bounded and does not regress Job Object cleanup. A separately authorized exact-head GitHub run is required for authoritative macOS evidence.

## 4. Documentation And Prevention

Update `.trellis/spec/backend/process-supervision.md` with a fixture rule: tests that assert a primary timeout/cancellation/output-limit result after active cleanup must keep the controlled child alive until the supervisor terminates it. Natural-exit races are separate scenarios and must not be mixed into those assertions.

This records the lesson from both the August 1 stderr flake and the August 9 stdout release failure.

## 5. Rollback And Remote Boundaries

The implementation is a small test-fixture and spec change with no migration or runtime state. Reverting those lines restores the previous behavior.

No remote mutation is needed for the local repair. Pushing a task branch, opening a PR, rerunning CI, moving/recreating the tag, or publishing `v1.0.1` requires separate authorization and exact-head verification.
