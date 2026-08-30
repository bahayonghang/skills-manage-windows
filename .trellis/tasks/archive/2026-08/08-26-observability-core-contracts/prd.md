# 可观测性核心契约、关联标识与审计生命周期

状态：**planning**。依赖：无；本子任务必须先完成并冻结 interface。

## Goal

建立一个深 observability 模块和单一 command policy registry，使所有 runtime commands 有显式日志策略，
所有需要审计的 operation 使用稳定定义、安全诊断、唯一 operation ID 和可恢复的 lifecycle。

## Requirements

- C1：`ipc_registry.rs` 的每个 runtime command 同点声明 `operation`、`runtime_only` 或受控 `excluded`；
  handler、name inventory 和 policy inventory 从同一 macro 输入生成。
- C2：新 observability interface 接受 typed `OperationDefinition`、target 与 typed safe result，不接受 raw
  error、任意 JSON 或自由 category/action/status。
- C3：Operation Log row ID 在执行前生成并作为 operation/correlation ID；`batch_id` 保持分组语义。
- C4：支持 `TerminalOnly` 与 `StartedThenTerminal`；启动时将上一进程遗留 `started` 标为 `interrupted`，
  不修改 recovery journal 的事实或推断 rollback。
- C5：`IpcError` 增加可选 `correlationId`；失败只接受 reviewed code/message/retryable，Operation details
  增加受控 category/phase，移除自动 raw `Display` error summary。
- C6：Operation Log 写入继续 best-effort；started/final 任一步失败均写安全 Runtime warning且不改变业务结果。
- C7：旧 Operation rows、旧 frontend/backend 和现有 filters/exports 保持兼容；不新增第三方依赖。

## Acceptance Criteria

- [ ] 所有 runtime commands（规划快照 204）由同一 registry 生成唯一 policy，无默认兜底和重复。
- [ ] observability interface 测试覆盖 success/failure/partial/cancel、terminal-only、started/final、startup interrupted。
- [ ] operation ID 同时是 row ID 与 Runtime/IPC correlation；ID 搜索与 export 可用，batch ID 不被复用。
- [ ] raw `Display`、path、host、credential、command/output 对抗种子不进入 IPC、Operation 或 Runtime event。
- [ ] start/final DB 写失败不改变模拟业务 `Result`，且安全 warning 可观察。
- [ ] 旧行 fixture、IPC additive field compatibility、registry parity、generated binding check 通过。

## Out of Scope

- 给每个业务 command 接入 recorder；由三个 coverage children 负责。
- Runtime UI、frontend/backend failure 去重和居中 Dialog。
- 回填历史行、远程 telemetry、真实远端探测或业务语义重构。
