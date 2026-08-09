# 诊断：无关 pending recovery 阻断整批 Central 更新

## Symptom

2026-08-09 10:28，Update Center 对六个可更新技能执行 Apply。两次结果均为 0 项成功、6 项失败。界面中的每个 toast 都是 `This update item could not be applied.`，Operation Log 详情只包含聚合计数和通用错误码。

## Deterministic Feedback Command

以下命令以 SQLite `mode=ro` 读取最近的 apply 记录，并只检查 Runtime Log 的字段名：

```powershell
rtk python -X utf8 .\.trellis\tasks\08-09-central-update-apply-diagnostics\research\verify_live_update_apply_diagnostics.py
```

已运行结果：

```text
status=failed failures=6
failure_codes=[central_updates.update_failed]
failure_categories=[central_updates.item_failure]
failure_item_count=0
runtime_fields=[action,duration_ms,failure_count,status,success_count]
RED: failed update items do not retain bounded per-item and runtime diagnostic context
```

该命令快速、确定、可由 agent 重复运行，并直接捕获截图中的「6 项失败但没有必要诊断信息」。实施阶段仍需先添加隔离 Rust fixture；live probe 不能替代回归测试。

## Read-Only Live Evidence

- 最近 inventory 是 `skills + regular + bypass cache`。六个 `updatable` 条目均有 `repository_id`，本地 hash 等于 baseline hash，远端 hash 不同。
- 最新两条 `update_center.apply` 分别创建于 `2026-08-09T02:28:30Z` 和 `02:28:36Z`，均为 `status=failed`、`updates=6`、`failures=6`、`updated=0`。
- 六个选中技能没有在该时段创建 `central_update` journal。部分技能在 2026-08-04/05 的历史 completed journal 证明同一更新路径此前可用。
- 唯一非终态 row 是 `yao-meta` 的 `central_delete/prepared`，错误码为 `delete_restore_collision`。其 `updated_at` 在每次 apply 结束前约数十毫秒变化，说明 apply 正在重试这条无关 row。
- Runtime Log 同时记录启动时的 `delete_restore_collision` 和两次 6/6 apply failure，但 apply 事件只包含 counts。

未读取凭据、设置值、manifest 内容、fingerprint、完整路径或原始远端输出。未运行 refresh、apply、retry、reconcile、migration 或任何写入操作。

## Code-Level Causal Chain

1. `update_central_skills_impl` 在 snapshot 与 remote content 准备完成后，将六个计划交给 `update_skills_batch`（`src-tauri/src/services/central_updates/core.rs:294`）。
2. batch 获取 target mutation guard 后，在任何 plan 创建 journal 前调用 `recover_pending_update_operations`（`core/batch.rs:37` 和 `:59`）。
3. recovery 先调用 `recover_pending_delete_operations_with_transport`。该函数遍历 target 的全部 pending rows，没有 skill filter（`core/batch.rs:573`；`central_operation/recovery.rs:96`）。
4. `yao-meta` restore 再次产生 `delete_restore_collision`。batch 将错误转成字符串，并为所有六个 plan 构造 `CentralMutation` failure（`core/batch.rs:59-67`）。
5. 因为函数在 `prepare_update` 前返回，六个选中技能均没有新的 journal。这与 live DB 完全一致。
6. core 再次把 `CentralUpdatesError` 转成字符串，并构造只含 `skill_id + error` 的 `CentralSkillUpdateFailure`（`core.rs:341`）。
7. inventory 把每个 update failure 交给 `SkillUpdateApplyFailure::new`（`inventory/mod.rs:617`）。该构造器只按 step 生成 `central_updates.update_failed / item_failure`（`inventory/types.rs:281`）。
8. `error` 序列化器固定输出同一句 public message（`inventory/types.rs:305`）。
9. apply log 只聚合通用 code/category；runtime warning 只记录 counts（`commands/skill_update_inventory_apply_log.rs:14` 和 `:59`）。

## Ranked Hypotheses And Results

| Rank | Hypothesis | Prediction | Result |
| --- | --- | --- | --- |
| 1 | 无关 pending recovery 被错误扩大为 target 级阻断 | apply 只更新时间属于 `yao-meta` 的 row；六个选中技能没有新 journal | Confirmed |
| 2 | 一个共享 prepare/commit 失败被复制给整批 | 至少一个选中技能应创建 journal，或失败发生在 stage/commit 后 | Rejected |
| 3 | 六个技能分别存在 Central filesystem/DB merge 冲突 | 应出现六组独立 journal/error evidence 或不同阶段错误 | Rejected |
| 4 | repository ownership 修复仍在 apply 阶段失效 | inventory 应缺少 `repository_id` 或 source assignment | Rejected |
| 5 | 网络或 snapshot 获取失败 | outer command 应失败，或无法生成完整 updatable inventory；第二次 324 ms 也不会只触碰 recovery row | Rejected |

## Root Causes

1. **Recovery scope mismatch**：batch 需要 per-skill partial outcome，却复用了 startup/manual recovery 的全 target fail-fast 入口。
2. **Diagnostic structure loss**：错误在 batch 和 core 两个边界被转换为字符串，之后只能按 `step=update` 重建通用错误码。
3. **Log contract too shallow**：Operation Log 与 Runtime Log 只保存 aggregate code/count，无法关联技能、阶段与错误族。

## Safety Boundary

现场 `yao-meta` row 仍是独立的数据恢复问题。本任务只保证它不再阻断无关技能，并改善诊断；不会自动恢复、核销、删除或覆盖该 row 及其 filesystem evidence。
