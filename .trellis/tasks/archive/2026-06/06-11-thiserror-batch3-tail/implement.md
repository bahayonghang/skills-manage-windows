# Implement：thiserror 批次 3（子任务 C3）

前置：C2（06-11-thiserror-batch2-mid）已归档。

## 执行清单

1. [x] db/repos 透传统一（18 个 repo + seed/migrations/pool + 10 schema）：`Result<_, sqlx::Error>` → 验证：`cargo test db::` 73 passed（commit 7cd0fe8）。
2. [x] 清除 C1/C2 全部 `// TODO(C3)` 适配点（130/130，grep 0 残留；六域 Other(String) 兜底变体删除）→ 验证：全量测试绿（同 commit 7cd0fe8，与步骤 1 同一编译单元）。
3. [x] 尾批五域逐域改造（usage 28 passed → obsidian → ai_provider 33 passed → ai_tagging 8 passed → portable_state 14 passed，每域一个 commit：ac0d618/4bdcf09/997a494/5353479/d62b8a5）。
4. [x] 非域归属散点处理：targets 单独成批（TargetsError 41 变体，commit ea6631f）；logging/resource_budget/paths/central_migration/fs_util（commit 83a802d）；secrets 已全程 SecretError 无需改动；operation_log 经 repos 透传覆盖；bootstrap/settings 属 commands 边界保留 → 验证：`cargo test` 全量 704 passed。
5. [x] 全局扫尾 grep：仅剩 commands/ 边界签名 + lib.rs active_db/active_target 文档化双助手；TODO(C3) 0 命中。
6. [x] 全量验证：`just ci` 通过 + clippy `-D warnings` 干净（含 7cd0fe8 引入的两处行数预算超限修复：拆出 collections/export_import.rs 与 skill_update_inventory/view.rs，commit 53d152b）。
7. [x] 父任务级复核：分析报告条目 #2 关闭，C3 核对记录已写入父任务 design.md（含 clippy --all-targets 11 个预存 lint 的 follow-up 建议）。

## 风险与回滚

- repos 签名改动波及全部域——步骤 1 必须最先做且单独 commit，出问题优先 revert 步骤 1。
- 散点模块（operation_log/logging）被几乎所有命令引用，改动后全量跑测试而非模块级。

## 启动前检查

- [ ] C2 已归档；工作区干净。
