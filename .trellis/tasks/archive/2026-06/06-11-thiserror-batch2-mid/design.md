# Design：thiserror 批次 2——中批五域（子任务 C2）

模式以父任务 `design.md` 第 1 节（C1 落地后回写的最终版）为唯一模板，本批不引入新模式。发现模板不适配 → 先回父 design.md 修订，再继续。

## 五域错误类型与边界

| 域                                         | 错误类型               | 主要 commands 边界                                                                          |
| ------------------------------------------ | ---------------------- | ------------------------------------------------------------------------------------------- |
| `services/central_skills`                  | `CentralSkillsError`   | `commands/skills.rs`、`commands/central_metadata.rs`、`commands/central_updates*.rs` 调用处 |
| `services/github_import`                   | `GithubImportError`    | `commands/github_import.rs`                                                                 |
| `services/projects`                        | `ProjectsError`        | `commands/projects.rs`                                                                      |
| `services/marketplace`                     | `MarketplaceError`     | `commands/marketplace.rs`                                                                   |
| `services/local_remote_sync`（.rs + 目录） | `LocalRemoteSyncError` | `commands/local_remote_sync.rs`                                                             |

## 注意点

- github_import 含 HTTP 失败类别（reqwest），变体需区分网络错误/限流/解析失败——这是 C1 模板未覆盖的新类别，预计需在父 design.md 增补 `Http` 变体约定后落地。
- central_skills 被多个 commands 模块调用，边界转换点最多，先列全调用清单再动手。
- repos 调用点继续 `// TODO(C3)` 临时适配。
