# Platforms

A platform is any AI coding agent or runtime that reads a `skills/` directory. SkillPort ships with 33 built-in platform definitions and lets you add custom ones from Settings.

## Categories

| Category | Meaning |
|----------|---------|
| Coding | AI coding agents and CLIs (Claude Code, Cursor, Codex, ...). |
| Lobster | OpenClaw-derived ecosystem (QClaw, EasyClaw, AutoClaw, WorkBuddy). |
| Central | The canonical Central Skills library; treated as a virtual platform. |

## Built-in platforms

| Category | Platform | Skills directory |
|----------|----------|-----------------|
| Coding | Claude Code | `~/.claude/skills/` |
| Coding | Codex CLI | `~/.agents/skills/` |
| Coding | Cursor | `~/.agents/skills/` |
| Coding | Antigravity | `~/.gemini/antigravity/skills/` |
| Coding | Gemini CLI (legacy) | `~/.agents/skills/` |
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
| Lobster | OpenClaw | `~/.openclaw/skills/` |
| Lobster | QClaw | `~/.qclaw/skills/` |
| Lobster | EasyClaw | `~/.easyclaw/skills/` |
| Lobster | EasyClaw V2 | `~/.easyclaw-20260322-01/skills/` |
| Lobster | AutoClaw | `~/.openclaw-autoclaw/skills/` |
| Lobster | WorkBuddy | `~/.workbuddy/skills-marketplace/skills/` |
| Central | Central Skills | `~/.skillsmanage/skills/` |

Several coding platforms (Codex, Cursor, OpenCode, Amp, Copilot, plus legacy Gemini CLI) read from the shared Universal Agents global path. Antigravity is Google's current recommended platform, but its global skills live at `~/.gemini/antigravity/skills/`; project/workspace skills still use the shared `.agents/skills/` directory. SkillPort does not manage Antigravity plugin bundles in this workflow.

## Platform view

Each platform shows the skills currently visible in its directory. The view supports:

- **Search** with deferred queries.
- **Inline install/uninstall** through the platform icon row on each skill card.
- **Source label** that distinguishes symlinked installs from independent copies.
- **Marketplace plugin rows** for Claude Code: read-only entries surfaced from `~/.claude/plugins/marketplaces/*` for transparency. Those rows are not managed like native skills.

## Custom platforms

Add your own platform from Settings → Custom Platforms with:

- A unique id and display name.
- The skills directory path (absolute or `~/`-relative).
- A category (Coding, Lobster, or Other).

Custom platforms participate in install, uninstall, and Discover scanning the same way as built-in ones.

## Platform visibility

You can hide platforms you do not use from Settings → Platform Visibility. Hidden platforms still scan in the background but are not rendered in the navigation.

## Where to go next

- Build a reusable bundle: [Collections](./collections).
- Connect to a remote machine: [SSH Remote](./ssh-remote).
- Adjust scan paths and visibility: [Settings](./settings).

---

Last reviewed: 2026-05-04
