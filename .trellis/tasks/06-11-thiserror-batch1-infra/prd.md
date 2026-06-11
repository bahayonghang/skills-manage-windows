# PRD：thiserror 改造批次 1——基建 + installation + scanner（子任务 C1）

> 父任务：`06-11-analysis-driven-fixes` ｜ 执行顺序：第 3 位
> 依赖：必须在 A（spawn_blocking 改造）合入后开始，避免同文件签名冲突。
> 来源：分析报告条目 #2（全量改造决议见父任务 prd.md「已决定」#1）

## Goal

建立后端类型化错误处理的基础设施与参照模板：引入 `thiserror`，定义域级错误枚举模式，并完成 installation、scanner 两个域（全项目测试最厚的两个域）的改造，为 C2/C3 批次提供可机械复制的模板。

## Requirements

1. `src-tauri/Cargo.toml` 引入 `thiserror`。
2. 确立并文档化域级错误模式（详见父任务 `design.md`）：
   - 每域定义 `pub enum XxxError`（`#[derive(Debug, thiserror::Error)]`），变体覆盖该域真实失败类别（IO、DB、解析、未找到、超时等）。
   - `sqlx::Error`、`std::io::Error` 经 `#[from]` 接入。
   - commands 壳层经 `map_err(|e| e.to_string())` 或 `From<XxxError> for String` 转回字符串，**IPC 对外契约（错误为字符串）不变**。
3. 改造 `services/installation`（含 `fs_util`、`native`、`project`、`centralize`、`skip`、`batch`、`remote` 等）与 `services/scanner`（含 `persistence`、`claude_plugin`、`ssh_batch` 等）内部 `Result<_, String>` 为域错误类型。
4. 消除 `commands/scanner.rs:100` 的 `error.contains("timed out")` 字符串判断，改为匹配错误枚举变体（如 `ScannerError::Timeout`）。
5. 两域对应的 commands 壳层（`commands/linker.rs`、`commands/scanner.rs` 等）完成边界转换。
6. 错误消息文本对用户可见部分保持语义等价（前端 toast 展示不回归）。

## Acceptance Criteria

- [ ] `grep -r "Result<.*, String>" src-tauri/src/services/installation src-tauri/src/services/scanner`（排除 tests）命中为 0。
- [ ] `commands/scanner.rs` 无 `contains("timed out")`；超时分支有对应测试。
- [ ] installation（2086 行）与 scanner（1452 行）现有测试全部通过（允许因错误类型断言而修改测试，不允许删除用例）。
- [ ] `cargo clippy -- -D warnings` 零警告；`just ci` 全绿。
- [ ] 父任务 `design.md` 的「错误模式」章节按实际落地情况修订，作为 C2/C3 的执行模板。

## Out of Scope

- 其余 10 个域（C2/C3）。
- db/repos 层错误透传（C3 收尾统一处理，本批次 repos 调用处暂用 `map_err` 适配）。
