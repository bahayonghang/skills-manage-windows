# CLI: just commands

The repository ships a `justfile` for repeatable development and packaging tasks. `just` is required: <https://just.systems>.

## Recipes

| Recipe | What it does |
| --- | --- |
| `just sync-version` | Reads `package.json` and writes the version into `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock`. Idempotent. |
| `just ci` | Runs frontend `typecheck` → `lint` → `sizecheck` → `test` → production `build` in parallel with Rust entrypoint contract → `fmt --check` → all-target Clippy → locked tests. The full local gate. |
| `just dev` | Starts the Tauri development app (`pnpm tauri dev`). |
| `just build` | Builds the desktop app for the current platform and copies the bundle to `outputs/`. |
| `just install` | On Windows, builds the NSIS installer, copies it to `outputs/`, and runs it in passive mode. On macOS, prints a reminder and runs `just build` instead. |

## Implementation

Most recipes are thin wrappers around Node scripts under `scripts/`; `just install` adds platform routing before invoking the build or install path:

```text
just sync-version  →  node scripts/sync-version.mjs
just ci            →  node scripts/run-ci.mjs
just build         →  node scripts/build.mjs
just install       →  macOS: just build; Windows: node scripts/install.mjs after just build
```

Reading the `justfile` plus the referenced Node scripts is the fastest way to learn what each recipe will do on your OS.

## Local Gate

```text
[just ci] ──► sync-version ──► scripts/run-ci.mjs
                                 │
                                 ├─ web:  pnpm typecheck
                                 │        pnpm lint
                                 │        pnpm sizecheck
                                 │        pnpm test
                                 │        pnpm build
                                 │
                                 └─ rust: pnpm entrypointcheck
                                          cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
                                          cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
                                          cargo test --manifest-path src-tauri/Cargo.toml --locked
```

The two chains run concurrently; either chain failing stops the sibling chain and fails the gate. `just ci` is what the CI workflow runs to gate PRs. Run it before pushing.

## GitHub Actions

The stable `just-ci` check runs for pull requests targeting `main`, pushes to `main` or `dev`, manual dispatches, and published releases. Windows, Linux, and macOS smoke-package jobs are intentionally limited to manual dispatches and published releases so routine pull requests get fast feedback without paying the full packaging cost.

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

Last reviewed: 2026-05-31
