# User scope decision: fail closed (2026-09-03)

- **Choice**: option 1 — stay fail-closed
- **Workflow edits**: none. `.github/workflows/release-desktop.yml` stays unmodified.
- **REL-001**: remains **open**. Pinned CLI 2.11.4 rehearsal did not prove the bundler consumes an Authenticode predecessor **digest** (`patch_binary` before NSIS `File`). See `tauri-windows-bundle-phase-evidence.md`.
- **REL-002**: remains **open**. Updater key / OIDC narrowing is not authorized without an R1 pass; this decision does not implement a partial REL-002-only workflow change.
- **Rejected here**: option 2 (REL-002-only workflow edit) and option 3 (custom bundler / inner-exe replace). Design already rejected option 3.
- **Installer child**: `09-02-windows-installer-verification` stays unstarted. Its D1 requires a passing signing R1 and implemented signing order. Do not claim NSIS/MSI inner-exe Authenticode from the current bundle-then-sign workflow.

Parent finding ledger must not mark REL-001 or REL-002 as `fixed` or `wontfix`. They stay open with this research + decision as evidence of the fail-closed stop.
