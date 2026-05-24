# AI Agent Skills 管理工具 — 行业调研报告

> 调研日期：2026-04-09

---

## 一、Agent Skills 开放标准 (agentskills.io)

业界已形成统一的开放标准，由 Vercel Labs 推动，被 Anthropic、Google、OpenAI 等主要厂商采纳。

### 核心格式

每个 skill = 一个目录，包含 `SKILL.md`（YAML frontmatter + Markdown 指令）。

```
skill-name/
├── SKILL.md          # 必须：元数据 + 指令
├── scripts/          # 可选：可执行脚本
├── references/       # 可选：参考文档
└── assets/           # 可选：模板、资源
```

**`SKILL.md` frontmatter 规范：**

| 字段 | 是否必须 | 说明 |
|------|---------|------|
| `name` | 是 | 小写字母+连字符，最多 64 字符，须与目录名一致 |
| `description` | 是 | 最多 1024 字符，描述功能和触发时机 |
| `license` | 否 | 许可证名称 |
| `compatibility` | 否 | 环境要求（系统包、网络等） |
| `metadata` | 否 | 任意键值对扩展 |
| `allowed-tools` | 否 | 预授权工具列表（空格分隔） |

**最小示例：**

```yaml
---
name: code-reviewer
description: Review code changes and identify bugs. Use when the user asks for code review or feedback.
---

# Code Reviewer

## Workflow
1. Read the changed files
2. Identify potential issues...
```

### Progressive Disclosure（渐进式加载）

1. **元数据**（~100 tokens）：启动时加载所有 skills 的 `name` 和 `description`
2. **指令**（< 5000 tokens 推荐）：技能激活时加载完整 `SKILL.md`
3. **资源**（按需）：`scripts/`、`references/` 等文件按需加载

---

## 二、各平台 Skills 路径对照表

| 平台 | Project 路径 | Global 路径 |
|------|-------------|-------------|
| **Claude Code** | `.claude/skills/` | `~/.claude/skills/` |
| **Cursor** | `.agents/skills/` | `~/.cursor/skills/` |
| **Codex (OpenAI)** | `.agents/skills/` | `~/.agents/skills/` (+ `/etc/codex/skills/` admin) |
| **Antigravity (Google app)** | `.agents/skills/` | `~/.gemini/antigravity/skills/` |
| **Antigravity CLI** | `.agents/skills/` | `~/.gemini/antigravity-cli/skills/` |
| **Gemini CLI (legacy / enterprise)** | `.agents/skills/` | `~/.gemini/skills/` |
| **Trae** | `.trae/skills/` | `~/.trae/skills/` |
| **Trae CN** | `.trae/skills/` | `~/.trae-cn/skills/` |
| **Factory Droid** | `.factory/skills/` | `~/.factory/skills/` |
| **OpenClaw** | `skills/` | `~/.openclaw/skills/` |
| **QClaw** | 待确认 | 待确认 |
| **EasyClaw** | 待确认 | 待确认 |
| **AutoClaw/WorkBuddy** | 待确认 | 待确认 |
| **Universal** | `.agents/skills/` | `~/.agents/skills/` |
| **Cline / Warp** | `.agents/skills/` | `~/.agents/skills/` |
| **GitHub Copilot** | `.agents/skills/` | `~/.copilot/skills/` |
| **Windsurf** | `.windsurf/skills/` | `~/.codeium/windsurf/skills/` |
| **OpenCode** | `.agents/skills/` | `~/.config/opencode/skills/` |
| **Goose** | `.goose/skills/` | `~/.config/goose/skills/` |
| **Junie** | `.junie/skills/` | `~/.junie/skills/` |
| **Kilo Code** | `.kilocode/skills/` | `~/.kilocode/skills/` |
| **Roo Code** | `.roo/skills/` | `~/.roo/skills/` |
| **Augment** | `.augment/skills/` | `~/.augment/skills/` |
| **Amp** | `.agents/skills/` | `~/.config/agents/skills/` |
| **Qwen Code** | `.qwen/skills/` | `~/.qwen/skills/` |

**2026-05-25 更新**：SkillPort 明确区分 Google 的 Antigravity 应用目标与 Antigravity CLI。Antigravity 全局技能目录仍是 `~/.gemini/antigravity/skills/`；Antigravity CLI 全局技能目录是 `~/.gemini/antigravity-cli/skills/`；两者的项目 / workspace 技能目录都是 `.agents/skills/`。Gemini CLI 保留为 legacy/shared Google 兼容平台，承载 `~/.gemini/skills/`。

**关键发现**：Codex、Cursor、Cline、Copilot、OpenCode、Amp、Antigravity、Antigravity CLI 等多个平台都兼容 `.agents/skills/` 作为项目或通用路径，这使得 `~/.agents/skills/` 仍是重要的 Universal Agents 目标。Gemini CLI 的全局 shared / legacy 目标则由 `~/.gemini/skills/` 承载。

---

## 三、各平台 Skills 机制详述

### Claude Code (Anthropic)

- **invocation**：用户输入 `/skill-name` 或 Claude 自动识别触发
- **frontmatter 扩展字段**：
  - `disable-model-invocation: true` — 仅用户可触发
  - `user-invocable: false` — 仅 Claude 可触发
  - `context: fork` — 在子 agent 中运行
  - `allowed-tools` — 预授权工具
  - `effort`, `model` — 控制推理强度和模型
  - `hooks` — skill 生命周期钩子
  - `paths` — glob 模式限制触发范围
- **特性**：支持 `$ARGUMENTS` 占位符、Shell 注入（`` `!command` ``）、子 Agent 集成
- **Plugin 生态**：支持 `.claude-plugin/marketplace.json` 插件市场机制

### Trae (ByteDance)

- **分类**：Global Skills（跨项目）/ Project Skills（项目级）
- **加载方式**：按需加载（扫描描述 -> 按需加载全文），节省 token
- **与 Rules 的区别**：Rules 全量注入上下文；Skills 按需加载
- **与 MCP 的关系**：MCP 提供工具，Skills 描述如何使用这些工具
- **支持格式**：上传 SKILL.md 或 .zip 文件导入
- **管理**：Settings > Rules & Skills 可视化管理

### Codex (OpenAI)

- **Scopes**：REPO / USER (`~/.agents/skills/`) / ADMIN (`/etc/codex/skills/`) / SYSTEM
- **支持 symlink**：文档明确说明支持软链接目录
- **可选元数据**：`agents/openai.yaml` — 控制 UI 展示、调用策略、工具依赖
- **Plugin 机制**：可将 skills 打包为 plugin 分发
- **禁用配置**：`~/.codex/config.toml` 中通过 `[[skills.config]]` 条目禁用

### Antigravity / Antigravity CLI / Gemini CLI (Google)

- **Antigravity（应用目标）**：全局 skills 使用 `~/.gemini/antigravity/skills/`，项目 / workspace skills 使用 `.agents/skills/`。
- **Antigravity CLI**：全局 skills 使用 `~/.gemini/antigravity-cli/skills/`，项目 / workspace skills 使用 `.agents/skills/`。
- **Gemini CLI（legacy / enterprise）**：保留 `~/.gemini/skills/` / `.agents/skills/` 兼容语义，避免破坏既有本地目录和数据库行。
- **插件边界**：Antigravity plugin bundle 属于 CLI 插件机制，不等同于 SkillPort 当前管理的 `SKILL.md` 技能目录；本轮不实现 plugin import/export。
- **配置文件**：Google 平台仍可能通过 settings / 项目说明文件控制行为，SkillPort 只管理 skill 文件夹落点。

### Factory Droid

- **兼容路径**：`.agent/skills/`（注意单数 agent）和 `.factory/skills/` 均支持
- **Droids 概念**：自定义子 Agent，可预加载 skills
- **命令迁移**：`.factory/commands/` 与 skills 统一，旧命令仍兼容
- **Enterprise**：支持 managed settings 企业级分发

### OpenClaw (开源 AI Agent 平台)

- **配置文件**：`~/.openclaw/openclaw.json` 下的 `skills` 节点
- **配置字段**：`allowBundled`、`load.extraDirs`、`load.watch`、`entries.<skillKey>`
- **QClaw**：腾讯基于 OpenClaw 的一键桌面版（Lobster 平台）
- **EasyClaw**：Easylab 开发的 OpenClaw 前端，2.0 引入多 Agent 系统
- **AutoClaw/WorkBuddy**：Zhipu AI 的浏览器自动化 Agent，走 OpenClaw 标准

---

## 四、现有管理工具对比分析

### 4.1 npx skills (vercel-labs/skills)

- **Stars**：13,300+（行业最大）
- **支持平台**：44+
- **安装源**：GitHub shorthand / 完整 URL / GitLab / git URL / 本地路径
- **核心命令**：`add`, `remove`, `list`, `find`, `check`, `update`, `init`
- **安装方法**：Symlink（推荐）/ Copy
- **Global 路径**：`~/.agents/skills/` 作为 Universal Agents canonical 源；Antigravity 与 Antigravity CLI 全局技能分别例外使用 `~/.gemini/antigravity/skills/`、`~/.gemini/antigravity-cli/skills/`；Gemini CLI legacy/shared 目标使用 `~/.gemini/skills/`
- **Lock 文件**：`~/.agents/.skill-lock.json`

**已知问题（来自 GitHub Issues）**：
- [#851] 全局安装 `--agent claude-code` 时不创建 `~/.claude/skills/` 软链接
- [#694] Global install 未为非 universal agent 创建 agent-specific symlinks
- [#423] `skills update` 在意料之外的 agent 目录创建软链接
- [#304] 缺乏目录级软链支持（`~/.cursor/skills` -> `~/.agents/skills`）

### 4.2 SkillsGate（参考项目）

**形态**：CLI + TUI（Ink/Bun）+ Desktop（Electron）+ Web（CF Workers）+ MCP Server

**架构**（monorepo）：
```
apps/
  desktop/    # Electron + React
  web/        # React Router v7 on CF Workers
packages/
  cli/        # 核心 CLI (skillsgate)
  tui/        # Terminal UI (Bun + Ink)
  local-db/   # SQLite (WAL 模式)
  ui/         # 共享 React 组件
```

**核心模块**：
- `core/agents.ts` — Agent 注册表（20个平台），每项包含 `skillsDir`、`globalSkillsDir`、`detectInstalled()`
- `core/installer.ts` — 安装/软链/卸载，`CANONICAL_SKILLS_DIR = ~/.agents/skills`
- `core/skill-discovery.ts` — 29+ 优先目录扫描 + 递归 fallback
- `core/skill-lock.ts` — lock 文件读写
- `core/scanners.ts` — AI 安全扫描（调用 claude/codex/opencode/goose/aider）
- `mcp/server.ts` — MCP Server 模式（stdio transport）

**软链接机制**：
```
安装时：
1. clone repo 到 tmp 目录
2. 发现 SKILL.md，解析 frontmatter
3. 写入 canonical: ~/.agents/skills/<name>/
4. 为每个 agent 创建相对路径软链:
   ~/.claude/skills/<name> -> ../../.agents/skills/<name>
5. 更新 ~/.agents/.skill-lock.json
```

**安全扫描功能**：
- 检测类别：prompt injection、data exfiltration、malicious shell commands、credential harvesting、social engineering、suspicious network access、file system abuse、obfuscation
- 输出：JSON 格式 `{ risk, findings[], summary }`
- 支持的扫描器：claude-code、codex-cli、opencode、goose、aider

**与 npx skills 的关系**：代码注释 "Portions adapted from vercel-labs/skills"，属于增强版实现。

### 4.3 localskills.sh

- 团队协作 + 版本控制 + skills 发布
- 支持 CLI / API 发布
- 闭源 SaaS，目前 Beta 阶段

### 4.4 其他工具

| 工具 | 特点 |
|------|------|
| `skill-rule` (@ngxtm) | 跨平台规则同步，TypeScript |
| `code-ai-installer` | 多平台安装，VSCode Copilot/GPT/Claude/Qwen |
| `ai-agent-skills` | npm 包封装 |
| `aialchemylabs/ai-agentic-rules` | 模块化规则系统 |
| `ruler` (intellectronica) | 跨 agent 规则同步 |
| `skillsio` | 安全扫描前置 |
| `samibs/skillfoundry` | 质量门控框架 |

---

## 五、市场/发现平台

| 平台 | 领域 | 特点 |
|------|------|------|
| **skills.sh** | 通用（偏编程） | Vercel 官方市场，91k+ skills |
| **ClawHub** | OpenClaw 生态 | 龙虾平台专属，电商/自动化类 |
| **agentwiki.org** | 知识库 | AI Agent 知识文档 |
| **aiagenttools.ai** | 工具目录 | 平台无关 skills 评测 |

---

## 六、软链接策略深度分析

### Skill 级软链（npx skills / SkillsGate 采用）

```
~/.agents/skills/
  frontend-design/    ← 真实文件
  code-reviewer/      ← 真实文件

~/.claude/skills/
  frontend-design -> ../../.agents/skills/frontend-design   ← 软链
  code-reviewer   -> ../../.agents/skills/code-reviewer     ← 软链

~/.cursor/skills/
  frontend-design -> ../../.agents/skills/frontend-design   ← 软链
```

**优点**：可选择性地为不同 agent 安装不同 skills
**缺点**：管理大量软链时较繁琐，容易出现孤立软链

### 目录级软链（社区提案 #304）

```
~/.agents/skills/
  frontend-design/    ← 真实文件

~/.cursor/skills -> ~/.agents/skills    ← 整个目录软链
~/.claude/skills -> ~/.agents/skills    ← 整个目录软链
```

**优点**：极简，任何新 skill 自动对所有 agent 生效
**缺点**：无法精细控制哪个 agent 用哪些 skills

**skills-manage 选择**：Skill 级软链（更灵活），但提供 `doctor` 命令诊断孤立软链。

---

## 七、行业趋势与洞察

1. **标准化加速**：agentskills.io 已成为实际标准，主要厂商均采纳
2. **`.agents/skills/` 成为通用路径**：多个平台同时兼容此路径
3. **安全意识提升**：vercel-labs/skills 对 OpenClaw 发出警告（大量重复和恶意 skills）
4. **MCP 与 Skills 协同**：Skills 描述工作流，MCP 提供工具，两者互补
5. **Plugin 生态**：Claude Code 和 Codex 均有 Plugin/Marketplace 概念，将 skills 打包分发
6. **龙虾平台（Lobster）崛起**：QClaw、EasyClaw、AutoClaw 等基于 OpenClaw 的中文平台快速增长
7. **行业垂直化需求**：电商、自媒体、视频创作等非编程领域 skills 需求旺盛
