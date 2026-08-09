# Best-Effort 写入约定

## 规则

非关键路径的数据库/状态写入（失败不应中断主流程的写入，如扫描状态标记、时间戳记录）：

- **禁止**裸 `let _ = db::xxx(...)` 丢弃错误——失败完全不可见，排障无从下手。
- **必须**走带 tracing 日志的 `*_best_effort` 辅助函数，失败时 `tracing::warn!` 记录 key 与错误。
- **恢复协议例外**：operation journal insert/transition/error marker、manifest 更新、rollback、restore 与 finalize 都是正确性路径，禁止 best effort，也禁止用原始 `%error` tracing 代替传播。任何失败必须保留非终态证据并返回调用方。
- **Marketplace sync 例外边界**：fresh cache replacement 与 success metadata 是 authoritative transaction，绝不 best effort。该 transaction 已失败并回滚后，`last_sync_status=error` 才是允许 best effort 的派生诊断 marker；helper 必须具名并在 marker 写失败时 `warn!`。

## 现有实现

- `db/repos/settings_repo.rs` — `set_setting_best_effort`（2026-06-11 引入，任务 06-11-eng-hygiene-quickfixes）
- `operation_log.rs` — `record_operation_log_best_effort`（先例，命名惯例来源）

## 新增 best-effort 场景时

复用上述函数；若是新的写入类型，按同一命名模式（`<动作>_best_effort`）在对应 repo 模块内新增，保持 `tracing::warn!` 字段风格一致（`key`/`error = %error`）。
