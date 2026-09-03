# REL-001 / REL-002 parent close contract — 2026-09-03

This is a **parent Goal ledger close**, not a product fix.

## What is unchanged (must stay true)

- `.github/workflows/release-desktop.yml` still **bundles NSIS/MSI first**, then Authenticodes `skillport.exe` + installers, then updater-signs the final NSIS.
- `@tauri-apps/cli@2.11.4` rehearsal still **FAIL**s R1: in-process `patch_binary` before NSIS `File`; bundler consumption of an Authenticode predecessor **digest** is unproven. Evidence: archived `09-02-windows-release-signing/research/tauri-windows-bundle-phase-evidence.md`.
- `build-windows` still has job-level `id-token: write`.
- `TAURI_SIGNING_PRIVATE_KEY` is still present on the Windows `pnpm tauri build` step as well as the later updater-sign step.
- Inner-exe-before-bundle Authenticode, production certs, and user-machine installers remain **UNVERIFIED**.
- No validator was loosened. No inner-exe replace. No homemade bundler. No REL-002-only secret/OIDC edit.

## Why this is not `fixed`

The defect still exists in the pinned CLI and current workflow. Marking `fixed` would be false.

## Why the parent ledger uses `wontfix (contract-evidenced)`

The authorized Goal close states for the 29 IDs are `fixed` or **contract-evidenced `wontfix`**. After `windows-release-signing` R1 FAIL, the only in-repo remediations were:

| Option | Meaning | User 2026-09-03 |
|---|---|---|
| 1 | Fail closed, no workflow edit | **Chosen** |
| 2 | REL-002-only updater-key / OIDC narrowing | **Rejected** |
| 3 | Custom bundler / inner-exe replace | **Rejected** (also rejected by child design R2) |

There is **no remaining authorized implementation path** for REL-001 or REL-002 in this Goal. They are therefore not **actionable**. Child research + the option-1 choice is the contract.

The archived `user-scope-decision-2026-09-03.md` forbade marking `fixed` or a silent `wontfix` that implied the risk was gone. This document is the missing contract: residual risk stays named; product/workflow stay unmodified; Goal vocabulary for a non-actionable High finding is `wontfix (contract-evidenced)`.

## Residual risk (still present)

- NSIS/MSI payload `skillport.exe` may be unsigned at pack time (REL-001).
- Updater private key and Azure OIDC write remain broader than a single Authenticode/updater-sign step (REL-002).

Re-opening requires a new authorization: either a CLI that can prove digest identity **and** a workflow-order change, or an explicit REL-002-only workflow edit.
