# Quality Check

Date: 2026-07-12

## Result

- `cargo test`: 770 passed before the final module split.
- `just ci`: passed after the final module split.
  - Rust: 771 tests passed; clippy passed with `-D warnings`.
  - Frontend: 123 files passed, 1329 tests passed, 1 skipped.
  - Typecheck, lint, and size budget passed.
- `git diff --check`: passed.
- WSL `/tmp` 10-skill benchmark: 3127 ms legacy-shape p50, 453 ms batch p50, 85.5% faster.
- Local 10-skill batch p50: 30 ms; WSL added time: 423 ms.
- FakeRunner process formulas and cancellation checks passed.

## Scope Review

- Update IPC payloads and user-visible errors remain compatible.
- Operation Logs store safe totals/counts; detailed phase timing remains in Runtime tracing.
- Remote execution remains behind `CommandRunner`.
- Normal update, force update, and force mirror share the batch executor.
- The conditional `07-11-ssh-persistent-session` child remains in `planning`; no measured evidence triggered it.

## Residual Evidence Gap

No live LAN SSH fixture credentials were available. SSH acceptance uses the existing measured operation-log slopes and the verified process-count reduction; `research/baseline-after.md` records the conservative projection and limitation explicitly.
