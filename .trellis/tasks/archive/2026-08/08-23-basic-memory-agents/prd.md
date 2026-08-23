# 为 8 个编码客户端配置 Basic Memory

## Goal

在本机把 [Basic Memory](https://github.com/basicmachines-co/basic-memory) 接到 Claude Code、Codex、Cursor、Grok Build、OMP、Kimi Code、OpenCode、Antigravity CLI。这 8 个客户端读写同一套 Markdown 知识图，会话之间不必重新解释项目。

用户价值：一次写入，任意客户端可检索、续写、沿 wikilink 展开上下文。Obsidian 打开同一目录作为 vault。

## Background

Basic Memory 是 AGPL-3.0、local-first 的 MCP 知识库。实体是带 Observations / Relations 的 Markdown。MCP 工具包括 `write_note`、`read_note`、`search_notes`、`build_context`、`recent_activity`。索引在 `~/.basic-memory`，笔记在 project 目录。

本任务是本机客户端接入，不是 SkillPort 产品功能。SkillPort 仓库只保留 Trellis 任务文件。

## Key decisions

| 决策 | 选择 |
|---|---|
| 安装形态 | Local：`uv tool install basic-memory` + stdio `uvx basic-memory mcp`。不使用 Cloud。 |
| 配置范围 | 用户全局。8 个客户端写用户级配置。跨仓库共用一份图。 |
| Claude / Codex plugin | 本轮安装。配置写用户级。 |
| 知识目录 | `D:\Documents\LYH\100-Work\100-Notes\basic_memory`。project 名 `basic_memory`。独立 git 仓库，Obsidian 管理。 |
| Grok 原生 memory | 保持双轨。不关闭 `[memory] enabled`。 |

## Confirmed facts

- 上游：`basicmachines-co/basic-memory`，PyPI `basic-memory`，Python 3.12+。stdio MCP 按 `project.mode` 走 local/cloud；本任务只有 local project。
- `uv` 0.12.5 已装；`uvx.exe` 在 `C:\Users\lyh\scoop\shims\uvx.exe`。`basic-memory` 未装。
- 知识目录已 `git init`（`main`，root commit `81d93aa`），含 Obsidian/OS/本地状态 `.gitignore`。
- Claude `mcp add` 默认 `--scope local`（项目级）；本任务必须 `--scope user`。
- Grok `mcp add` 默认 `--scope user`。Grok 兼容读取 Claude/Cursor MCP；仍写 Grok 自己的用户配置，避免只靠兼容层。
- OMP（Oh My Pi）会发现 Claude/Cursor/Codex/OpenCode 的 MCP。原生文件 `~/.omp/agent/mcp.json` 与外部源不得用同一名字注册两套不同命令。
- Claude `~/.claude/settings.json` 已有 `hooks` / `enabledPlugins` / `extraKnownMarketplaces`，无 `basicMemory`。合并写入，不整文件覆盖。
- Codex 尚无 `~/.codex/basic-memory.json`。Codex plugin 可能自带 MCP；若已提供 `basic-memory` server，不再用不同命令重复注册。
- Codex hooks 需用户在 `/hooks` 中信任。实施无法代替点击。

### 用户级接入点

| 客户端 | 写入位置 | 注册方式 |
|---|---|---|
| Claude Code | `~/.claude.json` | `claude mcp add --scope user basic-memory -- uvx basic-memory mcp` |
| Codex | `~/.codex/config.toml` | `[mcp_servers.basic-memory]`，或 plugin 自带 MCP |
| Cursor | `~/.cursor/mcp.json` | `mcpServers.basic-memory` stdio |
| Grok Build | `~/.grok/config.toml` | `grok mcp add --scope user` 或手写 `[mcp_servers.basic-memory]` |
| OMP | `~/.omp/agent/mcp.json` | 原生 stdio 条目，命令与其它客户端一致 |
| Kimi Code | `~/.kimi-code/mcp.json` | `mcpServers` JSON |
| OpenCode | `~/.config/opencode/opencode.json` | `mcp.basic-memory.type = local`，`command` 为数组 |
| Antigravity CLI | `~/.gemini/config/mcp_config.json` | `mcpServers` stdio；不往 `settings.json` 再写一份 |

不写项目级：`.cursor/mcp.json`、`.grok/config.toml`、`.agents/mcp_config.json`、`.omp/mcp.json`、`.kimi-code/mcp.json`、SkillPort 内 `.claude/settings.json`。

## Requirements

1. `uv tool install basic-memory`，`basic-memory --version` 可用。
2. `basic-memory project add basic_memory` 指向 `D:\Documents\LYH\100-Work\100-Notes\basic_memory`，设为 default；`basic-memory status` 健康。
3. 在上表 8 个用户级位置注册名为 `basic-memory` 的 stdio server，启动命令为已验证可 spawn 的 `uvx basic-memory mcp`（Windows 可用 `uvx.exe` 绝对路径或 `cmd` 包装）。
4. 每个客户端能列出至少 `search_notes` / `write_note` / `read_note` 之一；用 CLI 写入一条测试笔记后，第二个客户端配置侧能搜到同一 permalink。
5. 现有 MCP 保持：Claude `exa`，Gemini/Antigravity `playwright`（及其它已有项）。Grok `[memory] enabled` 保持 `true`。
6. 不把 PAT、API key、cloud token 写入仓库、日志或错误串。不登录 Cloud。
7. 安装 Claude plugin `basic-memory@basicmachines-co`，在 `~/.claude/settings.json` 合并写入 `basicMemory.primaryProject = "basic_memory"`。
8. 安装 Codex plugin `codex@basic-memory`，写入 `~/.codex/basic-memory.json`，`primaryProject` 同为 `basic_memory`。
9. 新会话可发现 Claude `/basic-memory:*` 与 Codex `bm-*` skills。Codex `/hooks` 信任作为用户步骤记入实施记录。
10. 知识目录保持独立 git 仓库；`.obsidian/` 与 `.basic-memory/` 不被跟踪。不把该仓库加入 SkillPort。vault 含 `START.md`、`MEMORY_POLICY.md` 以及 `core/`、`context/`、`procedures/`、`projects/`、`decisions/`、`sessions/`、`codex/`、`archive/`、`templates/`、`schemas/`。LYHNotes / claude-mem / 各客户端原生 memory 不导入、不关闭。
11. SkillPort 产品代码、i18n、IPC、安装包链路不改。实施后 SkillPort `git status` 只允许 Trellis 任务文件进入工作区。

## Constraints

- 各客户端写各自官方文件，不提交一份仓库内 `.mcp.json` 充当统一配置。
- 编辑用户 JSON/TOML 前先备份；只追加或合并 `basic-memory` 相关键。
- Windows stdio 以实际 spawn 为准，不以文档里的 Unix `uvx` 字面值为准。
- plugin 安装为用户级。SkillPort 仓库不添加 marketplace 或 `basicMemory` 块。

## Acceptance Criteria

- [ ] `basic-memory --version` 有输出。
- [ ] `basic-memory project list` 显示 default project `basic_memory`，路径为 `D:\Documents\LYH\100-Work\100-Notes\basic_memory`。
- [ ] 该路径仍是 git 仓库；`.gitignore` 仍忽略 `.obsidian/` 与 `.basic-memory/`。
- [ ] vault 含 `START.md` 与 `MEMORY_POLICY.md`；目录 `core/`、`projects/`、`decisions/`、`sessions/`、`codex/` 存在。
- [ ] 上表 8 个用户级配置均含名为 `basic-memory` 的 MCP server，指向本机 stdio；项目级 MCP 文件不新增该 server。
- [ ] `basic-memory tool` 或一次 MCP 调用写入测试笔记后，能按 permalink 再读到；笔记文件落在知识目录内。
- [ ] Claude `exa`、Antigravity/Gemini `playwright` 仍在；Grok `[memory] enabled` 仍为 true。
- [ ] `claude plugin list` 含 `basic-memory@basicmachines-co`（或等价已安装标识）；`codex plugin list` 含 `codex@basic-memory`。
- [ ] `~/.claude/settings.json` 的 `basicMemory.primaryProject` 与 `~/.codex/basic-memory.json` 均为 `basic_memory`。
- [ ] SkillPort `git status` 除 `.trellis/tasks/08-23-basic-memory-agents/` 外无个人 MCP、plugin 配置、密钥或知识目录内容。
- [ ] 实施记录写明 Codex `/hooks` 需用户信任；该步未完成时不把 hooks 标为已验证。

## Out of Scope

- Basic Memory Cloud 订阅、登录、Teams、HTTPS MCP、rclone 同步。
- 把 Basic Memory 做成 SkillPort 平台安装或 MCP 管理功能。
- 把共享 `memory-*` skills 装进 SkillPort Central / Universal Agents。
- 在仓库内跑交互式 `/basic-memory:bm-setup` 访谈。
- 语义搜索 embedding、Milvus、reranker。
- 从 Claude/ChatGPT 历史导入。
- 配置 VS Code、Claude Desktop、ChatGPT Custom GPT。
- 安装或配置 Obsidian 应用本体；用户自行把该目录加为 vault。
- 关闭、迁移或清空 `~/.grok/memory/`。
- 代替用户在 Codex `/hooks` 中点击信任。

## Notes

- 知识目录 git 与 `.gitignore` 已在规划阶段完成（`81d93aa`）。实施阶段不再重复 `git init`。
- 用户审阅下方最终规划摘要并明确允许实施后，才运行 `task.py start`。
- 证据：上游 README / `llms-install.md` / `SPEC-PER-PROJECT-ROUTING.md` / Claude 与 Codex plugin README；本机配置探查；Grok `07-mcp-servers.md`；Kimi / OpenCode / Antigravity CLI / Oh My Pi MCP 文档。
