# Typed IPC 剩余命令收口

## Goal

将剩余前端调用的 Tauri command 分批纳入现有 Rust-derived typed registry，让命令名、参数与返回值漂移在构建期失败，并最终删除生产 untyped 调用入口。

## Findings

- `ARCH-002`（Medium / M-L）：`src/lib/ipc/invoke.ts:45-53` 保留 string/generic fallback overload，`src/lib/ipc/commandMap.ts:394-442` 在审计时有 47 个未类型化命令。
- `src-tauri/src/ipc_registry.rs` 已是 runtime command 唯一注册表，`__skillport_generated_commands`、`src-tauri/src/ipc_codegen.rs`、`generatedCommandMap.ts` 和 parity tests 已提供可靠扩展路径；本任务不得建立第二套 IPC。

## Requirements

- R1： [ARCH-002] 实施开始时用 `UNTYPED_IPC_COMMANDS` 与生产 invoke 扫描重测并记录精确 baseline；审计时 baseline 为 47，任何差异先解释再迁移。
- R2： [ARCH-002] 每个剩余命令必须从 `ipc_registry.rs` 的既有 Rust command signature/Serde metadata 生成参数与返回类型；仅为实际 command boundary DTO 补 `specta::Type`，不得手写平行 schema。
- R3： [ARCH-002] 47 个命令按六个互不重叠的领域批次迁移；每批把对应命令加入 `__skillport_generated_commands`，更新唯一生成物，移除调用点显式泛型和对应 allowlist 条目，并保持该批行为测试通过。
- R4： [ARCH-002] baseline 归零后删除 `UNTYPED_IPC_COMMANDS` 和 `invoke`/`invokeRaw` 的 string/generic production fallback；错误命令名、缺失/多余参数和错误返回类型必须成为 TypeScript 编译错误。
- R5： [ARCH-002] runtime registry、generated/handwritten typed maps、frontend invoked commands 与 backend-only allowlist 保持现有 parity 和互斥；target-only command isolation 不变。
- R6： [ARCH-002] IPC codegen 与 docs generation 保持确定性/只读 check；组件/page 仍不得直接导入 Tauri invoke。
- R7： [ARCH-002] 不得新增第二个 invoke wrapper、运行时 schema validator、command alias、旧名 fallback 或业务 store/component 重写。

## Acceptance Criteria

- [x] AC1（R1）：实施记录给出可复跑扫描命令和精确 baseline count/name list。
- [x] AC2（R1）：六批命令集合的并集等于 baseline，且批次之间没有重复。
- [x] AC3（R2）：每个迁移命令都出现在 `GENERATED_IPC_COMMAND_NAMES`。
- [x] AC4（R2）：每个迁移命令的 generated args/result 都不含 `unknown`。
- [x] AC5（R2）：Rust command signature 或 Serde rename fixture 变更会触发 codegen byte drift。
- [x] AC6（R3）：完成任一批后，该批命令从 `UNTYPED_IPC_COMMANDS` 消失。
- [x] AC7（R3）：完成任一批后，该批生产调用点不再使用显式返回泛型。
- [x] AC8（R3）：批次迁移期间，尚未迁移的命令仍能通过原入口工作。
- [x] AC9（R3）：六批分别通过 IPC coverage/codegen check 与对应 store/runtime tests，并可独立 revert。
- [x] AC10（R4）：最终 baseline 为 0 后，`UNTYPED_IPC_COMMANDS` export 被删除。
- [x] AC11（R4）：`invoke.ts::invoke` 不再暴露接受任意 string 的 overload。
- [x] AC12（R4）：`invoke.ts::invokeRaw` 仅接受 typed command key 并推导 args/result。
- [x] AC13（R4）：错误 command name 的 `@ts-expect-error` fixture 被编译器拒绝。
- [x] AC14（R4）：缺失参数的 `@ts-expect-error` fixture 被编译器拒绝。
- [x] AC15（R4）：多余参数的 `@ts-expect-error` fixture 被编译器拒绝。
- [x] AC16（R4）：错误参数类型的 `@ts-expect-error` fixture 被编译器拒绝。
- [x] AC17（R4）：错误返回赋值的 `@ts-expect-error` fixture 被编译器拒绝。
- [x] AC18（R5）：每个 frontend invoked command 恰好存在于 generated 或 handwritten typed map。
- [x] AC19（R5）：runtime set 与 frontend set 的差集精确等于既有 backend-only commands。
- [x] AC20（R5）：generated 与 handwritten map 无重叠。
- [x] AC21（R5）：target isolation tests 通过，且 target-only command 未暴露为通用本地调用。
- [x] AC22（R6）：`pnpm ipc:codegen:check` 通过且不修改工作树。
- [x] AC23（R6）：IPC parity contract 通过。
- [x] AC24（R6）：`pnpm docs:gen:check` 通过且不修改工作树。
- [x] AC25（R6）：production component/page 的直接 Tauri invoke 数量保持 0。
- [x] AC26（R7）：全仓 transport adapter 仍只有 `src/lib/ipc/invoke.ts`。
- [x] AC27（R7）：diff 未新增 runtime validator、command alias、compatibility layer 或第二 wrapper。
- [x] AC28（R7）：领域行为测试、typecheck、Vitest、Rust tests 与 `just ci` 全部通过。

## Out of Scope

- 更换 Tauri IPC 协议、改造现有 error normalization 或增加运行时验证框架。
- 顺带重写业务 store/component、统一所有业务 DTO 或迁移已经类型化的 handwritten map。
- 为旧 command 名或旧参数形态提供兼容层。
