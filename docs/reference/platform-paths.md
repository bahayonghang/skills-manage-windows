# Platform Paths

The full list of platforms SkillPort manages and where each one stores its skills on disk. Lobster platforms are vendor-specific Chinese coding agents grouped together in the UI.

## Coding

| Platform | Skills Directory |
| --- | --- |
| Claude Code | `~/.claude/skills/` |
| Codex CLI | `~/.agents/skills/` |
| Cursor | `~/.agents/skills/` |
| Gemini CLI | `~/.agents/skills/` |
| Trae | `~/.trae/skills/` |
| Factory Droid | `~/.factory/skills/` |
| Junie | `~/.junie/skills/` |
| Qwen | `~/.qwen/skills/` |
| Trae CN | `~/.trae-cn/skills/` |
| Windsurf | `~/.windsurf/skills/` |
| Qoder | `~/.qoder/skills/` |
| Augment | `~/.augment/skills/` |
| OpenCode | `~/.agents/skills/` |
| KiloCode | `~/.kilocode/skills/` |
| OB1 | `~/.ob1/skills/` |
| Amp | `~/.agents/skills/` |
| Kiro | `~/.kiro/skills/` |
| CodeBuddy | `~/.codebuddy/skills/` |
| Hermes | `~/.hermes/skills/` |
| Copilot | `~/.agents/skills/` |
| Aider | `~/.aider/skills/` |

## Lobster

| Platform | Skills Directory |
| --- | --- |
| OpenClaw (开爪) | `~/.openclaw/skills/` |
| QClaw (千爪) | `~/.qclaw/skills/` |
| EasyClaw (简爪) | `~/.easyclaw/skills/` |
| EasyClaw V2 | `~/.easyclaw-20260322-01/skills/` |
| AutoClaw | `~/.openclaw-autoclaw/skills/` |
| WorkBuddy (打工搭子) | `~/.workbuddy/skills-marketplace/skills/` |

## Central

| Path | Role |
| --- | --- |
| `~/.skillsmanage/skills/` | Canonical Central library (legacy directory name kept for compatibility) |
| `~/.skillsmanage/db.sqlite` | SQLite database (WAL mode) |
| `~/.skillsmanage/targets/<id>/db.sqlite` | Per-target SQLite cache for SSH targets |
| `~/.agents/skills/` | Universal Agents target shared by Codex CLI / Cursor / Gemini CLI etc. |

## Shared Roots

Multiple agents resolve to `~/.agents/skills/`. Discover collapses these into a single scan root via `services::discovery::roots.rs` so the same project skill is not surfaced N times.

## Read-only Sources

Claude Code surfaces marketplace plugin directories under `~/.claude/plugins/marketplaces/*` as read-only rows. They are display-only.

## Custom Platforms

Settings → Platforms allows adding custom platforms. SkillPort auto-generates a directory aligned with the host's home-path style:

- Windows: `C:\Users\<name>\.<id>\skills\`
- macOS / Linux: `~/.<id>/skills/`

The directory is created lazily on first install.

Last reviewed: 2026-05-04
