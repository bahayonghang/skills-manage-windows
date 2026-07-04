# 补完 Path policy 的 remote 半边

## Goal

把 remote 路径构造与目录名字面量并入 `paths.rs` 的 path policy module，使 README 的路径语义（Central Skills / Universal Agents / 数据库 / targets 缓存）由代码单点强制执行——local 半边已收拢，本任务补齐 remote 半边。

## 背景与证据（2026-07-04 架构评审）

已收拢（不要重建）：`paths.rs`（712 行）持有 local 权威路径与部分 remote 原语——`central_skills_dir_from_home:113`、`universal_skills_dir_from_home:122`、`expand_remote_home_path:222`、`remote_join_home:401`、`APP_DATA_DIR_NAME` 常量。

仍散落（约 9 处重新拼写目录名）：

- `targets/exec.rs:38,53` — shell 侧 `$HOME/.skillsmanage/skills` 字面量；`:698` — `remote_join` 长在 targets 而非 paths seam 内。
- `services/local_remote_sync.rs:238,241,550` — `.skillsmanage/repos|skills|targets` 内联构造。
- `db/types.rs:36` — `.agents/skills` 常量。
- `github_import/types.rs:170`、`obsidian/query.rs:187` — 各自的路径字面量。

后果：目录名（`.skillsmanage/{skills,repos,targets}`、`.agents/skills`）被当作字面量反复重打，README 语义只有一半被 seam 强制。

## Requirements

1. 目录名常量单点化：`.skillsmanage` 子目录名（skills/repos/targets）与 `.agents/skills` 在 `paths.rs` 一次命名。
2. remote 路径构造（含 `remote_join`、shell 侧 `$HOME` 拼接）经 path policy module 提供或暴露；上述泄漏点全部改为调用方。
3. 纯收敛重构：所有实际生成的路径值与现状比特级一致（行为零变化）。

## Constraints

- 路径语义以 README 为准（CONTEXT.md「路径语义」节）：Central Skills `~/.skillsmanage/skills/`、Universal Agents `~/.agents/skills/`、本地库 `~/.skillsmanage/db.sqlite`、target 缓存 `~/.skillsmanage/targets/<target_id>/db.sqlite`。
- shell 侧 `$HOME` 展开、引号/转义语义不变（远程命令构造是敏感区）。
- 不改 SSH remote target lifecycle。

## Acceptance Criteria

- [ ] grep 验证：`.skillsmanage` / `.agents` 目录名字面量在 `src-tauri/src/` 内仅存在于 `paths.rs`（种子数据、测试、文档除外，白名单由 design 列出）。
- [ ] `paths.rs` 现有 26 条测试扩展覆盖 remote 构造（含上述各泄漏点迁移后的等价性用例）。
- [ ] `cd src-tauri && cargo test` 全过；`cargo clippy -- -D warnings` 通过。

## Notes

- 复杂度：中等，接近 lightweight+——仍按 complex 走三件套，但 `design.md` 预期很短（主要是白名单与等价性验证方案）。
- 呼应 CONTEXT.md 优先方向 #1 的剩余半边；建议在 `07-04-rust-test-support` 之后做。
