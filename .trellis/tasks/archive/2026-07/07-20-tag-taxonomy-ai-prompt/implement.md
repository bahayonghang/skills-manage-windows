# Implement（Parent）

parent 不做直接实现。执行顺序与收口：

1. 完成并归档 child `07-20-tag-builtin-taxonomy`（无依赖，先行）。
2. 完成并归档 child `07-20-ai-new-tag-review`（依赖 1 定稿的 taxonomy 契约）。
3. parent 收口（本任务）：
   - [x] 核对 `.trellis/spec/backend/central-skill-tags.md` 与两个 child 的最终实现一致。
   - [x] 集成冒烟：旧库（含同名自定义 tag）升级启动；AI 打标端到端（复用 + 新 tag review）。
   - [x] `just ci` 全绿后归档 parent。

各 child 的执行清单见各自 `implement.md`。
