# Codex 交接文档：本地状态维护报告

重新激活提示：

```text
我们正在从这份交接文档继续。请先阅读本文档，检查当前仓库状态，验证哪些内容仍然适用，然后从后续步骤继续，不要假设旧聊天上下文仍然可用。
```

## 仓库和分支

- 仓库：`D:\Documents\Code\Agents\skills-manage-windows`
- 创建交接文档时的分支：`dev`
- 交接文档创建日期：2026-05-04
- 会话主题：使用 `$keep-codex-fast` 检查本地 Codex Desktop/CLI 状态，产出安全维护计划，并在归档 Codex 历史前创建这份交接文档。

## 当前目标

为安全的 Codex 本地状态维护做准备，同时不丢失活跃仓库聊天中的有用连续性。

当前的直接目标不是修改 Codex 状态，而是保存本会话上下文。这样在交接文档存在之后，就可以在后续归档较旧或较重的 Codex 历史。

## 已完成工作

- 读取并遵循了 `$keep-codex-fast` skill 契约。
- 尝试执行必需的首次 report-only 脚本运行。
- 确认已安装的 skill 包存在，但缺少其捆绑的 `scripts/keep_codex_fast.py` 文件。
- 使用 PowerShell 复现了一次等价的只读本地状态检查。
- 总结了安全维护建议：
  - 在归档旧的活跃仓库聊天前先创建交接文档；
  - 在使用自动 apply 模式前，先修复或重新安装缺失的 `keep-codex-fast` 脚本；
  - 只有在确认交接文档已存在并且 Codex 已关闭后，才执行维护 apply；
  - 优先使用归档、移动和备份行为，绝不永久删除。
- 创建了这份仓库本地交接文档。

## 已触碰或调查的文件

已触碰：

- `docs/codex-handoffs/2026-05-04-codex-local-state-maintenance.md`

已调查：

- `C:\Users\lyh\.codex\skills\keep-codex-fast\SKILL.md`
- `C:\Users\lyh\.codex\skills\keep-codex-fast\README.md`
- `C:\Users\lyh\.codex\config.toml`
- `C:\Users\lyh\.codex\sessions\`
- `C:\Users\lyh\.codex\archived_sessions\`
- `C:\Users\lyh\.codex\worktrees\`
- `C:\Users\lyh\.codex\logs_2.sqlite`
- `C:\Users\lyh\.codex\state_5.sqlite`
- `C:\Users\lyh\.codex\log\codex-tui.log`
- `C:\Users\lyh\.codex\memories\MEMORY.md`，用于查询仓库本地的先前上下文。

## 已运行的命令和检查

报告脚本尝试：

```powershell
python C:\Users\lyh\.codex\skills\keep-codex-fast\scripts\keep_codex_fast.py
```

结果：失败，因为已安装的 skill 文件夹中不存在该脚本文件。

Skill/包检查：

```powershell
Get-ChildItem -Force C:\Users\lyh\.codex\skills\keep-codex-fast
Get-ChildItem -Recurse -Filter keep_codex_fast.py C:\Users\lyh\.codex\skills
Get-ChildItem -Recurse -Filter keep_codex_fast.py C:\Users\lyh\.skillsmanage\skills
Select-String -Path C:\Users\lyh\.codex\skills\keep-codex-fast\README.md,C:\Users\lyh\.codex\skills\keep-codex-fast\SKILL.md -Pattern "keep_codex_fast|scripts|report|apply"
```

只读本地状态检查：

```powershell
Get-ChildItem -Force C:\Users\lyh\.codex
```

PowerShell 汇总检查过：

- 活跃 session 的大小和数量；
- 已归档 session 的大小和数量；
- 按大小和年龄排序的最大活跃 session 文件；
- 陈旧 worktree 候选；
- `logs_2.sqlite`、`state_5.sqlite` 和 WAL 大小；
- Windows extended-path 文本命中；
- config 项目条目、缺失文件夹条目和 `\\?\` 条目；
- `~\.codex` 下最大的根级项目；
- 按内存排序的主要 Node/dev 进程。

仓库/交接文档检查：

```powershell
git branch --show-current
Get-ChildItem -Force docs
Select-String -Path C:\Users\lyh\.codex\memories\MEMORY.md -Pattern "skills-manage-windows|keep-codex-fast|handoff"
```

## 报告发现

- 活跃 sessions：`1.98 GB`，`1298` 个文件。
- 旧活跃 session 候选：`1033` 个文件早于 10 天。
- 最大活跃 sessions：约 `21.6 MB`、`20.3 MB`、`17.2 MB`、`16.7 MB`，另有多个在 `13-15 MB` 范围内，多数已有 8-25 天。
- 已归档 sessions：`7.1 MB`，`7` 个文件。
- Worktrees：`0 B`，`0` 个目录，没有陈旧 worktree 候选。
- 本地状态的主要体积来源：
  - `logs_2.sqlite`：`2.33 GB`
  - 活跃 `sessions`：`1.98 GB`
  - `codex-tui.log`：`1.57 GB`
  - `.sandbox-bin`：`186.5 MB`
  - `.tmp`：`71.5 MB`
- SQLite 状态：
  - `state_5.sqlite`：`22.6 MB`
  - `logs_2.sqlite-wal`：`4.2 MB`
- Config：
  - `29` 个项目条目；
  - `1` 个缺失文件夹候选；
  - `11` 个 `\\?\` extended-path 项目条目。
- Extended-path 文本命中：`684`。
- Dev 进程：多个 `node` 进程约 `133-136 MB`；只做了报告，没有杀进程。

## 已知错误、警告或失败检查

- 已安装的 `$keep-codex-fast` 文件夹缺少 `scripts/keep_codex_fast.py`；当前无法通过文档中的命令运行自动 report/apply 模式。
- Shell 启动时重复打印 sandbox/starship 警告：

```text
Unable to create log dir "C:\Users\CodexSandboxOffline\.cache\starship": PermissionDenied
```

该警告出现在 PowerShell 命令执行期间，但没有阻塞只读检查。

- 未运行仓库测试，因为本任务只新增交接文档并检查了本地 Codex 状态。

## 未决决策

- 是否修复或重新安装 `$keep-codex-fast` skill 包，使文档中的脚本在执行维护前真实存在。
- 哪些旧的活跃仓库聊天在归档前仍需要交接文档。
- 是否在后续带备份的 apply 中修剪缺失的 config 项目条目。
- 是否在后续带备份的 apply 中规范化 Codex config/本地 SQLite 状态里的 `\\?\` 路径条目。
- 在首次成功完成 repair/report/apply/verify 周期后，是否设置每周或每两周一次的 report-only 提醒。

## 约束和偏好

- 首次 `$keep-codex-fast` 运行必须是 report-only。
- 不要永久删除 Codex 聊天、日志、worktrees、memories、skills、plugins 或 automations。
- 使用归档或移动，不要删除。
- 在执行任何本地状态维护前先备份。
- 除非明确要求，不要修改或复制凭据文件。
- 除非明确要求 details，否则不要打印原始 thread ID、聊天标题、本地路径或进程路径。
- 如果 Codex 正在运行，默认保持 report-only。只有在 Codex 关闭后，或明确接受 wait-for-exit 流程后，才执行维护。
- 在归档任何可能重要的活跃仓库聊天前，先创建仓库本地交接文档和重新激活提示。
- 对本仓库，保留 Windows-first 行为和已记录的 AGENTS.md 约束。
- 除非后续任务明确要求修复，否则不要为了这次维护交接触碰源代码。

## 本交接任务的禁止触碰区域

- 不修改 Codex session/archive/log/worktree 状态。
- 不触碰凭据文件，包括 auth/config secrets。
- 不做源代码重构。
- 不改依赖。
- 不杀进程。
- 不设置自动递归的变更型维护。

## 建议的后续步骤

1. 检查或重新安装 `$keep-codex-fast` skill 包，确保 `scripts/keep_codex_fast.py` 存在。
2. 重新运行官方脚本的 report-only 模式，并与本交接文档中的手动检查结果对比。
3. 为其他需要在归档后保留连续性的旧/重活跃仓库聊天创建交接文档。
4. 在任何 apply 运行前关闭 Codex，或明确使用脚本的 wait-for-exit 选项。
5. 使用保守阈值运行带备份的 apply 路径，例如归档超过 10 天的非 pinned sessions，以及超过 7 天的 worktrees。
6. Apply 后用一次新的 report-only 运行做验证。
7. 决定是否创建每周或每两周一次的 report-only 提醒；它绝不能自动运行 `--apply`。

## 满足前置条件后的建议 Apply 命令

只有在脚本已恢复、交接文档已确认、且 Codex 已关闭后才执行：

```powershell
python C:\Users\lyh\.codex\skills\keep-codex-fast\scripts\keep_codex_fast.py --apply --archive-older-than-days 10 --worktree-older-than-days 7
```

然后验证：

```powershell
python C:\Users\lyh\.codex\skills\keep-codex-fast\scripts\keep_codex_fast.py
```

## 重新激活提示

```text
我们正在从 docs/codex-handoffs/2026-05-04-codex-local-state-maintenance.md 继续 D:\Documents\Code\Agents\skills-manage-windows 的 Codex 本地状态维护。

请先阅读那份交接文档，检查当前仓库状态和当前 C:\Users\lyh\.codex\skills\keep-codex-fast 安装，验证哪些内容仍然适用。然后从后续步骤继续，不要依赖旧聊天历史。

重要约束：
- 在用户明确要求 apply 前，保持 report-only；
- 不要删除 Codex chats/logs/worktrees/memories/skills/plugins/automations；
- 在归档活跃仓库聊天前，创建或确认交接文档；
- 任何 mutation 前先备份；
- 不要触碰凭据文件；
- 保留本仓库的 Windows-first AGENTS.md 契约。
```
