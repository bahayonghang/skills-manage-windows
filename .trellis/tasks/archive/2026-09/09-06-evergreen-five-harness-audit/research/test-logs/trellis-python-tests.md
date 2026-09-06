# Trellis Python tests — 2026-09-06

- Command: `rtk python -X utf8 -m unittest discover -s .trellis/scripts/tests -p 'test_*.py' -v`
- Exit code: `0`
- Result: `Ran 35 tests in 6.342s`; `OK (skipped=4)`; therefore 31 passed and 4 skipped.

The four skips are explicit platform evidence gaps on this Windows host:

```text
test_add_context_posix_symlink_escape ... skipped 'AC10 missing evidence / UNVERIFIED: POSIX symlink vectors not run on Windows host'
test_hook_posix_symlink_escape ... skipped 'AC10 missing evidence / UNVERIFIED: POSIX symlink vectors not run on Windows host'
test_task_slug_posix_symlink_escape ... skipped 'AC10 missing evidence / UNVERIFIED: POSIX symlink vectors not run on Windows host'
test_process_posix_process_group_cleanup ... skipped 'AC17 missing evidence / UNVERIFIED: POSIX process-group cleanup not run on Windows host'
```

Covered and passed surfaces include Windows junction/path containment, Codex/Claude hook-equivalence vectors, bounded process output/redaction, Windows descendant-process cleanup on timeout, sub-agent context budgets, and offline task creation/rollback behavior.

Coverage gap confirmed by the separate current-task probe: the suite has no regression for resolving or clearing a fresh context key when exactly one stale session pointer exists. The main audit's `research/probe-active-task.py` reproduces that gap without mutating the real runtime.
