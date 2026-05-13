# CLI: just commands

The repository ships a `justfile` for repeatable development and packaging tasks. `just` is required: <https://just.systems>.

## Recipes

| Recipe | What it does |
| --- | --- |
| `just sync-version` | Reads `package.json` and writes the version into `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock`. Idempotent. |
| `just ci` | Runs frontend `typecheck` + `lint` + `test` + `sizecheck`, then Rust `cargo test` and `cargo clippy -- -D warnings`. The full local gate. |
| `just dev` | Starts the Tauri development app (`pnpm tauri dev`). |
| `just build` | Builds the desktop app for the current platform and copies the bundle to `outputs/`. |
| `just install` | Windows-only. Builds the NSIS installer, copies it to `outputs/`, and runs it in passive mode. |

## Implementation

Each recipe is a thin wrapper around a Node script under `scripts/`:

```text
just sync-version  →  node scripts/sync-version.mjs
just build         →  node scripts/build.mjs
just install       →  node scripts/install.mjs
```

Reading the Node scripts is the fastest way to learn what each recipe will do on your OS.

## Local Gate

```text
[just ci] ──► sync-version
              │
              ├── pnpm typecheck
              ├── pnpm lint
              ├── pnpm test
              ├── pnpm sizecheck
              ├── cargo test --manifest-path src-tauri/Cargo.toml
              └── cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

`just ci` is what the CI workflow runs to gate PRs. Run it before pushing.

## Outputs

`outputs/` is gitignored. `just build` populates it deterministically. Examples:

```text
outputs/
├── SkillPort_0.10.0_x64-setup.exe       (Windows, NSIS)
├── SkillPort_0.10.0_x64.msi             (Windows, MSI)
├── SkillPort_0.10.0_x64.zip             (Windows portable)
├── SkillPort_0.10.0_universal.dmg       (macOS)
├── skillport_0.10.0_amd64.deb           (Linux Debian)
├── skillport-0.10.0-1.x86_64.rpm        (Linux RPM)
└── skillport_0.10.0_amd64.AppImage      (Linux AppImage)
```

`just build` only produces artifacts for the running platform.

Last reviewed: 2026-05-04
