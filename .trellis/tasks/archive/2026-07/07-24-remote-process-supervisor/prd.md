# SSH/WSL 异步进程监督（timeout/cancel/bounded output）

## Goal

远端命令执行获得受控生命周期：命令级超时、取消即 kill、输出有界、不再占用 Tokio worker。对应审计 P1-07（🟠）、M-02；关联 P3-02（SSH/WSL 重复实现）与 P2-11（host key TOFU，仅记录不实现）。

## 核对证据（2026-07-24 dev 分支）

- `src-tauri/src/targets/runner.rs:9,42,65`：`use std::process::{Command, Output, Stdio}`；`.output()` 与 `wait_with_output()` 同步等待，而上层 `run_script`/`run_command` 是 async 签名——"async façade, blocking core"。
- 无 `spawn_blocking` 包装、无 `tokio::process`、无命令级 timeout（SSH `ConnectTimeout=10` 只管连接段）、无 kill-on-cancel、stdout/stderr 无上限收集。
- `src-tauri/src/targets/exec.rs`：SSH 与 WSL 的 run/exists/inspect/read/write/copy/remove 大量平行实现（P3-02）。

## 规划补充证据（2026-07-26 dev 分支）

- `CommandRunner::run` 仍接收 `std::process::Command` 并同步返回 `std::process::Output`；`ConnectedSshTarget` / `ConnectedWslTarget` 的 async 方法会直接调用它，部分 bytes helper 甚至仍为同步函数，因此迁移必须把 runner trait 与所有调用链一起 async 化，不能只在最外层套 timeout。
- 主依赖的 Tokio 1.51 目前只启用 `macros` / `rt-multi-thread` / `sync` / `time`；实现需要在同一依赖上补 `process` 与 `io-util` feature。Tokio 可从既有 `std::process::Command` 转换，并在 Windows 暴露 `Child::raw_handle()`，现有纯 command builder 无需重写。
- `windows-sys 0.61.2` 已由 Tokio/Tauri 等依赖锁定在 `src-tauri/Cargo.lock`，但当前不是本 crate 的直接依赖。Job Object 实现需要 Windows-only direct dependency 的 `Win32_Foundation`、`Win32_System_JobObjects`、`Win32_System_Threading` features。
- 仓库在 DPAPI 与窗口前置逻辑中有手写 Win32 FFI 先例，但 Job Object 需要 `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` 等多层 ABI 结构。手写复制会扩大 unsafe/ABI 维护面；`taskkill /T` 又无法在 future Drop / 应用退出路径提供同等级 RAII 保证。

## 已定决策（2026-07-26）

- **Windows Job Object API 边界**：已批准增加 Windows-only 直接依赖 `windows-sys 0.61`，features 限定为 `Win32_Foundation`、`Win32_System_JobObjects`、`Win32_System_Threading`。锁文件已有 `windows-sys 0.61.2`，不引入新的 crate/version。
- Windows 进程树清理由 Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 保证；不接受 `taskkill /T` 或只终止直接 child 的降级方案。
- 本任务只统一 SSH/WSL 的进程生命周期与策略，不展开 P3-02 的 `RemoteFs`/业务编排重构；P2-11 host-key fingerprint UX 继续独立处理。
- 不新增 legacy runner feature flag。回滚以本子任务原子提交为边界，保留现有 command builder 与 fake seam，避免长期维护两套执行实现。

## Requirements

1. SSH/WSL 命令等待、stdin 写入和 stdout/stderr 读取不得阻塞 Tokio worker；所有现有执行入口统一经过一个可注入的异步监督边界。
2. 快速探测、常规操作和批量传输分别采用明确的 deadline/output policy；timeout、cancel、输出超限和终止失败返回语义化错误变体。
3. stdout/stderr 分别有硬上限；任一流超限立即终止整个进程树，内存占用不随远端输出无限增长。
4. 调用 future 被取消、显式 cancel、deadline、输出超限和应用退出都必须清理直接 child 及其后代；Windows 使用已批准的 Job Object 路径，Unix 提供等价进程组清理。
5. 保留现有 SSH/WSL command builder、askpass 环境和 `CommandRunner`/`FakeRunner` 注入缝，但 runner 与所有同步 bytes helper 一起 async 化。
6. 不改变正常成功输出、非零退出分类、SSH/WSL 既有用户可见错误文案和 target/installation 分层边界，新增监督错误除外。

## Acceptance Criteria

- [ ] fake never-exit process 在 deadline 后被 kill，调用方拿到 timeout 错误；Tokio runtime 其余 command 不受阻塞（并发测试）
- [ ] cancel 或调用 future drop 后 child（含子进程）在有界时间内退出；Windows Job Object handle 关闭与 Unix process-group guard 均有平台定向证明
- [ ] stdout/stderr 任一流超 cap 时中断，返回流别/上限信息，峰值缓冲受 policy 硬上限约束
- [ ] 审计 §7.6 矩阵覆盖：child 忽略终止信号、broken pipe、connect/run/read 各阶段 cancel、应用退出清理、多 target 并发
- [ ] SSH/WSL 生产执行路径不再出现同步 `.output()`、`wait_with_output()` 或 `std::io::Write::write_all()`；同步 fake 也迁为 async seam
- [ ] 现有 SSH/WSL 集成路径（连接测试、扫描、同步）回归通过；`just ci` 通过

## 非目标 / 依赖

- P2-11（`StrictHostKeyChecking=accept-new` 的首连 fingerprint 确认 UX）为独立产品决策，本任务不实现，只在 design.md 记录接口预留。
- 建议后于 07-24-target-context-snapshot（supervisor 的调用方签名会被 TargetContext 改动波及）。
- 属复杂任务：需 design.md + implement.md。
