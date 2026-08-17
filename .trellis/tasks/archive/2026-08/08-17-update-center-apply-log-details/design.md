# Design: apply 失败公开码与操作日志详情时间

## Boundaries

| 层 | 改什么 | 不改什么 |
| --- | --- | --- |
| `github_import::GithubImportError` | 为 apply 会碰到、但 `ipc_error_code()` 仍为 `None` 的变体补稳定码 | Display 文案、预览生命周期语义 |
| `central_updates` inventory apply | `from_github_import` 构造失败项；循环不再 `Err(_error)` | import 业务本身、force update / mirror |
| `skill_update_inventory_apply_log` | `errorCode` 用域码；失败时写 `error_summary` | `failureItems` 字段集合 |
| 操作日志详情 / 列表时间工具 | 本地时间主显示；失败项公开文案 | 列表相对时间、Runtime 面板 |
| i18n | `backendErrors.github_import.*` 新码；toast 沿用 `formatBackendError` | 其它 central_updates 泛化句（其它步骤仍可泛化） |

## Data flow

```
GithubImportError
  → ipc_error_code() / 新增审查码
  → SkillUpdateApplyFailure { error_code, error_category, identifier, step, phase }
  → apply IPC result（error 仍序列化为固定公开句）
  → apply_result_details.failureItems（allowlist + 新 errorCode）
  → OperationLogEvent.error_summary = public_message_for_code(first_code)
  → 详情抽屉：formatBackendError(errorCode) + identifier
  → toast：同一 formatBackendError(errorCode)
```

禁止的箭头：`error.to_string()` / Display / source_path → Operation Log、error_summary、toast。

## Contracts

### 1. `SkillUpdateApplyFailure::from_github_import`

```text
step        = "import_addition"
identifier  = safe_logical_identifier(repository_id)
phase       = "decision_apply"
error_code  = error.ipc_error_code()
              ?? 本任务新增审查码
              ?? "central_updates.import_addition_failed"
error_category = error.diagnostic_category()
error          = 内存可留 Display 仅供调试；Serialize 仍走 serialize_public_apply_error
```

`from_github_import` 是 import_addition 的唯一失败构造入口。`new("import_addition", …)` 只留给没有域错误的情况（例如缺仓库 URL）。

### 2. 需要补进 `ipc_error_code()` 的 apply 变体

现有码已覆盖 AccessDenied / RateLimited / Http / RepoNotFound / Budget / NoImportableSkills 等。

本任务补码（全部 `github_import.*` 字面量）：

| 变体 | 建议码 | 公开句方向 |
| --- | --- | --- |
| `SelectionUnavailable` | `github_import.selection_unavailable` | 选中的技能已不在仓库预览中 |
| `InvalidCandidate` | `github_import.invalid_candidate` | 仓库中有无法导入的技能清单 |
| `RepoPathGone` | `github_import.source_path_missing` | 选中路径不再包含可导入技能 |
| `TargetDirExists` | `github_import.target_exists` | 中央库目标目录已存在且不能覆盖 |
| `DuplicateSelection` | `github_import.duplicate_selection` | 同一次导入重复选择了同一路径 |
| `RenameIdInUse` / `RenameIdRequired` | `github_import.rename_conflict` | 重命名后的 id 不可用 |
| `NoSelections` / `NoValidOperations` | `github_import.no_importable_skills`（复用已有码） | 没有可执行的导入项 |

三表同步：`ipc_error_code`、`ipc_error::public_message_for_code`、`src/i18n/locales/{en,zh}.json` 的 `backendErrors.github_import.<suffix>`。

`formatBackendError` 的 key 是 `backendErrors.${code}`，而 i18n 树是 `backendErrors.github_import.access_denied`。现有 github 码已按点分层，`t("backendErrors.github_import.access_denied")` 在 i18next 里对应 code `github_import.access_denied`。新码必须同样分层，不能写成扁平 `backendErrors.github_import_selection_unavailable`。

### 3. Operation Log

`apply_operation_spec` 成功回调在 `failure_count > 0` 时：

```text
event = update_operation_event(...)
event = event.error(public_message_for_code(first_failure.error_code))
```

`first_failure.error_code` 必须已是稳定码。`public_message_for_code` 缺失时回退到已有 `"This update item could not be applied."`，禁止用 Display。

`apply_failure_diagnostics` 的 JSON 形状不变，只改 `errorCode` / `errorCategory` 取值。测试 `apply_operation_status_reflects_item_outcomes` 对 `new("update")` 的期望保持；新增 import 用例断言 github 码。

Runtime warn 已记录 `failure_codes` 数组，映射修复后会自动带上 `github_import.access_denied`，无需再写 Display。

### 4. 详情时间

把 `LogsListRow` 的绝对时间格式抽到 `logsUtils.ts`：

- `formatLogAbsoluteTime(iso, { withSeconds?: boolean })` — `Intl.DateTimeFormat(undefined, { year, month, day, hour, minute, second? })`
- `formatLogIsoTime(iso)` — 解析失败则原样返回；成功则返回**原始输入**（保留 `+00:00` 与亚秒），不要 `toISOString()` 改写

详情 Created at：

- 主文本：`formatLogAbsoluteTime(createdAt, { withSeconds: true })`
- `title`：原始 `entry.createdAt`

无效日期：主文本回退原始字符串。

### 5. 详情失败文案

在 JSON 区块之前增加只读列表：对 `details.failureItems[]` 每项显示

```text
formatBackendError({ code: item.errorCode, message: fallback, retryable: false }, t)
+ identifier
```

`failureItems` 缺失时，对 `failureCodes[]` 做同样渲染。解析失败则不渲染该块，只留 JSON。

对抗测试：detailsJson 即使被投毒包含 URL/token，可见文本断言不得出现这些种子（JSON `<pre>` 若原样展示投毒数据，测试应使用干净 fixture；投毒用例只覆盖 backend 落库，不要求抽屉二次脱敏 JSON）。

## Compatibility

- 旧日志仍可能是 `central_updates.import_addition_failed`。详情对未知码走 `formatBackendError` 回退到 payload message / 该泛化 i18n 句。
- `SkillUpdateApplyFailure` 的 serde 字段不增。
- 不改 IPC command 签名。

## Tradeoffs

- 不把 Display 写入日志：用户看不到「哪一个 SKILL.md 非法」的路径。这是 redaction 约束。公开码足以区分限流 / 拒绝 / 选区失效。
- `error_summary` 用英文公开句：与现有 Operation Log 语言一致；详情主文案再 i18n。

## Rollback

回退本任务 diff 即可。已写入的新日志行保留新 errorCode，旧客户端仍能展示 JSON。
