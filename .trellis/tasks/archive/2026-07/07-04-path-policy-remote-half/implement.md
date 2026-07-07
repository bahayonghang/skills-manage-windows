# Implement：补完 Path policy 的 remote 半边

前置：design.md §2-§3 为唯一迁移清单；全程不改行为。

## 执行清单

- [x] 1. `paths.rs`：常量单点化 + `remote_join` 搬入 + 两个 remote helper
  - `APP_DATA_DIR_NAME` 改 pub；新增 `CENTRAL_SKILLS_REL_FROM_HOME` / `REMOTE_REPOS_REL_FROM_HOME` / `TARGETS_CACHE_DIR_NAME` / `UNIVERSAL_AGENTS_DIR_NAME` / `UNIVERSAL_SKILLS_REL`
  - `remote_join` 函数体从 `targets/exec.rs:698` 逐字节搬入；新增 `remote_central_skills_root` / `remote_repos_root`
  - 补 design §5.1 测试
  - 验证：`cargo test paths::`
- [x] 2. `targets`：exec.rs 删除 `remote_join` 原体，模块层 `pub use crate::paths::remote_join;`；两处 probe 脚本抽 `remote_probe_script()`（format! + 常量）；补脚本逐字节等价测试
  - 验证：`cargo test targets::`
- [x] 3. 泄漏点迁移（design §3 表格逐项）：`local_remote_sync.rs:238,241,550`、`db/types.rs:36`、`github_import/types.rs:170`、`obsidian/query.rs:187`、`claude_plugin.rs:162`、`db/seed.rs:291-299`
  - 验证：`cargo test`（全量）
- [x] 4. 全量门禁 + grep 复核
  - `cd src-tauri && cargo test`
  - `cargo clippy -- -D warnings`
  - `rg -n '\.skillsmanage|\.agents' src-tauri/src` 残留仅剩 design §4 白名单
- [x] 5. spec 更新：`.trellis/spec/backend/` 新增/登记 path-policy 约定（目录名与 remote 构造只从 paths.rs 取）
- [ ] 6. 提交（git-commit skill，[AI] 标头）→ 任务归档

## 回滚点

单分支顺序提交；步骤 1-3 各自可独立 `git checkout -- <file>` 回退，无 schema/数据变更。
