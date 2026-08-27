# Skills CLI 远端链接与安全卸载

父任务：`08-27-skills-cli-remote-target`（源需求 U5）
序位：远端树第 3 个

## Goal

让远端 SSH / WSL 目标支持 Skills CLI 的目录链接管理（link / unlink）、
安全卸载（含 recovery）与 leftover 保护。这些是不需要访问 npm registry 的写操作。

## Confirmed Facts

- 本机 managed link 是 Windows junction（reparse API，禁止 `cmd.exe` / `mklink` / 符号链接特权 / copy 回退）
  或 Unix 目录 symlink（spec `skills-cli-global.md:95-98`）。
  `create_skills_cli_directory_link` 是**本机系统调用**，远端必须换成远端命令。
- 远端主机可能是 Linux / macOS（只有 symlink）或 Windows（junction 语义不同），
  需要按远端 OS 分支。`RemoteTargetConfig.remote_os`（`targets/model.rs:65-86`）已记录远端 OS。
- 禁止把 `direct_copy` 自动转换为 junction/symlink；
  禁止删除普通目录或对平台路径调用 `remove_dir_all`（spec `:95-98`）。
- link/unlink 只允许 `Missing ↔ ManagedLink` 互转（spec `:205-206`）。
- 卸载不 spawn `skills remove`，不使用未经验证的 `--force` / `--keep-links`；
  domain-local manifest 在 `skills_cli_remove_recovery_dir()`；
  lock fingerprint CAS；conflict 零写；direct copies 按字节保留且不进入变更路径（spec `:99-102`）。
- 阶段模型的精确形态：`remove.rs:43-46` 只定义**三个** manifest 阶段
  `prepared` → `staged` → `metadata_committed`；提交后的清理（删备份、删 manifest）
  是 `remove.rs:392-399` 的收尾步骤，**不是 manifest 阶段**。远端复用同一形态。
- **既有远端删除路径不可复用**：`InstallTransport::remove_install`
  （`installation/transport.rs:221-237`）的远端分支是无条件 `remove_tree`（`rm -rf`），
  代码注释自陈如此。它与 R3 直接冲突，本任务必须自建经分类闸门的删除路径。
- 锁顺序（spec `:112-116`）：`skills_cli` lease → `acquire_target_mutation_guard` →
  guard 下重新校验 ownership/placement → FS/lock 变更 → drop guard → drop lease。
  `acquire_target_mutation_guard`（`central_mutation/mod.rs:63-78`）已是 target-scoped，可直接传远端 target。
- leftover：本机扫描设 `cli_lock_protect=true` 并排除 lock 拥有的 canonical、已解析链接，
  以及 `{mapped_detected_agent.global_skills_dir}/<name>`；
  **远端 leftover 不得使用本机 lock**；远端 leftover apply 持有该 target 的 guard（spec `:128-133`）。
- 每次远端命令一次 SSH 握手，批量 link/unlink 需要控制往返数量，不能逐项各开一次连接。

## Requirements

- R1：远端 link / unlink 通过 seam 执行，按远端 OS 分支创建目录链接
  （Unix symlink / 远端 Windows junction），语义与本机一致。
- R2：远端 link/unlink 只允许 `Missing ↔ ManagedLink`。
  `direct_copy` / `conflict` / `unavailable` 拒绝且零写，返回既有稳定错误码。
- R3：远端绝不删除普通目录、绝不对平台路径做递归删除、绝不自动转换 `direct_copy`。
  平台 slot 的删除只允许 `rm -f`（Unix 链接）或 `rmdir`（远端 Windows junction），
  `rm -rf` 仅允许作用于我们自己生成的 canonical 备份路径。
  **不得复用** `InstallTransport::remove_install` 的远端分支或
  `ConnectedRemoteTarget::remove_tree` 作为平台 slot 的删除手段。
- R4：远端卸载复用本机的三个 manifest 阶段（`prepared` → `staged` → `metadata_committed`）
  加一个提交后收尾步骤，以及 lock fingerprint CAS；conflict 零写；
  independent direct copies 按字节保留。
  **recovery manifest 仍写在 SkillPort 本机**，按 target 命名空间隔离为
  `{app_data}/skills-cli/remove-recovery/{target_id}/{skill}.json`（design §2.4）。
  理由：recovery 由 SkillPort 驱动，而远端中断最常见的成因就是远端不可达，
  写在远端的 manifest 恰在最需要时读不到。Local 路径保持现状不变。
- R5：远端写操作遵守 lease → target guard → guard 下 recheck → 变更 → drop 顺序，
  guard 传入远端 target 而非 Local。
- R6：远端 leftover 扫描使用远端 lock，不读本机 lock；
  远端 leftover apply 全程持有该 target 的 guard。
- R7：批量远端 link/unlink 的远端命令次数按**固定分块**增长，不逐项一次往返。
  「不可接受」不是判据（TPR-07）。设分块大小为常量 `K`（由 design 选定，`K >= 16`，
  参照既有做法：usage 批量读 64、scanner 批量读 4、central_updates leftover 按索引分批），
  则一次批量操作的远端命令次数上限为 `ceil(N / K) + C`，
  其中 `N` 是选中项数、`C` 是与 `N` 和平台数都无关的固定开销（如一次 guard 下 recheck）。
  计数口径：fake `CommandRunner` 记录的子进程 spawn 次数；连接重试不计入。
  部分失败保留成功项并汇总 partial outcome（复用既有 `PlacementMutationOutcome` 语义）。
- R8：远端中断后的 recovery 可重试并收敛，不留半完成状态。
- R9：远端 stdout / stderr / 路径不进入 `IpcError.message` 或未脱敏操作日志。
- R10：新增文案 en/zh 成对。

## Acceptance Criteria

- [ ] AC1 (R1)：远端 link 在 `remote_os` 为 Unix 时创建目录 symlink、为 Windows 时创建 junction；
      两种情形创建后再列举都被分类为 `managed_link`。
- [ ] AC2 (R2)：对 `direct_copy` / `conflict` / `unavailable` 的远端 link 与 unlink 请求
      各有一条测试断言返回稳定错误码且远端文件系统零变更。
- [ ] AC3 (R3)：注入「平台路径下是普通目录」的场景，断言远端不执行任何删除命令，
      该项计入 skipped 而非 failed。
- [ ] AC3b (R3)：静态断言 Skills CLI 的远端删除路径未调用
      `InstallTransport::remove_install` 或 `ConnectedRemoteTarget::remove_tree`
      于平台 slot 路径；`rm -rf` 只出现在自建备份路径的删除上。
- [ ] AC3c (R1,R3)：远端 link 创建后回探验证失败时，删除刚创建的占位物并返回
      `skills_cli.placement_unavailable`，断言远端净写为零，
      且**不发生**向 copy 的退化（不产生 `direct_copy`）。
- [ ] AC4 (R4)：远端卸载后 canonical / lock 条目 / managed links 消失，
      independent direct copies 逐字节保留；存在 conflict 时零写并拒绝。
- [ ] AC5 (R4,R8)：在 prepared / staged / metadata_committed 三个阶段各注入一次中断，
      断言 recovery 可重试并最终收敛到一致状态。
- [ ] AC6 (R5)：持有远端 target guard 时，另一个远端写操作返回 Busy；
      同时断言持有远端 guard **不会**阻塞 Local 写操作。
- [ ] AC7 (R6)：远端 leftover 扫描测试断言未读取本机 lock 路径
      （沿用 spec §6「remote scan ignores local lock」的反向断言）。
- [ ] AC8 (R7)：批量远端 unlink 部分失败时保留成功项、汇总 succeeded/failed/skipped，
      并刷新远端库存。
- [ ] AC8b (R7)：往返次数有固定上限（TPR-07）。以 design 选定的分块常量 `K`，
      对 `N = 1`、`N = K`、`N = K + 1`、`N = 4K` 四种输入分别断言
      远端命令次数等于 `ceil(N / K) + C`；其中 `N = 1` 与 `N = K` 的次数**相同**。
      同一组输入再固定 `N` 改变平台数（1 个 vs 6 个），断言次数不变。
- [ ] AC9 (R9)：植入 stderr 哨兵 token，断言其不出现在 IPC message 与操作日志。
- [ ] AC10 (R10)：i18n en/zh parity 通过。
- [ ] AC11 (Completion Gate)：`just ci` 通过。真实 SSH 主机端到端行为标记 `UNVERIFIED`。
      来源是 `AGENTS.md` 的 Completion Gate 一节，不隶属本任务的任一 R（TPR-09）。

## Out of Scope

- 远端 install 与 update（属 `08-27-skills-cli-remote-install-update`）。
- 把 `direct_copy` 转换为链接。
- 重建指向已删除 canonical 的 platform link。
- 持久 SSH 会话池。

## Dependencies

- `08-27-skills-cli-remote-inventory` 必须先合入 `dev`：
  写操作的前置校验依赖远端 placement 分类结果。
