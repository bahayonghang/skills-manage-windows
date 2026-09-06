# GitHub Actions read-back — 2026-09-06

Commands were read-only `gh run list` / `gh run view` calls. No workflow was dispatched or rerun.

## Current remotely verified baseline

- Latest listed CI run: [`33710069460`](https://github.com/bahayonghang/skills-manage-windows/actions/runs/33710069460), PR head `9aca456d7e6894092b29a0aacbf23a5aff350298`, 2026-09-03, conclusion `success`.
- `common`, Windows/Linux/macOS Rust, supply-chain, and aggregate `just-ci` all succeeded.
- Current HEAD `a81b7c2da5faa70ad0ee83d5292e46808c8c3d2e` is the promotion merge whose second parent is that exact PR head; `git diff --name-status 9aca456d..HEAD` is empty. Thus the successful hosted checks cover the current tracked tree bytes, although there is no workflow run whose `headSha` is the merge SHA itself.
- Current workflow correspondence: `.github/workflows/ci.yml:79-80` runs the common lane; `:110-112`, `:157-158`, and the analogous macOS step run the same `rust-platform` entry; `:260-290` aggregates the five required results and fails closed. `scripts/check/run-ci.mjs:32-62` is the current lane definition exercised by that run.

## Most recent listed failed CI and verified repair

Run [`33171369538`](https://github.com/bahayonghang/skills-manage-windows/actions/runs/33171369538) (2026-08-28, head `b67111d`) failed four required lanes:

- Supply-chain: expired exceptions dated 2026-08-11, npm `GHSA-qwww-vcr4-c8h2` (`react-router`), Cargo `RUSTSEC-2026-0258` (`h2`), and `RUSTSEC-2023-0071` (`rsa`).
- Windows/Linux/macOS Rust: the same three `skills_cli` tests leaked host well-known `npx-cli.js` roots into a synthetic missing-npx fixture: `ac15_missing_npx_js_public_message_omits_candidate_paths`, `ac2_doctor_succeeds_when_npx_js_is_missing`, and `ac8_doctor_reports_missing_npx_js_without_path_mutation`.
- Aggregate `just-ci` correctly failed closed.

The failure was repaired before the next successful promotion by:

- `3308c0aa fix(skills-cli): ...`: injects explicit fallback roots so missing-npx tests use `[]` and adds a positive well-known-root test. The current repair is visible in `src-tauri/src/services/skills_cli/argv.rs:207-232,270-299`; the three formerly leaking tests now pass empty roots at `src-tauri/src/services/skills_cli/tests.rs:495-524,1013-1028`, while `:1031-1046` independently preserves the production well-known fallback contract.
- `d0d7b239 fix(deps): ...`: upgrades `react-router-dom` and `h2`, removes the npm exception, and refreshes the still-scoped Cargo exceptions. Current source/lock evidence is `package.json:68`, `pnpm-lock.yaml:80-82,5110-5118`, `src-tauri/Cargo.lock:1923-1927`, and `security/dependency-audit-exceptions.json:1-16`; the removed npm advisory is absent and the two remaining Cargo exceptions are explicit and dated.

Current local Rust (1553 passed) and the 2026-09-03 hosted five-lane CI both pass, so this historical failure must not be reopened as a current remediation item.

## Latest listed failed Release Desktop run

Run [`31308665794`](https://github.com/bahayonghang/skills-manage-windows/actions/runs/31308665794) (tag `v1.0.1`, 2026-08-09) stopped because macOS Rust test `targets::runner::tests::stdout_overflow_terminates_the_process` returned `TerminationFailed { trigger: OutputLimit, ... PermissionDenied / Operation not permitted }`; Windows, Linux, common, and supply-chain lanes passed. The aggregate then failed closed and all build/publish jobs were skipped.

The historical race was between bounded-read overflow and a fixture that could exit naturally before the parent terminated its process group. In current source, overflow becomes the primary error at `src-tauri/src/targets/runner.rs:356-375` and termination/reap happens at `:345-352`. Commit `5a4de0a5` made the large-output fixture remain alive after flushing; the exact retained repair is `src-tauri/src/targets/runner.rs:478-498`, and the exercised assertion is `:615-655`. Commit `b2e802eb` later stabilized a different timeout-fairness test (`:535-570`), so it is supporting runner-test history, not the direct fix for this release failure. The current 2026-09-03 macOS Rust lane passes. This is historical evidence, not a current release defect.

No Release Desktop run after the 2026-09-03 workflow changes was found in the latest 30 runs. Current end-to-end release, Windows installer, Authenticode/updater signature, and publication behavior therefore remain `UNVERIFIED` even though current hosted CI is green.
