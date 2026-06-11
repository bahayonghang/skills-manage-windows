# Implement：重 IO 路径 spawn_blocking 改造（子任务 A）

技术模式见父任务 `design.md` 第 2 节。前置：D（06-11-eng-hygiene-quickfixes）已归档。

## 执行清单

1. [ ] 提升 `services/installation/fs_util.rs` 的 spawn_blocking 包装为跨域共享模块（`src-tauri/src/fs_util.rs`），installation 原路径 re-export → 验证：`cargo test` 绿，installation 测试不变。
2. [ ] 改造 `commands/central_updates_fs.rs`（17 处）→ 验证：该模块相关测试 + 手动更新检测冒烟。
3. [ ] 改造 `services/github_import/import.rs`（15 处）→ 验证：github_import tests.rs（1962 行）绿。
4. [ ] 改造 `commands/central_store_location.rs`（10 处，含递归搬迁）→ 验证：相关测试 + 手动中央目录冒烟。
5. [ ] 改造 `services/projects/crud.rs`（10 处）→ 验证：projects tests.rs（838 行）绿。
6. [ ] 改造 `services/central_skills/delete.rs` + `files.rs`（共 12 处）→ 验证：central_skills tests.rs（1751 行）绿。
7. [ ] 逐项评估清单（11 个文件，见 prd.md Requirements #3）：每项给出「改造/豁免+理由」结论，记录于本文件附录。
8. [ ] 全量验证：`just ci` + clippy `-D warnings`。

## 风险文件与回滚

- 风险最高：`central_store_location.rs`（目录搬迁含中途失败恢复逻辑），改造时注意闭包捕获导致的所有权调整不得改变失败恢复语义。
- 回滚：单 commit revert。

## 启动前检查

- [ ] D 已归档，工作区干净，基于最新 dev 分支。

## 附录：评估结论（执行时填写）

| 文件     | 结论 | 理由 |
| -------- | ---- | ---- |
| （待填） |      |      |
