# 优化中央技能标签机制 - Design

## Scope

这次改动跨越后端 seed / SQLite 数据清理、AI prompt、前端筛选展示和 URL/Saved View 状态兼容。实现时应保持修改集中在现有标签链路，不引入新的 taxonomy 管理系统。

## Current Data Flow

1. `seed_builtin_skill_metadata()` 调用 `builtin_skill_tags()` 写入 `skill_tags`。
2. 前端 store 通过 `get_skill_tags` 读取所有 tags。
3. `CentralTopFilters`、手动分类抽屉、搜索 chip 直接消费 tags。
4. AI 打标通过 `prepare_ai_tagging_context()` 读取所有 tags，`build_tagging_prompt()` 把 tag name/id 列给模型。
5. `map_ai_suggestions()` 只接受已存在的 tag name/id；无可用建议时回退到 `UNCATEGORIZED_TAG_ID`。
6. 高置信度建议通过 `replace_skill_ai_tags()` 写入 `skill_tag_links(source='ai')`；低置信度建议写入 `skill_ai_tag_reviews`。

## Backend Design

### Product decision

`uncategorized` is retained as a system tag. Implementation must hide it from ordinary category surfaces and AI primary candidates, but must not remove the DB row or break smart-view / fallback semantics.

### Built-in taxonomy

保留两个内置 id：

- `academic-research-writing`：唯一普通可见默认分类。
- `uncategorized`：系统占位，用于未分类智能视图和 AI 回退。

建议在 `seed.rs` 内把普通默认分类和系统占位的意图写清楚，避免后续把 `uncategorized` 当普通业务分类删掉。

### Existing database cleanup

`seed_builtin_skill_metadata()` 目前不会 prune 旧 built-ins。后续实现应在 upsert 当前内置标签后执行一次幂等清理：

- 查出 `skill_tags WHERE is_builtin = 1 AND id NOT IN (current_builtin_ids)`。
- 删除这些 tag id 关联的 `skill_tag_links`。
- 删除这些 tag id 关联的 `skill_ai_tag_reviews`。
- 删除这些 `skill_tags` 行。

不清理 custom tags，即使 custom tag 名称和旧默认分类相同也保留。因为用户手动创建的标签是用户数据，不属于“默认标签”。

### AI prompt candidates

AI 主候选应使用“可分类标签”集合，而不是所有 tag：

- 包含 custom tags。
- 包含 `academic-research-writing`。
- 排除 `uncategorized` 主候选。
- 排除已 retired 的旧 built-in tags，因为 seed prune 后它们不应存在；prompt 测试仍应覆盖不会把它们列入候选。

Prompt 应要求模型：

- 只输出候选 tag id。
- 不创建、翻译、同义改写标签。
- 优先选择最具体的已有 custom tag。
- 只有强匹配时给 `confidence >= 0.7`。
- 没有明确匹配时返回 `{"tags":[]}`，不要为了凑数选择唯一默认分类。

后端 `map_ai_suggestions()` 可以继续把空/未知建议回退到 `uncategorized` 0.2，保证兼容现有 review 流程。

## Frontend Design

### Visible tag set

新增一个轻量 helper，集中表达普通 UI 可见标签：

- `isSystemTagId(id)` 识别 `uncategorized`。
- `isSpecialTagFilterId(id)` 识别 `uncategorized` / `updates` / `ai-review` 这些 view-state 特殊值。
- `getVisibleSkillTags(tags)` 过滤系统占位，供顶部标签条和手动分类候选复用。

这样避免在 `CentralTopFilters`、`useCentralSkillsDerivedData()`、SearchBar chip 等位置散落字符串判断。

### Stale tag ids

删除旧默认标签后，URL/Saved View 可能仍包含旧 id。建议在 Central V2 入口中做一次派生清理：

- 保留当前 `tags` 中存在的 id。
- 保留特殊筛选 id：`uncategorized` / `updates` / `ai-review`。
- 移除不存在的普通 tag id。

这比直接改 saved view 数据更安全：不会静默修改用户保存的历史查询，但当前视图不会被死筛选卡住。若后续要持久清理 saved views，可单独做维护任务。

## Compatibility Notes

- 删除旧 built-in tag links 会让部分技能变成无有效分类；这符合“删除默认标签”的语义。
- 手动 custom tags 不删除，手动关系不删除。
- `replace_skill_ai_tags()` 只删除 source=`ai` 的旧 AI 标签，不会删除手动标签，这个行为应保留。
- 便携导入可能把旧导出里的旧默认分类重新创建为 custom tag。该风险记录在 PRD 中，不纳入本 MVP。

## Other Issues Found

- `uncategorized` 同时是数据库 tag、特殊筛选值和 AI fallback，概念耦合偏重。短期用 helper 收口，长期可考虑把系统状态和用户标签分表或加 `kind` 字段。
- 顶部标签条当前按名称渲染所有 tags，用户自定义标签多起来后仍会横向拥挤。后续可考虑只显示有命中/常用/固定标签，其余放进 More。
- Prompt 目前只列 name/id，没有 description 和“不要发明标签”的硬约束，容易诱导模型输出新词或过度使用宽泛分类。
- Saved View 用 query string 透传，缺少 schema-aware 迁移；本次只做运行时清理，后续如果 taxonomy 经常变动，应考虑 saved view 版本化。
