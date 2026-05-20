# 平台

平台是指任何会读取 `skills/` 目录的 AI 编码 agent 或运行时。SkillPort 预置 33 个平台定义，并允许在设置中添加自定义平台。

## 分类

| 分类 | 含义 |
|------|------|
| Coding | AI 编码 agent 与 CLI（Claude Code、Cursor、Codex 等）。 |
| Lobster | 基于 OpenClaw 的生态（QClaw、EasyClaw、AutoClaw、WorkBuddy）。 |
| Central | 中央技能库；作为虚拟平台。 |

## 内置平台

| 类别 | 平台 | Skills 目录 |
|------|------|------------|
| Coding | Claude Code | `~/.claude/skills/` |
| Coding | Codex CLI | `~/.agents/skills/` |
| Coding | Cursor | `~/.agents/skills/` |
| Coding | Antigravity | `~/.gemini/antigravity/skills/` |
| Coding | Gemini CLI（legacy） | `~/.agents/skills/` |
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
| Lobster | OpenClaw（开爪） | `~/.openclaw/skills/` |
| Lobster | QClaw（千爪） | `~/.qclaw/skills/` |
| Lobster | EasyClaw（简爪） | `~/.easyclaw/skills/` |
| Lobster | EasyClaw V2 | `~/.easyclaw-20260322-01/skills/` |
| Lobster | AutoClaw | `~/.openclaw-autoclaw/skills/` |
| Lobster | WorkBuddy（打工搭子） | `~/.workbuddy/skills-marketplace/skills/` |
| Central | 中央技能库 | `~/.skillsmanage/skills/` |

Codex、Cursor、OpenCode、Amp、Copilot 以及 legacy Gemini CLI 读取共享的 Universal Agents 全局路径。Antigravity 是 Google 当前推荐平台，但它的全局 skills 位于 `~/.gemini/antigravity/skills/`；项目 / workspace skills 仍使用共享 `.agents/skills/` 目录。SkillPort 本轮不管理 Antigravity plugin bundle。

## 平台视图

每个平台展示当前目录里可见的 skills。视图支持：

- **搜索**，使用延迟查询。
- **内联安装/卸载**，点击卡片上的平台图标行直接切换。
- **来源标识**，区分 symlink 安装与独立副本。
- **Marketplace plugin 行**（仅 Claude Code）：以只读方式展示 `~/.claude/plugins/marketplaces/*`，仅作透明展示，不按原生 skill 管理。

## 自定义平台

进入 Settings → 自定义平台 添加：

- 唯一 id 与显示名称。
- skills 目录路径（绝对路径或 `~/` 相对路径）。
- 分类（Coding、Lobster 或 Other）。

自定义平台与内置平台一样参与安装、卸载和 Discover 扫描。

## 平台可见性

不需要的平台可在 Settings → 平台可见性 中隐藏。隐藏的平台仍在后台扫描，但不会出现在导航中。

## 下一步

- 组装一组可复用 skills：[集合](./collections)。
- 连接远程机器：[SSH 远程](./ssh-remote)。
- 调整扫描路径与可见性：[设置](./settings)。

---

Last reviewed: 2026-05-04
