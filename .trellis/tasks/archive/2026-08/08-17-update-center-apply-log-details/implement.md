# Implement: apply 失败公开码与操作日志详情时间

## Checklist

1. **补 GitHub import 公开码**
   - 在 `GithubImportError::ipc_error_code()` 为 SelectionUnavailable / InvalidCandidate / RepoPathGone / TargetDirExists / DuplicateSelection / RenameId* 增加 `github_import.*` 字面量。
   - `NoSelections` / `NoValidOperations` 复用 `github_import.no_importable_skills`。
   - 同步 `ipc_error::public_message_for_code`。
   - 同步 `en.json` / `zh.json` 的 `backendErrors.github_import.*`。
   - 扩展现有 github_import error 码表测试（三面同一码，无 Display 泄漏）。

2. **apply 失败映射**
   - 新增 `SkillUpdateApplyFailure::from_github_import(repository_id, error)`。
   - `inventory/mod.rs` import_addition 的 `Err(_error)` 改为该构造器。
   - 缺 URL 仍可用 `new("import_addition", id)`。
   - 单测：AccessDenied → `github_import.access_denied`；对抗 identifier 仍降级 `batch`；序列化 `error` 不含种子。

3. **Operation Log**
   - `apply_operation_spec` 在 `failure_count > 0` 时 `.error(public_message_for_code(first_code))`。
   - 更新 `apply_operation_status_reflects_item_outcomes` 的 import 新用例；保留 update 泛化用例。
   - 断言 details 不含 secret/URL/path。

4. **时间与详情 UI**
   - 将绝对时间 / ISO 辅助格式抽到 `logsUtils.ts`；`LogsListRow` 改用共享函数。
   - `OperationLogDetailDrawer` Created at 用本地绝对时间（含秒），`title` 为原始 ISO。
   - 解析 `failureItems` 渲染公开文案 + identifier。
   - `UpdateCenterDialog.formatApplyFailure` 继续 `formatBackendError(errorCode)`（映射修好后自动变准）。
   - 测试：固定 `2026-08-17T11:49:15.213736600+00:00` 主文本不含 `+00:00`；`title` 含原始串。
   - 测试：`errorCode=github_import.access_denied` 时中文或英文公开句可见，identifier 可见。

5. **质量门**
   - `pnpm test -- OperationLogDetailDrawer logsUtils UpdateCenterDialog`
   - `cd src-tauri && cargo test skill_update_inventory_apply_log inventory::types github_import::error`
   - `just ci`

## Validation commands

```bash
pnpm test -- src/test/components/logs src/test/pages/OperationLogsView.test.tsx
cd src-tauri && cargo test skill_update_inventory_apply_log apply_failure -- --test-threads=1
just ci
```

## Risky files

- `src-tauri/src/services/github_import/error.rs` — 码表是 IPC / 日志 / i18n 单点
- `src-tauri/src/ipc_error.rs` — 公开英文句
- `src-tauri/src/commands/skill_update_inventory_apply_log.rs` — allowlist 测试很脆
- `src/i18n/locales/en.json` / `zh.json` — key 必须是 `github_import.<suffix>` 分层

## Rollback

`git restore` 上述文件。无需 migration。

## Before `task.py start`

- [x] prd.md 已收敛
- [x] design.md / implement.md 已写
- [x] implement.jsonl / check.jsonl 已填真实 spec
- [ ] 用户批准本规划摘要
