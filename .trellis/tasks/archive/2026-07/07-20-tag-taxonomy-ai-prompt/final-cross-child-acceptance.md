# 父任务最终跨子任务验收

日期：2026-07-20
基线：`dev` / child B archive commit `51e53f84` 之后
模式：Codex inline；parent 保持 planning，只承载集成验收

## 结论

两个 child 均按顺序完成、通过各自 `just ci` 并归档。最终实现同时满足
custom-first 内置 taxonomy、AI 候选规则、按使用显隐、proposal review-only
持久化，以及接受时原子创建/复用的父任务决策。跨子任务冒烟与最终全量门禁通过，
parent 可以归档。

## 子任务状态

| Child | 工作提交 | 归档提交 | 验收 |
| --- | --- | --- | --- |
| `07-20-tag-builtin-taxonomy` | `52a03268` | `c6324407` | `just ci`：1402 frontend / 876 Rust，通过 |
| `07-20-ai-new-tag-review` | `12d546f6` | `51e53f84` | `just ci`：1403 frontend / 886 Rust，通过 |

## 父任务 AC 对照

| AC | 状态 | 证据 |
| --- | --- | --- |
| 两个 child 独立验收并归档 | Passed | 两个 archive 目录、validation 文件及上表提交存在。 |
| spec 与最终实现一致 | Passed | `central-skill-tags.md` 包含 12 个 built-in、候选排除规则、usage visibility、proposal schema/ID/accept/skip/并发契约及测试矩阵。 |
| 旧库 custom 同名冲突安全 | Passed | `test_init_preserves_custom_tag_when_builtin_name_conflicts` 定向通过。 |
| 已有 tag 可自动应用或进入 review | Passed | bulk AI 定向测试确认高置信度 `academic-research-writing` 实际写入链接；低置信度路径由 child B DB/service suite 覆盖。 |
| 新 tag 仅接受后创建 | Passed | proposal-only service 测试确认无 tag/fallback link；多 skill 接受测试确认只创建一个 custom tag 并链接两项。 |
| 无空 built-in UI 噪音 | Passed | `centralTags` / `CentralTopFilters` 与 proposal review drawer 共 19 个 Vitest 通过。 |

## 集成冒烟

```text
cargo test test_init_preserves_custom_tag_when_builtin_name_conflicts --locked
  1 passed

cargo test bulk_ai_tagging_emits_progress_limits_parallelism_and_continues_on_failure --locked
  1 passed

cargo test proposal_is_persisted_for_review_without_tag_or_fallback_link --locked
cargo test test_accepting_same_name_proposals_reuses_one_tag_for_multiple_skills --locked
  2 passed

pnpm vitest run src/test/centralTags.test.ts src/test/CentralTopFilters.test.tsx \
  src/test/CentralSkillsView.categorize.test.tsx
  3 files passed; 19 tests passed

just ci (parent confirmation run)
  frontend: 128 files passed; 1403 passed; 1 skipped; build passed
  Rust: Clippy -D warnings passed; 886 passed; 4 ignored; integration suites passed
  all checks passed
```

## 已知边界

- bulk job 的候选 taxonomy 在任务开始时快照一次；同批 proposal 不动态加入后续请求。
- 非 ASCII proposal 使用稳定 SHA-256 前缀 ID 归并；实际 tag ID 撞异名行时回退 UUID。
- 本 parent 不承载产品代码，归档只提交本验收工件和 checklist 状态。
