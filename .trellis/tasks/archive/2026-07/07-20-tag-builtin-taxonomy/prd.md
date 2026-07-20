# PRD：内置 tag 分类集：升级迁移与 UI 可见策略

Parent：`07-20-tag-taxonomy-ai-prompt`（决策 D2/D3/D4/D5/D6 对本任务生效）。

## Goal

扩充内置基础 tag 集并放开 AI 候选，同时保证：升级不破坏/不劫持自定义 tag、
不触发启动失败，且空内置 tag 不制造顶部筛选栏噪音。

## Requirements

### R1 内置基础 tag 集

- `builtin_skill_tags()` 扩充为约 12 个（保留学术、未分类）：清单见 design.md，
  每个含中文 name、英文 description（供 AI 判别）、颜色。
- 内置 tag id 一经发布不改（parent D4）。

### R2 升级迁移——custom 优先（parent D2）

seed 写入每个内置 tag 前处理冲突，两类都必须覆盖：

- 已存在**同 id** 且 `is_builtin=0` 的自定义 tag → 跳过写入，保留自定义 tag
  及其链接，不得改 is_builtin/name/description。
- 已存在**同 name**（不同 id）的自定义 tag → 跳过写入，不得触发 UNIQUE(name)
  错误导致数据库初始化失败。
- 已存在同 id 且 `is_builtin=1` → 照常 upsert（刷新 name/description/color）。
- `prune_obsolete_builtin_skill_tags` 行为不变：只 prune `is_builtin=1` 且不在
  当前集合的行。

### R3 AI 候选放开内置 tag

- `build_tagging_prompt` 候选过滤改为仅排除 `uncategorized`：全部内置 tag
  （除未分类）+ 自定义 tag 进入候选。

### R4 UI 可见性——按使用显隐（parent D3）

- 内置 tag 仅在关联技能数 ≥ 1 时出现在普通 tag 列表（中央库顶部筛选栏等）；
  自定义 tag 行为不变（始终可见）。
- 已选中后技能数归零的内置 tag：按现有 stale-id sanitize 规则处理，特殊
  filter id（uncategorized/updates/ai-review）不受影响。

### R5 spec 同步（parent D5）

- 同步重写 `.trellis/spec/backend/central-skill-tags.md` 中被本任务推翻的契约：
  内置 tag 清单（§3）、候选规则（§3/§7 Wrong-Correct 示例）、升级测试矩阵（§4/§6）、
  UI 可见性规则。必做，不可选。

## Acceptance Criteria

- [x] 新装 DB：`skill_tags` 含全部新内置 tag（is_builtin=1）+ 未分类。
- [x] 升级测试 A：旧库已有自定义 tag id 与某内置 tag 撞 id → seed 后仍为
      is_builtin=0，name/链接不变。
- [x] 升级测试 B：旧库已有自定义 tag name 与某内置 tag 撞 name → seed 成功
      （无 UNIQUE 错误），自定义 tag 不变，该内置 tag 未写入。
- [x] 重复 seed 幂等；prune 不删除新集合内 tag，不删除自定义 tag。
- [x] `build_tagging_prompt` 候选含内置 tag、不含未分类，有单测。
- [x] Vitest：技能数为 0 的内置 tag 不出现在普通 tag 列表；有关联后出现；
      自定义 tag 不受影响；特殊 filter id 保留。
- [x] spec 文档更新完成并与实现一致。
- [x] `just ci` 全绿。

## 非目标

- 不涉及 AI 提示词的「新建 tag」能力（child `07-20-ai-new-tag-review`）。
- 不改 tag_groups、不引入层级。
