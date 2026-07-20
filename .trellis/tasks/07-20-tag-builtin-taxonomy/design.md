# Design：内置 tag 分类集：升级迁移与 UI 可见策略

## 1. 内置 tag 清单（seed.rs `builtin_skill_tags()`）

保留 `academic-research-writing`、`uncategorized`，新增：

| id                    | name          | description（英文，供 AI 判别）                                | color   |
| --------------------- | ------------- | -------------------------------------------------------------- | ------- |
| frontend-development  | 前端开发      | Web UI, React/Vue, CSS, component and page building.           | #3b82f6 |
| backend-development   | 后端开发      | Server-side APIs, databases, business logic, system services.  | #8b5cf6 |
| devops-deployment     | DevOps 与部署 | CI/CD, containers, infrastructure, release and ops automation. | #f97316 |
| testing-quality       | 测试与质量    | Test writing, code review, linting, QA workflows.              | #22c55e |
| docs-writing          | 文档与写作    | Technical docs, README, blogs, general writing and editing.    | #eab308 |
| data-analysis         | 数据与分析    | Data processing, SQL, visualization, reports and analytics.    | #14b8a6 |
| design-ui             | 设计与 UI     | Visual design, prototyping, design systems, UX polish.         | #ec4899 |
| ai-prompt-engineering | AI 与提示工程 | LLM prompts, agents, RAG, model integration workflows.         | #6366f1 |
| productivity-tools    | 效率与工具    | Automation scripts, CLI helpers, personal productivity.        | #64748b |
| office-documents      | 办公文档      | Word/Excel/PPT/PDF creation and manipulation.                  | #a16207 |

id 为发布后不可变契约（parent D4）。

## 2. seed 冲突迁移（custom 优先，parent D2）

替换现有无条件 `INSERT ... ON CONFLICT(id) DO UPDATE`。对每个内置 tag：

```sql
SELECT id, is_builtin FROM skill_tags WHERE id = ? OR name = ?
```

- 无命中 → INSERT（is_builtin=1）。
- 命中且全部为「同 id 且 is_builtin=1」→ UPDATE name/description/color/updated_at
  （注意：若 UPDATE 后 name 与另一个自定义 tag 撞名，跳过 name 更新，仅更新其余
  字段，避免 UNIQUE(name) 失败）。
- 命中包含 is_builtin=0（撞 id 或撞 name）→ 整条跳过，不 INSERT 不 UPDATE。

逻辑写在 seed.rs 的辅助函数中（单条 tag 处理），保持在既有 seed 事务/顺序内；
`prune_obsolete_builtin_skill_tags` 不变（只针对 is_builtin=1）。

## 3. AI 候选放开（prompt.rs）

过滤条件 `!tag.is_builtin || tag.id == ACADEMIC_RESEARCH_WRITING_TAG_ID`
→ `tag.id != UNCATEGORIZED_TAG_ID`。`ACADEMIC_...` 常量保留（seed/测试仍用）。
候选行的 `kind: built-in/custom` 标注保留，提示词继续要求「优先最具体的匹配，
custom 优先于宽泛 built-in」。

## 4. UI 可见性（纯前端，parent D3）

- `src/lib/centralTags.ts`：新增
  `getVisibleSkillTagsWithUsage(tags, counts)` —— 在现有 `getVisibleSkillTags`
  基础上再过滤「`is_builtin && (counts[id] ?? 0) === 0`」。
- `CentralTopFilters.tsx`：改用新函数（`facetCounts.tags` 已在组件内可得，
  上移到列表过滤处）。其他普通 tag 列表消费点（如 tag 管理设置页）**不变**——
  管理页需要看到全部 tag 才能编辑。
- `sanitizeSelectedTagIds` 不变：它按「已知 tag id」而非可见性过滤，选中的
  空内置 tag 仍是合法 filter（技能数归零仅从展示列表消失）。
- 需盘点 `getVisibleSkillTags` 现有调用点，逐个决定是否切换（Vitest 断言
  筛选栏行为，管理页保持全量）。

## 5. spec 重写（.trellis/spec/backend/central-skill-tags.md）

- §3 契约：内置集合改为「12 tag 清单 + custom-first 冲突跳过规则」；候选规则改
  为「排除 uncategorized 的全部 tag」。
- §4 矩阵新增两行：撞 id 自定义（保留 custom、跳过 seed）、撞 name 自定义
  （seed 不失败、跳过写入）。
- §6 测试清单同步；§7 Wrong/Correct 示例反转（旧 Correct 变 Wrong）。
- §3 UI 条款补「空内置 tag 按使用显隐」。

## 权衡

- custom 优先意味着少数用户拿不到某个内置 tag（他们已有同名分类，语义等价，
  可接受）；不做「合并链接到内置 tag」的复杂迁移。
- 可见性放前端而非 SQL：counts 已有现成 facet 数据，后端零改动。
