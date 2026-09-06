# `just doctor` — 2026-09-06

- Command: `rtk just doctor`
- Exit code: `1`
- Repository worktree mutation: none observed (`git status --short --branch` stayed on `dev` with only the active untracked planning task)

```text
node scripts/check/doctor.mjs
error: recipe `doctor` failed on line 18 with exit code 1
[doctor] Read-only development environment check
[ok] Node.js: 26.7.0 (expected major 26)
[mismatch] pnpm: could not run command (spawnSync pnpm ETIMEDOUT) (expected 10.34.5; activate pnpm 10.34.5 without changing the repository)
[ok] rustc: 1.98.0 (expected 1.98.0)
[ok] Cargo: 1.98.0 (expected 1.98.0)
[ok] just: 1.58.0 (expected available)
[ok] Git: 2.55.0 (expected available)
[ok] Windows MSVC Rust target: x86_64-pc-windows-msvc (expected x86_64-pc-windows-msvc)
[ok] Tauri CLI: installed (expected installed)
[doctor] 1 check(s) need attention; no changes were made.
```

## Root-cause evidence

- `scripts/check/doctor.mjs:46-52` runs every probe through `spawnSync(..., timeout: 5_000)`.
- `Get-Command pnpm` resolves `C:\Users\lyh\scoop\shims\pnpm.exe`.
- The shim points at `C:\Users\lyh\scoop\apps\pnpm\current\pnpm.exe`.
- The installed Scoop manifest says pnpm `12.3.4`, while `package.json#packageManager` requires `pnpm@10.34.5`.
- Direct `rtk pnpm --version` produced no output for more than 60 seconds and was interrupted (exit `1`).
- During the doctor/direct invocation, pnpm created a user-level engine-download area under `...\package-manager-store\v11\tmp\pnpm-engine-10.34.5-...` and left `@pnpm+exe@10.34.5.lock`; the project working tree stayed unchanged.
- Safe child-process A/B: `$env:pnpm_config_pm_on_fail = 'ignore'; rtk pnpm --version` exited `0` immediately with `12.3.4`. With the same non-persistent environment override, `rtk just doctor` completed in 1.64 s and correctly reported `pnpm: 12.3.4 (expected 10.34.5)` instead of timing out. This proves the probe can suppress pnpm 12's automatic project-version bootstrap without modifying the global pnpm installation. It does not make pnpm 12 a valid substitute for the pinned pnpm 10 gate.

Classification: local package-manager bootstrap/toolchain blocker plus a doctor diagnostic-contract defect. It blocks the canonical `pnpm`-based CI wrapper, but it does not prove a product source-code or test failure. The fixed 5-second doctor probe does not distinguish a wrong installed pnpm from a hung automatic project-version bootstrap and has a user-level cache side effect despite the documented read-only/no-install diagnostic role. The project must not guess removed pnpm configuration names: a planned fix should inject only the verified `pnpm_config_pm_on_fail=ignore` into the pnpm probe's child environment and regression-test both environment propagation and the mismatch result under pinned pnpm 10 and newer pnpm probes.
