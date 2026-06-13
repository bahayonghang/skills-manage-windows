# Installation

There are two paths: install a prebuilt release, or build from source.

## Prebuilt downloads

- Latest release: <https://github.com/bahayonghang/skills-manage-windows/releases/latest>
- Targets: Windows x64 (`.exe`, `.msi`, `.zip`), macOS Universal (`.dmg`, `.zip`, `.tar.gz`), Linux x86_64 / arm64 (`.deb`, `.rpm`, `.AppImage`).
- Builds are unsigned. Linux arm64 availability depends on the GitHub Actions runner matrix.

### macOS unsigned build

The current public macOS build is not notarized. If macOS shows:

- `"SkillPort" is damaged and can't be opened`
- `"SkillPort" cannot be opened because Apple could not verify it`

it is being blocked by Gatekeeper quarantine, not actually corrupted. Move the app to `/Applications`, then run:

```bash
xattr -dr com.apple.quarantine "/Applications/SkillPort.app"
```

Then launch from Finder. If your app is stored elsewhere, replace the path with the actual `.app` path.

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [pnpm](https://pnpm.io/)
- [Rust toolchain](https://rustup.rs/) (stable)
- Tauri v2 system dependencies: <https://v2.tauri.app/start/prerequisites/>

### Install dependencies

```bash
pnpm install
```

### Run in development

```bash
pnpm tauri dev
```

The Vite dev server runs on port `24200`.

### Validation

```bash
pnpm test
pnpm sizecheck
pnpm typecheck
pnpm lint
cd src-tauri && cargo test
cd src-tauri && cargo clippy -- -D warnings
```

### Just shortcuts

```bash
just ci
just dev
just build
just install
```

- `just ci` runs frontend `typecheck` + `lint` + `test` + `sizecheck`, plus Rust `cargo test` and `cargo clippy`.
- `just dev` starts the Tauri dev app.
- `just build` builds the desktop app for the current platform and copies the latest bundle to `outputs/`.
- `just install` builds the Windows NSIS installer and runs it in passive mode on Windows. On macOS, it prints a reminder and runs `just build` instead.

## Documentation site

To preview this documentation site locally:

```bash
pnpm docs:dev
pnpm docs:build
pnpm docs:preview
```

The site source lives under `docs/`. The build output is written to `dist-docs/` at the repository root, so it never collides with the desktop app build at `dist/`.

---

Last reviewed: 2026-05-04
