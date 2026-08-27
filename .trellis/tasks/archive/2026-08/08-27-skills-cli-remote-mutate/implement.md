# 执行计划 — Skills CLI 远端链接与安全卸载

依据 `prd.md` 与 `design.md`。按段执行，每段结束跑该段验证命令再进入下一段。

**前置**：`08-27-skills-cli-remote-inventory` 已合入 `dev`（写操作的前置校验依赖远端分类）。

## 段 1 — `SkillsCliFs` 变更原语（回滚单元 A，纯重构）

- [ ] 1.1 `SkillsCliFs` 增加四个方法：
      `create_managed_link(target, link)`、`remove_verified_link(link)`、
      `rename(from, to)`、`atomic_write(path, bytes)`。
- [ ] 1.2 `LocalSkillsCliFs` 实现全部转调现有函数：
      `create_skills_cli_directory_link`（`directory_link.rs:150`）、
      `remove_verified_directory_link`、`fs::rename`、`remove.rs:605-632` 的 `atomic_write`。
      **行为逐字不变**。
- [ ] 1.3 `link.rs:245` 与 `remove.rs:530` 的调用点改为走 `tx.fs()`。

验证：`cargo test -p skillport skills_cli` — 既有用例全绿且无断言改动。

## 段 2 — 远端链接创建与状态机闸门（回滚单元 B）

- [ ] 2.1 `RemoteSkillsCliFs::create_managed_link`：
      按 `remote_os()` 分支——非 Windows 用 `ln -s`，Windows 用
      `cmd.exe //c mklink /J <link> <target>` 经远端 sh 调起（design §2.1）。
- [ ] 2.2 创建后**必须回探验证**：复用 `remote-inventory` 的探测，
      确认 slot 分类为 `managed_link` 且指向正确 canonical。
- [ ] 2.3 验证失败：删除刚创建的占位物，返回 `SkillsCliError::PlacementUnavailable`。
      **绝不 fallback 到 copy** —— 不要复用
      `InstallTransport::resolve_method`（`transport.rs:95-113`）的远端退化策略。
- [ ] 2.4 实现 §2.2 的状态机闸门：guard 下重新分类后按表决定放行/拒绝/幂等。
      三个拒绝码全部复用既有变体，不新增。
- [ ] 2.5 `RemoteSkillsCliFs::remove_verified_link`：生成"先验证再删"脚本
      （design §2.3）。`[ -L ]` 才删，`rm -f`（Unix）/ `rmdir`（Windows junction）；
      普通目录报 `skipped_not_link`。**脚本里不得出现 `rm -rf`**。

验证：`cargo test -p skillport skills_cli`

## 段 3 — 批量与往返预算（回滚单元 B 续）

- [ ] 3.1 批量 link/unlink 实现为：一次 guard 下探测（`C = 1`）
      + `ceil(N / 32)` 次变更脚本（`K = 32`，design §2.6）。
      分块常量定义为具名常量，测试直接引用它而非硬编码 32。
- [ ] 3.2 每个变更脚本逐条回报 `removed` / `skipped_not_link` / `absent`，
      汇总进既有 `PlacementMutationOutcome` 的 succeeded / failed / skipped。
- [ ] 3.3 某一块失败不丢弃前面已完成块的结果——partial outcome 必须包含它们。

验证：`cargo test -p skillport skills_cli`

## 段 4 — 远端安全卸载（回滚单元 C）

- [ ] 4.1 `paths/skills_cli.rs`：recovery 目录增加 target 命名空间
      `{app_data}/skills-cli/remove-recovery/{target_id}/`（design §2.4）。
      **Local 保持现有无子目录路径**，避免破坏既有 recovery 文件。
- [ ] 4.2 `remove.rs` 的 `execute_remove` 逐步骤改走 `tx.fs()`：
      读 lock、mv canonical→备份、删 links、CAS 写回 lock、删备份。
      三个 manifest 阶段与 `remove.rs:392-399` 的收尾步骤形态不变。
- [ ] 4.3 lock 指纹**在本机算 sha256**（读回字节后算），不依赖远端有 `sha256sum`。
- [ ] 4.4 远端 lock 写回：写 temp + `mv -f` 脚本，参照
      `central_operation/fs.rs` 的 `REMOTE_*` 脚本风格。
- [ ] 4.5 conflict 在建计划阶段 bail，**脚本不发出**（零写）。
      independent direct copies 不进入任何删除脚本的路径清单。
- [ ] 4.6 备份目录删除是唯一允许 `rm -rf` 的地方，路径由我们生成。
      加注释说明这条例外的边界。

验证：`cargo test -p skillport skills_cli`

## 段 5 — 远端 leftover（回滚单元 D）

- [ ] 5.1 `central_updates/inventory/scan.rs`：`cli_lock_protect=true` 的保护集合
      改为从传入的 lock ownership 构造，而非固定读本机 lock。
- [ ] 5.2 排除项中的 `{mapped_detected_agent.global_skills_dir}/<name>`
      取远端 target 的 DB agent 行。
- [ ] 5.3 `leftover_cleanup.rs`：远端 apply 全程持有该远端 target 的 guard。
- [ ] 5.4 反向断言点：远端扫描流程中不得出现对本机 lock 路径的读取（AC7）。

验证：`cargo test -p skillport`

## 段 6 — 测试

- [ ] 6.1 AC1：`remote_os` 为 Unix / Windows 两分支各创建一次链接，
      断言创建后再列举被分类为 `managed_link`。
      远端 Windows 结果标注 `UNVERIFIED`。
- [ ] 6.2 AC2：`direct_copy` / `conflict` / `unavailable` 各一条 link 用例 + 一条 unlink 用例，
      断言返回既有稳定错误码且远端 FS 零变更（fake FS 写调用计数为 0）。
- [ ] 6.3 AC3：平台路径下是普通目录时不执行任何删除命令，计入 skipped。
- [ ] 6.4 AC3b：静态断言远端删除路径未调用 `remove_install` / `remove_tree` 于 slot 路径；
      `rm -rf` 只出现在备份路径删除处。可用源码扫描测试实现。
- [ ] 6.5 AC3c：回探验证失败 → 占位物被删除 + `placement_unavailable` + 净写为零 + 无 `direct_copy`。
- [ ] 6.6 AC4：远端卸载后 canonical / lock 条目 / managed links 消失，
      independent direct copies 逐字节保留；存在 conflict 时零写并拒绝。
- [ ] 6.7 AC5：在 `prepared` / `staged` / `metadata_committed` 三个阶段各注入一次中断，
      断言 recovery 可重试并最终收敛。
- [ ] 6.8 AC6：持有远端 target guard 时另一远端写返回 Busy；
      **同时断言持有远端 guard 不阻塞 Local 写**（guard 以 target id/kind 为键）。
- [ ] 6.9 AC8：批量远端 unlink 部分失败保留成功项并汇总 succeeded/failed/skipped，刷新库存。
- [ ] 6.10 AC8b：对 `N = 1` / `N = K` / `N = K+1` / `N = 4K` 断言远端命令次数
      等于 `ceil(N/K) + 1`；`N=1` 与 `N=K` 次数相同。
      再固定 N 改变平台数（1 vs 6），断言次数不变。
- [ ] 6.11 AC9：远端 stderr 植入哨兵 token，断言不出现在 IPC message 与操作日志。

验证：`cargo test -p skillport`

## 段 7 — spec 与收尾

- [ ] 7.1 spec `skills-cli-global.md` managed link 段补远端行：
      远端 Unix `ln -s`、远端 Windows `cmd.exe //c mklink /J`，
      并**写明理由**——本机禁令的前提（有 reparse API）在远端不成立；
      同时明确「不得退化为 copy」在远端同样成立。
- [ ] 7.2 同文件删除路径补远端行：平台 slot 只用 `rm -f` / `rmdir`，
      `rm -rf` 仅限自建备份路径。
- [ ] 7.3 能力矩阵翻五行：`LinkPlatform` / `UnlinkPlatform` / `PreviewRemove` /
      `RemoveGlobal` / `LeftoverScan`。
- [ ] 7.4 AC10：新增文案 en/zh 成对。
- [ ] 7.5 确认未新增 IPC 错误码 → `pnpm docs:gen:check` 应无 diff。
- [ ] 7.6 全量：`just ci`。真实 SSH 主机端到端行为标记 `UNVERIFIED`。

## 风险文件与回滚点

回滚单元见 `design.md` §6。

| 文件 | 风险 | 回滚单元 |
| --- | --- | --- |
| 远端 `create_managed_link` | 退化到 copy 会把 `managed_link` 意图变成 `direct_copy` 事实，且不可逆 | B |
| 远端删除脚本 | 一个写错的 `rm -rf` 就是用户数据损失。脚本必须先验证再删，且不含 `-r` | B、C |
| `remove.rs` | 阶段顺序或 CAS 位置改动会破坏 recovery 收敛 | C |
| `paths/skills_cli.rs` | Local recovery 路径若被一并改动，会让既有未完成 recovery 找不到 manifest | C |
| `central_updates/inventory/` | 误用本机 lock 会把远端健康技能判成 leftover 并删除 | D |

## 前置检查

- [ ] `08-27-skills-cli-remote-inventory` 已合入 `dev`。
- [ ] 确认 `08-26-ssh-update-observability-dialog` 未在同一工作树改
      `central_updates/inventory/`。
- [ ] 工作树干净。
