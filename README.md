# SkillPort

`SkillPort` is a Tauri desktop app for managing AI coding agent skills across multiple platforms from one place.

[中文文档](README_CN.md)

> **Disclaimer**
>
> `SkillPort` is an independent, unofficial desktop application for managing local skill directories and importing public skill metadata. It is not affiliated with, endorsed by, or sponsored by Anthropic, OpenAI, GitHub, MiniMax, or any other supported platform, publisher, or trademark owner.

## Overview

`SkillPort` follows the [Agent Skills](https://github.com/anthropics/agent-skills) open pattern, but keeps its private Central library in `~/.skillsmanage/skills/`. The shared Universal Agents target remains `~/.agents/skills/`, so only skills explicitly installed there are exposed to Codex CLI, Cursor, Gemini CLI, and other tools that read that location.

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

## SSH Remote Mode

SkillPort can manage a remote Linux or macOS user's global skills through SSH. The desktop UI still runs locally, while the backend connects to the selected target and scans the remote user's Central and platform skill directories.

- Add, test, delete, and switch SSH targets from Settings.
- SSH targets support key-based and password-based OpenSSH login. Private key contents are never stored; password login stores the password in the system credential store instead of SQLite.
- Remote HOME is detected after connection, then remote Central Skills use `~/.skillsmanage/skills/` and Universal Agents use `~/.agents/skills/` on that host.
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

## Download

- Latest release: <https://github.com/bahayonghang/skills-manage-windows/releases/latest>
- Current prebuilt packages: Windows x64 (`.exe`, `.msi`, `.zip`) and macOS Universal (`.dmg`, `.zip`, `.tar.gz`)
- Other platforms: run from source for now

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
| Coding | Cursor | `~/.agents/skills/` |
| Coding | Gemini CLI | `~/.agents/skills/` |
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

> Note: Claude Code also surfaces marketplace plugin directories under `~/.claude/plugins/marketplaces/*` as read-only rows in the Claude view. Those entries are display-only and are not managed like native skills in `~/.claude/skills/`.

Custom platforms can be added through Settings.

## Privacy & Security

- **Local-first storage** — metadata, collections, scan results, settings, and cached AI explanations stay in `~/.skillsmanage/db.sqlite` or the local skill directories you manage. The `.skillsmanage` path is kept for compatibility with existing installations.
- **No telemetry** — the app does not include analytics, crash reporting, or usage tracking.
- **Network access is feature-driven** — outbound requests only happen when you explicitly use marketplace sync/download, GitHub import, or AI explanation generation.
- **SSH is target-scoped** - SSH connections are made only for the active remote target, and remote file changes stay under that remote user's configured skills directories.
- **Credentials are stored locally** — GitHub PAT and AI API keys are kept in the local SQLite settings table and are not encrypted at rest by the app.
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
```

- `just ci` runs frontend `typecheck` + `lint`, plus Rust `cargo test` and `cargo clippy`.
- `just dev` starts the Tauri development app directly.
- `just build` builds the desktop app and copies the latest NSIS installer from `src-tauri/target/release/bundle/nsis/` to `outputs/`.

### Run in Development

```bash
pnpm tauri dev
```

The Vite dev server runs on port `24200`.

### Validation

```bash
pnpm test
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
├── public/                     # Static assets
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

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE).
