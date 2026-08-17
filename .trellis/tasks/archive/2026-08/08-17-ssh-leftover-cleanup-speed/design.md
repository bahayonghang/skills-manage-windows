# Design: SSH leftover cleanup batching

## Scope And Boundaries

本设计只改 Update Center 步骤 7：`apply_remove_deleted_platform_copies_step` 的 **SSH/WSL** 半边，以及成功后的 DB 收尾。

不改：leftover 扫描分组键、一键清理前端 payload、Local `uninstall_skill`、Central 更新批处理、russh。

## Current Flow

```text
apply_skill_update_decisions
  -> CentralFs::from_active_target          # leftover-only 也会 open
  -> steps 1–6 (empty for leftover-only)
  -> for each (agent, skill, path):
        connect_remote_target()             # 新对象，非会话
        ssh.exe/wsl.exe  rm -rf -- path     # 1 进程 / 路径
        delete_skill_installation()         # 不删 observation
  -> loadInventory -> rescan leftovers
```

`ConnectedSshTarget` / `ConnectedWslTarget` 每次 `run_command` 都新开进程。复用连接对象省不了握手。

## Target Flow

```text
apply_remove_deleted_platform_copies_step (remote)
  -> load agents once; check central-missing per unique skill_id
  -> validate each path with existing remote guards
  -> group removals by normalized POSIX path
  -> one CommandRunner script:
        for path in unique_validated_paths:
            rm -rf -- "$path"
            print OK|MISSING|ERR
  -> for each original removal:
        if path OK/MISSING:
            delete installation + observations for that path
            record removed path (once per requested path)
        else:
            typed failure, continue
```

Local 分支保持逐条 `remove_deleted_platform_copy_local`。

## Contracts

### Validation (before any remote process)

对每条 `DeletedPlatformCopyRemoval.paths` 条目：

1. `allowed_agent_ids` 过滤（现状）。
2. `agent_id != "central"`。
3. `ensure_central_still_missing(skill_id)`（按 skill 缓存结果）。
4. `path == remote_join(agent.global_skills_dir, skill_id)`。
5. `ensure_remote_child_path(root, path, agent_id)`。

失败的条目进入 `failures`，不进入脚本。全部失败则 runner 调用次数为 0。

### Remote script

- 入口：现有 `ConnectedRemoteTarget::run_script` / stdin 脚本 + 位置参数，或等价的 `CommandRunner` 包装。禁止把未校验路径拼进脚本源码。
- 路径列表：已守卫的唯一路径。可用位置参数；数量可能超过 argv 上限时，改为 stdin 中 NUL 分隔路径，脚本 `read -d ''`。
- 每条路径输出一行稳定协议，例如 `OK\t<index>` / `MISSING\t<index>` / `ERR\t<index>`。不要把 stderr 原文送进用户可见 Display。
- `MISSING`（`rm` 前不存在，或等价 `no such file`）与现状一样视为成功。
- Policy：`ProcessPolicy::bulk_transfer()`。
- Cancel：`ProcessCancellation::Atomic(apply cancel flag)`。分块时，取消后不再启动下一块。
- 分块：仅当实现证明单次 stdin/stdout 会撞 cap 时启用。建议上限每块 256 条路径。默认一块。测试必须锁住块大小。

### Shared-root DB cleanup

物理路径是删除单元。路径成功后：

- 删除所有 `skill_installations`，其 `installed_path` 与该规范化路径等价，且 `skill_id` 仍不在 Central。
- 删除所有 `agent_skill_observations`，其 `dir_path` 与该路径等价，且不是 plugin/read-only。
- 最低实现：清本次 payload 里所有指向该路径的 `(skill_id, agent_id)`，并按路径再扫一遍 observation。目标是 `scan_deleted_platform_copies_with_pool` 不再返回该路径。

一次 `rm` 共享根目录会从所有 Universal Agents 拿走该 skill。这是目录事实。DB 必须与磁盘一致。

### Errors

继续用 `CentralUpdatesError`，命令层 `to_string()`。用户可见失败走现有 `SkillUpdateApplyFailure` 映射（`remove_deleted_platform_copy` / `central_updates.remove_deleted_platform_copy_failed`）。禁止 `error.contains("no such file")` 作为新的主路径；脚本协议用退出码或 `MISSING` 行。现有英文 `to_string()` 嗅探只留在兼容单条 `remove_tree` 的 Local/旧路径（若该函数删除则一起删）。

### Transport seam

不强制把 leftover 收进 `InstallTransport`。leftover 在 Central 已不存在时删除平台副本，与 `uninstall_skill`（要求可解析的安装记录）不同。本任务在 `apply_steps` 内打开 **一次** `connect_remote_target`，整步复用。

`CentralFs::from_active_target` 的额外 open 可保留，避免改命令壳。

### Observability

Operation Log 已有 `removeDeletedCopies` 计数。不要加完整路径。可选：tracing span 记录 `target_kind`、`removal_count`、`unique_path_count`、`remote_chunks`。

前端可继续只显示「Cleaning leftovers…」。进度事件不是 MVP。

## Compatibility

- IPC `SkillUpdateDecisions` / `SkillUpdateApplyResult` 不变。
- 一键清理与「Apply selected」共用后端。
- WSL 与 SSH 同一脚本与协议。

## Rollback

改动集中在 `apply_steps.rs` + 测试。回退该步骤即可恢复逐条 `remove_tree`。无 schema 迁移。

## Tradeoffs

| 选择 | 结果 |
| --- | --- |
| 去重 + 单脚本，不换 russh | 进程数从 N 降到 1；握手只付一次。覆盖 309 条场景。 |
| 并发多个 ssh.exe | 墙钟可能下降，认证与进程数仍线性。规格禁止。 |
| 按路径折叠 UI | 改善 309 张卡，但不减少 SSH 次数，除非同时做本设计。本任务不做。 |
| 扫描改远端 exists | 会让 Refresh 再付 N 次 SSH。用删 observation 解决幽灵列表。 |
