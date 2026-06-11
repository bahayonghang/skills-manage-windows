# 优化中央技能标签机制

## Goal

降低 Central Skills 顶部默认标签噪音，并让 AI 打标优先复用用户已经维护出来的标签体系。

本任务先停留在分析和规划阶段，不启动实现。后续实现时，默认内置可见分类只保留 `学术研究与写作`；其他当前内置分类从新库和既有库里移除。`uncategorized` 仍建议作为系统占位/智能视图保留，不作为普通默认分类曝光。

## Assumptions

- 用户说的“默认标签”指后端 seed 的 built-in taxonomy，不包括用户手动创建的 custom tags。
- 用户手动创建的标签、手动打上的标签关系不应被删除。
- `uncategorized` 是系统占位，不应等同于普通默认分类。当前代码依赖它支撑“未分类”智能视图和 AI 回退。

## Decisions

- `uncategorized` 保留。它作为系统占位、未分类智能视图和 AI fallback 继续存在，但不作为普通默认分类在顶部标签条、手动标签候选或 AI 主候选里展示。

## Confirmed Facts

- 默认标签来源是 `src-tauri/src/db/seed.rs` 的 `builtin_skill_tags()`，当前包含 8 个业务分类和 `uncategorized`。
- `seed_builtin_skill_metadata()` 只 upsert 当前 built-in 标签；如果代码里删掉旧 built-in id，既有数据库里的旧标签不会自动删除。
- 前端 `CentralTopFilters` 会渲染 store 里返回的全部 `tags`，因此截图里的默认标签噪音不是纯样式问题，而是后端 seed + 前端全量展示共同造成。
- AI 打标候选来自 `db::get_skill_tags()`。当前 prompt 只列出 `name (id)`，要求从候选大类选择 1 到 3 个标签。
- AI 映射逻辑只接受已存在的标签 name/id；模型返回未知标签时会被忽略，最终无可用建议时回退到 `uncategorized`，置信度 0.2。
- AI 自动应用阈值是 0.7，低于阈值的建议进入 review 队列。
- Saved View 和 URL state 可能持有旧 tag id；删除旧默认标签后，如果不清理或忽略这些 id，用户可能看到死筛选或裸 id chip。
- 便携导入会按导出文件里的 tag 名称创建标签；旧导出中的旧默认分类未来可能作为 custom tag 被恢复。

## Requirements

- R1. Fresh database 默认 taxonomy 中，普通可见内置分类只保留 `academic-research-writing` / `学术研究与写作`。
- R2. 既有数据库升级时，移除不再支持的 built-in 分类标签及其 `skill_tag_links` / pending AI reviews；保留用户创建的 custom tags。
- R3. `uncategorized` 保留为系统占位和智能视图能力，但不作为普通默认标签在顶部标签条或手动标签候选中制造噪音。
- R4. AI 打标 prompt 必须强调“只输出候选 tag id、不要发明新标签、优先复用最具体的已有 custom tag，不能为了凑数使用宽泛默认标签”。
- R5. AI 主候选应来自现有可用标签，优先包含 custom tags 和 `学术研究与写作`，不把 `uncategorized` 当作普通候选；无明确匹配时允许返回空数组，由后端回退到低置信度未分类。
- R6. 旧 URL / Saved View 中引用已删除 tag id 时，不应让用户陷入不可见筛选；应忽略或清理已不存在的普通 tag id，同时保留 `uncategorized` / `updates` / `ai-review` 等特殊筛选值。
- R7. 不自动把所有现有技能重新分类；删除旧默认标签后，失去有效分类的技能进入未分类状态，由用户手动或 AI 重新处理。

## Acceptance Criteria

- [ ] Fresh in-memory DB 初始化后，built-in tags 只包含 `academic-research-writing` 和系统占位 `uncategorized`。
- [ ] 模拟旧库中存在旧 built-in 标签、相关 tag links、pending AI reviews 后再次初始化，旧 built-in 标签及相关关系被清理，custom tags 不受影响。
- [ ] Central 顶部标签条和手动标签候选不再显示旧默认分类，也不把 `uncategorized` 当普通标签展示。
- [ ] 旧 URL / Saved View 中的已删除普通 tag id 会被移除或忽略，不留下无法解释的死筛选。
- [ ] AI 打标 prompt 测试覆盖：候选使用现有 tag id、排除 retired defaults 和 `uncategorized` 主候选、提示词明确要求复用已有标签并允许无匹配返回空数组。
- [ ] AI 返回未知标签或空建议时不创建新标签，仍回退为低置信度 `uncategorized` review/建议。
- [ ] 相关 Rust tests、Vitest 目标用例通过；收尾前 `just ci` 通过。

## Out Of Scope

- 不做自动批量重打标。
- 不引入标签层级、合并、重命名、别名管理 UI。
- 不改变 AI provider 设置、速率限制、review 抽屉主流程。
- 不处理旧便携导出文件的标签改写；仅记录它可能恢复旧标签为 custom tag。

## Open Questions

- None.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
