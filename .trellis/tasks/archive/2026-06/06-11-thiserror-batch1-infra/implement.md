# Implement：thiserror 批次 1（子任务 C1）

前置：A（06-11-spawn-blocking-io）已归档。设计见本任务 `design.md` + 父任务 `design.md` 第 1 节。

## 执行清单

1. [ ] `src-tauri/Cargo.toml` 添加 `thiserror` → 验证：`cargo build` 通过。
2. [ ] 定义 `ScannerError`（`services/scanner/error.rs`），自底向上改造 scanner 域内部函数签名 → 验证：`cargo test scanner` 绿。
3. [ ] 改造 `commands/scanner.rs`：超时分支用 `ScannerError::Timeout`，消除 `contains("timed out")`；补超时分支变体匹配测试 → 验证：`cargo test scanner` 绿。
4. [ ] 定义 `InstallationError`（`services/installation/error.rs`），自底向上改造 installation 域 → 验证：`cargo test installation` 绿。
5. [ ] 改造 `commands/linker.rs` 等边界转换 → 验证：`cargo test` 全绿。
6. [ ] grep 扫尾：两域目录（排除 tests）`Result<.*, String>` 0 命中。
7. [ ] 按实际落地修订父任务 `design.md` 第 1 节（模板回写），列出 C2/C3 可直接复制的样板。
8. [ ] 全量验证：`just ci` + clippy `-D warnings`；手动冒烟扫描与安装/卸载各一次。

## 风险与回滚

- 模式风险集中点（见父 implement.md）：模板不可行时停止后续批次，本批单独 revert，回父任务重新设计。
- repos 调用点的临时 `Other(String)` 适配必须带 `// TODO(C3)` 标记，否则 C3 扫尾会遗漏。

## 启动前检查

- [ ] A 已归档；工作区干净。
