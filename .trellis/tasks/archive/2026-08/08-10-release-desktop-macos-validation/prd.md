# 修复 Release Desktop macOS 校验失败

## Goal

消除 `v1.0.1` Release Desktop 在 macOS Rust 质量门中的非确定性失败，让输出超限测试稳定验证“runner 主动终止仍存活的进程树”，同时保持生产环境对真实终止/回收失败的 fail-closed 语义。

## Background

- Release Desktop run [31308665794](https://github.com/bahayonghang/skills-manage-windows/actions/runs/31308665794) 绑定 tag 对应提交 `365b7080a63cd9489e00b3314d866e8537b313d7`，仅 `Validate frozen release commit / rust (macos-14)` 失败；其他 required lanes 已通过，后续构建与发布 job 因质量门失败而跳过。
- 精确失败为 `targets::runner::tests::stdout_overflow_terminates_the_process` 得到 `Err(TerminationFailed { trigger: OutputLimit, source: PermissionDenied("Operation not permitted") })`，而断言要求 `OutputLimitExceeded { stream: Stdout, limit: 1024 }`。该 job 共通过 1185 个测试，仅此 1 个失败。
- 同一提交的 rehearsal run [31306113181](https://github.com/bahayonghang/skills-manage-windows/actions/runs/31306113181) 在约一小时前通过完整 macOS Rust lane 和 macOS universal bundle，排除了固定的源码、工具链或 universal CLI 缺失。
- 归档任务 `08-01-ci-feedback-acceleration` 记录过同类现场：macOS 的 `stderr_overflow_terminates_the_process` 曾以相同 `TerminationFailed(OutputLimit, PermissionDenied)` 瞬时失败，rerun 后通过。该竞态此前未被消除。
- `large_stdout` / `large_stderr` fixture 写完有限数据后立即自然退出。runner 观察到输出超限并关闭读取端时，fixture 可能正好退出；macOS 对仍未回收的进程组执行 `SIGKILL` 可在该窗口返回 `EPERM`，生产代码按契约将其提升为 `TerminationFailed`。
- Windows 本地最小命令 `cargo test --manifest-path src-tauri/Cargo.toml --locked targets::runner::tests::stdout_overflow_terminates_the_process -- --exact --nocapture` 已通过 1 次。它锁定了目标断言，但 macOS hosted runner 仍是该平台竞态的权威验证环境。
- 远端 `v1.0.1` tag 已存在；GitHub Release 尚未创建，失败发生在 draft 创建之前。

## Requirements

1. 修复必须限定在 test-only overflow fixture 及其对应监督规范，不改变生产 `ProcessRunner`、`terminate_and_reap`、`ProcessTreeGuard` 或错误映射。
2. stdout 与 stderr fixture 必须先产生明显超过 1024-byte 测试上限的数据，再保持存活超过各自 request deadline，使测试稳定进入主动进程树终止路径，而不是与自然退出竞争。
3. 真实的 tree terminate 或 direct-child reap 失败必须继续返回 `TerminationFailed`，不得忽略 `EPERM`、放宽 overflow 测试断言或通过 workflow retry 掩盖失败。
4. 修复必须同时覆盖 stdout 与 stderr 两条对称路径，并保留 timeout、cancellation、descendant cleanup、closed stdin 等相邻监督行为。
5. 更新 `process-supervision.md`，固化“需要验证主动终止的 fixture 必须保持存活直至 supervisor 终止”的测试约定，防止相同竞态再次进入 CI。
6. 完成 Windows 本地聚焦压力验证、Rust 格式/Clippy/完整测试和仓库 `just ci`；没有实际 macOS hosted 运行时，不得把平台修复报告为 hosted PASS。

## Acceptance Criteria

- [x] stdout 与 stderr overflow 聚焦测试在本地各重复运行 25 轮，全部返回各自的 `OutputLimitExceeded`，无 `TerminationFailed`、timeout 或零测试误通过。
- [x] `targets::runner::tests` 模块测试全部通过，覆盖 timeout、cancellation、stdout/stderr limit、closed stdin 和 descendant cleanup。
- [x] `cargo fmt --all -- --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked` 和 `just ci` 全部通过。
- [x] 最终 diff 不修改生产进程终止逻辑、release workflow、版本元数据或打包配置；除 Trellis 记录/规范外，仅修改 `src-tauri/src/targets/runner.rs` 的 test-only fixture。
- [x] 本次没有 push/PR 授权，新的 exact-head macOS Rust lane 记录为未执行的 hosted 验证，而不是本地 PASS；后续若单独授权，仍必须验证两条 overflow 测试。

## Local Evidence

- stdout exact `1/1`、stderr exact `1/1`；独立压力验证 stdout `25/25`、stderr `25/25`。
- `targets::runner::tests`：`8 passed / 1 ignored`，过滤集合非零。
- Rust fmt 与 Clippy `-D warnings` 通过；完整 locked Rust suite：`1198 passed / 0 failed / 7 ignored`。
- `just ci` 143.81 秒通过：Vitest `149 files / 1641 passed / 1 skipped`，Rust `1198 passed / 7 ignored`，lint 与 typecheck 通过。
- Trellis validation 与 `git diff --check` 通过；独立 `trellis-check` 未发现问题，也未修改文件。
- 本地实际 Node/pnpm 为 `24.14.0` / `11.16.0`，与声明的 `22.x` / `10.12.3` 不同；`just ci` 报告 warning 但通过。该环境漂移不计为声明版本上的验证。

## Out Of Scope

- 将 `EPERM` 或任意 process-group termination error 视为成功。
- 修改生产 timeout/cancellation/output-limit 错误优先级或 descendant cleanup 行为。
- 给 Release Desktop 增加自动 retry、删除 macOS gate，或调整 runner image/toolchain 来绕过 fixture 竞态。
- 修改 tag、GitHub Release、draft、environment、Secrets、ruleset 或分支保护。
- 未经单独授权 push 分支、创建 PR、rerun workflow 或重新发布 `v1.0.1`。

## Risks And Deferred Evidence

- 当前主机是 Windows，无法本地执行 Darwin process-group 行为；本地压力测试证明 fixture 生命周期和跨平台回归，macOS hosted CI 才能提供最终平台证据。
- fixture 使用有界长 sleep 保持存活；正常路径会由 supervisor 立即终止，不增加测试墙钟时间。若 overflow 机制回归，现有 3/5 秒 deadline 会有界失败，不会永久挂起。
