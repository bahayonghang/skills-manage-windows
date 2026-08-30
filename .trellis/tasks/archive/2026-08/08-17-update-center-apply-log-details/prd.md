# 更新中心 apply 失败日志与操作日志详情时间

## Goal

用户在操作日志详情里能读到更新中心 apply 单项失败的**已审查公开原因**（稳定 error code + 对应中英文文案），并且 **Created at** 按本机时区显示。不再只看到 `central_updates.import_addition_failed` 和 UTC ISO。

## User value

诊断「远端新增导入失败」时，详情页能直接告诉用户是访问被拒、限流、仓库不存在还是其它已分类原因；时间与本机时钟一致，不必手工换算 UTC。

## Background

2026-08-17 本地复现：`update_center.apply` 导入 `emilkowalski/skill` 的 `skills/animate` / `skills/ask-sonner` 失败。详情 JSON 只有：

- `errorCode`: `central_updates.import_addition_failed`
- `identifier`: `github:emilkowalski-skill-main`

`error_summary` 为空。Created at 为 `2026-08-17T11:49:15.213736600+00:00`（本地 19:49:15）。真实 `GithubImportError` 在 apply 循环里被丢掉。

## Confirmed facts

- Apply 导入失败走 `SkillUpdateApplyFailure::new("import_addition", repository.id)`，`Err(_error)` 丢弃域错误。锚点：`src-tauri/src/services/central_updates/inventory/mod.rs:597-600`。
- `new()` 把 errorCode 固定为 `central_updates.import_addition_failed`，error 固定为 `"This update item could not be applied."`。锚点：`inventory/types.rs:293-302`。
- IPC 序列化再把 `error` 压成同一句固定英文：`serialize_public_apply_error`（`inventory/types.rs:507-512`）。
- Operation Log `failureItems` 是 allowlist：只允许 `step`、安全 `identifier`、`phase`、`errorCode`、`errorCategory`。不得写入 item `error`、路径、URL、Display。锚点：`.trellis/spec/backend/redaction-policy.md` 第 9 条；实现 `skill_update_inventory_apply_log.rs:127-134`。
- `GithubImportError::ipc_error_code()` 已有 `github_import.access_denied` / `rate_limited` / `transport_failed` 等稳定码；`SelectionUnavailable`、`InvalidCandidate`、`RepoPathGone` 等 apply 路径变体目前返回 `None`。锚点：`github_import/error.rs:356-400`。
- 前端 toast 用 `formatBackendError({ code: failure.errorCode })`，但 code 是泛化的 `import_addition_failed`，i18n 也是同一句「无法应用」。锚点：`UpdateCenterDialog.tsx:75-87`；`src/i18n/locales/{en,zh}.json` `backendErrors.central_updates.import_addition_failed`。
- 详情抽屉直接渲染 `entry.createdAt` 原串。锚点：`OperationLogDetailDrawer.tsx:130`。
- 列表行已用 `Intl.DateTimeFormat` 转本地时间；tooltip 仍是 `toISOString()` UTC。锚点：`LogsListRow.tsx:28-43,124-128`。
- `update_operation_event` 不写 `error_summary`，所以失败行详情顶部没有 `errorSummary` 横幅。锚点：`skill_update_inventory.rs:451-461`。
- 用户可见文案必须走 `src/i18n/`。Operation Log 与 Runtime Log 不得记录 PAT、路径、URL、仓库 source path、命令输出。

## Requirements

- **R1** 导入新增失败时，`SkillUpdateApplyFailure.error_code` 必须来自 `GithubImportError` 的稳定公开码（已有 `ipc_error_code()`，或为本任务新增的 apply 相关审查码），不得一律写成 `central_updates.import_addition_failed`。
- **R2** `error_category` 必须来自 `GithubImportError::diagnostic_category()`（或等价静态族名），不得用 Display 派生。
- **R3** Operation Log `failureItems` 继续遵守 allowlist。`errorCode` 改为 R1 的稳定码。禁止把域错误 Display、路径、URL 写入 details JSON 或 `error_summary`。
- **R4** 当 apply 结果含失败项时，Operation Log 的 `error_summary` 写入该次**第一条失败**对应的已审查公开英文句（`public_message_for_code`），供详情横幅使用。
- **R5** 详情抽屉 Created at 按本机时区格式化（含日期与时分秒）。原始 ISO 仅作 `title` / 可复制辅助，不作为主显示。
- **R6** 详情抽屉对 `failureItems` / `failureCodes` 用 `formatBackendError` 渲染本地化公开文案，并显示安全 `identifier`。不得渲染 item `error` 原文。
- **R7** 更新中心 apply toast 使用同一套 `errorCode` → `formatBackendError` 文案，与详情一致。
- **R8** 中英文 `backendErrors.github_import.*` 覆盖本任务新增或首次对用户露出的码。
- **R9** 对抗种子（token、URL、绝对路径）不得出现在 Operation Log details、`error_summary`、IPC apply failure `error`、toast 测试断言里。

## Acceptance criteria

- [x] 用 `GithubImportError::AccessDenied`（或测试替身）走 import_addition 失败时，持久化 `failureItems[0].errorCode` 为 `github_import.access_denied`，`errorCategory` 为对应静态族；不是 `central_updates.import_addition_failed`。
- [x] 同一失败的 Operation Log `error_summary` 等于 `public_message_for_code("github_import.access_denied")` 的已审查句子。
- [x] 对抗种子（`token=secret`、`https://example.invalid`、`C:/Users/private`）不出现在 apply Operation Log details、`error_summary`、序列化后的 `SkillUpdateApplyFailure.error`。
- [x] `SelectionUnavailable` / `InvalidCandidate` / `RepoPathGone` 至少各有一个稳定 `github_import.*` 码，并有中英文 i18n。
- [x] 详情抽屉对 `2026-08-17T11:49:15.213736600+00:00` 显示本机本地时间，主文本不含 `+00:00`；`title` 仍含原始 ISO。
- [x] 详情抽屉对含 `failureItems` 的 apply 失败行显示本地化原因 + identifier；JSON 区块可保留，但用户不必只靠 JSON 才能读懂原因。
- [x] apply toast 对同一 `errorCode` 显示与详情相同的本地化句子。
- [x] 相关 Rust 单元测试与前端组件测试通过；提交前跑 `just ci`。

## Out of scope

- 修复 `emilkowalski/skill` 导入本身（网络、限流、tree 导入）。
- 常规检查 vs 增量和删减的扫描范围。
- 修改 Operation Log `failureItems` allowlist，把原始 `error` 写进 details。
- 改列表行相对时间算法。
- Runtime Log 写入域错误 Display 或 URL。
- 历史已落库的旧日志回填。

## Key decisions

| 决策 | 选择 | 依据 |
| --- | --- | --- |
| 「准确」的含义 | 稳定公开码 + 已审查文案，不是 Display | redaction-policy 第 9 条、async-error-feedback |
| 时间主显示 | 本机时区绝对时间（含秒） | 用户要求；列表已有本地格式可复用 |
| 原始 ISO | 仅 tooltip / title | 与 `LogsListRow` 一致 |
| toast | 与日志同一套 code 文案 | 避免详情与 toast 两套语义 |

## Risks

- 为 apply 路径补码会扩大 `ipc_error_code` / `public_message_for_code` / i18n 三表。三处必须一次对齐，否则又会出现「一边有码、一边 `internal.unexpected`」。
- `error_summary` 只能用已审查英文句；前端再用 i18n 按 `errorCode` 本地化详情主文案，避免把未翻译英文当唯一展示。
