# Introduction

`SkillPort` is a Tauri desktop app that manages AI coding agent skills across multiple platforms from one place.

## Overview

`SkillPort` follows the [Agent Skills](https://github.com/anthropics/agent-skills) open pattern, but keeps its private Central library in `~/.skillsmanage/skills/`. The shared Universal Agents target remains `~/.agents/skills/`, so only skills explicitly installed there are exposed to Codex CLI, Cursor, OpenCode, Amp, Copilot, and other tools that read that location. Google's current recommended platform is Antigravity: SkillPort installs its global skills to `~/.gemini/antigravity/skills/`, while Antigravity project skills use `.agents/skills/`. Gemini CLI remains available as a legacy / enterprise compatibility target.

The app combines four roles in a single window:

- **Manager** for the local Central library and per-platform installs.
- **Browser** for Marketplace publishers and GitHub repository imports.
- **Discoverer** for project-level skill libraries on local disks.
- **Remote agent** for managing skills on a Linux or macOS host over SSH.

## Highlights

- Central skill library plus per-platform install and uninstall flows.
- Claude Code surfaces native skills and read-only marketplace plugin skills in one platform view.
- Full skill detail view with Markdown preview, raw source view, and AI explanation generation.
- Collections for organizing skills and batch-installing them to platforms.
- Discover scan for project-level skill libraries on local disks.
- Marketplace browsing and GitHub repository import with authenticated requests and retry fallback.
- Fast search for large skill libraries with deferred queries, lazy indexing, and virtualization.
- Bilingual UI, Catppuccin themes, accent colors, onboarding, and responsive navigation.

## Privacy

- **Local-first storage** — metadata, collections, scan results, settings, and cached AI explanations stay in `~/.skillsmanage/db.sqlite` or the local skill directories you manage.
- **No telemetry** — the app does not include analytics, crash reporting, or usage tracking.
- **Network access is feature-driven** — outbound requests only happen when you explicitly use marketplace sync/download, GitHub import, or AI explanation generation.
- **SSH is target-scoped** — SSH connections are made only for the active remote target, and remote file changes stay under that remote user's configured skills directories.
- **Credentials are stored locally** — GitHub PAT and AI API keys are kept in the local SQLite settings table.

## Disclaimer

`SkillPort` is an independent, unofficial desktop application for managing local skill directories and importing public skill metadata. It is not affiliated with, endorsed by, or sponsored by Anthropic, OpenAI, GitHub, MiniMax, or any other supported platform, publisher, or trademark owner.

## Where to go next

- Set the app up: [Installation](./installation).
- Read the project README on [GitHub](https://github.com/bahayonghang/skills-manage-windows).

---

Last reviewed: 2026-05-04
