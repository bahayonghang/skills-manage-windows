# Build And Test

Use the repository root as the working directory. `.node-version`, `package.json`, and
`rust-toolchain.toml` pin Node 26 and Rust 1.98.0. Prefer the latest stable pnpm. The repository
does not declare a pnpm version; hosted setup requests `latest`.

`pnpm-workspace.yaml` owns dependency overrides. `verifyDepsBeforeRun: false` prevents
`pnpm run` and `pnpm exec` from installing or replacing dependencies implicitly. Install
dependencies explicitly with `pnpm install --frozen-lockfile` before running checks.

## Read-only review entry

Hosted CI and local review that must not rewrite tracked files use:

```text
node scripts/check/sync-version.mjs --check
node scripts/check/run-ci.mjs --lane quick|common|rust-platform|all
```

`just version-check` is the same version check. These commands do not install a toolchain, do not
switch PATH, and do not rewrite version metadata.

A successful `just doctor` result reports the installed pnpm version. Doctor does not query the
registry or verify that the installed version is the latest release. Direct CLI such as
`node node_modules/vitest/vitest.mjs` is **direct** evidence only; the direct command is
not a substitute for canonical `pnpm exec` or `just ci`.

`just ci` is the parent repository completion gate. This harness-rules child does not treat it as
already run.

## Fast Feedback

- `just doctor` is read-only environment diagnostics. The pnpm probe is `pnpm --version` with a 5s
  timeout. Doctor checks availability and reports the actual version. Only the probe child env
  sets `pnpm_config_pm_on_fail=ignore`. Doctor never installs packages, switches a toolchain,
  modifies PATH, or prints credentials.
- `just check` runs the quick CI lane after synchronizing version metadata locally.
- `just version-check` verifies package, Tauri, and Cargo versions without changing files.

## Python lane

The `rust-platform` lane is `clippy --all-targets --locked` → `cargo test --locked` →
`trellis-python`. Windows uses `python`; POSIX uses `python3`. The step runs
`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p test_*.py`. Missing required
inject hooks fail closed. Platform-inapplicable skips remain skips. Linux and macOS hosted
`rust-platform` runners remain **UNVERIFIED**.

## Required Gates

- `just ci` is the repository completion gate. It runs common checks, frontend validation/build,
  documentation checks, Rust formatting/IPC checks, current-platform Clippy, locked Rust tests,
  and the Trellis Python suite on the rust-platform lane.
- `just audit` checks dependency advisories.
- Frontend changes normally require `pnpm typecheck` and `pnpm lint`; interaction/state changes
  also require the focused Vitest tests.
- Rust changes normally require `cd src-tauri; cargo fmt --all -- --check`,
  `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`, and
  `cd src-tauri; cargo test --locked`.

## Desktop Build

- `just dev` starts Tauri development mode.
- `just build` runs the platform bundle and copies the newest Windows NSIS installer into
  `outputs/`.
- `just install` builds first, then runs the newest Windows installer passively.
- Packaging changes require a successful `pnpm tauri build` and confirmation that the installer
  artifact exists. A frontend-only build is insufficient.

`just check`, `just ci`, and `just build` synchronize version metadata locally; CI workflows use
the read-only review entry above and fail on drift instead of rewriting tracked files.
