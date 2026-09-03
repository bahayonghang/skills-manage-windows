# Parent integration evidence — 2026-09-03

Parent is `in_progress` and unarchived. 12/12 children archived under `.trellis/tasks/archive/2026-09/`.

## Gates (this machine, after last child archive)

| Command | Exit | Notes |
|---|---:|---|
| `pnpm docs:gen:check` | 0 | IPC dict + schema table byte-stable |
| `just audit` | 0 | `currentness: unverified`; evidence total=21 shown=21 truncated=0; 2 blocking advisories with 2 approved exceptions |
| `just ci` | 0 | `[ci] All checks passed.` (~96s) |
| `pnpm sizecheck` (scan rerun) | 0 | 732 production files, max 800; `batch.rs` 775 |

## Finding ledger (29 IDs in parent `prd.md`)

### Passed / fixed (27)

BE-CORR-001, BE-CORR-002, BE-CONC-001, BE-CORR-004, SEC-001, SEC-002, FE-CORR-001, FE-CORR-002, FE-CORR-003, TOOL-001, TOOL-002, TOOL-003, TOOL-004, QUAL-001, QUAL-002 (contract+fixture only), QUAL-003, QUAL-SIZE-001, FE-ARCH-001, FE-ARCH-002, FE-ARCH-003, FE-I18N-001, ARCH-001, ARCH-002, ARCH-003 (progressive), ARCH-004 (no-growth, 191 ≤ 199), REL-003, REL-004.

### Open (fail-closed) — not `wontfix` (2)

| ID | Why still open |
|---|---|
| REL-001 | Tauri CLI 2.11.4 `patch_binary` before NSIS `File`; no digest proof bundler consumes Authenticode predecessor. User scope 1: do not change signing-order / inner-exe replace. |
| REL-002 | Updater key / OIDC surface left unchanged under the same scope decision. |

### Failed

None in local `just ci` / `just audit` / `docs:gen:check` / `pnpm sizecheck`.

### Skipped

- Parent archive (REL-001/002 still open).
- Push.

### Missing evidence / UNVERIFIED

- `windows-2022` NSIS/MSI install → launch → uninstall with final signed assets (QUAL-002 AC1/AC2/AC5–AC9).
- Inner-exe-before-bundle Authenticode (REL-001).
- Real user-machine installer compatibility.
- Live npm/Cargo advisory currentness (`just audit` reported `currentness: unverified`).
- Real GitHub/AI provider, SSH/WSL, WebView2, production publish.

Original scan envelope rerun: `research/scan-rerun-2026-09-03.md` (completed; 2 IDs remain open). Independent parent trellis-check: **FINDINGS**.

## Goal conjunction (not met)

1. REL-001/REL-002 remain open by user decision (`user-scope-decision-2026-09-03.md` forbids `fixed` and `wontfix`).
2. Parent not archived (intentional until those IDs close).
3. No push.

Do not mark the parent or the authorized goal complete.
