# Design：补完 Path policy 的 remote 半边

## 1. 方案概述

纯收敛重构：目录名常量与 remote 路径构造收进 `src-tauri/src/paths.rs`，泄漏点改为调用方。所有生成的路径值与现状比特级一致。

## 2. paths.rs 新增 API

### 2.1 常量（目录名单点命名）

```rust
pub const APP_DATA_DIR_NAME: &str = ".skillsmanage";        // 现有私有 const 改 pub
pub const CENTRAL_SKILLS_REL_FROM_HOME: &str = ".skillsmanage/skills";  // POSIX 相对家目录形式
pub const REMOTE_REPOS_REL_FROM_HOME: &str = ".skillsmanage/repos";
pub const TARGETS_CACHE_DIR_NAME: &str = "targets";
pub const UNIVERSAL_AGENTS_DIR_NAME: &str = ".agents";
pub const UNIVERSAL_SKILLS_REL: &str = ".agents/skills";
```

说明：`paths.rs` 内部实现（如 `central_skills_dir_from_home` 的 `.join(".skillsmanage").join("skills")`）保持不动——policy 单点指的是文件级单点，文件内字面量不算泄漏，且现有 26 条测试已 pin 住这些值。

### 2.2 函数

- **`remote_join(parent, child)` 从 `targets/exec.rs:698` 原样搬入 `paths.rs`**（函数体逐字节不变）。`targets` 模块 `pub use crate::paths::remote_join;` re-export，全部约 25 处 `crate::targets::remote_join` 调用点零改动；`targets/tests.rs:240` 现有测试继续生效。
- 新增两个语义化 helper（内部走 `remote_join`，保证与现调用比特级一致）：

```rust
pub fn remote_central_skills_root(remote_home: &str) -> String;  // = remote_join(home, CENTRAL_SKILLS_REL_FROM_HOME)
pub fn remote_repos_root(remote_home: &str) -> String;           // = remote_join(home, REMOTE_REPOS_REL_FROM_HOME)
```

## 3. 泄漏点迁移清单

| 位置                                    | 迁移方式                                                                                                                                                                                      |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `targets/exec.rs:38,53`                 | 两处相同 probe 脚本抽为一个 `fn remote_probe_script() -> String`，用 `format!` 原样模板 + `CENTRAL_SKILLS_REL_FROM_HOME` 拼入 `$HOME/…`；raw string 模板保持 `\t`/`\n`、引号、`--` 逐字节不变 |
| `targets/exec.rs:698`                   | `remote_join` 搬入 paths.rs + re-export（见 2.2）                                                                                                                                             |
| `services/local_remote_sync.rs:238`     | `paths::remote_repos_root(&remote_home)`                                                                                                                                                      |
| `services/local_remote_sync.rs:241`     | `paths::remote_central_skills_root(&remote_home)`                                                                                                                                             |
| `services/local_remote_sync.rs:550`     | `pair[0] == paths::APP_DATA_DIR_NAME && pair[1] == paths::TARGETS_CACHE_DIR_NAME`（组件已 lowercase，常量本身全小写，语义不变）                                                               |
| `db/types.rs:36`                        | `pub const UNIVERSAL_PROJECT_SKILLS_DIR: &str = crate::paths::UNIVERSAL_SKILLS_REL;`                                                                                                          |
| `services/github_import/types.rs:170`   | `PRIORITY_SKILL_ROOTS` 数组内该项换为 `crate::paths::UNIVERSAL_SKILLS_REL`                                                                                                                    |
| `services/obsidian/query.rs:187`        | `PathBuf::from(crate::paths::UNIVERSAL_SKILLS_REL)`                                                                                                                                           |
| `services/scanner/claude_plugin.rs:162` | `.agents` 判断换 `paths::UNIVERSAL_AGENTS_DIR_NAME`（`.codex` 是平台特定目录名，不属 README 路径语义，保留字面量）                                                                            |
| `db/seed.rs:291-292`                    | `posix_join(&home, &[paths::APP_DATA_DIR_NAME, "skills"])` / `&[paths::UNIVERSAL_AGENTS_DIR_NAME, "skills"]`                                                                                  |
| `db/seed.rs:296,299`                    | 后缀检查改 `format!("/{}", paths::CENTRAL_SKILLS_REL_FROM_HOME)` / `UNIVERSAL_SKILLS_REL`（运行时构造，值不变）                                                                               |

> seed.rs 虽在 PRD 白名单候选（"种子数据"）内，但 `builtin_agents_for_posix_home` 是 remote 家目录的路径**构造**逻辑，正属本任务 remote 半边范畴，故迁移；真正的种子字面量行（agent 内置路径字符串如 `~/.claude/skills`）不含 `.skillsmanage`/`.agents`，不受影响。

## 4. grep 白名单（AC-1 例外清单）

`rg '\.skillsmanage|\.agents' src-tauri/src` 允许残留：

1. `paths.rs` —— policy 定义点本身（含测试）。
2. **所有测试代码**：`*/tests.rs`、`central_migration.rs` `#[cfg(test)]` 块、`commands/settings.rs` 测试块、`services/central_updates/*/tests.rs` 等。
3. **注释/文档**：`central_migration.rs:63`、`db/types.rs:16,38`、`services/installation/fs_util.rs:140-141`、`paths.rs` 注释、`services/central_updates/inventory/tests.rs:38` 等 doc comment。
4. **用户可见文案（逐字保留契约）**：
   - `targets/error.rs:85` `#[error("Remote probe did not confirm ~/.skillsmanage/skills creation.")]`——域错误 Display 文案逐字保留（domain-error-enums spec）。
   - `services/github_import/types.rs:148` `NO_IMPORTABLE_SKILLS_ERROR` 提示文案。
   - `lib.rs:145` `expect("Failed to create ~/.skillsmanage directory")` panic 消息。

## 5. 等价性验证方案

1. **paths.rs 新增测试**：
   - `remote_central_skills_root("/home/alice")` == `"/home/alice/.skillsmanage/skills"`；带尾斜杠家目录、根目录 `"/"` 两个边界各一例。
   - `remote_repos_root("/home/alice")` == `"/home/alice/.skillsmanage/repos"`。
   - `remote_join` 搬家后在 paths.rs 补基础用例（根父目录、空 child）；targets 侧原测试经 re-export 继续跑。
2. **targets 测试**：`remote_probe_script()` 输出与旧 raw string 字面量逐字节相等（旧字符串作为期望值写死在测试里）。
3. **既有测试即等价性回归**：`db/tests.rs`、`services/central_skills/tests.rs`、`local_remote_sync` 相关、`db/seed` 相关测试大量断言了 `.skillsmanage/.agents` 具体路径值，迁移后全绿即证明值未漂移。
4. 门禁：`cd src-tauri && cargo test`、`cargo clippy -- -D warnings`、AC-1 grep 复核。

## 6. 不做的事

- 不动 `remote_parent` / `shell_quote`（PRD 只点名 `remote_join`）。
- 不动 SSH remote target lifecycle、不改任何 shell 引号/转义语义。
- 不改 `paths.rs` 既有函数的内部实现与签名。
