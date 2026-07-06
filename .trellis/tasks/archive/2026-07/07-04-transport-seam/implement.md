# Implement：transport seam 试点（install/uninstall）

> 前置阅读顺序：`prd.md` → `design.md` → `research/*.md`。每步带验证命令；步间为回滚点（分段提交）。

## Stage ①：targets 层 CommandRunner（纯加法，可独立提交/revert）

- [x] 1. 在 `targets/` 新增 `CommandRunner` trait + `ProcessRunner` 默认实现（契约见 design 2.1；同步签名，两种执行形态收进 runner 内部，行为逐字节保留——含 stdin 写入失败/`wait_with_output` 的现有错误路径）。
- [x] 2. `ConnectedSshTarget`（askpass.rs）/ `ConnectedWslTarget`（exec.rs）增加 `runner: Arc<dyn CommandRunner>` 字段；生产构造路径默认 `ProcessRunner`；`#[cfg(test)]` 提供注入构造器。exec.rs 12 个 spawn 现场改 `self.runner.run(...)`（`base_command()`、`wsl_discovery.rs` 不动）。
- [x] 3. `targets/tests.rs` 新增 FakeRunner 测试：exists 退出码三分支、inspect_path 解析、run_script stdin/参数、非零退出错误传播（SSH/WSL 代表路径各覆盖）。
- [x] 验证：`cd src-tauri && cargo test targets::` 全绿；`cargo clippy -- -D warnings`。
- [x] 提交 ①（refactor(targets)）。

## Stage ②：installation 编排/transport + 调用面迁移

- [x] 4. 新增 `services/installation/transport.rs`：`InstallTransport { Local, Remote(ConnectedRemoteTarget) }` + `for_target`；transport 方法表：`ensure_centralized` / `detect_existing` / `place_install` / `remove_install` / 路径拼接 / method 解析（差异归属严格按 design 2.2 表格；`Remote(String)` 拍平收敛到此单点）。
- [x] 5. 新增编排 `install_skill` / `uninstall_skill`（design 2.2 骨架）；native.rs/remote.rs 的骨架函数体拆迁：可复用的执行半边（fs_util 步骤、REMOTE_CENTRAL_INSTALL_SCRIPT、ensure_remote_centralized、remove_install_path、claude observation 路径）保留为 transport/编排的被调件。
- [x] 6. 调用面迁移：linker.rs 5 命令（install/uninstall/batch_uninstall/batch_install_to_agents/batch_install_central_skills）、collections.rs `batch_install_collection`、batch.rs（`by_method` → 编排；远程中央批量循环从 linker 迁入）、project.rs 增加 dispatcher-only 分发入口。操作日志字段尽量保持现状（批量日志两 arm 合一时取字段超集，design 2.3）。
- [x] 7. 删除死代码：`install_skill_to_agent_ssh_impl`、`uninstall_skill_from_agent_ssh_impl`、被编排替代的骨架函数、linker.rs `pub use` 桥收缩、mod.rs re-export 同步。**central_skills 域一概不动。**
- [x] 8. 新增远程执行路径测试（fake-backed `ConnectedRemoteTarget` + `mem_pool_with_home`）：远程 install 脚本六参断言、RemoteSymlinkDisabled、同根 native 捷径、远程 uninstall remove_tree+DB 清理。
- [x] 验证：`cd src-tauri && cargo test` 全量绿；`cargo clippy -- -D warnings`；grep 复核 design §4 四项指标（install 家族=1、命令层 6 文件 12 处、commands/ 无 connect_remote_target、installation 死 _ssh_impl=0）。
- [x] 提交 ②（refactor(installation)）：`bc8a2907`。

## Stage ③：收尾

- [x] 9. 试点结论写回：本任务 notes + design §6 清单落到父任务（推广范围建议、不动清单）；若 CLAUDE.md 架构段涉及 installation 描述需微调则一并更新（复核后无需微调，linker 壳层/installation 实现的描述仍准确）。
- [x] 10. spec 更新（trellis-update-spec 判断）：transport seam 约定是否值得单独 spec（建议：`.trellis/spec/backend/transport-seam.md`，记录 InstallTransport/CommandRunner 契约与「新操作单实现」规则）。
- [x] 11. 全门禁：`pnpm typecheck`（前端无改动应零影响）+ `cd src-tauri && cargo test && cargo clippy -- -D warnings`。
- [x] 提交 ③（docs/spec）→ journal → 归档流程。

## 硬红线（实施中随时对照）

- `#[error(...)]` 文案逐字不动；shell 脚本 exit 42/43 及其文案不动；REMOTE_CENTRAL_INSTALL_SCRIPT 字节不动。
- 远程 method 强转结果逐调用点与今天一致（collections 远程=copy；单装 remote symlink 仅在显式 symlink 且 symlink_allowed）。
- 不引入 spawn_blocking 到 exec.rs（行为零变化）；fs_util 包装规则照旧。
- 发现骨架被撕裂（差异 hook 超出 design 2.2 表）→ 停手改 design，不硬编。
