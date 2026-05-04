# 平台路径

SkillPort 管理的所有平台及其磁盘上的技能目录。Lobster 平台是中文厂商出品的编码 agent，UI 中单独分组。

## 编码（Coding）

| 平台 | 技能目录 |
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

## 龙虾（Lobster）

| 平台 | 技能目录 |
| --- | --- |
| OpenClaw（开爪） | `~/.openclaw/skills/` |
| QClaw（千爪） | `~/.qclaw/skills/` |
| EasyClaw（简爪） | `~/.easyclaw/skills/` |
| EasyClaw V2 | `~/.easyclaw-20260322-01/skills/` |
| AutoClaw | `~/.openclaw-autoclaw/skills/` |
| WorkBuddy（打工搭子） | `~/.workbuddy/skills-marketplace/skills/` |

## 中央

| 路径 | 作用 |
| --- | --- |
| `~/.skillsmanage/skills/` | 中央仓库（保留兼容性命名） |
| `~/.skillsmanage/db.sqlite` | SQLite 数据库（WAL 模式） |
| `~/.skillsmanage/targets/<id>/db.sqlite` | SSH 目标的本地缓存 |
| `~/.agents/skills/` | Universal Agents 共享目录（Codex CLI / Cursor / Gemini CLI 等读取） |

## 共享根

多个 agent 都解析到 `~/.agents/skills/`。Discover 通过 `services::discovery::roots.rs` 把这些共享模式去重，避免同一项目技能在 UI 重复出现。

## 只读来源

Claude Code 把 `~/.claude/plugins/marketplaces/*` 下的市场插件目录作为只读行展示。仅展示用，不像 `~/.claude/skills/` 那样接受管理操作。

## 自定义平台

设置 → 平台 支持添加自定义平台。SkillPort 按宿主 home 路径风格自动生成目录：

- Windows：`C:\Users\<name>\.<id>\skills\`
- macOS / Linux：`~/.<id>/skills/`

目录在首次安装时按需创建。

Last reviewed: 2026-05-04
