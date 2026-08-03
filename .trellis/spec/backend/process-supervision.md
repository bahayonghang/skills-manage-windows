# Async Process Supervision Contract

## 1. Scope / Trigger

适用于 `targets` 层启动的 SSH、WSL 命令与 WSL discovery。任何新增远端 process 入口都必须经过 `CommandRunner`，不得在 async 函数内直接调用同步 `.output()` / `wait_with_output()`，也不得绕开 supervisor 自行收集无界输出。

## 2. Signatures

```rust
pub(crate) struct ProcessRequest<'a> {
    command: std::process::Command,
    stdin: Option<Vec<u8>>,
    policy: ProcessPolicy,
    cancellation: ProcessCancellation<'a>,
}

#[async_trait]
pub(crate) trait CommandRunner: Send + Sync {
    async fn run(&self, request: ProcessRequest<'_>)
        -> Result<std::process::Output, RunnerError>;
}

pub(crate) enum ProcessCancellation<'a> {
    Never,
    Atomic(&'a AtomicBool),
}
```

生产 policy 是单一来源：Probe = 30 s / 每流 1 MiB；Standard = 120 s / 每流 8 MiB；BulkTransfer = 15 min / 每流 32 MiB。测试通过 `ProcessPolicy::for_tests` 使用短 deadline/cap，不修改生产常量。

Bounded remote file reads retain the Standard 120 s deadline but derive stdout
capacity from the caller's explicit file budget: exactly `max_bytes + 1`.
This permits a reviewed 32 MiB repository entry without silently falling back
to the Standard 8 MiB cap, while still giving every read a hard allocation
ceiling. Stderr remains capped at 1 MiB.

## 3. Contracts

- 保留 `std::process::Command` 纯 builder；runner 转换为 `tokio::process::Command`，设置 `kill_on_drop(true)`，异步并发执行 stdin writer、stdout/stderr bounded reader 与 child wait。
- stdout/stderr 分别在追加 chunk 前检查 cap；超额 chunk 不进入 buffer。任一 reader 超限、IO 失败、timeout 或 cancel 都进入同一 tree terminate + 最长 5 s reap 路径。
- 现有 `AtomicBool` 作业取消通过 50 ms Tokio polling adapter 接入。Central batch 仍在 chunk 之间检查取消，同时把同一 flag 传给正在运行的 bulk process。
- Windows child 在返回调用方前必须进入带 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 的 Job Object；assign 失败时杀掉并 reap child，禁止降级为无监督执行。`windows-sys` 只启用已批准的 Foundation/JobObjects/Threading features；由于 generated `CreateJobObjectW` 被不必要地 gate 到 `Win32_Security`，仅该 null-security-attributes 函数使用最小 FFI，其他 ABI 全用 generated types/functions。
- Unix command spawn 前进入新 process group；取消和 guard Drop 对负 pgid 发强制终止，直接 child 始终 reap。`kill_on_drop` 只是直接 child 后备，不等价于 process-tree 清理。
- 正常非零退出继续由 SSH/WSL transport 分类。错误、日志与 operation details 不包含 command、host、username、stdin/stdout/stderr 或 askpass 环境。

## 4. Validation & Error Matrix

| 条件 | 结果 |
| --- | --- |
| spawn / stdin / reader / wait IO 失败 | `RunnerError::Io { phase, source }`；Start/WriteStdin/Wait 保留旧用户文案 |
| deadline 到达 | 终止树并 reap；`TargetsError::ProcessTimedOut` 携带 transport/class/ms |
| cancel flag 置位 | 最迟 50 ms 被观察；终止树并返回 `ProcessCancelled` |
| stdout 或 stderr 超 cap | 不追加超额 chunk；终止树并返回 stream/limit |
| tree terminate 或 reap 失败 | `ProcessTerminationFailed` 携带触发原因和 source；不得假报原始 timeout/cancel 已安全完成 |
| future drop / 应用退出 | Job handle/process-group guard Drop 清理后代；直接 child 由 `kill_on_drop` 兜底 |
| Job Object assign 失败 | kill + reap direct child，返回 Start IO；不得继续运行 |

## 5. Good / Base / Bad Cases

- Good：Central bulk script 使用 BulkTransfer policy，并把现有 cancel flag 传到正在等待的 process。
- Base：普通 `run_command` 使用 Standard policy；`exists`、`inspect`、probe 与 WSL list 使用 Probe policy。
- Bad：在新 helper 里调用 `Command::output()`；用 `taskkill /T`；只 kill direct child；先 `read_to_end` 再检查长度；为 rollback 保留第二套 legacy runner。

## 6. Tests Required

- 真实 fixture process：never-exit timeout、单线程 runtime 公平性、两个并发 supervisor、stdin broken pipe、stdout/stderr cap。broken-pipe fixture 必须关闭继承的 OS stdin descriptor/handle；仅 drop `std::io::stdin()` 返回的非 owning 句柄不会关闭底层 pipe，不能作为跨平台证据。并发性用两个 child 互等 marker 的 barrier 证明启动重叠，禁止用紧固定墙钟阈值；后者在 `just ci` 的 Web/Rust 并行负载下会产生假失败。
- process tree：fixture parent 生成 descendant，cancel 和 supervisor future drop 后等待 marker 窗口，断言 descendant 未存活；Windows 由 Job Object 路径执行，Unix 由 process group 路径执行。
- FakeRunner：program/args/stdin 保持字节兼容，并断言 probe/standard/bulk policy 选择。
- TargetsError：timeout 与 output cap 映射为 typed variant；既有 start/write/wait Display 文案不变。
- 运行 `cargo test targets --locked`、受影响服务定向测试、全量 locked Rust gate 与 `just ci`。

## 7. Wrong vs Correct

```rust
// Wrong: blocking wait + unbounded capture in an async call chain.
let output = command.output()?;

// Correct: one injectable lifecycle boundary with explicit semantics.
let request = ProcessRequest::new(command, ProcessPolicy::probe())
    .with_cancellation(cancel.into());
let output = runner.run(request).await?;
```

> 来源任务：07-24-remote-process-supervisor（2026-07-26）
