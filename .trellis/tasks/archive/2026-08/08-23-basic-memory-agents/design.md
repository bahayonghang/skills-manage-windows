# 技术设计：本机 Basic Memory 用户级接入

> `prd.md` 定义结果。本文定义进程边界、配置写入、去重、Windows spawn 与回滚。

## 0. 设计结论

| 决策点 | 方案 |
|---|---|
| 运行时 | 每客户端按需 spawn 自己的 stdio 进程：`uvx.exe basic-memory mcp`。不常驻 HTTP/SSE。 |
| 知识权威 | Markdown 在 `D:\Documents\LYH\100-Work\100-Notes\basic_memory`。SQLite 索引在 `~/.basic-memory`。 |
| 配置范围 | 只写用户级文件。SkillPort 与其它仓库的项目级 MCP 不动。 |
| 命令字面值 | 配置里优先 `C:\Users\lyh\scoop\shims\uvx.exe`。CLI 封装器若拒绝绝对路径，再用 `uvx`，并以 spawn 探测为准。 |
| Claude MCP | `claude mcp add --scope user`。禁止默认 `local` scope。 |
| Codex MCP | 先装 plugin；若 plugin 已注册 `basic-memory`，只保留一套命令。否则写 `[mcp_servers.basic-memory]`。 |
| Grok | 写 `~/.grok/config.toml` 的 `[mcp_servers.basic-memory]`。原生 `[memory] enabled = true` 不改。 |
| OMP | 写 `~/.omp/agent/mcp.json`，server 名与命令与其它客户端相同，避免发现层拼出第二套进程。 |
| Antigravity CLI | 只写 `~/.gemini/config/mcp_config.json`。不复制到 `settings.json`。 |
| Plugin 配置 | Claude 合并 `~/.claude/settings.json`；Codex 新建 `~/.codex/basic-memory.json`。`primaryProject` 均为 `basic_memory`。 |
| SkillPort | 不改产品代码。工作区只允许 Trellis 任务文件。 |

## 1. 数据流

```text
编码客户端
  -> 用户级 MCP 配置 (stdio)
  -> uvx.exe basic-memory mcp
  -> Basic Memory (local project basic_memory)
       -> 读/写 D:\Documents\LYH\100-Work\100-Notes\basic_memory\*.md
       -> 索引  ~\.basic-memory\
  -> Obsidian 打开同一目录（无 MCP）

Grok 额外：
  -> 原生 memory 工具读写 ~\.grok\memory\
  -> MCP 工具读写 basic_memory 知识图
```

stdio 进程随客户端会话起停。8 个客户端同时开会有最多 8 个 MCP 进程，都指向同一 project 路径。文件锁与 SQLite 由 Basic Memory 自己处理；本任务不引入第二套同步守护进程，不跑长期 `basic-memory sync --watch`，除非 CLI `status` 明确要求一次 `sync`。

## 1.1 本机记忆分层（对照指南 + 现有目录）

依据 `ref/Agent-Memory-Basic-Memory-智能体长期记忆系统完整指南.md` 与本机探查。原生 Memory 是短缓存，Basic Memory 是跨 Agent 长期层。不关闭、不迁移其它存储。

| 层 | 本机位置 | 职责 |
|---|---|---|
| 人类 PKM | `D:\Documents\LYH\100-Work\100-Notes\LYHNotes`（及 Knowledge/Research） | 日常、文献、领域笔记。不导入 Basic Memory。 |
| Claude 原生 | `~/.claude/projects/<project>/memory/` | Claude 短缓存。保留。 |
| Codex 原生 | `~/.codex/memories/`（若开启） | Codex 短缓存。保留。 |
| Grok 原生 | `~/.grok/memory/` | Grok 会话摘要与 `/remember`。双轨保留。 |
| claude-mem | `~/.claude-mem/` | 会话轨迹库。保留，不导入。 |
| OMP 原生 | `~/.omp/agent`，`memory.backend: local` | OMP 自带 local memory。保留。 |
| Basic Memory | vault `...\100-Notes\basic_memory` + 索引 `~/.basic-memory` | 8 个客户端共享的长期事实、决策、项目状态、checkpoint。 |

MCP 只提供工具。Agent 要读长期记忆，还依赖：vault 内 `START.md` / `MEMORY_POLICY.md`、用户级 `CLAUDE.md`/`AGENTS.md` 追加段、Claude/Codex plugin hooks。

### Vault 目录

方式二（按记忆类型）+ 插件默认写入目录：

```text
basic_memory/
  START.md                 启动路由，先读
  MEMORY_POLICY.md         写什么、不写什么、去重、归档
  instructions/使用速查.md
  core/                    相对稳定的用户事实与偏好
  context/                 当前焦点
  procedures/              可复用工作方法
  projects/项目索引.md     长期项目入口
  decisions/               跨 Agent 决策（人/非 Codex 写入）
  sessions/                Claude captureFolder
  codex/decisions|remember Codex plugin 默认目录
  archive/
  templates/
  schemas/
```

不把 LYHNotes 的 PARA 结构复制进来。不编造个人传记。`core/` 只放本机已确认的工作环境事实。SkillPort 产品仓库的 `AGENTS.md` / `CLAUDE.md` 不改。

## 2. 安装与 project

1. `uv tool install basic-memory`
2. `basic-memory project add basic_memory "D:\Documents\LYH\100-Work\100-Notes\basic_memory"`
3. `basic-memory project default basic_memory`
4. `basic-memory sync` 一次，建立索引
5. `basic-memory status` 确认 local、路径正确

不调用 `bm cloud login`。不设置 `BASIC_MEMORY_FORCE_CLOUD`。

知识目录 git 已存在。实施不 `git init`，不强制提交测试笔记。测试笔记若写入 vault，留在工作区由用户用 Obsidian/git 处理。

## 3. MCP 配置合同

Server 名一律 `basic-memory`。

### 3.1 推荐 stdio 形

JSON 客户端（Cursor、Kimi、OMP、Antigravity）：

```json
{
  "mcpServers": {
    "basic-memory": {
      "command": "C:\\Users\\lyh\\scoop\\shims\\uvx.exe",
      "args": ["basic-memory", "mcp"]
    }
  }
}
```

OpenCode：

```json
{
  "mcp": {
    "basic-memory": {
      "type": "local",
      "command": ["C:\\Users\\lyh\\scoop\\shims\\uvx.exe", "basic-memory", "mcp"],
      "enabled": true
    }
  }
}
```

OpenCode 若实际版本要求 `mcp.servers`（v2），按已安装 OpenCode 的 schema 写入，仍保持 server 名 `basic-memory`。

Codex TOML（在 plugin 未提供 MCP 时）：

```toml
[mcp_servers.basic-memory]
command = "C:\\Users\\lyh\\scoop\\shims\\uvx.exe"
args = ["basic-memory", "mcp"]
```

Grok 可用：

```text
grok mcp add --scope user basic-memory -- C:\Users\lyh\scoop\shims\uvx.exe basic-memory mcp
```

Claude：

```text
claude mcp add --scope user basic-memory -- C:\Users\lyh\scoop\shims\uvx.exe basic-memory mcp
```

若 Claude 在 Windows 上仍无法 spawn，改用与现有 `exa` 相同的 `cmd /c` 包装，只改这一处。

### 3.2 合并规则

- Cursor `mcp.json` 现为空对象：填入 `mcpServers.basic-memory`。
- OpenCode `opencode.json` 已有 `$schema` / `plugin` / `agent` / `lsp` / `shell`：只增加 `mcp` 段。
- Antigravity `mcp_config.json` 已有 `playwright`：追加 `basic-memory`。
- Kimi、OMP 文件不存在：新建合法 JSON，只含 `mcpServers`。
- 已存在同名 `basic-memory`：若 command/args 已能启动同一 CLI，则跳过；若指向其它程序，先停手并写入实施记录，不覆盖。

### 3.3 发现层去重

| 客户端 | 风险 | 处理 |
|---|---|---|
| Grok | 会读 Claude/Cursor MCP | 仍写 Grok 用户配置。同名时 Grok 自身配置优先，不出现两套工具。 |
| OMP | 会发现外部 MCP | 原生 `mcp.json` 使用相同 command/args。 |
| Codex | plugin `.mcp.json` + `config.toml` | 只保留一套。 |
| Antigravity vs Gemini settings | `settings.json` 与 `mcp_config.json` 都有 MCP | 只改 `mcp_config.json`。 |

## 4. Plugin

### Claude Code

```text
claude plugin marketplace add basicmachines-co/basic-memory --sparse .claude-plugin plugins/claude-code
claude plugin install basic-memory@basicmachines-co
```

合并到 `~/.claude/settings.json`（不删现有 `hooks` / `enabledPlugins`）：

```json
{
  "basicMemory": {
    "primaryProject": "basic_memory",
    "captureFolder": "sessions"
  }
}
```

不在本轮设置 `outputStyle: basic-memory`（捕获反射可选，prd 未要求）。不写 SkillPort 的 `.claude/settings.json`。

### Codex

```text
codex plugin marketplace add basicmachines-co/basic-memory --sparse .agents/plugins --sparse plugins/codex
codex plugin add codex@basic-memory
```

新建 `~/.codex/basic-memory.json`：

```json
{
  "basicMemory": {
    "primaryProject": "basic_memory",
    "focus": "code/dev",
    "checkpointOnCompact": true
  }
}
```

不设置 `approval_policy = "never"`。可选 `default_tools_approval_mode = "approve"` 仅在该 MCP server 表上，不改全局审批。

Codex `/hooks` 信任是用户步骤。实施记录写未完成，不把 checkpoint hook 标为已验证。

## 5. 测试笔记

用 CLI 写一条可识别的测试实体（标题含 `skillport-bm-setup` 或等价 permalink），确认：

- 文件出现在知识目录
- `search_notes` / `read_note` 能取回
- `.gitignore` 不忽略该 Markdown

不把测试笔记提交到 SkillPort。是否提交到 notes 仓库由用户决定。

## 6. 备份与回滚

编辑前复制：

- `~/.claude.json`
- `~/.claude/settings.json`
- `~/.codex/config.toml`
- `~/.cursor/mcp.json`
- `~/.grok/config.toml`
- `~/.config/opencode/opencode.json`
- `~/.gemini/config/mcp_config.json`

备份目录：`%TEMP%\basic-memory-agents-backup-<timestamp>\`。不提交备份。

回滚：从备份覆盖对应用户文件；`claude plugin uninstall` / `codex plugin remove`；`uv tool uninstall basic-memory` 仅在用户要求卸载时执行。知识目录 git 与笔记保留。

## 7. SkillPort 边界

允许出现在 SkillPort 工作区的路径：`.trellis/tasks/08-23-basic-memory-agents/**`。

禁止：改 `src/`、`src-tauri/`、i18n、README、项目级 `.cursor/mcp.json`、`.grok/config.toml`、`.mcp.json`。
