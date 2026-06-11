# Design：thiserror 批次 3——尾批五域 + 全局收尾（子任务 C3）

模式以父任务 `design.md` 第 1 节为唯一模板。本批含两类工作：尾批五域改造（机械推广）+ db/repos 层透传统一（结构性）。

## 尾批五域

| 域                                  | 错误类型             | 主要 commands 边界                                  |
| ----------------------------------- | -------------------- | --------------------------------------------------- |
| `services/usage`（含 providers/\*） | `UsageError`         | `commands/usage.rs`                                 |
| `services/obsidian`                 | `ObsidianError`      | `commands/obsidian.rs`                              |
| `services/ai_provider`              | `AiProviderError`    | `commands/marketplace.rs`（AI 解释）、settings 相关 |
| `services/ai_tagging`               | `AiTaggingError`     | tag 相关命令                                        |
| `services/portable_state`           | `PortableStateError` | `commands/portable_state.rs`                        |

## db/repos 透传统一

- `db/repos/*` 全部 18 个模块：`Result<_, String>` → `Result<_, sqlx::Error>`（直接透传，不引入薄包装——repos 无需附加上下文，调用方域错误经 `#[from] sqlx::Error` 接入）。
- `db/` 其余模块（seed.rs、migrations.rs、pool.rs）同步处理。
- 清除 C1/C2 留下的全部 `// TODO(C3)` 临时适配点。

## 非域归属散点

`central_migration.rs`、`operation_log.rs`、`logging.rs`、`secrets/`、`targets/`、`commands/bootstrap.rs`、`commands/settings.rs` 等：就近归入相应错误类型；确属 IPC 边界胶水的字符串错误保留在 commands 层。

## 扫尾标准（本批硬性验收）

排除 tests 后，`Result<.*, String>` 仅允许出现在 `commands/` 下 `#[tauri::command]` 函数签名及其直接辅助函数。
