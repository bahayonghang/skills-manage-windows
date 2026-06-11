# Design：thiserror 批次 1——基建 + installation + scanner（子任务 C1）

错误模式总设计见父任务 `design.md` 第 1 节，本文档只记录两域的落地细节。

## installation 域

- 错误类型：`services/installation/error.rs` 定义 `InstallationError`。
- 预期变体（按现有失败路径梳理，落地时核对）：`Io { path, source }`、`Db(#[from] sqlx::Error)`、`SkillNotFound`、`AlreadyInstalled`、`CentralizeFailed`、`SymlinkUnsupported`、`Remote(...)`（SSH/WSL 分发失败）。
- 涉及文件：`mod.rs`、`native.rs`、`project.rs`、`centralize.rs`、`skip.rs`、`batch.rs`、`remote.rs`、`fs_util.rs`、`types.rs`。
- 边界：`commands/linker.rs`（5 命令）及其他调用 installation 的命令处 `map_err(|e| e.to_string())`。

## scanner 域

- 错误类型：`services/scanner/error.rs` 定义 `ScannerError`。
- 预期变体：`Io { path, source }`、`Db(#[from] sqlx::Error)`、`Timeout(u64)`、`Parse { path, reason }`、`Remote(...)`。
- 关键改造点：`commands/scanner.rs:29-40` 的 `run_remote_scan_with_timeout` 返回 `ScannerError::Timeout`，第 100 行 `error.contains("timed out")` 改为 `matches!(e, ScannerError::Timeout(_))`。
- 涉及文件：`mod.rs`、`persistence.rs`、`claude_plugin.rs`、`ssh_batch.rs`、`types.rs`。
- 边界：`commands/scanner.rs`（1 命令）+ re-export 调用方（`commands/discover` 已废弃路径若仍编译需同步处理）。

## 跨域注意

- A 任务已将 `fs_util` 提升为共享模块：其包装函数签名同步泛化为 `Result<T, E>` 或在两域各自适配，落地时择优并回写父 design.md。
- db/repos 仍返回 String（C3 才统一），本批用 `map_err` + `Other(String)` 临时适配 repos 调用点，并标注 `// TODO(C3)` 便于扫尾定位。

## 测试影响评估

- installation tests.rs 2086 行、scanner tests.rs 1452 行：断言 `unwrap_err()` 字符串内容的用例需改为匹配变体或 `to_string()` 等价文本；逐条在 PR 列明，不允许删用例。
