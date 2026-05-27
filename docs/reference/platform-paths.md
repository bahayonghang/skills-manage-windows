# Platform Paths

The full list of platforms SkillPort manages and where each one stores its skills on disk. Lobster platforms are vendor-specific Chinese coding agents grouped together in the UI.

## Coding

| Platform | Skills Directory |
| --- | --- |
| Claude Code | `~/.claude/skills/` |
| Codex CLI | `~/.agents/skills/` |
| Cursor | `~/.agents/skills/` |
| Antigravity | `~/.gemini/antigravity/skills/` |
| Antigravity CLI | `~/.gemini/antigravity-cli/skills/` |
| Gemini CLI (legacy) | `~/.gemini/skills/` |
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
| `~/.agents/skills/` | Universal Agents global target shared by Codex CLI / Cursor / OpenCode / Amp / Copilot and other universal agents. |
| `~/.gemini/skills/` | Legacy/shared Google target carried by Gemini CLI. |

## Shared Roots

Multiple global agents resolve to `~/.agents/skills/`. Antigravity and Antigravity CLI are deliberately separate globally (`~/.gemini/antigravity/skills/` and `~/.gemini/antigravity-cli/skills/`) but both share `.agents/skills/` at project/workspace scope. The legacy/shared Google path is represented by Gemini CLI at `~/.gemini/skills/`. Project scanning collapses workspace-compatible members so the same project skill is not surfaced N times.

## Read-only Sources

Claude Code surfaces marketplace plugin directories under `~/.claude/plugins/marketplaces/*` as read-only rows. They are display-only. Antigravity plugin bundles are a separate CLI plugin mechanism and are not imported/exported by SkillPort.

## Custom Platforms

Settings → Platforms allows adding custom platforms. SkillPort auto-generates a directory aligned with the host's home-path style:

- Windows: `C:\Users\<name>\.<id>\skills\`
- macOS / Linux: `~/.<id>/skills/`

The directory is created lazily on first install.

Last reviewed: 2026-05-25
