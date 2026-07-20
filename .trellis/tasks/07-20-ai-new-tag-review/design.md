# Design：AI 新标签 proposal/review 流程

依赖：`07-20-tag-builtin-taxonomy` 定稿的候选规则（排除 uncategorized 的全部 tag）。

## 1. 响应格式与类型（types.rs）

```json
{"tags":[{"tag":"id","confidence":0.0,"reason":"≤20字"}],
 "new_tag":{"name":"...","description":"...","confidence":0.0,"reason":"≤20字"}}
```

- 新增 `RawAiNewTagSuggestion { name, description, confidence, reason }`；
  `RawAiTagSuggestionEnvelope` 加 `new_tag: Option<...>`（serde default）。
- `parse_ai_tag_suggestions` 返回 envelope；裸数组/旧格式 → new_tag=None。
- 内部建议模型：`SkillTagSuggestion` 增加变体或并列结构
  `SkillTagProposal { skill_id, proposed_name, proposed_description, proposal_tag_id, confidence, reason }`
  （倾向并列结构，避免污染现有 SkillTagSuggestion 序列化契约；
  `SkillTagSuggestionResult` / progress payload 增加可选 `proposals` 字段）。

## 2. 归一化与降级（prompt.rs / mod.rs 纯函数，可单测）

`resolve_proposal(candidates, raw_new_tag) -> Resolved`：

- name trim；空或超长（>12 个中文字符/等长）→ 丢弃。
- `proposal_tag_id = normalize_repository_component(name)`（与
  `create_skill_tag` 同源，保证接受时 id 一致）。
- 撞已有候选（id 相等或 name 相等，trim 比较）→ 降级为该已有 tag 的普通复用
  建议（confidence 沿用 proposal 的）。
- 否则 → 合法 proposal。每 skill 至多 1 个（模型输出即单个字段，天然满足）。

## 3. proposal 存储（schema + repo）

`skill_ai_tag_reviews` 现结构：PK(skill_id, tag_id)，`tag_id NOT NULL`，
pending 查询 `JOIN skill_tags`。改动：

- `ensure_column` 增量加列：`proposed_name TEXT`、`proposed_description TEXT`
  （复用 `db/schema` 现有 ensure_column 机制，旧库安全升级）。
- proposal 写入：`tag_id = proposal_tag_id`（此时 `skill_tags` 无此行），
  `proposed_name/description` 填值；普通低置信度建议两列为 NULL。
- `get_pending_ai_tag_reviews`：`JOIN skill_tags` → `LEFT JOIN`，tag 字段
  `COALESCE(t.name, r.proposed_name)` 等；返回结构 `SkillAiTagReview` 增加
  `is_proposal: bool`（`t.id IS NULL AND proposed_name IS NOT NULL`）。
  JOIN 不到且无 proposed_name 的孤儿行过滤掉（现状即防御）。
- `replace_pending_ai_tag_reviews` 中「tag 必须存在」的校验对 proposal 行放开
  （proposed_name 非空即合法）。
- 同批同名归并：proposal_tag_id 由归一化 name 决定，天然按
  PK(skill_id, tag_id) 去重跨 skill 共享同一 tag_id。

## 4. 接受/跳过（tags_repo.rs）

- `create_skill_tag` 原子化：`INSERT ... ON CONFLICT(name) DO NOTHING` 后按
  name SELECT 返回（消除先查后插竞态）；归一化 id 与异名已有 tag 撞 id →
  现有空 id 分支扩展为「撞 id 也回退 UUID」（INSERT 前查 id 或捕获 id 冲突）。
- `accept_ai_tag_reviews`：对每个 tag_id，若 `skill_tags` 无此行且 review 行带
  proposed_name → 先 `create_skill_tag(proposed_name, proposed_description)`；
  得到的实际 tag id 可能 ≠ review 的 tag_id（同名已存在时复用既有 tag）——
  链接挂到**实际 id**，review 行状态置 accepted（保留原 tag_id 作历史）。
- `skip_ai_tag_reviews` 不变：仅状态更新，无 tag 创建、无残留。
- 建议流水线（mod.rs）：proposal 永不进 `replace_skill_ai_tags` 自动应用路径，
  只进 `replace_pending_ai_tag_reviews`；`low_confidence_count` 计入 proposal。

## 5. 提示词（prompt.rs，保持中文 + JSON-only）

在 child A 定稿的候选列表基础上追加规则段：

- 优先从候选选择最具体的已有 tag；custom 优先于宽泛 built-in。
- 仅当明确属于候选中不存在的类别 → `new_tag`（name ≤ 12 字、description 一句
  英文）；能用已有 tag 就禁止提议；不确定不提议。
- 不得输出「未分类」；无匹配无提议 → `{"tags":[]}`。

## 6. 前端最小适配

- `SkillAiTagReview` TS 类型加 `isProposal`；review 面板（AI review 列表）对
  proposal 显示「AI 新建标签」badge（i18n 双语），接受/跳过按钮走现有命令，
  后端语义已保证正确。无其他 UI 改动。

## 权衡

- proposal 复用 review 表（加两列）而非新表：改动最小，PK 语义不变；代价是
  `tag_id` 对 proposal 行是「未来 id」，靠 LEFT JOIN 区分——以 `is_proposal`
  显式化，避免前端误判。
- 接受时才创建（parent D1）：无残留 tag、无需清理任务；代价是接受路径多一次
  创建调用，可接受。
