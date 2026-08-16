# Implement — 未使用技能面板 unlink 操作与徽章优化

## Checklist（按序）

1. 读 `.trellis/spec/backend/skill-deletion-integrity.md` + `installation/native.rs` 现状，确定"推广现有命令 vs 新命令"（倾向推广 `uninstall_skill_from_agent` 的 row_id 分支到全 agent，claude 行为不变）。
2. 后端报告扩展：platforms 条目改 per-agent `installs` 数组（row_id/skill_id/link_type/source_kind/is_read_only/installed_path）；Central 条目 agents 附 linkType。TS 类型 + commandMap 同步。
3. 后端 unlink 路径：全 agent observation unlink（守卫 + dir_path 校验 + 真目录删除 + observation/installations 行清除）；generic 路径补删 observation 行。
4. 后端测试（见 design.md 测试节）。
5. 前端 store：`unlinkUnusedSkill` action + 成功后 `refreshUnused()` + 错误 toast（formatBackendError）。
6. 面板 UI：行操作区 unlink 两段式按钮（hit-area spec）、Central 按 agent 的禁用态 tooltip、State 徽章 chip 化去截断、行 hover 可发现性。
7. i18n en+zh。
8. 前端测试：确认流转/刷新/失败/禁用态/徽章。
9. `pnpm docs:gen`（若命令签名变化）+ `just ci`。

## Validation

- `cargo test --locked` 定向：installation + usage；前端 `pnpm test`；`just ci`。

## Risky files / rollback points

- `installation/native.rs` 删除路径推广——claude 回归测试必须锁死既有行为。
- 真目录删除：dir_path 校验先行，任何不匹配拒绝执行。
- 回滚：增量改动，无迁移。

## Follow-up before task.py start

- 无。范围已确认（全部可 unlink）。
