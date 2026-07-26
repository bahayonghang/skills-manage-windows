# 设计：SSH/WSL 异步进程监督

## 1. 系统不变量与范围

每次 SSH/WSL 执行只有五类终态：正常退出、启动/IO 失败、deadline、显式取消、输出超限。除正常退出外，只要 child 已产生，就必须先终止进程树并完成有界 reap，再把错误返回调用方。监督 future 被 drop 或应用退出时，RAII guard 也必须清理整棵进程树。

本任务统一的是 `targets` 层 process lifecycle，不合并 SSH/WSL 的远端文件系统业务实现。P3-02 的 `RemoteFs`/编排去重和 P2-11 host-key fingerprint UX 不在本子任务内；现有 command builder、askpass 环境、退出码分类与 installation transport seam 保持不变。

## 2. 依赖与模块边界

在现有 Tokio 依赖上增加 `process`、`io-util` features。增加已批准的 Windows-only 直接依赖：

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_System_JobObjects",
  "Win32_System_Threading",
] }
```

`targets/runner.rs` 继续拥有注入缝并承载平台无关 supervisor；平台树清理下沉到 `targets/process_tree.rs`（内部用 `cfg(windows)` / `cfg(unix)` 分支）。生产代码不得绕过 `CommandRunner` 直接 spawn SSH/WSL。

## 3. 请求、策略与异步 seam

runner 接收拥有型请求，避免 stdin/控制对象跨 await 借用：

```rust
pub(crate) struct ProcessRequest {
    command: std::process::Command,
    stdin: Option<Vec<u8>>,
    policy: ProcessPolicy,
    cancellation: ProcessCancellation,
}

#[async_trait]
pub(crate) trait CommandRunner: Send + Sync {
    async fn run(&self, request: ProcessRequest) -> Result<std::process::Output, RunnerError>;
}
```

`ProcessCancellation` 是 targets 层的小型异步等待抽象，当前支持 `Never` 与借用既有 `AtomicBool`。适配器在 Tokio interval 上以最多 50 ms 的响应窗口检查标志，因此无需改写 P2-01 的全局 JobRegistry/lease ownership。显式 process cancellation 必须沿需要即时终止的 remote operation 调用链传入，不能只在循环边界检查 `AtomicBool`；后续 lease 子任务可在不改 supervisor API 的情况下替换取消源。

`ProcessPolicy` 固化三类默认值，测试可构造毫秒级 policy，但生产值不做 UI/setting 配置：

| class | 适用入口 | deadline | stdout/stderr cap（各自） |
|---|---|---:|---:|
| Probe | SSH/WSL probe、exists、inspect | 30 s | 1 MiB |
| Standard | run/read/write/mkdir/remove/list | 120 s | 8 MiB |
| BulkTransfer | copy/sync/import/update 批量脚本 | 15 min | 32 MiB |

policy 在语义调用点显式选择，不解析 shell 字符串猜测。`ConnectedSshTarget`、`ConnectedWslTarget` 与 `ConnectedRemoteTarget` 的现有便捷方法委托默认 policy；批量/可取消调用增加内部 control 版本。所有当前同步 bytes helper 改为 async，调用链逐层补 `.await`。

## 4. Supervisor 数据流

1. 从既有 `std::process::Command` builder 转换为 `tokio::process::Command`，设置 piped stdout/stderr、按 stdin 是否存在选择 piped/null，并开启 `kill_on_drop(true)`。
2. 创建平台 process-tree guard，spawn child 后立即纳入 Job Object/process group；纳入失败时不得无监督继续运行，必须杀掉 child 并返回启动阶段错误。
3. stdout/stderr 各自用固定 chunk 异步读取，分别累计到独立 buffer；追加前检查 `buffer.len() + chunk.len()`，超出 limit 时不追加超额 chunk，立即返回 overflow。stdin 在独立 async 分支 `write_all` 后关闭，避免与大量输出互相等待。
4. 一个 completion future 并发等待 stdin writer、两条 reader 与 `child.wait()`；外层 `tokio::select!` 同时监听 completion、deadline 和 cancellation。reader overflow 会立即使 completion 失败并进入同一终止路径。
5. timeout/cancel/overflow/IO 失败统一调用 tree termination，再在短 grace deadline 内 reap child；若终止或 reap 失败，保留原始原因并附加语义化 termination error，不静默泄漏进程。

竞态采用确定性优先级：已经观察到的启动错误直接返回；child 正常完成优先保留真实退出结果；尚未完成时，显式 cancel 优先于 deadline，任一输出超限优先于继续等待。正常非零 exit 仍由 SSH/WSL 现有 `remote_command_error` 分类。

## 5. 跨平台进程树

### Windows

spawn 前创建 Job Object，设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`；spawn 后通过 Tokio child 的 raw process handle 立即 `AssignProcessToJobObject`。guard 独占 Job handle，显式失败路径可 `TerminateJobObject`，Drop 关闭 handle，应用退出/future drop 自动清理 Job 内全部进程。所有 Win32 返回值转为 `std::io::Error::last_os_error()`，handle 使用单一 RAII 类型，unsafe 只留在该模块。`windows-sys` 对 `CreateJobObjectW` 的 generated binding 额外 gate 到未批准且本调用不需要的 `Win32_Security`，因此仅该 null-security-attributes creator 使用最小 FFI，其余 Job ABI 均使用批准 features 下的 generated API。

### Unix

command 在 spawn 前设置新 process group；guard 保存 pgid。显式终止与 Drop 都向负 pgid 发送强制终止信号，并 reap 直接 child。只为稳定的 `kill(pid_t, signal)` ABI 写最小 `cfg(unix)` FFI，不新增通用 Unix dependency。

两端都保留 `kill_on_drop(true)` 作为直接 child 的最后防线，但验收以整棵树消失为准，不能把它当成 process-tree 实现。

## 6. 错误契约

`RunnerError` 从仅有 `{ phase, source }` 扩展为明确类别：

- `Io { phase: Start | WriteStdin | ReadStdout | ReadStderr | Wait | Terminate, source }`
- `TimedOut { class, deadline }`
- `Cancelled`
- `OutputLimitExceeded { stream: Stdout | Stderr, limit }`
- `TerminationFailed { reason, source }`

`ssh_runner_error` / `wsl_runner_error` 继续单点映射到 `TargetsError`。Start/WriteStdin/Wait 保留现有前缀；broken pipe 归入 WriteStdin。新增 timeout/cancel/output-limit/termination 变体携带 transport、policy/stream 等稳定字段，Display 不包含 stdout/stderr 内容、密码、命令行或 askpass 环境。connect/run/read 阶段通过调用 policy 与 reader/writer phase 测试覆盖，不把敏感 command spec 放进错误。

## 7. 迁移顺序与兼容性

1. 先落 policy、cancel token、错误类型和平台 guard，并用 supervisor 级测试封闭生命周期。
2. 将 `ProcessRunner` 与 `FakeRunner` 改成 async owned-request seam。
3. 迁移 SSH 全部 run/script/bytes/FS helper，再迁移 WSL 对等入口；每一侧迁完即跑 targets tests，避免半边继续阻塞。
4. 迁移 `ConnectedRemoteTarget` 和 central update 等直接依赖同步 bytes helper 的调用链，为 bulk/cancel 场景显式传 control。
5. 最后用静态断言清零 targets 生产路径中的同步 process wait/write。

不修改 IPC payload、数据库 schema、target 配置或正常错误文案。askpass command 环境由原 builder 产生，转换为 Tokio command 后必须通过既有记录式 fake 和密码认证回归证明未丢失。

## 8. 测试矩阵

- supervisor 单元/平台测试：never-exit timeout、显式 cancel、future drop、stdout cap、stderr cap、stdin broken pipe、reader error、终止失败。
- runtime 公平性：挂起一个 process 时，单 worker Tokio runtime 上的独立 task 仍能按时完成；多个 SSH/WSL fake target 并发互不阻塞。
- process tree：child 忽略普通终止并生成 descendant；timeout/cancel/drop 后用 PID/marker 证明父子均退出。Windows 单独覆盖 Job handle Drop，Unix 覆盖 process group。
- transport 回归：SSH/WSL command/args/stdin、probe/exists/inspect、非零退出与 UTF-8 错误；askpass env 不变。
- 作业回归：扫描、central sync/import/update 的 bulk policy 与 cancel during connect/run/read。

## 9. Spec、回滚与完成门禁

新增 backend `process-supervision.md` 并在 index 登记；更新 `transport-seam.md` 的 async runner 签名、禁止绕过规则与测试契约，更新 `domain-error-enums.md` 的新增监督错误。

不保留 legacy feature flag，避免两套 runner 漂移。实现按“基础设施 + 全调用链迁移 + tests/spec”组成一个原子工作提交；若验收失败，回滚该提交即可恢复旧 runner。提交前必须通过 Rust 定向检查、全量 `cargo` gate 与 `just ci`，且不得混入其他审计子任务或 Trellis runtime 改动。
