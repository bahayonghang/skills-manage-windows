# Glossary

Vocabulary that appears across the UI, source code, and docs.

| Term | 中文 | Meaning |
| --- | --- | --- |
| Skill | 技能 | A directory with `SKILL.md` plus optional helpers; the unit SkillPort manages |
| Central Skills | 中央技能库 | The canonical store under `~/.skillsmanage/skills/` |
| Platform | 平台 | An agent that reads skills from a known directory (Claude Code, Cursor, …) |
| Agent | Agent | Synonym for Platform in source code (`agent_id`, `agents` table) |
| Lobster | 龙虾 | UI grouping for vendor-specific Chinese coding agents (OpenClaw, QClaw, …) |
| Universal Agents | 通用平台 | Shared global `~/.agents/skills/` target read by Codex CLI / Cursor / OpenCode / Amp / Copilot and other universal agents; at project scope Antigravity and Antigravity CLI also share `.agents/skills/` |
| Install | 安装 | Materialize a Central skill into a platform's skills directory |
| Symlink | 符号链接 | Default install method on Linux / macOS and Windows with Developer Mode |
| Copy | 拷贝 | Install method that duplicates the directory; default for Windows fallbacks |
| Auto | 自动 | Install method that tries symlink first and falls back to copy |
| Centralize | 集中化 | Promote a non-Central skill into Central by copying it to `~/.skillsmanage/skills/` |
| Discover | 项目发现 | Walk project directories looking for SKILL.md files not yet in Central |
| Marketplace | 市场 | Curated list of remote skill registries (GitHub repos / mirrors) |
| Registry | 源 | One row in `skill_registries`; a single remote source |
| Collection | 集合 | User-defined group of skills for batch installs and import / export |
| Repository | 技能仓库 | Local metadata grouping Central skills by their source repo |
| Tag | 标签 | Local taxonomy entry, manual or AI-suggested |
| Operation Log | 操作日志 | Structured row in `operation_logs` recording user-visible actions such as install, uninstall, scan, settings, target switch, import, or export |
| Runtime Log | 运行时日志 | Bounded daily `skillport-YYYY-MM-DD.log` file for frontend/backend diagnostics; separate from Operation Log |
| Observability Console | 可观测性控制台 | `/logs` UI with separate Operation and Runtime modes |
| Target | 目标 | Either Local or an SSH host; the destination AppState resolves for commands |
| Active Target | 活动目标 | The Target currently selected in the SSH banner |
| Vault | Vault | An Obsidian-managed directory; SkillPort source-only scans them under `/obsidian` |
| Bootstrap | 启动快照 | Cached snapshot served at app start so the Dashboard can render before scans |
| Backfill | 回填 | One-shot data migration that fills new columns with `datetime('now')` etc. |

## Naming Conventions

| Concept | Convention |
| --- | --- |
| Skill ID | Directory name (e.g. `python-style`) |
| Agent ID | Lowercase snake-case (e.g. `claude_code`) |
| Collection ID | UUID v4 |
| Tag ID | UUID v4 |
| Registry ID | UUID v4 |

## Cross-references

- Skill protocol: see [Skill Protocol](./skill-protocol.md)
- Platform → directory: see [Platform Paths](./platform-paths.md)
- Install method semantics: see [Architecture → Installation Engine](../architecture/installation-engine.md)

Last reviewed: 2026-06-03
