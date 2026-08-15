# Implement — Skill Usage 未使用技能视图

## Checklist（按序）

1. 后端 repo 查询：
   - [ ] `usage_repo.rs`：`list_central_unused_candidates(target_id)`（skills LEFT JOIN metadata/calls 聚合 + linked agents）
   - [ ] `usage_repo.rs`：`list_platform_unused_candidates(target_id)`（跨 agent 安装聚合 + normalized name 查 calls）
2. 后端 service：`build_unused_report(source, threshold_days)` + 类型定义（`aggregate.rs` 或新文件）；never_used / stale 分类。
3. 命令层：`usage_get_unused_skills`（`commands/usage.rs`、`ipc_registry.rs`、`src/lib/ipc/commandMap.ts`、`src/types/usage.ts`）。
4. Rust 单测：零调用、stale 边界（恰好阈值当天）、ambiguous/unmatched 名称、source 过滤、空库。
5. 前端 store：`usageStore` 新增 `unused` slice + 序列号 + target 失效；`skillUsageBindings.ts` 接入刷新。
6. 前端组件：`UnusedSkillsPanel.tsx`（分组/状态/阈值/排序全部视图本地）；接入 `SkillUsageView.tsx` 网格。
7. i18n：`skillUsage.unused.*` 中英文。
8. 生成物：`pnpm docs:gen`（新命令会改变 IPC 字典）。

## Validation

- 迭代期：`pnpm test`（前端）、`cargo test` 于 src-tauri（或项目规定的最小集）。
- 收尾：`just check` → `just ci`；`pnpm docs:gen:check` 与 `docs:build` 保持只读通过。
- 手测：本地 target 下页面展示两个分组；切换阈值 30/60/90 无额外 IPC（前端本地）；切换 target 后面板正确刷新。

## Risky files / rollback points

- `usage_repo.rs` 聚合 SQL（JOIN  correctness，尤其 metadata CHECK 约束 matched ⇔ resolved_skill_id 非空）。
- `usageStore.ts` 序列号模式（照抄现有三序列号，勿引入新的竞态）。
- 回滚：删面板 + 命令注册即可，无数据迁移。

## Follow-up before task.py start

- 确认 `agent_skill_observations` vs `skill_installations` 哪个是平台维度的权威来源（实现第 1 步时以扫描写入方为准）。
