# Implement：thiserror 批次 2（子任务 C2）

前置：C1（06-11-thiserror-batch1-infra）已归档，父 design.md 模板已回写定稿。

## 执行清单（每域一个 commit 节点，按依赖从少到多排序）

1. [x] `services/local_remote_sync` → 验证：`cargo test local_remote_sync` 绿（7 passed）。
2. [x] `services/marketplace`（含 Http 变体约定增补，先回写父 design.md）→ 验证：`cargo test marketplace` 绿（11 passed）。
3. [x] `services/projects` → 验证：`cargo test projects` 绿（31 passed）。
4. [x] `services/github_import` → 验证：`cargo test github_import` 绿（59 passed）。
5. [x] `services/central_skills`（先列全 commands 调用清单）→ 验证：`cargo test central_skills` 绿（50 passed）。
6. [x] grep 扫尾：五域目录（排除 tests）`Result<.*, String>` 0 命中。
7. [x] 全量验证：`just ci` 通过 + clippy `-D warnings` 干净（含 sizecheck 超限修复：拆出 `repository_sync/summaries.rs` 与 `central_skills/delete/repository.rs`）；手动冒烟（marketplace 同步、GitHub 导入、项目扫描）待人工在桌面应用执行。

## 风险与回滚

- 每域独立 commit，单域问题单独 revert。
- 测试断言调整逐条在 PR 列明，不允许删用例。

## 启动前检查

- [ ] C1 已归档；父 design.md 模板为最终版；工作区干净。
