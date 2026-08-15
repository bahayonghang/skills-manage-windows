# GitHub import preview fail and default branch

## Goal

用户预览导入一个没有 `SKILL.md` 的公开 GitHub 仓库时，向导必须直接说明“没有可导入 Skill”。该失败必须带稳定 IPC code，并出现在 Runtime Log。向导默认分支保持 `main`。

## Background

复现仓库 `https://github.com/wanshuiyin/HERO-Anti-OverDefense` 的 GitHub `default_branch` 是 `main`，树里没有 `SKILL.md`。预览返回 `GithubImportError::NoImportableSkills`（`preview.rs:382` / `preview.rs:448`）。该变体没有 `ipc_error_code`，`to_ipc_error()` 输出裸 Display。`IpcError::from_legacy_boundary` 将其收成 `internal.unexpected`，公开文案变成 `The operation failed. See runtime logs for details.`（`ipc_error.rs:4-5`）。

`preview_github_repo_import` 不走 `with_operation_log`，Operation Log 没有预览失败。前端 `recordIpcFailure` 只记录规范化后的 `IpcInvokeError`，`errorToDetails` 不写 `code` 字段。用户截图停在 Operation layer，日期筛在 2026-08-13 至 2026-08-14。

分支截图是旧 UI：placeholder `dev`（`eaf3035d`）。当前源码已在 `d676bff4` 改为 `main` / `dev` / Custom 单选，空值提交 `main`。

Q1 已确认：不把 `RULES.md` / `AGENTS.md` 仓库导入成 Skill。本任务只修失败展示与诊断。

## Requirements

- R1. 预览返回 `NoImportableSkills` 时，向导内联错误显示 reviewed 公开文案。不得显示 `See runtime logs for details`。
- R2. 该失败使用稳定 code `github_import.no_importable_skills`。`ipc_error_code`、`legacy_code_message`、前端 `canonicalMessage`、中英 `backendErrors.github_import` 必须同时认识该 code。
- R3. Runtime Log 的 `ipc.failure` 记录必须包含该稳定 code。
- R4. 向导默认分支选择是 `main`。未改动时 Preview 发送 `branch: "main"`。不得把 `dev` 当作空字段 placeholder。
- R5. 预览保持只读。本任务不把预览失败写入 Operation Log。
- R6. 不改 `NoImportableSkills` 的 `#[error]` Display。CLI `CliApiError::NotFound` 继续使用该 Display。

## Acceptance Criteria

- [ ] AC1. 对无 `SKILL.md` 的预览夹具点 Preview import 后，向导停在 Repo URL 步，内联错误是 “no importable skills” 语义。页面没有 `See runtime logs for details`。
- [ ] AC2. `IpcError::from(GithubImportError::NoImportableSkills.to_ipc_error())` 的 code 是 `github_import.no_importable_skills`，message 是 reviewed 静态句，不含路径、token 或动态细节。
- [ ] AC3. 向导经 `formatBackendError` 渲染该 code 的中英 i18n 文案。
- [ ] AC4. `recordIpcFailure("preview_github_repo_import", …, IpcInvokeError)` 写入的 Runtime Log details 含 `github_import.no_importable_skills`。
- [ ] AC5. 打开向导时分支控件选中 `main`。未改动时 Preview 发送 `branch: "main"`。
- [ ] AC6. 现有 snapshot / 分支冲突 / PAT / archive redirect 编码错误测试保持绿色。`just ci` 通过。

## Out of Scope

- 不把 `RULES.md`、`AGENTS.md`、`CLAUDE.md` 或普通 Markdown 仓库当成 Skill 导入。
- 不给其他未编码预览变体（`InvalidCandidate`、URL 族、`PreviewFileManifestIncomplete`）补码，除非实现时必须顺手改同一 match 臂。
- 不改 GitHub 鉴权、archive redirect、preview snapshot 生命周期。
- 不把预览失败记入 Operation Log。
- 不改 Observability Console 的默认日期范围或 layer 切换。
- 不自动探测远程 `default_branch` 并回填输入框。

## Key Decisions

- 导入非 Skill 仓库的预期是可读失败，不是扩大 Skill 模型。
- 公开文案走 reviewed 静态句 + i18n，不把长 Display 送过 IPC。
- Runtime Log 通过记录 `IpcInvokeError.code` 满足诊断，不新增 Operation Log 事件。
- 默认分支以当前 `d676bff4` 单选为准；无回归则不改分支 UI。
