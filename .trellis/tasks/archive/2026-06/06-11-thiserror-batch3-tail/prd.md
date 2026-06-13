# PRD：thiserror 改造批次 3——尾批五域 + 全局收尾（子任务 C3）

> 父任务：`06-11-analysis-driven-fixes` ｜ 执行顺序：第 5 位
> 依赖：必须在 C2 完成并归档后开始。
> 来源：分析报告条目 #2

## Goal

完成剩余五个域的改造，统一 db/repos 层错误透传，并做全局扫尾验证：`Result<_, String>` 仅存在于 IPC 边界（commands 壳层签名）。

## Requirements

1. 改造尾批 5 个域：
   - `services/usage`（含 providers/*）
   - `services/obsidian`
   - `services/ai_provider`
   - `services/ai_tagging`
   - `services/portable_state`
2. **db/repos 层透传规范**：`db/repos/*` 当前 `pool.begin().await.map_err(|e| e.to_string())` 等字符串化处理，统一改为返回 `sqlx::Error`（或父任务 design.md 定义的薄包装 `DbError`），由各域错误枚举经 `#[from]` 接入。
3. 同步处理非域归属的散点：`central_migration.rs`、`operation_log.rs`、`logging.rs`、`secrets/`、`targets/`、`commands/bootstrap.rs` 等处的内部字符串错误，按就近原则归入相应错误类型或保留在边界。
4. 对应 commands 壳层完成边界转换。
5. **全局扫尾验证**：除 `commands/` 下 `#[tauri::command]` 函数签名（IPC 边界）外，`src-tauri/src` 内不再有 `Result<_, String>`。

## Acceptance Criteria

- [ ] 扫尾 grep 验证：`services/`、`db/` 目录（排除 tests）`Result<.*, String>` 命中为 0；全仓命中仅存于 `commands/` 边界签名。
- [ ] 全部 709+ Rust 测试通过（允许调整断言，不允许删用例）。
- [ ] `cargo clippy -- -D warnings` 零警告；`just ci` 全绿。
- [ ] 父任务级验收复核：分析报告条目 #2 关闭。
