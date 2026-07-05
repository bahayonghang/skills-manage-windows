# Path Policy（路径语义单点约定）

## 契约

README 路径语义（Central Skills `~/.skillsmanage/skills/`、Universal Agents `~/.agents/skills/`、数据库 `~/.skillsmanage/db.sqlite`、targets 缓存 `~/.skillsmanage/targets/<id>/`）由 `src-tauri/src/paths.rs` 单点强制执行，local 与 remote 两个半边都是。

### 目录名常量（唯一来源 `paths.rs`）

| 常量                           | 值                     |
| ------------------------------ | ---------------------- |
| `APP_DATA_DIR_NAME`            | `.skillsmanage`        |
| `CENTRAL_SKILLS_REL_FROM_HOME` | `.skillsmanage/skills` |
| `REMOTE_REPOS_REL_FROM_HOME`   | `.skillsmanage/repos`  |
| `TARGETS_CACHE_DIR_NAME`       | `targets`              |
| `UNIVERSAL_AGENTS_DIR_NAME`    | `.agents`              |
| `UNIVERSAL_SKILLS_REL`         | `.agents/skills`       |

### remote 路径构造（唯一来源 `paths.rs`）

- `remote_join(parent, child)`：POSIX 拼接原语，本体在 `paths.rs`，`targets` 模块 `pub use` re-export（调用方继续写 `crate::targets::remote_join` 也合法）。
- `remote_central_skills_root(remote_home)` / `remote_repos_root(remote_home)`：远端家目录下的中央技能 / repos 根。
- shell 脚本内的 `$HOME/...` 拼接（如 targets probe 的 `mkdir -p`）必须用 `format!` 引入上述常量，禁止重打字面量；模板改动需保持逐字节等价（见 `targets/tests.rs` 的 `remote_probe_script_matches_historical_literal_byte_for_byte`）。

## 规则

1. **生产代码禁止重打 `.skillsmanage` / `.agents` 目录名字面量**——新增路径构造一律引用 `paths.rs` 常量或函数；需要新目录名时先在 `paths.rs` 命名。
2. `paths.rs` 文件内部的字面量不算泄漏（policy 定义点本身），既有 26+ 条测试 pin 值。
3. 允许字面量的白名单（grep `\.skillsmanage|\.agents` 复核口径）：
   - 测试代码（`*/tests.rs`、`#[cfg(test)]` 块）；
   - 注释 / doc comment；
   - 用户可见文案：`targets/error.rs` 的 `#[error(...)]` Display 文案（逐字保留契约，见 [domain-error-enums](./domain-error-enums.md)）、`github_import/types.rs` 的 `NO_IMPORTABLE_SKILLS_ERROR`、`lib.rs` 的 `expect` panic 消息。

## 背景

2026-07-05 任务 `07-04-path-policy-remote-half`：local 半边此前已收拢，本次把 remote 半边的 9 处泄漏（targets probe 脚本、`local_remote_sync` 的 repos/skills/targets 构造、`db/types.rs`、`db/seed.rs` 的 remote 家目录改写、`github_import`/`obsidian` 的目录清单、`claude_plugin` 的 `.agents` 判断）全部迁入 `paths.rs`，纯收敛、路径值比特级不变。
