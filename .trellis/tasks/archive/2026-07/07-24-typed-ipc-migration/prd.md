# Typed IPC 渐进迁移

## Goal

让高风险 Tauri command 的参数名、参数类型和成功返回类型以 Rust/Serde 为权威源生成
前端契约，并将所有 Tauri command 的失败边界统一为
`IpcError { code, message, retryable }`。CI 必须在参数 rename、返回 shape 漂移、
handler/caller 漏登记、错误载荷回退为字符串或 allowlist 回增时失败。

本子任务闭环审计 P2-05 / M-06 与 P3-03：完成结构化错误全边界迁移和第一批
Typed IPC；剩余 47 个 untyped command 留给后续批次。

## Background

- 2026-07-28 对 `src/lib/ipc/commandMap.ts` 的 TypeScript AST 复核结果为
  **88 typed / 89 untyped / 177 frontend contract commands**；旧审计中的 105 项和
  旧 PRD 中的 104 项基线均已失效。
- Rust 侧有 **184** 个已注册 `#[tauri::command]`：其中 **180** 个返回
  `Result<_, String>`，4 个不返回 `Result`。`tauri::generate_handler!` 与这 184 个
  annotation 完全一致。
- Rust 比前端多 7 个已注册、当前无 invoke 字面量调用的 handler：
  `detect_agents`、`get_active_target`、`get_central_skills_page`、
  `read_skill_content`、`suggest_skill_tags`、`sync_registry`、
  `sync_registry_with_options`。它们是显式 backend-only 集合，不应被误报为 parity
  缺陷。
- `src/test/contracts/ipcCommandCoverage.test.ts` 只校验前端 invoke 字面量属于
  typed map 或 allowlist、无 overlap/zombie，并未读取 Rust command 签名或运行时
  handler registry。
- `src/lib/ipc/commandMap.ts` 的 typed 项仍由 TypeScript 手写，因此 Rust 参数
  rename、camelCase、`Option`/null 和嵌套返回类型变化不会自动更新调用方。
- `.trellis/spec/frontend/ipc-adapter.md` 要求所有调用经过 `@/lib/ipc`，以保留
  browser fixture 与 failure recorder；标准 tauri-specta bindings 直接调用
  `@tauri-apps/api/core`，不能原样成为本仓库的运行时入口。
- `src/lib/ipc/invoke.ts` 当前原样透传 Tauri rejection。后端改为对象后，现有
  `String(err)` 调用会退化为 `[object Object]`，必须在唯一 adapter 内兼容归一化。
- 用户于 2026-07-28 选择方案 2：本任务同步引入结构化 `IpcError`，不继续保留
  raw string command error boundary。

## Requirements

1. 使用 feature-gated 的 Rust-derived metadata/codegen；相关 RC 依赖必须精确固定
   版本，正常桌面运行仍使用现有 Tauri runtime handler，不引入第二套 invoke 路径。
2. 生成物必须适配 `@/lib/ipc` 的 `{ command: { args, result } }` 契约，不得绕过
   browser fixtures、failure recorder 或 store-only invoke 分层。
3. 第一批从当前 allowlist 迁移 42 个 destructive、secret、import/install、Central
   store/update/sync command；完成后 typed 数量为 130，allowlist 从 89 收紧为 47。
4. CI 以临时生成 + byte-for-byte diff 验证 checked generated artifact；参数 rename、
   Serde shape 或返回类型变化但未提交生成物时必须失败。
5. contract 必须分别校验：184 个 runtime handler 的单一登记源、42 个 generated
   command 是 handler 子集且均有 frontend caller、177 个 frontend command 均已注册，
   以及 7 个 backend-only handler 的显式集合。不得用 184=177 的错误等式冒充 parity。
6. 生成器必须覆盖 `rename_all`、serialize/deserialize phase、optional/null、嵌套枚举、
   command 注入参数与 `Result` 的成功值/错误值分离；不得用字符串替换或 name-only
   source scan 冒充类型一致性。
7. 新增 Rust `IpcError { code, message, retryable }` 与 `IpcResult<T>`；180 个现有
   command 的 raw `Result<_, String>` 边界必须清零，4 个非 `Result` command 保持不变。
8. `code` 使用稳定、locale-neutral 的小写点分命名；`retryable` 默认 `false`，仅在
   mapper 能证明失败发生在 mutation 前且重试安全时显式设为 `true`。本任务不增加
   自动重试行为。
9. `message` 在不泄漏敏感或机器本地细节的前提下保留现有用户可见语义；凭据、
   绝对路径、命令文本、环境变量、捕获 stdout/stderr、snapshot token/digest 和文件
   内容不得进入 IPC error payload、failure recorder 或状态导出。
10. 前端 adapter 将结构化 payload 包装为 `IpcInvokeError extends Error`；
    `String(error)` 与 toast 继续得到 message，同时调用方可读取 `code` 和
    `retryable`。保留 strict coded-string 与 transport/legacy rejection 的防御性兼容，
    但新 Rust command 边界不得再发 raw string。
11. 所有按错误文案分支的逻辑改按 code：portability cancel、manifest JSON/kind/version、
    GitHub auth/rate-limit/configured-token guidance 与 SSH password unavailable。普通展示型
    `String(err)` 可继续使用兼容 wrapper 的 message 语义。
12. browser fixtures 的预期失败使用结构化 fixture helper；missing-fixture 等前端本地
    Error 保持可诊断。既有 coded error i18n（`ai.*`、`github_import.*`、
    `local_archive.*`）必须兼容新对象载荷。

## Acceptance Criteria

- [ ] `design.md` 记录 exact-version 选型、184/180/4/177/7 基线、错误契约、42 项首批
      命令集、生成数据流、兼容策略和回滚方案。
- [ ] `implement.md` 给出结构化错误、adapter/fixture/UI、registry/codegen、42 项调用方
      迁移和验证的有序批次。
- [ ] Rust annotation inventory 显示 180 个 `Result<_, String>` command 降为 0，
      180 个 command 返回 `IpcResult<_>`，4 个非 `Result` command 保持不变。
- [ ] adapter 对结构化对象、legacy coded string、plain string、JS `Error` 与未知 transport
      rejection 有测试；`String(IpcInvokeError)` 不产生 `[object Object]` 或错误名前缀。
- [ ] cancellation、manifest、GitHub auth/token 和 SSH password 特殊流程均按 stable code
      分支，不再 sniff message。
- [ ] seeded credential/path/command/output 内容不会出现在序列化 `IpcError`、前端展示、
      failure recorder 或相关状态中；安全的既有 message 语义保持。
- [ ] 第一批 42 个 command 从 allowlist 移除并使用 Rust-derived generated args/result；
      最终为 **130 typed / 47 untyped / 177 frontend commands**。
- [ ] contract fixture 人为修改一个 Rust 参数名或 Serde rename 后，codegen check 在运行
      应用前失败。
- [ ] runtime handler、generated set、frontend caller 与 7 项 backend-only allowance 的
      集合关系全部由测试证明，漏注册或意外多注册会失败。
- [ ] `pnpm typecheck`、`pnpm lint`、相关 Vitest、Rust fmt/clippy/test、codegen diff、
      `just ci` 全部通过。
- [ ] Windows `pnpm tauri build` 通过并生成实际安装产物，证明 codegen feature 与 RC
      依赖未污染正常 bundle。

## Out Of Scope

- 在本子任务一次性迁移剩余 47 项 Typed IPC；它们继续由 ratchet 保护并在后续批次清零。
- Typed Tauri events；本任务只处理 command args/result/error。
- 给 `IpcError` 增加 `details`、stack、raw source 或任意诊断字段。
- 自动重试、retry queue 或新的 UI retry 控件；`retryable` 只建立契约。
- 改变 payload 内部的逐项 `error: String` 字段；本任务只迁 whole-command rejection。
- 远端 API、数据库 schema、用户数据或 CLI JSON envelope 的兼容性迁移。
