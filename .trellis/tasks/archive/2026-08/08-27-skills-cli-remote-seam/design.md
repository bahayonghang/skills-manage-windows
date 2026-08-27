# 技术设计 — Skills CLI 远端传输接缝与 spec 修订

对应 `prd.md` 的 R1–R9。本任务不交付面向用户的远端功能，只建立地基。

## 1. 现状结构

### 1.1 Local 闸门的分布

后端唯一的 target 判据：

```247:252:src-tauri/src/services/skills_cli/mod.rs
pub fn ensure_local_target(target: &ActiveTarget) -> Result<(), SkillsCliError> {
    match target {
        ActiveTarget::Local => Ok(()),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => Err(SkillsCliError::LocalTargetOnly),
    }
}
```

它在 `commands/skills_cli.rs` 被调用 **18 次**，每个 IPC 命令一次：
`:108`（doctor）、`:122`（list_global）、`:138`（install_targets）、`:166`（preview_source）、
`:189`（add_global）、`:237`（remove_global）、`:277`（read_skill_md）、`:292`（reveal_skill_folder）、
`:328`（link_platform）、`:376`（unlink_platform）、`:416`（preview_remove_global）、
`:433`（export_inventory）、`:458`（cancel_job）、`:514`（check_updates）、`:551`（update_inventory）、
`:571`（verify_update_baseline）、`:616`（apply_updates）、`:661`（retry_update_recovery）。

这是**好消息**：闸门集中在命令层一处模式，不散落在业务逻辑里。
R4 的替换因此是一次机械的、逐命令的替换，而不是重构。

### 1.2 本机路径解析的入口

`resolve_home_dir()`（`paths.rs:29-36`，读 `HOME` / `USERPROFILE` / `HOMEDRIVE+HOMEPATH`）
在 `skills_cli/` 内有 9 个调用点：
`mod.rs:361`、`lock.rs:242`、`link.rs:41,62`、`files.rs:84,148`、`remove.rs:103,137`、
`updates/apply.rs:124`。

这 9 处是 R2「禁止远端流程调用本机 `resolve_home_dir()`」的全部作业面。

lock 路径规则本身**已经是参数化的**，这对远端化非常有利：

```69:82:src-tauri/src/services/skills_cli/lock.rs
pub fn skills_cli_lock_path_from_env(xdg_state_home: Option<&str>, home_dir: &Path) -> PathBuf {
    match xdg_state_home.filter(|value| !value.trim().is_empty()) {
        Some(state_home) => Path::new(state_home)
            .join("skills")
            .join(".skill-lock.json"),
        None => home_dir
            .join(crate::paths::UNIVERSAL_AGENTS_DIR_NAME)
            .join(".skill-lock.json"),
    }
}
```

分支逻辑不含 `.agents` 字面量（用 `UNIVERSAL_AGENTS_DIR_NAME` 常量），符合 spec `:74-76`。
只有外层 `skills_cli_lock_path` 从本机 `std::env::var("XDG_STATE_HOME")` 取值。

### 1.3 两个可参照的接缝范式

| 范式 | 形状 | 适用 |
| --- | --- | --- |
| `InstallTransport`（`installation/transport.rs:27-30`） | `enum { Local, Remote(Box<ConnectedRemoteTarget>) }`，每个方法内部 `match self` | 操作集小且固定 |
| `Scope` + `FsBackend`（`usage/mod.rs:74-85`、`usage/fs_backend.rs:37-76`） | `Scope` 枚举携带身份与路径；`FsBackend` 是 trait，两个实现 | IO 原语多、需要注入 fake |

Skills CLI 两者都需要：既有身份/路径（canonical root、lock path、平台目录），
又有大量 FS 原语（`symlink_metadata` 等价、读文件、原子写、建目录链接、删树、列目录），
还有进程执行（node）。

### 1.4 远端基座已具备的能力

`ConnectedRemoteTarget`（`targets/remote.rs`）提供：
`remote_home()`（`:50`）、`remote_os()`（`:57`）、`symlink_allowed()`（`:64`）、
`run_script()`（`:71`）、`run_command()`（`:98`）、`exists()`（`:152`）、
`inspect_path() -> Option<RemotePathInfo>`（`:159`）、`mkdir_p()`（`:166`）、
`write_file()`（`:173`）、`read_file_bounded()`（`:187`）、`remove_tree()`（`:209`）、
`list_dir() -> Vec<RemoteDirEntry>`（`:223`）。

`inspect_path` 返回 `RemotePathInfo { file_type, symlink_target }`（`exec.rs:125-129`），
这正是 `std::fs::symlink_metadata` 的远端等价物——placement 分类需要的语义它都有。

**关键物理约束**：无持久会话。`SSH_CONNECT_TIMEOUT_SECS = 10`（`exec.rs:7`），
每次方法调用一次新握手。`ConnectedRemoteTarget` 复用的是配置与 runner，不是 TCP 连接。

## 2. 目标结构

### 2.1 接缝形状：一个门面，三个从属件

选 `Scope`/`FsBackend` 的形状而非 `InstallTransport` 的纯枚举，
因为 Skills CLI 的 IO 原语数量远超 install 的 6 个方法，且测试必须能注入 fake。
但对外只暴露**一个**类型，满足 R1「单一接缝」：

```rust
pub(crate) struct SkillsCliTransport {
    scope: SkillsCliScope,
    caps: SkillsCliCapabilities,
}

pub(crate) enum SkillsCliScope {
    Local,
    Remote(Arc<ConnectedRemoteTarget>),
}

impl SkillsCliTransport {
    pub(crate) async fn for_target(target: &ActiveTarget) -> Result<Self, SkillsCliError>;
    pub(crate) fn paths(&self) -> &SkillsCliPaths;
    pub(crate) fn fs(&self) -> &dyn SkillsCliFs;
    pub(crate) fn runner(&self) -> &dyn SkillsCliRunner;
    pub(crate) fn ensure_capability(&self, cap: SkillsCliCapability) -> Result<(), SkillsCliError>;
}
```

业务逻辑签名从 `(pool: &DbPool, …)` 变为 `(tx: &SkillsCliTransport, pool: &DbPool, …)`。
`match ActiveTarget` 只允许出现在 `for_target` 与 `SkillsCliScope` 的方法体内（AC1 的静态断言点）。

`SkillsCliRunner` trait 已存在（`runner.rs:52-54`），不新造；
远端加一个 `RemoteNodeRunner` 实现，把 `RunnerRequest` 的 program+args 组装成远端命令。

### 2.2 能力矩阵（R4）

`ensure_local_target` 被逐能力查询取代：

```rust
pub(crate) enum SkillsCliCapability {
    Doctor, ListGlobal, InstallTargets, ReadSkillMd, RevealFolder, ExportInventory,
    PreviewSource, AddGlobal,
    LinkPlatform, UnlinkPlatform, PreviewRemove, RemoveGlobal, LeftoverScan,
    CheckUpdates, UpdateInventory, VerifyUpdateBaseline, ApplyUpdates, RetryUpdateRecovery,
}
```

`ensure_capability` 对「远端尚未支持」的能力返回 **既有的**
`SkillsCliError::LocalTargetOnly` → `skills_cli.local_target_only`。
不新增 IPC 码，因此**不需要** `pnpm ipc:codegen`，也不改任何公开句（R9 因此通常不触发）。

**逐任务开闸表**——这张表是远端子树的进度看板，每个子任务只翻自己那几行：

| 能力 | seam（本任务） | inventory | mutate | install-update |
| --- | --- | --- | --- | --- |
| `Doctor` | ✅ 开 | | | |
| `ListGlobal` / `InstallTargets` / `ReadSkillMd` / `ExportInventory` | ✗ | ✅ 开 | | |
| `RevealFolder` | ✗ 永久（远端无本机文件管理器） | ✗ | ✗ | ✗ |
| `LinkPlatform` / `UnlinkPlatform` / `PreviewRemove` / `RemoveGlobal` / `LeftoverScan` | ✗ | ✗ | ✅ 开 | |
| `PreviewSource` / `AddGlobal` / `CheckUpdates` / `UpdateInventory` / `VerifyUpdateBaseline` / `ApplyUpdates` / `RetryUpdateRecovery` | ✗ | ✗ | ✗ | ✅ 开 |

`RevealFolder` 是唯一**永久**不支持远端的能力（`files.rs:147` 调本机文件管理器），
它在矩阵里是显式的一行，不是遗漏。

### 2.3 路径解析（R2）

```rust
pub(crate) struct SkillsCliPaths {
    canonical_root: RemoteOrLocalPath,
    lock_path: RemoteOrLocalPath,
}
```

Local 分支沿用现状：`resolve_home_dir()` + `skills_cli_lock_path_from_env(env XDG, home)`。

Remote 分支：

- `home` 取自 `ConnectedRemoteTarget::remote_home()`，**不调** `resolve_home_dir()`。
- `XDG_STATE_HOME` **必须在远端求值**。本机的同名变量与远端无关，
  猜测会在「本机设了、远端没设」时把 lock 指到不存在的路径。
  取值合并进 §2.4 的 doctor 探测脚本，只花那一次往返。
- 路径拼接用 `crate::targets::remote_join`（`paths.rs:435-445`，POSIX `/`），
  不用 `Path::join`（在 Windows 宿主上会产生 `\`）。
- 分支规则与 `skills_cli_lock_path_from_env` **逐字对应**，同样引用
  `UNIVERSAL_AGENTS_DIR_NAME` 常量而非 `.agents` 字面量：

```rust
pub(crate) fn remote_lock_path(xdg_state_home: Option<&str>, remote_home: &str) -> String {
    match xdg_state_home.filter(|v| !v.trim().is_empty()) {
        Some(state) => remote_join(&remote_join(state, "skills"), ".skill-lock.json"),
        None => remote_join(
            &remote_join(remote_home, crate::paths::UNIVERSAL_AGENTS_DIR_NAME),
            ".skill-lock.json",
        ),
    }
}
```

两个函数的分支等价性由一条参数化测试守住（AC3），避免日后只改一边。

### 2.4 远端 doctor（R3）

一次 `run_script` 拿齐所有信息，**往返次数为 1，与平台数无关**：

```sh
# 单脚本，四行输出，固定顺序
printf 'XDG=%s\n' "${XDG_STATE_HOME-}"
printf 'HOME=%s\n' "$HOME"
if command -v node >/dev/null 2>&1; then
  printf 'NODEV=%s\n' "$(node --version 2>/dev/null)"
else
  printf 'NODEV=\n'
fi
```

- 版本解析复用既有 `parse_node_version`（`argv.rs:251`），不另写解析。
- `NODEV` 为空 → `SkillsCliError::NodeMissing`；解析出的版本 < 22.20 → 同样 `NodeMissing`
  （与本机一致：`error.rs:160` 把 `NodeMissing | NodeTooOld` 映射同一个码）。
- **与 `doctor-gate` 对齐**：不探测 `skills --help`。远端 doctor 只回答「Node 够不够」。
  这是 `doctor-gate` D1 的直接继承，不发明第三套语义。
- 顺带产出的 `XDG` 供 §2.3 使用，`HOME` 用于与 `remote_home()` 交叉校验（不一致时以配置为准并 warn）。
- 脚本走 `ConnectedRemoteTarget::run_script`，其策略是 `ProcessPolicy::standard()`（120s）。

### 2.5 命令入口顺序（R5）

现状每个命令已经是「先 `resolve_target_context()` 再 `ensure_local_target`」
（如 `commands/skills_cli.rs:187-189`）。改动是把第二步换掉、第三步新增：

```rust
let context = state.resolve_target_context().await?;          // 冻结 target + DbPool（不变）
let tx = SkillsCliTransport::for_target(context.target())     // 建立传输
    .await.map_err(|e| to_ipc_error(&e))?;
tx.ensure_capability(SkillsCliCapability::AddGlobal)          // 取代 ensure_local_target
    .map_err(|e| to_ipc_error(&e))?;
```

`context` 在整个 `.await` 链中不重新解析，符合 `target-context.md`。

## 3. 数据流

```
IPC 命令
  → resolve_target_context()            冻结 ActiveTarget + DbPool
  → SkillsCliTransport::for_target()    Local: 零成本 / Remote: connect_remote_target
  → ensure_capability(cap)              未开闸 → local_target_only（零写）
  → 业务逻辑(&tx, …)                     只通过 tx.paths() / tx.fs() / tx.runner() 访问外界

远端 doctor
  → tx.runner().run(node --version)     实际是一次 run_script，同时带回 XDG / HOME
  → parse_node_version                  复用本机解析
  → SkillsCliDoctorReport               形状与本机完全一致
```

## 4. 契约与兼容性

- **IPC 形状零变化**：不新增命令、不改签名、不新增错误码。
  因此 `pnpm docs:gen` 与 `ipc_registry` 日志策略都不触发（R9 的条件未满足，仍需在收尾确认）。
- `SkillsCliDoctorReport { node_version, npm_spec }` 远端返回同样字段，
  `npm_spec` 仍是 PIN 常量（声明值，不代表远端已验证可执行）。
- 业务函数签名新增 `&SkillsCliTransport` 参数——内部契约变更，测试需同步。
- `skills_cli_lock_path_from_env` 保持不变；新增的 `remote_lock_path` 与它并列。
- spec 修订见 R6，与代码同批提交。

## 5. 权衡

- **一次往返的 doctor vs 更细的诊断**：合并脚本让 doctor 无法区分「node 不存在」与
  「node 存在但执行失败」。两者都映射 `node_missing`，与本机 doctor 在
  `doctor-gate` §2.4.1 后的行为一致，所以不是新的信息损失。
- **enum scope vs 泛型**：泛型能在编译期消除分发，但会把 `SkillsCliTransport`
  的类型参数传染到所有业务函数签名与 18 个命令。选枚举 + trait object，
  与仓库既有两个范式一致。
- **不做持久会话池**：属 `plans/ssh-perf`，本树 Out of Scope。
  代价是 §2.4 的一次往返和后续子任务的分块设计都要围绕「握手贵」来做。

## 6. 回滚点

| 单元 | 内容 | 可否单独回滚 |
| --- | --- | --- |
| A | 接缝类型 + 路径解析 + Local 实现（业务逻辑改为走 tx，行为不变） | 可 |
| B | 能力矩阵替换 18 处 `ensure_local_target` | 依赖 A |
| C | 远端 doctor + `RemoteNodeRunner` | 依赖 A |
| D | spec 修订 | 与 B、C 同批 |

A 是纯重构，若 B/C 出问题可只回滚它们而保留 A。
D 不能单独保留——spec 说远端支持而代码不支持，就是 PAC4 禁止的矛盾中间态。
