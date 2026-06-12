# PRD：thiserror 改造批次 2——中批五域（子任务 C2）

> 父任务：`06-11-analysis-driven-fixes` ｜ 执行顺序：第 4 位
> 依赖：必须在 C1 完成并归档后开始（按 C1 验证后的模板执行）。
> 来源：分析报告条目 #2

## Goal

按 C1 确立的域错误模板，完成中批五个域的类型化错误改造。

## Requirements

1. 改造以下 5 个域的 `Result<_, String>` 为域错误枚举（模式严格遵循父任务 `design.md` + C1 落地模板）：
   - `services/central_skills`
   - `services/github_import`
   - `services/projects`
   - `services/marketplace`
   - `services/local_remote_sync`（含 `local_remote_sync.rs` 与 `local_remote_sync/` 目录）
2. 对应 commands 壳层（`skills.rs`、`github_import.rs`、`projects.rs`、`marketplace.rs`、`local_remote_sync.rs`、`central_metadata.rs` 等涉及处）完成边界转换。
3. IPC 对外契约不变；用户可见错误消息语义等价。
4. 发现模板不适配的场景时，先回父任务 `design.md` 修订模式再继续，不允许批内自创第二种模式。

## Acceptance Criteria

- [ ] 上述 5 个域目录内（排除 tests）`grep "Result<.*, String>"` 命中为 0。
- [ ] 5 个域现有测试全部通过（github_import 1962 行、central_skills 1751 行、projects 838 行等；允许调整断言，不允许删用例）。
- [ ] `cargo clippy -- -D warnings` 零警告；`just ci` 全绿。
- [ ] 手动冒烟：marketplace 同步、GitHub 导入、项目扫描各一次无回归。
