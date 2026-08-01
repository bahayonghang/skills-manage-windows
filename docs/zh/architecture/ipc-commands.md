# IPC 命令字典

每个标注 `#[tauri::command]` 的 Rust 函数都可由前端通过 `invoke('name', args)` 调用。下方表格由脚本自动生成，禁止手工编辑。

## 字典如何构建

```text
[scripts/build-ipc-dict.mjs] ── 读取 src-tauri/src/commands/**/*.rs
                                    │
                                    ▼
                         抽出 #[tauri::command]
                                    │
                                    ▼
                  按模块分组（commands::* / services::*…）
                                    │
                                    ▼
       写入 docs/architecture/_generated/ipc-commands.md
```

执行 `pnpm docs:gen` 刷新，并与对应 Rust 源码共同提交。CI 通过 `pnpm docs:build` 运行只读的 `pnpm docs:gen:check`；表格过期时直接失败，不会改写工作树。

## 调用约定

- **命名。** Rust 蛇形函数名与 JS `invoke()` 入参 1:1 对应：`invoke('scan_all_skills', {})`。
- **入参。** Tauri 通过 serde 把 JS 驼峰键映射为蛇形参数；传普通对象即可。
- **返回。** 所有命令返回 `Result<T, String>`。前端把字符串当作可见错误展示；详细诊断走 Runtime Log，而不是 `operation_logs`。
- **注入参数。** `State<AppState>`、`Window`、`AppHandle`、`Emitter` 由 Tauri 注入，JS 调用时不传。

## 真相源

生成产物位于 `docs/architecture/_generated/ipc-commands.md`，包括：

- 模块路径（`commands::scanner`、`services::installation::centralize`）
- 命令名
- 业务入参（已过滤 Tauri 注入参数）
- 返回类型
- 函数上方第一段 `///` 文档注释

<!--@include: ../../architecture/_generated/ipc-commands.md-->

Last reviewed: 2026-05-04
