# Parent integration evidence — 2026-09-03

Parent remains `planning` and unarchived. 12/12 children archived under `.trellis/tasks/archive/2026-09/`. Original audit envelope (`dev@7c2134ce` whole-project rescan) was not rerun.

## Gates (this machine, after last child archive)

| Command | Exit | Notes |
|---|---:|---|
| `pnpm docs:gen:check` | 0 | IPC dict + schema table byte-stable |
| `just audit` | 0 | `currentness: unverified`; evidence total=21 shown=21 truncated=0; 2 blocking advisories with 2 approved exceptions |
| `just ci` | 0 | `[ci] All checks passed.` (~96s) |

Installer-child `just ci` (working tree, before product commit `880098cc`) also passed; parent rerun is at HEAD after archive `dbf5293f` plus uncommitted parent artifacts only.

## Finding ledger (28 IDs in parent `prd.md`)

### Passed / fixed (26)

BE-CORR-001, BE-CORR-002, BE-CONC-001, BE-CORR-004, SEC-001, SEC-002, FE-CORR-001, FE-CORR-002, FE-CORR-003, TOOL-001, TOOL-002, TOOL-003, TOOL-004, QUAL-001, QUAL-002 (contract+fixture only), QUAL-003, FE-ARCH-001, FE-ARCH-002, FE-ARCH-003, FE-I18N-001, ARCH-001, ARCH-002, ARCH-003 (progressive), ARCH-004 (no-growth), REL-003, REL-004.

Spot-check: journaled GitHub import apply; `resolve_contained_path`; `/collections` + `hasLoaded`; usage `exists -> Result`; `run_bounded_process`; misspelled `trellis-spec-bootstarp` gone; generated IPC/schema check green.

### Open (fail-closed) — not `wontfix` (2)

| ID | Why still open |
|---|---|
| REL-001 | Tauri CLI 2.11.4 `patch_binary` before NSIS `File`; no digest proof bundler consumes Authenticode predecessor. User scope 1: do not change signing-order / inner-exe replace. |
| REL-002 | Updater key / OIDC surface left unchanged under the same scope decision. |

### Failed

None in local `just ci` / `just audit` / `docs:gen:check`.

### Skipped

- Parent `task.py start` and parent archive (keep `planning`).
- Independent `trellis-check` subagent on the parent (no started parent task). Child reviews already ran per child. This file is a parent-side ledger+gate recap, not a substitute child check.

### Missing evidence / UNVERIFIED

- Original 2026-09-02 scan envelope rerun on `dev@7c2134ce` (or current HEAD with the same envelope).
- `windows-2022` NSIS/MSI install → launch → uninstall with final signed assets (QUAL-002 AC1/AC2/AC5–AC9).
- Inner-exe-before-bundle Authenticode (REL-001).
- Real user-machine installer compatibility.
- Live npm/Cargo advisory currentness (`just audit` reported `currentness: unverified`).
- Real GitHub/AI provider, SSH/WSL, WebView2, production publish.

## Goal conjunction (not met)

1. REL-001/REL-002 remain open by user decision.
2. Original scan not rerun.
3. Parent not archived (intentional).
4. No push.

Do not mark the parent or the authorized goal complete.
