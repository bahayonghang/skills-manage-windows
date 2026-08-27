# Skills CLI 远端传输接缝与 spec 修订

父任务：`08-27-skills-cli-remote-target`（源需求 U5）
序位：远端树第 1 个，其余三个的前置

## Goal

为 Skills CLI 建立单一的 Local/Remote 传输接缝、远端路径解析与远端 doctor，
并把 `.trellis/spec/backend/skills-cli-global.md` 的 Local-only 契约修订为 Local/Remote 能力矩阵。
本任务不交付任何面向用户的远端功能。

## Scope Rationale

远端化的四道闸门中，`ensure_local_target` 与 spec 契约是所有后续工作的共同前置。
把接缝、路径、doctor、spec 集中在一个不改变用户可见行为的子任务里，
可以让后续三个子任务在稳定地基上并行推进，也让「Local 零回归」这件事能被单独验收。

## Confirmed Facts

- `ensure_local_target()`（`skills_cli/mod.rs:247-252`）是后端唯一的 target 闸门。
- lock 路径规则（spec `skills-cli-global.md:74-76`）：
  `$XDG_STATE_HOME/skills/.skill-lock.json`，否则 `home / UNIVERSAL_AGENTS_DIR_NAME / .skill-lock.json`，
  **不得出现 `.agents` 字面量**。当前 `home` 来自本机 `resolve_home_dir()`。
- 远端 home 来自 `RemoteTargetConfig.remote_home`（`targets/model.rs:65-86`）；
  远端也可能自带 `XDG_STATE_HOME`，需在远端求值而非本机猜测。
- 现有两个可参照的接缝范式：`InstallTransport`（`services/installation/transport.rs:27-73`，
  spec `transport-seam.md`，单编排 + 每传输 hook）与 `Scope`/`FsBackend`
  （`services/usage/mod.rs:74-167`，trait + 两实现）。
- `TargetContext` 契约（spec `target-context.md`）：命令入口冻结 target + DbPool，
  禁止在 `.await` 中途重新解析。
- 每次 `ConnectedRemoteTarget::run_command` 都是一次新的 SSH 握手（`targets/exec.rs:201-259`），
  连接超时 10s。远端 doctor 应把 Node 探测合并为尽量少的往返。
- doctor 的本机语义由 `08-27-skills-cli-doctor-gate` 决定（该任务移除 `skills --help` 探测，
  只保留 Node 检测）。远端 doctor 必须与之对齐，不得独立发明第三套语义。

## Requirements

- R1：新增 Skills CLI 的 Local/Remote 单一接缝，与 `InstallTransport` / `Scope` 同构。
  业务逻辑通过接缝访问文件系统与命令执行，不得散落 `match ActiveTarget`。
- R2：路径解析（canonical root、lock path、平台 global skills dir）改为由接缝提供，
  远端从 `remote_home` 与远端求值的 `XDG_STATE_HOME` 推导。禁止远端流程调用本机 `resolve_home_dir()`。
  lock 路径规则中「不得出现 `.agents` 字面量」的约束在远端同样成立。
- R3：远端 doctor 探测远端 Node 存在性与 `>= 22.20` 版本，语义与本机 doctor 对齐，
  往返次数为常数且不随平台数变化。
- R4：`ensure_local_target()` 被能力矩阵查询取代。尚未远端化的能力返回稳定错误码且零写，
  不得静默降级或产生半完成状态。
- R5：命令入口先 `resolve_target_context()` 冻结 target + DbPool，再建立传输。
- R6：修订 `.trellis/spec/backend/skills-cli-global.md`：
  §1 的「MVP is Local target only」、§3 的 Local gate 契约、§4 错误矩阵
  `skills_cli.local_target_only` 行、§5 Base/Bad case、§6 中
  「Non-Local IPC reject」与「remote scan ignores local lock」等测试要求，
  全部替换为逐能力的 Local/Remote 矩阵。spec 与实现不得存在矛盾中间态。
- R7：Local 行为零回归。既有 Skills CLI 测试全绿，不允许删除断言。
- R8：远端 stdout / stderr / 路径不进入 `IpcError.message` 或未脱敏操作日志。
- R9：若新增或变更 Tauri 命令签名，运行 `pnpm docs:gen` 并提交生成文件，同步 `ipc_registry` 日志策略。

## Acceptance Criteria

- [ ] AC1 (R1,R5)：静态检查或测试证明 `services/skills_cli/` 业务逻辑中不存在
      对 `ActiveTarget` 变体的直接 `match`（接缝构造点除外）。
- [ ] AC2 (R2)：单元测试注入与本机不同的 `remote_home`，断言远端 lock 路径与 canonical root 随之改变；
      同一测试断言未回落到本机 home。
- [ ] AC3 (R2)：远端 lock 路径解析测试覆盖「远端有 `XDG_STATE_HOME`」与「远端无 `XDG_STATE_HOME`」两种分支，
      且结果字符串中不含 `.agents` 字面量硬编码。
- [ ] AC4 (R3)：fake runner 断言远端 doctor 的远端命令调用次数在
      「1 个平台」与「6 个平台」输入下相同；Node 缺失与版本过旧分别返回 `skills_cli.node_missing`。
- [ ] AC5 (R4)：能力矩阵中标记为「远端尚未支持」的每个能力都有测试断言返回稳定错误码且零写。
- [ ] AC6 (R6)：`skills-cli-global.md` 中不再出现 Local-only 断言，
      能力矩阵逐条覆盖 doctor / list / link / unlink / remove / install / update / export / leftover。
- [ ] AC7 (R7)：`cargo test` 中既有 Skills CLI 用例全绿，diff 中无被删除的断言。
- [ ] AC8 (R8)：植入 stderr 哨兵 token，断言其不出现在 IPC message 与操作日志。
- [ ] AC9 (R9)：`pnpm docs:gen:check` 通过。
- [ ] AC10 (Completion Gate)：`just ci` 通过。
      来源是 `AGENTS.md` 的 Completion Gate 一节，不隶属本任务的任一 R（TPR-09）。

## Out of Scope

- 任何面向用户的远端功能（列举、链接、卸载、安装、更新）。
- 前端解闸（`SkillsCliView.tsx:210-217`、`Sidebar.tsx:113-114`）——属 `remote-inventory`。
- 持久 SSH 会话池或 russh 迁移。

## Dependencies

- `08-27-skills-cli-doctor-gate` 必须先合入 `dev`：远端 doctor 需与定型后的本机 doctor 语义对齐。
