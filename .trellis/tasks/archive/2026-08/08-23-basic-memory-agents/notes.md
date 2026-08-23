# 实施记录

- Backup: `%TEMP%\basic-memory-agents-backup-20260823-164338`
- CLI: `basic-memory` 0.22.1。该版本无 `sync` 子命令；`status` 会显示文件索引。
- stdio command: `C:\Users\lyh\scoop\shims\uvx.exe basic-memory mcp`
- Claude MCP：user scope，`exa` 仍在；`claude mcp list` 显示 `basic-memory` Connected。
- Codex MCP：写入 `~/.codex/config.toml` `[mcp_servers.basic-memory]`，plugin `codex@basic-memory` 0.22.1 installed+enabled。
- **Codex `/hooks` 未验证。** 用户需在 Codex 打开 `/hooks` 并信任 Basic Memory hook。未完成前不把 SessionStart/PreCompact 标为已验证。
- Grok：`[memory] enabled = true` 仍在；已加 MCP 与 `~/.grok/rules/basic-memory.md`。
- 未安装官方通用 `memory-*` skills 到 `~/.agents/skills`（prd 排除 SkillPort/Universal Agents 技能安装）。
- SkillPort 工作区仅 Trellis 任务文件。未跑 `just ci`（无产品代码改动）。
