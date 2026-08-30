# Build And Test

Use the repository root as the working directory. The toolchain is pinned by `.node-version`,
`package.json`, and `rust-toolchain.toml`: Node 26, pnpm 10.34.5, and Rust 1.98.0.

## Fast Feedback

- `just doctor` diagnoses the local toolchain without installing dependencies or changing PATH.
- `just check` runs the quick CI lane after synchronizing version metadata.
- `just version-check` verifies package, Tauri, and Cargo versions without changing files.

## Required Gates

- `just ci` is the repository completion gate. It runs common checks, frontend validation/build,
  documentation checks, Rust formatting/IPC checks, current-platform Clippy, and locked Rust tests.
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
read-only version checks and fail on drift instead of rewriting tracked files.
