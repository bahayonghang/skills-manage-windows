# Transport Seam（Local/SSH/WSL 操作级传输缝）

## 规则

跨 Local/SSH/WSL 三种目标的**安装类操作**（install / uninstall 及其批量形态）：

- **一份编排**：业务骨架只写一次（`services/installation/install.rs`），禁止按 transport 复制 `_impl` / `_ssh_impl` / `_remote` 平行实现。差异下沉到 `InstallTransport` 的 per-transport hook。
- **一次解析、不再分支**：命令层用 `InstallTransport::for_target(&active_target)` 把 `ActiveTarget` 解析成 transport（SSH/WSL 在此打开一条连接，批量调用整个循环复用），之后任何代码不得再 `match ActiveTarget`。
- **单点拍平**：`TargetsError` 进入 installation 域只能经 `transport::transport_error`（→ `InstallationError::Remote(String)`）。禁止在各调用点各自 `.map_err(|e| ...)` 造第二种文案。

## Signatures

```rust
// services/installation/transport.rs
pub enum InstallTransport { Local, Remote(Box<ConnectedRemoteTarget>) }
impl InstallTransport {
    pub async fn for_target(target: &ActiveTarget) -> Result<Self, InstallationError>;
    pub fn is_remote(&self) -> bool;
    // per-transport hooks（pub(crate)）：shares_central_root / resolve_method /
    // validate_source / centralize_shared_root / prepare_target /
    // detect_existing / place_install / remove_install
}

// services/installation/install.rs —— 唯一业务编排
pub async fn install_skill(pool, transport: &InstallTransport, skill_id, agent_id, method)
    -> Result<InstallOutcome, InstallationError>;
pub async fn uninstall_skill(pool, transport: &InstallTransport, skill_id, agent_id, row_id: Option<&str>)
    -> Result<(), InstallationError>;
```

骨架顺序（改动前必须逐处证明排序等价）：中央守卫 → agent/central 查行 → `validate_source` → 同根捷径（centralize + native 记账）→ `resolve_method` → `prepare_target` → `detect_existing` skip → `place_install` → 记账。

## Hook 归属：语义不对称是特性，不是 bug

hook 是**复合动作**（plan/execute 切分），不是 FS 原语。远程侧的一次原子脚本回合（`REMOTE_CENTRAL_INSTALL_SCRIPT`，exit 42/43 文案字节不动）绝不能被拆成多次往返。两侧刻意保留的不对称：

| 语义 | Local | Remote |
| --- | --- | --- |
| skip 检测 | 有（同源已装则跳过） | 无（脚本置换受管条目） |
| 中央化时机 | 落位前 eager | 脚本内 lazy（同根捷径除外） |
| method=auto | symlink 失败回退 copy | 强转 copy |
| method=symlink | 直接执行 | 需 `symlink_allowed()`，否则 `RemoteSymlinkDisabled` |
| 同根判定 | 文件系统等价（`paths_equivalent`） | POSIX 字符串字面相等 |
| 卸载 | 按记录 link_type 分类，拒删无记录真目录 | 无条件 `rm -rf` 安装槽 |
| row_id（claude 观测行） | 支持 | 忽略 |

## targets 层：CommandRunner 可注入执行缝

```rust
// targets/runner.rs
pub(crate) trait CommandRunner: Send + Sync {
    fn run(&self, command: Command, stdin: Option<&[u8]>) -> Result<Output, RunnerError>;
}
pub(crate) struct ProcessRunner; // 生产默认，两种执行形态字节等价保留
```

- `ConnectedSshTarget` / `ConnectedWslTarget` 持 `runner: Arc<dyn CommandRunner>`；`base_command()` 纯构建器不动，进程执行一律走 `self.runner.run(...)`。
- 新增远程原语禁止直接 `Command::spawn()` / `.output()`——那会绕开测试缝。

## Tests Required

- targets 执行半边：`ConnectedSshTarget::for_tests_with_runner` / 直构 `ConnectedWslTarget { runner }` + `test_support::FakeRunner`（记录 program/args/stdin，FIFO 弹响应）。断言点：完整命令行字符串、stdin 字节、退出码分支、RunnerPhase 错误映射。
- installation 远程路径：`FakeRunner` 假连接 + `mem_pool_with_home("/home/…")`，不需要真 SSH。样板见 `services/installation/tests.rs` 的 `fake_ssh_transport`；断言点：脚本六参顺序（canonical/source/target/agent_dir/method/managed_copy）、stdin == `REMOTE_CENTRAL_INSTALL_SCRIPT`、DB 记账行。

## Wrong vs Correct

```rust
// Wrong：命令层再开分支 / 平行实现
match active_target {
    ActiveTarget::Local => install_skill_to_agent_impl(...).await,
    _ => install_skill_to_agent_remote_impl(...).await,   // 第二份实现必然漂移
}

// Correct：解析一次，编排一份
let transport = InstallTransport::for_target(&active_target).await?;
installation::install_skill(&pool, &transport, &skill_id, &agent_id, method).await
```

## 推广边界

试点仅覆盖 install/uninstall 家族。其他域收敛前先读试点结论（`.trellis/tasks/archive/**/07-04-transport-seam/notes.md`）：central_skills delete/preview 族值得收；scanner/agents/github_import/usage 观望；local_remote_sync（remote-only）与 obsidian（守卫非分发）不收。

> 来源任务：07-04-transport-seam（2026-07-06）
