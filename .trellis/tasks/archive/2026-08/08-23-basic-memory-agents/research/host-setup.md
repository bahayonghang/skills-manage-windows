# Basic Memory 本机接入调研

实施与检查只改用户级配置和 `uv tool`，不改 SkillPort 产品代码。

## 知识库

- 路径：`D:\Documents\LYH\100-Work\100-Notes\basic_memory`
- git：`main`，root `81d93aa`，已有 `.gitignore`
- Basic Memory project 名：`basic_memory`
- Obsidian：用户自行 Open folder as vault。官方文档：指向同一目录即可，无需 Obsidian MCP。

## Windows spawn

- `uv.exe` / `uvx.exe`：`C:\Users\lyh\scoop\shims\`
- Claude 现有 `exa` 使用 `command: cmd`。若 `uvx.exe` 直接 spawn 失败，对该客户端改 `cmd` 包装。
- Grok 会按 `PATHEXT` 解析裸命令；仍优先写绝对路径 `uvx.exe`。

## Claude MCP scope

`claude mcp add` 默认 `--scope local`（项目级）。本任务必须 `--scope user`，否则会在 SkillPort 工作区写出项目 MCP。

## 客户端文件

| 客户端 | 文件 | 探查结果（2026-08-23） |
|---|---|---|
| Claude Code | `~/.claude.json` | `mcpServers` 仅 `exa` |
| Claude plugin cfg | `~/.claude/settings.json` | 有 hooks/plugins，无 `basicMemory` |
| Codex | `~/.codex/config.toml` | 无 `[mcp_servers.*]` |
| Codex plugin cfg | `~/.codex/basic-memory.json` | 不存在 |
| Cursor | `~/.cursor/mcp.json` | `mcpServers: {}` |
| Grok | `~/.grok/config.toml` | `[memory] enabled = true`；无 mcp_servers |
| OMP | `~/.omp/agent/mcp.json` | 不存在 |
| Kimi | `~/.kimi-code/mcp.json` | 不存在 |
| OpenCode | `~/.config/opencode/opencode.json` | 无 `mcp` 段 |
| Antigravity CLI | `~/.gemini/config/mcp_config.json` | 已有 `playwright` |
| Gemini legacy | `~/.gemini/settings.json` | 有 `drawio`/`obsidian`/`playwright`；不要再写 BM |

## Plugin 安装命令

Claude：

```text
claude plugin marketplace add basicmachines-co/basic-memory --sparse .claude-plugin plugins/claude-code
claude plugin install basic-memory@basicmachines-co
```

Codex：

```text
codex plugin marketplace add basicmachines-co/basic-memory --sparse .agents/plugins --sparse plugins/codex
codex plugin add codex@basic-memory
```

Codex 文档：sparse 必须同时包含 `.agents/plugins` 与 `plugins/codex`。hooks 需用户在 `/hooks` 信任。

## 去重

- Grok 合并顺序：自身 config.toml > Claude > Cursor > `.mcp.json`
- OMP 原生配置优先于外部工具发现
- 同名 server 必须命令一致

## 安全

- 不登录 Cloud，不写 API key
- 不把 `~/.claude.json` 全文贴进 Trellis 记录（体积大且含其它工具状态）
- 备份放 `%TEMP%`，不进 git
