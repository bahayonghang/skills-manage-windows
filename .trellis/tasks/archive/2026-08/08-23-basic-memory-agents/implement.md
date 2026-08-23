# 实施清单：本机 Basic Memory 用户级接入

## 顺序

1. 备份第 6 节列出的用户配置到 `%TEMP%\basic-memory-agents-backup-<timestamp>\`。
2. `uv tool install basic-memory`，确认 `basic-memory --version`。
3. `basic-memory project add basic_memory "D:\Documents\LYH\100-Work\100-Notes\basic_memory"`；已存在则跳过 add，只 `project default`。
4. `basic-memory project default basic_memory`
5. `basic-memory sync` 然后 `basic-memory status`
6. 探测 stdio：直接运行 `C:\Users\lyh\scoop\shims\uvx.exe basic-memory mcp` 应启动（随后结束进程）。失败则改 `cmd /c` 包装，并在任务记录写实际命令。
7. Claude：`claude mcp add --scope user basic-memory -- <resolved-uvx> basic-memory mcp`。确认未写入项目级 `.mcp.json`。
8. Cursor：合并 `~/.cursor/mcp.json`。
9. Codex：先 marketplace + `codex plugin add codex@basic-memory`；检查是否已有 MCP；缺则写 `~/.codex/config.toml`。
10. Grok：`grok mcp add --scope user ...`。确认 `[memory] enabled` 仍为 true。
11. OMP：创建或合并 `~/.omp/agent/mcp.json`。
12. Kimi：创建 `~/.kimi-code/mcp.json`。
13. OpenCode：合并 `~/.config/opencode/opencode.json` 的 `mcp` 段（按已装版本 schema）。
14. Antigravity CLI：合并 `~/.gemini/config/mcp_config.json`，不改 `settings.json` 的 `mcpServers`。
15. Claude plugin：marketplace add + install；合并 `~/.claude/settings.json` 的 `basicMemory`。
16. 写 `~/.codex/basic-memory.json`。
17. CLI 写测试笔记并读回。
18. 对各客户端跑 list/doctor（能跑的才跑）：`claude mcp list`、`grok mcp list`、`codex` 侧检查、`grok inspect` 若可用。
19. SkillPort `git status`：仅 Trellis 任务文件。
20. 在任务 `notes.md` 记录 Codex `/hooks` 待用户信任，以及各客户端实际采用的 command 字面值。

## 验证命令

```powershell
basic-memory --version
basic-memory project list
basic-memory status
claude mcp list
grok mcp list
claude plugin list
codex plugin list
git -C "D:\Documents\LYH\100-Work\100-Notes\basic_memory" status
git -C "D:\Documents\Code\Agents\skills-manage-windows" status --short
```

Grok 原生 memory：

```powershell
Select-String -Path "$env:USERPROFILE\.grok\config.toml" -Pattern 'enabled'
```

应仍能看到 `[memory]` 下 `enabled = true`。

## 风险文件

| 文件 | 风险 |
|---|---|
| `~/.claude.json` | 体量大；损坏会导致 Claude 丢失其它 MCP。必须备份后用官方 `mcp add`。 |
| `~/.claude/settings.json` | 已有 hooks/plugins。只合并 `basicMemory`。 |
| `~/.codex/config.toml` | 已有大量桌面/模型配置。只追加 MCP 表。 |
| `~/.grok/config.toml` | 不改 `[memory]`。 |
| `~/.config/opencode/opencode.json` | 保留现有 plugin/agent/lsp/shell。 |
| `~/.gemini/config/mcp_config.json` | 保留 `playwright`。 |

## 回滚点

- 任一步 JSON/TOML 解析失败：立即从备份恢复该文件，停止后续客户端。
- plugin 安装失败：MCP 注册仍算 MCP 验收项；plugin 项单独记失败，不卸载已成功的 MCP。
- 测试笔记写坏：只删知识目录里该测试文件，不动其它笔记。

## `task.py start` 前

- `prd.md` / `design.md` / `implement.md` 已齐。
- `implement.jsonl` / `check.jsonl` 已有真实 research 条目。
- 用户已批准最终规划摘要。
- 不在批准前提前改用户 MCP 文件。
