# 技术设计：Central 删除强制放弃陈旧 prepared journal

## 边界

- 后端：`central_operation` 增加 force-abandon 预览/执行；`central_skills` 删除编排读取 `force` 后先放弃再走既有 journaled delete。
- 前端：删除对话框 / 批量 / 仓库删除共用预览字段与 `force` 确认。组件不直接 `invoke`。
- 不改 `restore_delete_local_blocking` / 远端 restore 脚本。Retry 与普通恢复保持 fail-closed。
- 不改 Reconcile 的指纹 / owned-path 规则。

## 决策

| 决策 | 选择 | 原因 |
| --- | --- | --- |
| 逃脱口 | 删除对话框强制删除 | 用户确认要删当前 Central 副本 |
| 指纹漂移 | 强制删除忽略 fingerprint | 2026-08-17 已确认；Reconcile 仍校验 |
| restore `(false, false)` | 不放宽 | 避免把证据丢失静默收成 already-gone |
| IPC | 现有删除命令加 `force`，不新增命令 | 同一把 mutation lock 内完成放弃 + 新删除 |
| 单删预览 | 改为 `preview_delete_central_skills` | `get_skill_detail` 不含 journal；批量已走该命令 |

## 数据契约

`DeleteCentralSkillPreview` 增加可选字段（camelCase 序列化与现有删除类型一致；现有字段保持 snake_case 以免破坏批量对话框）：

```rust
pub struct PendingDeleteRecoveryPreview {
    pub operation_id: String,
    pub operation_kind: String,
    pub phase: String,
    pub error_code: Option<String>,
    pub force_delete_eligible: bool,
    pub blocker_codes: Vec<String>,
}
```

`blocker_codes` 只允许稳定 recovery code：

- `recovery.reconcile_unsupported_kind`
- `recovery.reconcile_unsupported_phase`
- `recovery.reconcile_invalid_manifest`
- `recovery.reconcile_inconsistent_duplicate`
- `recovery.reconcile_target_mismatch`
- `recovery.reconcile_artifact_remaining`

不得加入 `recovery.reconcile_fingerprint_drift` 或 `recovery.reconcile_owned_path_missing`。这两项不阻止强制删除。

`BatchDeleteCentralSkillRequest` 与单删参数增加 `force: bool`，缺省 `false`。

禁止在预览、Operation Log、toast 中写出 original / backup / marker / fingerprint / `manifest_json`。

## 强制删除资格

在目标 mutation lock 下检查该技能当前非终态行：

1. 无非终态行 → 预览 `pending_recovery = None`，`force=true` 时按普通删除执行。
2. `operation_kind != central_delete` 或 `phase != prepared` → 不可强制删除。
3. manifest 无效、目标不一致、重复路径证据不一致 → 不可强制删除。
4. 任一去重后路径存在 backup 或 marker → 不可强制删除。
5. 其余情况（含平台 already gone、Central 指纹漂移、Central 仍在）→ `force_delete_eligible = true`。

与 Reconcile 的差别：Reconcile 仍检查指纹与 owned missing；强制删除只拒绝“还拿得住的恢复证据”。

## 编排

`delete_central_skills_under_guard`（Local / SSH / WSL 共用）：

```
acquire target mutation lock
if request.force:
    inspect force-abandon
    if pending prepared delete and not eligible -> typed error, stop that skill
    if eligible -> prepared -> rolled_back   // journal only
recover selected pending rows               // 放弃后该技能应无 pending
reload current owned paths
insert new prepared delete + stage + db_committed + finalize
```

放弃与新删除必须在同一把 `acquire_target_mutation_guard` 内。放弃不得改文件系统。新删除只包含**当前** Central 目录与当前勾选/自动清理的安装路径，不得复用旧 manifest 的平台路径。

单删 `delete_central_skill` 继续走同一批编排。仓库删除只是批量删除的调用方，传同一 `force`。

## 错误映射

`delete_central_skill` 今天 `.map_err(|e| e.to_string())`。`Central operation recovery collision (delete_restore_collision)` 不是 `family.code:` 前缀，IPC 收成 `internal.unexpected`。

改为按 `CentralSkillsError::stable_delete_error_code()` 构造 `IpcError`：

| code | 公开 message |
| --- | --- |
| `central_operation.delete_restore_collision` | 与现有 i18n 一致：恢复证据冲突 |
| `central_skills.force_delete_blocked` | 强制删除当前不可用 |
| 已有 `central_skills.mutation_lock_failed` 等 | 保持 |

`ipc_error.rs` 的 `legacy_code_message` 与前端 `backendErrors.*` 必须登记上述 code。批量失败项继续用 `FailedCentralSkillDelete.error_code`；渲染走 `formatBackendError`。

## 前端

- `loadDeletePreview` 改为 `preview_delete_central_skills([skillId])`，单删对话框改吃 `DeleteCentralSkillPreview`。
- 有 `pendingRecovery` 时展示恢复说明；`forceDeleteEligible` 时 Footer 增加「强制删除」destructive 按钮。普通「删除」仍发 `force: false`（会再次碰撞并显示同一文案）。
- 强制删除发 `force: true`，并再确认一次（对话框内文案，不新开窗）。
- `centralSkillsDeleteWorkflow` / store 用 `formatBackendError`，停止 `String(err)`。
- 批量 / 仓库对话框对每个 eligible 技能在对应 request 上设 `force: true`；不可用的技能保持 `force: false` 并显示 blocker 文案。
- i18n：`central.forceDeleteSkill`、`central.forceDeleteHint`、`central.forceDeleteBlocked`；恢复文案复用 `backendErrors.central_operation.delete_restore_collision`。

## 兼容

- `force` 缺省 false，旧调用方行为不变。
- 预览新字段缺省空 / false，旧前端忽略即可。
- 改命令签名后跑 `pnpm docs:gen`。
- 无 schema 迁移。

## 回滚

功能开关不需要。回滚即还原命令参数与对话框按钮；已 `rolled_back` 的 journal 保持终态，不会自动复活。
