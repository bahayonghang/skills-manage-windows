# SkillPort

`SkillPort` is a Tauri desktop app for managing AI coding agent skills across multiple platforms from one place.

[中文文档](README_CN.md)

> **Disclaimer**
>
> `SkillPort` is an independent, unofficial desktop application for managing local skill directories and importing public skill metadata. It is not affiliated with, endorsed by, or sponsored by Anthropic, OpenAI, GitHub, MiniMax, or any other supported platform, publisher, or trademark owner.

## Overview

`SkillPort` follows the [Agent Skills](https://github.com/anthropics/agent-skills) open pattern, but keeps its private Central library in `~/.skillsmanage/skills/` by default. On the Local target, the Central page can change this location with a previewed migrate-and-switch flow: current Central skills overwrite same-name target skills, target-only skills are kept and scanned in, and the old directory is not deleted. The shared Universal Agents target remains `~/.agents/skills/`, so only skills explicitly installed there are exposed to Codex CLI, Cursor, OpenCode, Amp, Copilot, and other tools that read that location. Grok is managed as its own upstream-compatible target at `~/.grok/skills/`, with project installs under `.grok/skills/`. SkillPort distinguishes Google's Antigravity app target from Antigravity CLI: Antigravity global skills stay in `~/.gemini/antigravity/skills/`, Antigravity CLI global skills use `~/.gemini/antigravity-cli/skills/`, and both use `.agents/skills/` for workspace/project installs. Gemini CLI remains available as a legacy/shared Google target at `~/.gemini/skills/`.

## Relationship to upstream

`SkillPort` is derived from [`iamzhihuix/skills-manage`](https://github.com/iamzhihuix/skills-manage). This fork is independently maintained and distributed. The upstream project remains credited as the original base, while this fork keeps a Windows-first build and release contract for installer packaging and release workflows.

## Highlights

- Central skill library plus per-platform install and uninstall flows.
- Claude Code can surface native skills and read-only marketplace plugin skills in one platform view.
- Full skill detail view with Markdown preview, raw source view, and AI explanation generation.
- Collections for organizing skills and batch-installing them to platforms.
- Discover scan for project-level skill libraries on local disks.
- Marketplace browsing and GitHub repository import with authenticated requests and retry fallback.
- Fast search for large skill libraries with deferred queries, lazy indexing, and virtualization.
- Bilingual UI, Catppuccin themes, accent colors, onboarding, and responsive navigation.
- **Central Library V2 (default)**: structured query syntax (`tag:`, `repo:`, `owner:`, `has:source`, etc.), URL-as-state, saved views, command palette (`Ctrl+K`), tag groups, group-by views (none / repository / owner / tag / status). Use the "Switch to classic layout" link in the Beta badge area, or set `featureFlag.central.newLayout=off` in DevTools localStorage, to fall back to the V1 layout.

## SSH Remote Mode

SkillPort can manage a remote Linux or macOS user's global skills through SSH. The desktop UI still runs locally, while the backend connects to the selected target and scans the remote user's Central and platform skill directories.

- Add, test, delete, and switch SSH targets from Settings.
- SSH targets support key-based and password-based OpenSSH login. Private key contents are never stored; password login stores the password in the system credential store instead of SQLite.
- Remote HOME is detected after connection, then remote Central Skills use `~/.skillsmanage/skills/`, Universal Agents use `~/.agents/skills/`, and Grok uses `~/.grok/skills/` on that host.
- Each SSH target has its own local cache database under `~/.skillsmanage/targets/<target_id>/db.sqlite`.
- Remote installs use copy mode by default. Symlink install and remote Discover project scanning are not enabled in this version.
- File-manager actions are replaced by copying the remote path, because the path exists on the remote host, not on the local machine.

Remote mode manages the selected remote user's directories only. It does not modify local skills unless the active target is switched back to Local.

## Screenshots

### Central skills and platform installs

![Central skills library view](images/01.png)

### Review installed skills on a specific platform

![Platform skill view](images/06.png)

### Discover local project skill libraries

![Discover project skill libraries](images/03.png)

### Browse marketplace publishers and skills

![Marketplace view](images/04.png)

### Import skills from a GitHub repository

![GitHub repository import wizard](images/02.png)

### Organize reusable collections

![Skill collections view](images/05.png)

## Local CLI

The `skillport-cli` binary uses the same Local SQLite database, stable skill `uid`, GitHub/skills.sh import services, installation service, and cross-process Central mutation lock as the desktop app.

```powershell
npm run cli -- skills list
npm run cli -- skills show <uid-or-slug-or-unique-name>
npm run cli -- skills search "react" --limit 10
npm run cli -- skills install vercel-labs/agent-skills@react-best-practices --sync
npm run cli -- skills sync <uid-or-slug> --agent codex --method copy --dry-run
```

Duplicate installs stop by default; use `--replace` explicitly, and add `--yes` when replacing multiple skills from one GitHub URL. The first CLI release manages only the Local target. A running desktop window does not receive a push event from CLI changes, so refresh the relevant view to reload state. GitHub credentials continue to come from SkillPort's existing protected secret store.

Install the binary on `PATH` with:

```powershell
cargo install --path src-tauri --bin skillport-cli --locked --force
```

See the [full SkillPort CLI reference](docs/reference/skillport-cli.md) for command
options, JSON output, exit codes, duplicate safety, and sync workflows.

## Download

- Latest release: <https://github.com/bahayonghang/skills-manage-windows/releases/latest>
- Current desktop release targets: Windows x64 (`.exe`, `.msi`, `.zip`), macOS Universal (`.dmg`, `.zip`, `.tar.gz`), and Linux x86_64 / arm64 (`.deb`, `.rpm`, `.AppImage`)
- Windows auto-update uses a Tauri-signed NSIS artifact plus `latest.json`; macOS remains unsigned / not notarized, and Linux arm64 availability depends on the GitHub Actions runner matrix
- Maintainers: before publishing a desktop tag, run the scripted release preflight documented in `docs/reference/release-process.md` to validate the updater config, NSIS signature, and `latest.json`.

### macOS Unsigned Build

The current public macOS build is not notarized. If macOS shows a warning such as:

![macOS damaged app warning](images/app-damaged.png)

- `"SkillPort" is damaged and can't be opened`
- `"SkillPort" cannot be opened because Apple could not verify it`

the app is usually not actually corrupted; it is being blocked by Gatekeeper quarantine on an unsigned build.

After moving the app to `/Applications`, run:

```bash
xattr -dr com.apple.quarantine "/Applications/SkillPort.app"
```

Then launch the app again from Finder. If your app is stored somewhere else, replace the path with the actual `.app` path.

## Supported Platforms

| Category | Platform | Skills Directory |
|----------|----------|-----------------|
| Coding | Claude Code | `~/.claude/skills/` |
| Coding | Codex CLI | `~/.agents/skills/` |
| Coding | Grok | `~/.grok/skills/` |
| Coding | Cursor | `~/.agents/skills/` |
| Coding | Antigravity | `~/.gemini/antigravity/skills/` |
| Coding | Antigravity CLI | `~/.gemini/antigravity-cli/skills/` |
| Coding | Zed (community-compatible) | `~/.config/zed/skills/` |
| Coding | Gemini CLI (legacy) | `~/.gemini/skills/` |
| Coding | Trae | `~/.trae/skills/` |
| Coding | Factory Droid | `~/.factory/skills/` |
| Coding | Junie | `~/.junie/skills/` |
| Coding | Qwen | `~/.qwen/skills/` |
| Coding | Trae CN | `~/.trae-cn/skills/` |
| Coding | Windsurf | `~/.windsurf/skills/` |
| Coding | Qoder | `~/.qoder/skills/` |
| Coding | Augment | `~/.augment/skills/` |
| Coding | OpenCode | `~/.agents/skills/` |
| Coding | KiloCode | `~/.kilocode/skills/` |
| Coding | OB1 | `~/.ob1/skills/` |
| Coding | Amp | `~/.agents/skills/` |
| Coding | Kiro | `~/.kiro/skills/` |
| Coding | CodeBuddy | `~/.codebuddy/skills/` |
| Coding | Hermes | `~/.hermes/skills/` |
| Coding | Copilot | `~/.agents/skills/` |
| Coding | Aider | `~/.aider/skills/` |
| Lobster | OpenClaw (开爪) | `~/.openclaw/skills/` |
| Lobster | QClaw (千爪) | `~/.qclaw/skills/` |
| Lobster | EasyClaw (简爪) | `~/.easyclaw/skills/` |
| Lobster | EasyClaw V2 | `~/.easyclaw-20260322-01/skills/` |
| Lobster | AutoClaw | `~/.openclaw-autoclaw/skills/` |
| Lobster | WorkBuddy (打工搭子) | `~/.workbuddy/skills-marketplace/skills/` |
| Central | Central Skills | `~/.skillsmanage/skills/` |

> Note: Claude Code also surfaces marketplace plugin directories under `~/.claude/plugins/marketplaces/*` as read-only rows in the Claude view. Those entries are display-only and are not managed like native skills in `~/.claude/skills/`. Antigravity plugin bundles are a separate CLI plugin mechanism; SkillPort currently manages Google `SKILL.md` skill folders only, not plugin bundle import/export. Zed is listed with a community-compatible skills path; SkillPort does not claim this is an official Zed-native skills specification.

Custom platforms can be added through Settings.

## Privacy & Security

- **Local-first storage** — metadata, collections, scan results, settings, and cached AI explanations stay in `~/.skillsmanage/db.sqlite` or the local skill directories you manage. The `.skillsmanage` path is kept for compatibility with existing installations.
- **No telemetry** — the app does not include analytics, crash reporting, or usage tracking.
- **Network access is feature-driven** — outbound requests only happen when you explicitly use marketplace sync/download, GitHub import, or AI explanation generation.
- **SSH is target-scoped** - SSH connections are made only for the active remote target, and remote file changes stay under that remote user's configured skills directories.
- **Credentials stay on this device** — GitHub PATs, AI API keys, and SSH passwords are saved through the OS credential store when available. On Windows, if the credential store is unavailable, SkillPort falls back to an app-local DPAPI-protected secret file under `~/.skillsmanage/protected-secrets/`.
- **Legacy secret migration** — older GitHub PAT and AI API key values found in SQLite settings are migrated to the secret store and then removed from settings. If no persistent protected store is available, the value is kept only for the current app session.
- Never post real secrets in issues, pull requests, screenshots, or logs.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri v2 |
| Frontend | React 19, TypeScript, Tailwind CSS 4 |
| UI components | shadcn/ui, Lucide icons |
| State management | Zustand |
| Markdown | react-markdown |
| i18n | react-i18next, i18next-browser-languagedetector |
| Theming | Catppuccin 4-flavor palette |
| Backend | Rust (serde, sqlx, chrono, uuid) |
| Database | SQLite via sqlx (WAL mode) |
| Routing | react-router-dom v7 |

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [pnpm](https://pnpm.io/)
- [Rust toolchain](https://rustup.rs/) (stable)
- Tauri v2 system dependencies: <https://v2.tauri.app/start/prerequisites/>

### Install Dependencies

```bash
pnpm install
```

### Common Just Commands

```bash
just ci
just dev
just build
just install
```

- `just ci` runs the frontend chain (`typecheck` → `lint` → `sizecheck` → `test`) in parallel with the Rust chain (`cargo clippy` → `cargo test`).
- `just dev` starts the Tauri development app directly.
- `just build` builds the desktop app for the current platform and copies the latest bundle artifact to `outputs/` (`.exe` on Windows, `.app` + `.dmg` on macOS, `.AppImage`/`.deb` on Linux).
- `just install` builds the Windows NSIS installer, copies it to `outputs/`, and runs it in passive mode. On macOS, it prints a reminder and runs `just build` instead.

### Run in Development

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

## Project Structure

```text
skillport/
├── src/                        # React frontend
│   ├── components/             # UI components
│   ├── i18n/                   # Locale files and i18n setup
│   ├── lib/                    # Frontend helpers
│   ├── pages/                  # Route views
│   ├── stores/                 # Zustand stores
│   ├── test/                   # Vitest + RTL tests
│   └── types/                  # Shared TypeScript types
├── src-tauri/                  # Rust backend
│   └── src/
│       ├── commands/           # Tauri IPC handlers
│       ├── db.rs               # SQLite schema, migrations, queries
│       ├── lib.rs              # Tauri app setup
│       └── main.rs             # Desktop entry point
├── docs/                       # VitePress docs, product notes, design assets
├── public/                     # Static assets
├── scripts/                    # Build and maintenance helpers
├── CHANGELOG.md                # English changelog
├── CHANGELOG.zh.md             # Chinese changelog
└── release-notes/              # GitHub release notes
```

## Database

The SQLite database lives at `~/.skillsmanage/db.sqlite` and is initialized automatically on first launch. This legacy directory name is retained so existing installations keep using their current data.

## Changelog

- English: [CHANGELOG.md](CHANGELOG.md)
- Chinese: [CHANGELOG.zh.md](CHANGELOG.zh.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, validation commands, and pull request expectations.

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting and data-handling notes.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=bahayonghang/skills-manage-windows&type=Date)](https://www.star-history.com/#bahayonghang/skills-manage-windows&Date)

## Documentation

A bilingual VitePress documentation site lives under `docs/`. To preview locally:

```bash
pnpm docs:dev
pnpm docs:build
pnpm docs:preview
```

The English entry point is `/`, and the Chinese mirror is `/zh/`. Build output is written to `dist-docs/` at the repository root.

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE).
