# Original scan envelope rerun — 2026-09-03

Scanner identity remains **UNVERIFIED** (no standalone 2026-09-02 report file; ledger-only at `7c2134ce`). This pass reapplies the parent `prd.md` Audit Scope And Evidence Boundary to **current HEAD** `086a07ee` plus the uncommitted QUAL-SIZE-001 ledger row.

## Envelope (original parameters)

| Field | Value |
|---|---|
| scanner | 2026-09-02 deep whole-project engineering audit named in parent `prd.md` Goal |
| config | `.trellis/tasks/09-02-engineering-audit-remediation/prd.md` Audit Scope And Evidence Boundary; no extra flags |
| inputs | 29 Finding Ledger IDs (28 original + `QUAL-SIZE-001` from github-import sizecheck) |
| targets | `src/`, `src-tauri/src/`, SQLite schema/repos, `.github/`, `scripts/`, `.agents/`, `.codex/`, `.claude/`, `.trellis/scripts/`, root build/quality config |
| excluded | `ref/`, generated/cache/build, archived task prose review, live provider/SSH/WSL/installer/prod publish |
| method | Independent explore re-audit of evidence anchors + parent `trellis-check` |

## Dispatch

| Role | Agent | Result |
|---|---|---|
| explore (BE/usage/size) | [BE/usage scan](0da77154-7da8-4159-8033-00c82e77a4cf) | 5/5 fixed |
| explore (SEC/TOOL) | [SEC/TOOL scan](38c778a5-50cf-4eab-b5d2-1f1442dbefde) | 6/6 fixed; adjacent note below |
| explore (frontend/ARCH) | [frontend scan](a7bd30ec-2772-4135-8f76-9f991648f10c) | 9/9 fixed |
| explore (REL/QUAL/ARCH) | [release scan](af4a6044-cd32-4245-b8e3-9eceff43e921) | REL-001/002 still-open; others fixed |
| trellis-check | [parent check](6143063a-95b3-4ef2-ba61-3ef98a508145) | **FINDINGS** (AC2 blocked; 28 vs 29 docs) |

Cheap parent checks this pass: `pnpm sizecheck` exit 0 (732 files, max 800; `batch.rs` 775). Full `just ci` not rerun (already exit 0 after child archive).

## 29 IDs

| Verdict | Count | IDs |
|---|---:|---|
| fixed | 27 | BE-CORR-001/002/004, BE-CONC-001, SEC-001/002, FE-CORR-001/002/003, TOOL-001/002/003/004, QUAL-001/002/003/SIZE-001, FE-ARCH-001/002/003, FE-I18N-001, ARCH-001/002/003/004, REL-003/004 |
| still-open (fail-closed, not `wontfix`) | 2 | REL-001, REL-002 |
| not-reproducible / failed | 0 | — |

REL-001/REL-002: workflow still bundles NSIS/MSI then Authenticodes; `@tauri-apps/cli@2.11.4` `patch_binary` before NSIS `File`; `TAURI_SIGNING_PRIVATE_KEY` still on the full bundle step; `build-windows` still has `id-token: write`. Parent ledger **must not** mark them `fixed` or `wontfix` (`research/user-scope-decision-2026-09-03.md` in archived signing child).

QUAL-002 remains contract+fixture **fixed**; live `windows-2022` lifecycle **UNVERIFIED**.

ARCH-004 production `@/types` barrel importers measured **191** (baseline 199, no-growth).

## Out-of-envelope residual (not one of the 29)

`resolve_task_dir` in `.trellis/scripts/common/task_utils.py` still accepts absolute/`..` paths without `resolve_contained_path`. SEC-002 (explicit slug create) stays fixed. This adjacency is recorded, not added to the 29-ID ledger and not implemented in this parent.

## Envelope vs goal conjunction

This rerun **completed** with original scope/inputs. It did **not** yield 0 open IDs: REL-001/REL-002 remain open by user contract. Goal completion is still blocked.
