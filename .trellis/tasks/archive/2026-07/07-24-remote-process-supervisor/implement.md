# 实施计划：SSH/WSL 异步进程监督

## 1. 激活与规范加载

- [x] 在最终规划摘要获得再次批准后，运行 `python ./.trellis/scripts/task.py start 07-24-remote-process-supervisor`；不得把本次依赖审批当成 start 审批。
- [x] 加载 `trellis-before-dev`，阅读 backend index、transport seam、domain errors、test support、target context 与 CI quality gate。
- [x] 记录并保护现有 Trellis runtime/config、父任务、其他子任务和审计报告改动，只提交本子任务产品/规范/归档文件。

## 2. Supervisor 与平台 guard

- [x] 给主 Tokio 依赖增加 `process`、`io-util`；按已批准配置增加 Windows-only `windows-sys 0.61` 直接依赖并核对锁文件只改变 direct-use metadata/必要 feature 汇合。
- [x] 定义 `ProcessPolicy`、三类生产默认值、`ProcessCancellation`（含现有 `AtomicBool` 的 <=50 ms 过渡适配器）、拥有型 `ProcessRequest`、stream/phase/error enums。
- [x] 实现 bounded stdout/stderr reader 与异步 stdin writer；limit 检查不得先无界扩容。
- [x] 实现 `ProcessSupervisor` completion/deadline/cancel/overflow select、显式 tree terminate、有界 reap 与 `kill_on_drop(true)`。
- [x] 实现 Windows Job Object RAII：create、limit、assign、terminate、Drop；任何 assign 失败都终止 child。
- [x] 实现 Unix 新 process group 与最小 RAII group kill；直接 child 始终 reap。

## 3. Async seam 与 SSH/WSL 迁移

- [x] 将 `CommandRunner::run`、`ProcessRunner`、`FakeRunner` 和 `CancellingRunner` 改为 async owned-request contract，继续记录 program/args/stdin/policy。
- [x] 扩展 `RunnerError` 并在 SSH/WSL 映射函数中保留 Start/WriteStdin/Wait 旧文案；新增 semantic `TargetsError` 变体不得泄露 command/env/output。
- [x] 迁移 SSH run/script/stdin/bytes/exists/inspect/read/write/copy/list 等全部入口，删除同步 bytes helper。
- [x] 迁移 WSL 对等入口，并更新 `ConnectedRemoteTarget` wrapper 全部 await 链。
- [x] 在 probe/standard/bulk 语义入口显式选择 policy；为需要即时取消的 central update/sync/import 调用链传 process cancellation control，不重构 P2-01 JobRegistry ownership。
- [x] 回归 askpass 环境、隐藏窗口 flag、参数顺序、stdin 字节和非零退出分类。

## 4. 监督与回归测试

- [x] 用毫秒级 test policy 覆盖 never-exit timeout；断言错误类别、child reap 和 runtime 独立 task 不被阻塞。
- [x] 覆盖显式 cancel、future drop、应用退出 guard、child 忽略普通终止、父子 process tree 全部退出。
- [x] 分别覆盖 stdout/stderr 超限，断言 stream/limit 且缓冲不超过硬上限；覆盖 stdin broken pipe 与 reader/wait/termination IO。
- [x] 覆盖 cancel during connect/run/read，以及多 SSH/WSL target 并发隔离。
- [x] 更新 targets、test_support、central_updates fake tests，断言 command/env/stdin/policy 与成功/失败兼容。
- [ ] Windows Job Object 定向测试已通过；非 Windows process-group cross-check 已尝试，但在 repository code 前被缺少 cross-compilation `libdbus` pkg-config/sysroot 阻断，未作 live PASS 报告。

## 5. 静态审计与分层验证

- [x] `rg -n "std::process::Command|\.output\(\)|wait_with_output|std::io::Write" src-tauri/src/targets`，逐项证明仅纯 builder/类型兼容或测试允许，生产等待/写入为零。
- [x] `cd src-tauri; cargo test process_supervisor --locked`
- [x] `cd src-tauri; cargo test targets --locked`
- [x] `cd src-tauri; cargo test central_updates --locked`
- [x] `cd src-tauri; cargo fmt --all -- --check`
- [x] `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`
- [x] `cd src-tauri; cargo test --locked`
- [x] `just ci`

## 6. Spec、检查、提交与归档

- [x] 新增 backend process-supervision spec；同步 backend index、transport seam 与 domain-error-enums。
- [x] 运行 `trellis-check`，检查 async 调用链、错误映射、跨平台 Drop 路径、测试矩阵和 diff scope。
- [x] 检查 `Cargo.lock` 没有新增 crate/version，且 Windows-only features 与审批完全一致。
- [ ] 形成一个原子工作提交；失败回滚整提交，不留下 legacy feature flag 或双 runner。
- [ ] 再次确认 `just ci` 通过，归档 `07-24-remote-process-supervisor`，更新父任务完成计数并记录 journal；不 push。
