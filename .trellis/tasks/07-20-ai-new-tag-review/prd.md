# PRD：AI 新标签 proposal/review 流程

Parent：`07-20-tag-taxonomy-ai-prompt`（决策 D1/D4/D5/D6 生效）。
**依赖**：child `07-20-tag-builtin-taxonomy` 已归档（候选规则与内置 taxonomy
契约定稿后，本任务的提示词与测试才有稳定基线）。此依赖为工件级声明，非树形隐含。

## Goal

AI 打标提示词改为「优先复用已有 tag，允许提议新 tag」；新 tag 以 proposal 形式
进入 review，**用户接受后才创建** `skill_tags` 记录（parent D1），并定义并发、
归一化与归并策略（codex 问题 4）。

## Requirements

### R1 提示词「优先复用、允许提议」

- 只从候选选 0–3 个已有 tag（输出 tag id），优先最具体匹配、custom 优先于宽泛
  built-in。
- 仅当技能明确属于候选中不存在的类别时，额外提议至多 1 个新 tag：
  `new_tag: {name, description}`，name ≤ 12 个中文字符（或等长英文短语）。
- 已有 tag 能覆盖时禁止提议；不确定时不提议；无匹配且无提议返回空
  （「未分类」fallback 仍由代码路径处理，模型不得直接输出）。
- 解析层向后兼容旧响应格式（无 new_tag 字段、裸数组）。

### R2 proposal 存储（不创建 tag）

- 建议阶段**不写 `skill_tags`**：proposal 以 proposed name/description 存入
  review 存储（schema 增量列，见 design），pending review 列表能展示 proposal
  （名称、描述、置信度、理由，标注「AI 新建标签」）。
- proposal 一律进 review，永不自动应用（无论 confidence）。

### R3 接受/跳过语义（parent D1）

- 接受：原子地创建 tag（is_builtin=0，按 name 幂等——同名已存在则复用该 tag）
  并挂 skill 链接；review 置 accepted。
- 跳过：仅改 review 状态，**不产生任何 `skill_tags` 行**，无残留。

### R4 归一化、归并与并发（codex 问题 4）

- proposal name 统一 trim + 归一化生成候选 id；同一批次内多个 skill 提出同名
  proposal 归并为同一 proposal 身份，接受任一后其余接受操作幂等复用同一 tag。
- proposal 与已有 tag 撞 name（或撞候选 id）→ 降级为对该已有 tag 的普通复用建议，
  不作为 proposal 存储。
- tag 创建路径改为原子（数据库层 ON CONFLICT，消除先查后插竞态）；归一化 id 与
  异名已有 tag 撞 id 时回退随机 id。
- 已知限制（记录即可）：批量任务候选快照任务开始加载一次，批内 proposal 不进入
  后续请求的候选；靠归一化归并保证不产生重复 tag。

### R5 spec 同步（parent D5）

- 更新 `.trellis/spec/backend/central-skill-tags.md`：新增 proposal/review 契约
  （创建时机、跳过语义、归并规则、并发要求）及测试矩阵行。

## Acceptance Criteria

- [ ] 单测覆盖响应解析：纯复用 / 纯提议 / 混合 / 空 / 旧格式（无 new_tag）。
- [ ] 提议阶段后 `skill_tags` 无新行；pending review 可见 proposal 信息。
- [ ] 接受后：tag 创建（is_builtin=0）+ 链接建立；同名重复接受不产生重复 tag。
- [ ] 跳过后：无 tag 行、无链接，review 状态正确。
- [ ] 撞名 proposal 降级为复用建议，有单测。
- [ ] 并发同名创建幂等（原子路径单测）。
- [ ] 前端 review 面板最小适配：proposal 正常展示与接受/跳过（Vitest）。
- [ ] spec 更新完成；`just ci` 全绿。

## 非目标

- 不做 proposal 的编辑/改名 UI（接受即按 AI 提议名创建）。
- 不做批内候选动态刷新。
