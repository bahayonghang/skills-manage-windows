# Parent integration evidence — 2026-09-03

Parent `in_progress` until archive. 12/12 children archived under `.trellis/tasks/archive/2026-09/`.

## Gates (this machine, after last child archive)

| Command | Exit | Notes |
|---|---:|---|
| `pnpm docs:gen:check` | 0 | IPC dict + schema table byte-stable |
| `just audit` | 0 | `currentness: unverified`; evidence total=21 shown=21 truncated=0; 2 blocking advisories with 2 approved exceptions |
| `just ci` | 0 | `[ci] All checks passed.` (~96s) |
| `pnpm sizecheck` (scan rerun) | 0 | 732 production files, max 800; `batch.rs` 775 |

## Finding ledger (29 IDs)

### fixed (27)

BE-CORR-001, BE-CORR-002, BE-CONC-001, BE-CORR-004, SEC-001, SEC-002, FE-CORR-001, FE-CORR-002, FE-CORR-003, TOOL-001, TOOL-002, TOOL-003, TOOL-004, QUAL-001, QUAL-002 (contract+fixture), QUAL-003, QUAL-SIZE-001, FE-ARCH-001, FE-ARCH-002, FE-ARCH-003, FE-I18N-001, ARCH-001, ARCH-002, ARCH-003 (progressive), ARCH-004 (no-growth), REL-003, REL-004.

### wontfix (contract-evidenced) (2)

REL-001, REL-002 — `research/rel-001-002-wontfix-contract-2026-09-03.md`. Residual risk remains. Workflow unsigned-inner-exe / broad updater key **unchanged**. Not product-fixed.

### open (actionable)

None.

### Failed

None in local `just ci` / `just audit` / `docs:gen:check` / `pnpm sizecheck`.

### Skipped

- Push.

### Missing evidence / UNVERIFIED

- `windows-2022` NSIS/MSI install → launch → uninstall with final signed assets.
- Inner-exe-before-bundle Authenticode (REL-001 residual).
- Real user-machine installer compatibility.
- Live npm/Cargo advisory currentness.
- Real GitHub/AI provider, SSH/WSL, WebView2, production publish.

Scan rerun: `research/scan-rerun-2026-09-03.md`. REL contract: `research/rel-001-002-wontfix-contract-2026-09-03.md`. Independent parent trellis-check after REL close: **PASS**.
