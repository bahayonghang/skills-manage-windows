# Implement：thiserror 批次 3（子任务 C3）

前置：C2（06-11-thiserror-batch2-mid）已归档。

## 执行清单

1. [ ] db/repos 透传统一（18 个 repo + seed/migrations/pool）：`Result<_, sqlx::Error>` → 验证：`cargo test db::` 绿（tests.rs 2186 行）。
2. [ ] 清除 C1/C2 全部 `// TODO(C3)` 适配点（grep 验证 0 残留）→ 验证：受影响域测试绿。
3. [ ] 尾批五域逐域改造（usage → obsidian → ai_provider → ai_tagging → portable_state，每域一个 commit）→ 验证：各域测试绿。
4. [ ] 非域归属散点处理（central_migration / operation_log / logging / secrets / targets / bootstrap / settings）→ 验证：`cargo test` 全绿。
5. [ ] 全局扫尾 grep：
   ```bash
   grep -rn "Result<.*, String>" src-tauri/src --include="*.rs" | grep -v tests
   # 输出仅允许 commands/ 边界签名
   ```
6. [ ] 全量验证：`just ci` + clippy `-D warnings`。
7. [ ] 父任务级复核：分析报告条目 #2 关闭，结果记录到父任务。

## 风险与回滚

- repos 签名改动波及全部域——步骤 1 必须最先做且单独 commit，出问题优先 revert 步骤 1。
- 散点模块（operation_log/logging）被几乎所有命令引用，改动后全量跑测试而非模块级。

## 启动前检查

- [ ] C2 已归档；工作区干净。
