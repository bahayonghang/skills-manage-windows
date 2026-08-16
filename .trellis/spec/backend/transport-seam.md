# Transport Seam（Local/SSH/WSL 操作级传输缝）

## 规则

跨 Local/SSH/WSL 三种目标的**安装类操作**（install / uninstall 及其批量形态）：

- **一份编排**：业务骨架只写一次（`services/installation/install.rs`），禁止按 transport 复制 `_impl` / `_ssh_impl` / `_remote` 平行实现。差异下沉到 `InstallTransport` 的 per-transport hook。
- **一次快照、一次解析、不再分支**：命令层先用 `resolve_target_context()` 冻结 target 与 DB，再用 `InstallTransport::for_target(context.target())` 把该 `ActiveTarget` 解析成 transport（SSH/WSL 在此打开一条连接，批量调用整个循环复用），之后任何代码不得重读 active target 或再 `match ActiveTarget`。
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
| row_id（本地 user observation） | 支持 | 忽略 |

## targets 层：CommandRunner 可注入执行缝

```rust
// targets/runner.rs
pub(crate) struct ProcessRequest<'a> {
    command: Command,
    stdin: Option<Vec<u8>>,
    policy: ProcessPolicy,
    cancellation: ProcessCancellation<'a>,
}
pub(crate) trait CommandRunner: Send + Sync {
    async fn run(&self, request: ProcessRequest<'_>) -> Result<Output, RunnerError>;
}
pub(crate) struct ProcessRunner; // 生产默认，统一 async supervision
```

- `ConnectedSshTarget` / `ConnectedWslTarget` 持 `runner: Arc<dyn CommandRunner>`；`base_command()` 纯构建器不动，进程执行一律走 `self.runner.run(...)`。
- 新增远程原语禁止直接 `Command::spawn()` / `.output()`——那会绕开测试缝与 timeout/cancel/bounded-output/process-tree 契约。完整生命周期见 `process-supervision.md`。

## Tests Required

- targets 执行半边：`ConnectedSshTarget::for_tests_with_runner` / 直构 `ConnectedWslTarget { runner }` + `test_support::FakeRunner`（记录 program/args/stdin/policy，FIFO 弹响应）。断言点：完整命令行字符串、stdin 字节、policy class、退出码分支、RunnerPhase/监督错误映射。
- installation 远程路径：`FakeRunner` 假连接 + `mem_pool_with_home("/home/…")`，不需要真 SSH。样板见 `services/installation/tests.rs` 的 `fake_ssh_transport`；断言点：脚本六参顺序（canonical/source/target/agent_dir/method/managed_copy）、stdin == `REMOTE_CENTRAL_INSTALL_SCRIPT`、DB 记账行。

## Wrong vs Correct

```rust
// Wrong：命令层再开分支 / 平行实现
match active_target {
    ActiveTarget::Local => install_skill_to_agent_impl(...).await,
    _ => install_skill_to_agent_remote_impl(...).await,   // 第二份实现必然漂移
}

// Correct：冻结一个 request context，解析一次，编排一份
let context = state.resolve_target_context().await?;
let transport = InstallTransport::for_target(context.target()).await?;
installation::install_skill(context.db(), &transport, &skill_id, &agent_id, method).await
```

## 推广边界

试点仅覆盖 install/uninstall 家族。其他域收敛前先读试点结论（`.trellis/tasks/archive/**/07-04-transport-seam/notes.md`）：central_skills delete/preview 族值得收；scanner/agents/github_import/usage 观望；local_remote_sync（remote-only）与 obsidian（守卫非分发）不收。

> 来源任务：07-04-transport-seam（2026-07-06）

## Scenario: Local observation unlink

### 1. Scope / Trigger

修改 `uninstall_skill(..., row_id=Some(...))`、scanner observation 身份、未使用技能
unlink，或本地 native 技能目录删除时适用。该路径删除真实目录，必须比普通 symlink/copy
卸载更严格。

### 2. Signatures

```rust
pub async fn uninstall_skill(
    pool: &DbPool,
    transport: &InstallTransport,
    skill_id: &str,
    agent_id: &str,
    row_id: Option<&str>,
) -> Result<(), InstallationError>;

pub async fn delete_skill_installation_with_observations(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    observation_row_ids: &[String],
) -> Result<(), sqlx::Error>;
```

### 3. Contracts

- 顶层 target mutation guard 与 `reject_pending_recovery` 先于任何文件系统/DB 修改；
  helper 不重复取锁。
- `row_id` 仅在 Local 生效。按 row 取 observation 后，必须验证 agent、skill、
  `source_kind=user`、`is_read_only=false` 与 scanner row identity；Central agent 和与
  Central 共根的 agent 一律拒绝。
- scanner 只观察 skills 根的直接子目录。删除前目标父目录必须与配置的 agent skills
  根路径等价；仅仅 `starts_with(root)` 不够，根内嵌套目录也必须拒绝。
- row-aware 路径可删除该 observation 证明的 native 真目录；普通卸载仍只允许受管
  symlink/copy，不得把 `allow_native_dir` 推广到 generic path。
- 文件删除成功后，在一个 SQLite transaction 内删除准确 observation row 与该
  agent/skill installation。普通卸载也应在成功移除后按路径等价清理匹配 observation，
  避免 unused report 留陈旧行。
- Claude 可有不同目录映射到同一逻辑 skill，目录身份以 scanner 生成的 `row_id` 区分；
  非 Claude observation 还必须满足目录名归一化后等于 `skill_id`。

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| pending target/skill recovery | fail before filesystem mutation |
| row belongs to another agent/skill | typed mismatch error; keep FS and DB rows |
| plugin/read-only/non-user row | reject; keep FS and DB rows |
| target is Central/shared-root | `SharedCentralUninstall`; never touch Central storage |
| target equals root, is outside root, or is nested below a direct child | reject before delete |
| native direct-child user row | remove directory, then atomically remove observation + installation |
| filesystem removal fails | preserve observation and installation rows |
| DB cleanup fails after removal | return error; never report success |

### 5. Good / Base / Bad Cases

- Good: Codex user observation points to an immediate native skill directory; unlink removes the
  directory and both DB facts under the existing mutation guard.
- Base: generic symlink/copy uninstall succeeds and removes only observations whose `dir_path` is
  path-equivalent to the recorded installation path.
- Bad: accept any descendant under the skills root or infer a delete target from `skill_id` without
  fetching the observation row.

### 6. Tests Required

- Native observation unlink for Claude and a non-Claude agent; observation and installation rows
  disappear only after disk removal succeeds.
- Agent/skill mismatch, missing row, read-only/plugin source, Central/shared-root, root deletion,
  outside-root, nested-root, and inconsistent non-Claude directory identity all preserve disk/DB.
- Generic symlink/copy success removes the matching observation; a failed real-directory generic
  uninstall keeps it. Batch uninstall retains row identity and partial-failure semantics.
- Run focused installation tests, then final `just ci`.

### 7. Wrong vs Correct

```rust
// Wrong: any descendant passes, so a corrupted row can widen deletion scope.
child_parent.canonicalize()?.starts_with(root.canonicalize()?);

// Correct: scanner observations are immediate children only.
paths_equivalent(root, child_parent);
```
