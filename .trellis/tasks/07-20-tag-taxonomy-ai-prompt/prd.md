# PRD（Parent）：优化技能 tag 分类与 AI 打标

> 本任务为 parent：持有需求总集、任务地图与跨子任务验收；实现工作在两个 child 中
> 独立完成与验收。本 PRD 已根据 2026-07-20 codex 审阅报告修订（7 条问题全部采纳）。

## 背景与问题（代码分析，2026-07-20）

1. **内置基础 tag 太少**：`db/seed.rs` `builtin_skill_tags()` 仅「学术研究与写作」
   +「未分类」（fallback）。冷启动用户 AI 打标几乎全落「未分类」。
2. **AI 候选过滤排除内置 tag**：`services/ai_tagging/prompt.rs` 候选过滤为
   `!is_builtin || id == academic-research-writing`，新增内置 tag 也进不了候选。
3. **提示词禁止新建 tag**：与期望的「优先复用、允许新建」相反，候选外建议被丢弃。
4. **fallback 体验差**：未命中以 0.2 置信度强制「未分类」。

## 已定的产品决策（对两个 child 都生效）

- **D1 新 tag 创建时机**：AI 提出的新 tag **必须等用户在 review 中接受后才创建**
  `skill_tags` 记录；建议阶段只保存 proposed name/description，skip 不产生任何
  tag 行。（codex 问题 2 的裁决）
- **D2 seed 冲突策略——custom 优先**：seed 内置 tag 时，若已存在同 id 或同 name
  的自定义 tag（is_builtin=0），**跳过该内置 tag 的写入**，绝不劫持或覆盖自定义
  tag，绝不触发 UNIQUE(name) 启动失败。（codex 问题 1）
- **D3 内置 tag UI 可见性——按使用显隐**：内置 tag 仅在「至少关联 1 个技能」时
  出现在中央库顶部筛选栏等普通 tag 列表；自定义 tag 行为不变。避免 10 个空 tag
  重新制造 06-11 任务消除过的顶部噪音。（codex 问题 3）
- **D4 内置 tag id 稳定契约**：内置 tag id 一经发布不改；从内置集移除（prune）
  是删除用户链接的破坏性操作，必须作为显式迁移决策，不作为常规回滚手段。
  （codex 问题 6）
- **D5 spec 同步为必做交付物**：`.trellis/spec/backend/central-skill-tags.md`
  的契约与测试矩阵随本次变更同步重写，两个 child 各自更新其触及的条款。
  （codex 问题 5）
- **D6 门禁**：两个 child 完成验收均以 `just ci` 为准（含 fmt/clippy locked
  all-targets/测试/sizecheck 等完整链）。（codex 问题 7）

## 任务地图

| Child | 目录 | 交付物 | 依赖 |
| --- | --- | --- | --- |
| 内置 taxonomy | `07-20-tag-builtin-taxonomy` | 内置 tag 集扩充 + 升级迁移（id/name 冲突）+ AI 候选放开 + UI 可见性规则 | 无 |
| AI 新标签流程 | `07-20-ai-new-tag-review` | 提示词「优先复用+可提议新 tag」+ proposal 存储 + review 接受后创建 + 并发/归并策略 | 依赖前者定稿的 taxonomy 契约（见其 prd） |

## 跨子任务验收（parent 收口）

- [x] 两个 child 分别通过 `just ci` 并归档。
- [x] `.trellis/spec/backend/central-skill-tags.md` 与最终实现一致：内置 tag
      清单、候选规则、新 tag review 契约、UI 可见性规则、升级测试矩阵全部更新。
- [x] 集成冒烟：升级路径（带自定义同名 tag 的旧库）启动不失败；AI 打标一轮后，
      复用建议直接生效/进 review，新 tag 建议仅在接受后出现在 tag 列表。
- [x] 无 UI 噪音回归：空内置 tag 不出现在中央库顶部筛选栏（Vitest 覆盖在 child A）。

## 非目标

- 不引入 tag 层级结构；tag_groups 机制不动。
- 不做存量技能批量重打标（复用现有批量打标入口）。
