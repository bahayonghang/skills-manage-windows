# Design（Parent）

原单任务设计已按 2026-07-20 codex 审阅拆分并修订，技术设计移至两个 child：

- `07-20-tag-builtin-taxonomy/design.md` — 内置 tag 集、seed 冲突迁移、候选放开、UI 可见性。
- `07-20-ai-new-tag-review/design.md` — 提示词、proposal 存储、review 接受后创建、并发与归并。

共享决策（D1–D6）见本任务 `prd.md`。parent 不承载直接实现，仅做最终集成验收。

## 已废弃的原设计要点（勿采用）

- ~~新 tag 在建议阶段即时 `create_skill_tag` 落库~~ → 改为 review 接受后创建（D1）。
- ~~seed 仅按 id upsert~~ → 改为 custom 优先的冲突跳过策略（D2）。
- ~~前端零改动~~ → 需要内置 tag 按使用显隐（D3）。
