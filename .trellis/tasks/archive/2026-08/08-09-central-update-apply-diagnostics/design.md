# 设计：按技能隔离 Central 恢复并保留单项诊断

## 1. Problem Boundary

当前 apply 的错误发生在以下边界：

```text
Update Center decisions
  -> update_central_skills_impl
  -> target mutation guard
  -> recover every pending row for target
  -> clone one string error to every selected skill
  -> generic apply failure DTO
  -> aggregate-only Operation/Runtime Logs
```

目标流程为：

```text
selected skill ids
  -> target mutation guard
  -> load pending rows once
  -> recover only matching skill rows
  -> blocked result per matching skill
  -> continue unrelated update plans
  -> preserve typed phase/code/category
  -> bounded item diagnostics + aggregate runtime fields
```

target guard 仍然串行化同一 Local/SSH/WSL target 的 Central 写入。变化仅限 recovery 的业务作用域和错误数据模型。

## 2. Recovery Scope

### 2.1 Shared row recovery

将 delete/update recovery 中「列出全部行」「恢复一行」「失败即返回」拆开：

- 全 target 入口继续遍历所有 pending 行并保持 fail-fast，用于 startup recovery 和显式 Retry。
- update batch 新增 selected-skill 入口。该入口一次读取 pending rows，先按 `skill_id` 筛选，再校验和恢复匹配行。
- 每个匹配行返回独立 outcome。row-specific failure 只占用对应技能的结果槽位；后续无关 row 和 plan 继续执行。
- 查询 pending rows、创建 transport 或获取 target guard 等 global failure 无法归属单项时，映射到所有尚未执行的计划。

非终态唯一索引保证同一 `(target_id, skill_id)` 最多一行。实现仍应防御重复输入 skill ID，并保持首次请求顺序。

### 2.2 Batch integration

`update_skills_batch` 在持有 target guard 后执行以下顺序：

1. 为每个请求 plan 建立稳定结果槽位。
2. 读取并恢复 selected skill 对应的 pending rows。
3. 将恢复失败写入对应结果槽位。
4. 只为没有失败的 plan 构建新 manifest 和 `prepared` journal。
5. 沿用现有 stage、swap、DB transaction、copy refresh 和 finalize 流程。
6. 按原请求顺序组装 outcomes。

无关 pending row 不调用 recovery helper，因此其 `phase`、`updated_at`、`last_error_code` 和 filesystem evidence 均不变化。

## 3. Typed Failure Contract

### 3.1 Internal outcome

batch 内部错误增加稳定阶段，不将 `CentralUpdatesError` 提前转换为字符串：

```rust
struct CentralUpdateItemError {
    phase: CentralUpdateFailurePhase,
    error: CentralUpdatesError,
}
```

`CentralUpdateFailurePhase` 是受控枚举，并序列化为固定 snake_case 字符串。建议值：

```text
mutation_lock
recovery
prepare
stage
database_commit
copy_refresh
result_finalization
decision_apply
```

阶段归属在错误发生处确定。调用方不得根据错误文本猜测阶段。

### 3.2 Public fields

`CentralSkillUpdateFailure` 和 `SkillUpdateApplyFailure` 增加或保留以下安全字段：

```text
step
identifier
phase
errorCode
errorCategory
error
```

映射规则：

| Source | `errorCode` | `errorCategory` |
| --- | --- | --- |
| `CentralOperationError` | `central_operation.<code()>` | `central_updates.central_operation` |
| reviewed `CentralUpdatesError` | 现有 reviewed code | `diagnostic_category()` |
| unreviewed update error | `central_updates.update_failed` | `diagnostic_category()` |
| 非 update decision step | 现有 step-specific code | `central_updates.item_failure` 或更具体静态分类 |

`error` 只序列化固定 public message。原始 `Display`、数据库错误、远端输出、URL 和路径不得进入公共结构。需要更新 `SkillUpdateState.error` 时，使用 reviewed public message，不保存原始错误作为替代诊断渠道。

`identifier` 由 step-specific constructor 生成，不复用现有动态字符串：

| Step | Safe identifier |
| --- | --- |
| update / keep / delete | 单个 skill ID；无法拆分的 outer batch failure 使用 `batch` |
| import / skip / unskip addition | repository ID |
| platform duplicate cleanup | `agent_id::skill_id` |

完整 filesystem path、repository source path 和逗号拼接的 ID 列表不得成为 public identifier。现有 call site 中包含 path 的 identifier 必须改为安全逻辑标识。

## 4. Logging Contract

### 4.1 Operation Log

`apply_result_details` 保留现有 counts、`failureCodes` 和 `failureCategories`，并增加：

```json
{
  "failureItems": [
    {
      "step": "update",
      "identifier": "skill-a",
      "phase": "recovery",
      "errorCode": "central_operation.delete_restore_collision",
      "errorCategory": "central_updates.central_operation"
    }
  ],
  "failureItemsTruncated": 0
}
```

数组最多保存 50 项，保持结果顺序。`failureItemsTruncated` 是未保存的数量。Operation Log 的既有递归 redaction 继续作为 defense in depth，但调用方必须先构造只含安全字段的 payload。

### 4.2 Runtime Log

partial/failed 事件记录：

- `failure_codes`：排序去重的稳定码。
- `failure_categories`：排序去重的稳定分类。
- `phase_counts`：固定阶段到数量的映射。
- 现有 `success_count`、`failure_count`、`duration_ms`。

Runtime Log 不记录 identifier 列表和原始错误。Operation Log 负责单项关联，Runtime Log 负责低体积的运行时分类。

## 5. Frontend Contract

TypeScript failure type 增加 `phase`。Update Center 使用 `errorCode` 调用 `formatBackendError`，并将安全 `identifier` 与本地化 public message 组合为 toast。已知 recovery collision 文案引导到 Operation Logs；未知码回退到固定通用文案。

Operation Log 详情抽屉已能格式化并复制 `detailsJson`。本任务不增加 update-specific drawer 组件，以免同一日志 payload 出现两套展示逻辑。

## 6. Compatibility And Security

- 不修改 SQLite schema；旧 Operation Log 保持原样。
- 新 IPC 字段使用 Serde default/optional 兼容旧 fixture。生成 TypeScript map 与架构文档随类型变化更新。
- 不改变 startup recovery、显式 Retry/Reconcile 或 target mutation lock 的入口语义。
- 不对现场 `yao-meta` 执行恢复或核销。实现验证只使用内存/临时目录、FakeRunner 和只读 live probe。

## 7. Rollback

恢复筛选、typed failure、Operation Log payload、Runtime Log 字段和前端类型必须作为一个兼容单元提交。若回滚诊断字段，恢复隔离仍可独立保留；若回滚恢复隔离，必须同时移除对应 scoped-recovery helper，避免留下未使用的第二套恢复语义。

## 8. Delete Recovery Boundary

shared delete 的 top-level single/batch 入口负责 target guard、pending inventory 和可选 remote transport；具体单项删除拆成 under-guard helper：

```text
deduplicate delete requests
  -> acquire target guard once
  -> list pending rows once
  -> select rows by requested skill id
  -> recover selected row and retain typed per-skill failure
  -> delete remaining eligible skills under the same guard
  -> preserve first-request ordering
```

删除 recovery 复用 `central_operation` 的单行 typed helper，不复制 manifest/marker/fingerprint 校验。单项公开入口等价于一项 batch。startup、显式 Retry/Reconcile 继续调用 full-target wrapper。`FailedCentralSkillDelete` 增加受控 phase/code/category 字段，preview 构造使用静态 prepare fallback；实际删除从 `CentralSkillsError` 直接映射，inventory adapter 不再丢弃这些字段。

## 9. Snapshot Retry And Error Taxonomy

`GithubImportError` 将当前过宽的 `Http(String)` 拆出不会暴露动态内容的 typed acquisition families，保留既有 public code/message compatibility，同时提供静态 `diagnostic_category()` 与 `is_snapshot_retryable()`：

```text
initial batch (concurrency 4)
  -> successes enter cache/result
  -> failures classified without Display parsing
  -> retryable failures queued in stable repository order
  -> one serial retry after every initial future settled
  -> recovered snapshots enter cache/result
  -> final failures returned once
```

自动重试只发生在只读 snapshot 获取阶段，因此没有 mutation/rollback 风险。重试包含 transient transport/request/timeout/body 和 retryable server-status exhaustion；policy、authentication、not-found、invalid input、parse/integrity 与 budget fail closed。进度 reporter 在最终 outcome 时只调用一次 completed/failed，内部 attempt 可用独立 retry aggregate，不复用完成计数。

为避免测试依赖真实网络，snapshot acquisition 内核接受私有 downloader seam 和 concurrency 参数；生产 wrapper 固定真实 downloader、并发 4、补偿并发 1、最多一次。测试用闭包记录首轮/补偿并发峰值和每仓库调用次数。

## 10. Refresh Diagnostics

`SnapshotAcquisition` 增加 retry summary 和每个最终 failure 的静态 category。inventory 的 `FailedRepository` 用 optional/default 字段向后兼容持久化。refresh/retry Operation Log 只保存最多 50 个 `{repositoryId,errorCode,errorCategory}` 以及 truncated、attempted、recovered；Runtime Log 保存排序去重 code/category 与 retry counts。两层均从 typed error 映射构造，不记录动态 transport detail。

## 11. Updated Rollback

delete scoped recovery 与 typed delete DTO 一起回滚。snapshot typed taxonomy、retry policy、inventory optional diagnostics 和日志聚合一起回滚；cache schema、SQLite migration 和 redirect policy不受影响。
