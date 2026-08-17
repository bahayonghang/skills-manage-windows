# Optimize SSH leftover cleanup speed

## Goal

在 SSH remote target 上，Update Center 一键或勾选清理 Platform leftover 时，墙钟时间随 **唯一物理路径数** 增长，不随「平台 × 技能」分组数线性增长。清理完成后，这些 leftover 不会因为 observation 残留而立刻重新出现。

用户价值：309 条量级的远端残留可以在一次确认后完成，不必对着「Cleaning leftovers…」等数分钟，也不必反复点清理。

## Background

2026-08-17 现场：SSH 目标的 Update Center 显示 Platform leftovers (309)，一键清理停在「Cleaning leftovers…」。可见路径为 `/home/lyh/.agents/skills/<skill>`（Universal Agents 共享根）。领域名：Platform leftover。

代码已证实：

- 一键清理只发送 `removeDeletedPlatformCopies`，复用 `apply_skill_update_decisions`。
- 远端对每条路径新建 `ConnectedRemoteTarget` 并 `remove_tree`。该对象不是持久会话；每次 `rm -rf` 都启动新的 `ssh.exe` 并握手。
- leftover 按 `(agent_id, skill_id)` 分组。10 个 Universal Agents 共享 `~/.agents/skills/`。309 条与「约 31 个 skill × 10 个平台」一致（未在用户主机点验）。
- 路径计数不跨组去重。同一物理目录会被 `rm` 多次。
- 远端成功后只删 `skill_installations`。`loadInventory` 会按 observation 再扫 leftover；本机 `std::fs` 对 POSIX 路径得到 `NotFound` 仍算可删。列表可能马上恢复。

详细锚点见 `research/leftover-cleanup-bottleneck.md`。

## Requirements

- **R1** SSH/WSL leftover 清理必须先在本地完成路径守卫，再对 **去重后的物理路径** 做远端删除。禁止对同一规范化路径重复启动远端进程。
- **R2** 一次 leftover-only apply 中，远端删除进程数必须有上界：默认 **1** 次 `CommandRunner` 调用。路径极多需要分块时，块大小必须固定且可测，不得退回「一条路径一个进程」。
- **R3** 必须保留现状的部分成功语义：`MISSING`（无此文件）算该路径成功；单条 `ERR` 只让使用该路径的 removal 失败，其它路径继续。
- **R4** 路径守卫不得弱于现状远端规则：只删除 `remote_join(agent.global_skills_dir, skill_id)`；拒绝 `central` 伪平台、平台根、越出平台根、`..`、NUL、非绝对路径。未通过守卫的路径不得进入远端脚本。
- **R5** Central Skill 在 apply 期间重新出现时，不得删除对应平台副本（保留 `ensure_central_still_missing`）。
- **R6** 远端路径删除成功或已缺失后，必须删除该路径上的 `skill_installations` **和** `agent_skill_observations`。共享同一物理路径的其它平台记账一并清除。
- **R7** Local leftover 行为保持现有 `uninstall_skill` 路径，本任务不改 Local 删除语义。
- **R8** leftover 步骤必须使用已有 `CommandRunner` / process policy，禁止在 service 里直接 `Command::spawn`。凭据不得写入 SQLite、日志、错误或测试产物。
- **R9** leftover 远端删除必须响应 apply job 的 cancel flag。取消后不得再启动后续分块。
- **R10** 用户可见文案继续走 i18n。本任务不改一键清理的确认框语义。

## Acceptance Criteria

- [ ] AC1：FakeRunner 覆盖「10 个 Universal Agent 组、同一 POSIX 路径」的 leftover-only apply：runner 调用次数为 1，stdin/argv 只含该唯一路径一次。
- [ ] AC2：FakeRunner 覆盖 3 条不同路径：一次（或固定分块次数）远端调用后，成功路径出现在 `removedDeletedPlatformCopyPaths`，失败路径进入 `failures`，步骤名为 `remove_deleted_platform_copy`。
- [ ] AC3：守卫失败的路径不产生任何 runner 调用，且不删除磁盘或 DB。
- [ ] AC4：远端 `MISSING` 与成功 `rm` 一样清理 installation + observation。随后 `scan_deleted_platform_copies_with_pool` 不再返回这些路径。
- [ ] AC5：共享根路径删除成功后，amp 与 cursor 等共享该路径的 leftover 组都不再被扫描到。
- [ ] AC6：已有 Local leftover 单测继续通过；Central 重新出现、越权 agent、非托管路径的拒绝用例继续成立。
- [ ] AC7：相关 Rust 测试与 leftover 前端测试通过。完整 `just ci` 在实现阶段执行。

## Out of Scope

- 不落地 russh / 持久 SSH session。
- 不改 leftover 卡片是否按平台拆行（309 张卡可保留；后端按唯一路径删）。
- 不把 leftover 扫描改成远端 `find` / 逐条 `exists`。
- 不改 Platform duplicates、orphans、Central 更新写入的批处理。
- 不改一键清理的确认交互，不新增第二种清理 IPC。
- 不把 leftover 收进 `InstallTransport` 编排，除非实现时证明能零语义漂移且更短。

## Technical Notes

- 复用 `apply_skill_update_decisions`；不要新命令。
- 远端脚本走 `CommandRunner`。多路径删除用 `ProcessPolicy::bulk_transfer`（15 min），因 `rm -rf` 可能超过 Standard 120 s。
- 操作日志只记数量，不记完整路径、host、用户名或凭据。
- Windows-first：用 FakeRunner 断言进程数；无可用 SSH 主机时不把实机耗时当作门禁。

## Open Questions

无阻塞问题。共享根「一张卡还是十张卡」留作后续 UX，不影响本任务验收。
