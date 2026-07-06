# transport-seam 试点结论（写于 Stage ③）

## 试点验证的判断

1. **操作级 seam 成立**：`InstallTransport { Local, Remote }` 枚举入口 + plan/execute 复合 hook 的形态，能让一份业务编排（`install.rs`）服务三个 adapter，且远程侧「单脚本原子回合」不被撕成 N 次往返。design §2.2 的 hook 归属表在实施中未被突破（7 处语义不对称全部逐字保留在各自 arm 内）。
2. **CommandRunner 缝达成可测性目标**：SSH/WSL 执行半边（`base_command()` 之后）在无真实进程/连接下可单元测试——targets 层 8 个 FakeRunner 测试 + installation 层 4 个假连接远程路径测试（脚本六参断言、RemoteSymlinkDisabled、同根 native 捷径、远程卸载 remove_tree+DB 清理）。
3. **成本画像**：installation 域迁移约 +1100/−899 行（含测试重写）；主要工时花在逐处证明排序等价性（本地先中央化 vs 骨架先同根判定、auto 回退只重试落位、远程 skill 预取时机）。推广到其他域前应先做同样的排序分析，而不是直接套骨架。

## 推广建议（按优先级，供父任务集成审查/后续任务参考）

- **值得收**：central_skills 的 delete×3 族——`delete_central_skills_impl` / `_remote_impl` 已是 dispatcher 形态，`apply_steps.rs` 步骤 2 仍在调用侧分发；收敛后可顺势删除 central_skills 域 3 个死 `_ssh_impl`（本次任务按红线未动）。
- **值得收**：preview×2 族——先做 `_ssh_impl` 命名统一（现命名与实际承载 SSH+WSL 不符），再决定是否并入 seam。
- **观望**：scanner（1 处）/ agents（2 处）/ github_import（2 处）/ usage（1 处）——fork 密度低，单独收敛的迁移风险大于收益；等该域出现新操作需求时顺势收进 seam。
- **不收**：local_remote_sync（remote-only 语义，无 Local arm）；obsidian（`is_remote_like` 是拒绝守卫不是双实现分发）。

## 类型化与既有债（暂不动，登记在案）

- `InstallationError::Remote(String)` 拍平边界已收敛到 `transport_error` 单点；若未来需要按错误类别分支，再把它类型化为语义变体（禁止字符串嗅探的域规则不受影响）。
- exec.rs 在 async 上下文直跑同步 `std::process` 的既有债与 CommandRunner 正交；若要补 `spawn_blocking`，应做在 runner 边界一处，而不是 10 个调用点。

## 已接受微偏差（行为审计线索）

1. uninstall 命令改为 connect-eager（`for_target` 先连）：远端不可达且 agent 无效时，错误优先级变为连接错误先报——与 install 既有行为一致。
2. 批量命令空集操作日志文案两 transport arm 统一。
3. 批量日志 detail 字段取两 arm 超集（`skillIds`/`agentIds` 始终记录）。
4. `batch_uninstall` 空 `skill_id` 守卫补齐到远程侧——修正了远程侧对 agent 目录整树 `rm -rf` 的潜在风险（安全性修正，非行为漂移）。

## 落点

- Stage ① 提交：`571b1c9c refactor(targets)`（CommandRunner 执行缝）。
- Stage ② 提交：`bc8a2907 refactor(installation)`（InstallTransport 收拢 + 调用面迁移 + 远程路径测试）。
- 量化指标复核（design §4）：install 家族 1 份编排；linker 6 处/collections 1 处命令层分发归零；linker 不再 import `connect_remote_target`；installation 域死 `_ssh_impl` = 0。
