# 日志覆盖契约、文档与集成验收

状态：**planning**。依赖：其余六个 observability 子任务全部完成并通过各自检查。

## Goal

以 runtime IPC registry 为权威入口，收口每条命令的日志策略、隐私边界、跨层关联、文档和验证证据，确保
“左右操作都有迹可查，有错可追”成为可执行契约，而不是依赖人工记忆的约定。

## Requirements

- R1：逐条比较当前 IPC registry 与日志策略元数据；每条命令恰好归入 `operation`、`runtime-only` 或
  `excluded-success-read`，无未分类、重复注册或手工维护的总数快照。
- R2：汇总三个 domain coverage 子任务的覆盖矩阵。每个 `operation` policy 必须有稳定 category/action、
  lifecycle、目标语义、成功/失败/中断测试与唯一 owner；每个排除项必须有机器可检查的理由。
- R3：静态和运行时隐私检查阻止 raw `Display` error、动态 tracing 字符串、凭据、私钥、token、URL、用户路径、
  命令参数和未审查 source chain 进入 Operation/Runtime 磁盘、读取结果、导出或 DOM fixture。
- R4：检查嵌套调用、兼容 adapter 和批处理，确保一个逻辑操作不被多层重复写入；仅在所有调用点迁移后删除
  临时兼容 adapter，且不改变原有业务结果、锁、事务或恢复语义。
- R5：将稳定契约写入 `.trellis/spec/`，架构页面只解释边界、入口和排障流程；动态覆盖事实由 registry/tests/
  generated docs 产生。Tauri command 或 schema 有实际变化时运行 `pnpm docs:gen` 并提交生成结果。
- R6：运行聚焦测试、`just ci`、`pnpm docs:build`，并在 Windows 原生 Tauri 中验证居中详情窗、关联跳转、
  retention/clear、失败与受控异常终止；分别记录 PASS、FAIL、SKIPPED 和 UNVERIFIED。
- R7：只处理 observability 集成缝和由本任务引入的缺陷，保留无关工作区改动；不 push、不发布、不清理用户数据。

## Acceptance Criteria

- [ ] 当前 registry 的每条命令恰好有一个可验证日志策略，新增命令缺少策略时契约测试失败。
- [ ] 所有 Operation policy 都有唯一 owner、稳定 action/category、明确 lifecycle 和成功/失败覆盖。
- [ ] 所有 fallible backend rejection 都留下安全 Runtime evidence；同一失败可用一个 correlation ID 串联
  Operation、backend Runtime、frontend Runtime 和界面提示。
- [ ] 嵌套/批处理不会生成不可解释的重复行，started 行在终止或重启后不会永久悬空。
- [ ] 对抗种子中的 secret、path、URL、raw error、stack 和 args 不出现在新日志、查询、导出和 DOM fixture。
- [ ] stable diagnostic 使提示说明发生了什么、目标/阶段在哪里、下一步做什么，同时保留可复制 correlation ID。
- [ ] spec、架构文档、生成文档和代码一致，未写死易漂移的命令数量或运行状态。
- [ ] 聚焦检查与 `just ci` 通过；Windows 原生验收有明确证据，无法执行的项目保持 UNVERIFIED。

## Out of Scope

- 新业务功能、远程 telemetry、完整 console/stdout 代理或改变现有 14 天 Runtime retention。
- 修改 provider、SSH、GitHub 或远端环境以制造验证条件。
- push、PR、发布、清空现有日志，或用自动化结果代替尚未完成的 Windows 原生视觉证据。
