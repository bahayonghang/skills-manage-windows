# 平台技能用量排序与来源区分

## Goal

在 Agents / 平台技能页（`/platform/:agentId`）上，用户能按 Skill Usage 的全历史用量把常用技能排到前面，在每张卡片右下角读到当前列表名次和全历史次数，并能靠收口后的徽章、卡片表面和来源筛选立刻分清插件只读副本与用户自己的 Central Skills 安装。

## Key Decisions

| 决策 | 选择 | 来源 |
| --- | --- | --- |
| 用量窗口 | Skill Usage 全历史（`count` + `lastUsedMs`），与 `/usage` Top skills 同口径 | 用户 Q1 = B |
| 右下角 | 当前筛选结果的 1-based 名次 + 全历史次数；无记录显式标明 | 用户 Q1 = B |
| 30 天徽标 | 留在 meta 行，继续标明 30 天；降为 muted，不和全历史名次抢 primary | 用户确认 B，并要求优化徽章 |
| 徽章 | 合并插件行冗余 pill，中央/独立安装改为短 chip | 用户：优化相关徽章样式 |
| 插件筛选 | 徽章 + 外壳 + 全平台轴 B 筛选；默认 origin 仍是 All | 用户 Q2 = B |
| 中央库页 | 不含 | 原需求只指向平台页 |

## Background

2026-08-17 Universal 截图：默认 `Name ↑`；`agent-browser` 为中央 symlink 且带 `1x · 30d`；`artifact-template-*` 同时带 `Plugin source`、`Read-only`、`Standalone — copy`；All 92 = SkillPort 安装 14 + 独立安装 78。插件因 `link_type = "copy"` 落入独立安装。用户要求用量基于 Skill Usage，并优化相关徽章。

## Confirmed Facts

用量两套窗口不得混称（`.trellis/spec/backend/skill-usage-analytics.md:58`）：

- 全历史：`list_top_skills` 无时间 cutoff（`src-tauri/src/db/repos/usage_repo.rs:371-398`）。`/usage` 默认 `count desc, lastUsed desc, name`（`SkillUsageTable.tsx:270-274`）。
- 近 30 天：`usage_get_skill_counts(skills, days)`。`days=0` 的 cutoff 是「现在」（`commands/usage.rs:384`），不能当全历史。
- 匹配键是技能名。同名用户副本与插件副本共享统计。
- `useSkillCallCounts(names, 30)` 只服务 30 天徽标。组件禁止直接 `invoke()`。平台页不得接入 `usageStore` 的 source/骨架状态机。

平台排序已有 `callCount` 字段（`platformSkillViewModel.ts:9-10`），默认仍是 `name` + `asc`（`PlatformView.tsx:99-100`）。平台卡片无 footer；`usageBadge` 在 meta 行且 `>0` 才渲染。虚拟化 `itemHeight={204}`；meta 行 nowrap。

徽章四套几何并存于 `SkillCardBadges.tsx`。用户/插件 Tab 仅 `isClaudePage` 渲染；Universal 把 `sourceFilter` 锁成 `"all"`。现有测试锁定「Universal / 非 Claude 页不显示来源 Tab」。Q2 = B 后，有插件行的非 Claude 页必须显示 Tab；无插件的 Universal fixture 可继续隐藏。

轴 A：`getPlatformSkillOrigin` = `link_type === "symlink"`。轴 B：`source_kind === "plugin"`。禁止用 `installed_at` / `repository` 存在性改分类。只用 `UnifiedSkillCard`。文案走 i18n。色走 `statusTone`。字号徽章用 `text-xs`，计数 `tabular-nums`。

## Requirements

### R1. 默认按全历史用量排序

- 默认：全历史 `count` 降序 → `lastUsedMs` 降序 → 名称。与 `SkillUsageTable` 的 `count` 模式一致。
- 零次沉底后再按名称。用量未返回时不得把「未知」写成 0 参与降序；未就绪时按名称排，不画名次。
- 名称 / 安装时间 / 更新时间 / 仓库排序保留。`callCount` 菜单文案标明「全部记录」。
- 匹配键继续用技能名。只读/插件行参与排序。
- 新读取必须带 `lastUsedMs`。禁止 `usage_get_skill_counts(..., 0)`。禁止平台页订阅 `usageStore`。

### R2. 右下角展示当前列表全历史名次

- 平台卡片右下角：有记录时 `#N · {count}`；无记录时「无记录」。禁止 `#0`。
- `N` 是 source + origin + search 之后的竞赛名次（1,2,2,4）。分组视图按组内重算。
- tooltip / aria-label 写清：当前列表第 N、全部已记录历史、次数。不得暗示这是未筛的 Skill Usage 全局第 N。
- 锚点用卡片内绝对定位，不进 meta 行，不占标题行动作，不把虚拟化行高顶破 204。
- 30 天徽标留在 meta 行，继续标明 30 天。

### R3. 徽章收口与扫视区分

| 角色 | 触发 | 视觉 | 文案 |
| --- | --- | --- | --- |
| 插件只读 | `source_kind === "plugin"` | `statusChipClass.warning` + Lock | 一枚「插件来源」 |
| 中央安装 | `link_type === "symlink"` | `statusChipClass.info` + Link2 | 短标签「中央技能库」；安装方式进 title |
| 独立安装 | 非 symlink 且非 plugin | muted chip + FolderOpen | 短标签「独立安装」 |
| 用户来源 pill | `source_kind === "user"` | 不再单独占一枚 | — |
| 只读 | 插件行 | 并入插件 chip | 非插件只读仍可单独显示 |
| 近 30 天 | `usageBadge > 0` | muted chip | `N 次 · 近 30 天` |
| 全历史名次 | 右下角 | `#N` 主数字，次数次级 | 见 R2 |
| 仓库名 | `SourceChip` | 保持 mono muted | 不变 |

同一 chip 几何（高度、padding、12px 图标、`text-xs`、统一圆角）。meta 行 nowrap；可见顺序：来源角色 → 30 天 → 仓库名。溢出不得切掉来源角色。

卡片外壳：插件 warning 弱化表面 + 左侧竖条；中央 symlink 用 info/primary 左侧竖条。不复用 central 的 `statusAccent`（那是更新状态 chip）。只读插件行保持不可选、不可卸载。分类函数不改。

### R4. 全平台轴 B 筛选

- 显示条件：`isClaudePage || pluginCount > 0`。Claude 页始终显示（现有行为）。无插件的 Cursor / 无插件 Universal fixture 不显示。
- Tab 仍是 全部 / 用户来源 / 插件来源，过滤继续走 `source_kind`。
- 左侧 `PlatformOriginNav` 仍只表达轴 A。默认 origin 仍是 All。
- Tab 区 aria-label 改为平台来源筛选，不再写死「Claude」。

### R5. 边界

- 不新增采集器，不改 `skill_calls` 语义。
- 不改 Central Skills、Marketplace、Projects、Collections、技能详情、Skill Usage 页本身。
- 新平台卡片 props 走判别联合，更新 variant 负例。
- 新 Tauri 命令登记 `commandMap`、浏览器 fixture，并跑 `pnpm docs:gen`。

## Acceptance Criteria

- [ ] 打开任一平台页时，默认按全历史次数降序；零次在后；平局按最近使用再按名称。
- [ ] 排序菜单标明全部记录窗口；可切回名称 / 安装时间 / 更新时间 / 仓库。
- [ ] 用量未返回时按名称排且不画名次；失败不把缺失画成 0 次名次。
- [ ] 每张平台卡片右下角为 `#N · 次数` 或「无记录」；origin / 搜索 / 来源 Tab 变化后名次重算；分组按组内重算。
- [ ] 同名用户副本与插件副本共享次数；竞赛名次有测试。
- [ ] 插件行只留一枚 warning「插件来源」；不再并排 Read-only + Standalone 长文本。
- [ ] 中央 symlink 行是短「中央技能库」chip，与插件色相不同；外壳左侧竖条可扫视区分。
- [ ] 30 天徽标仍标明 30 天，且不再用 primary 实心强调。
- [ ] 有插件的 Universal 显示全部/用户/插件 Tab；无插件的非 Claude 页不显示。默认 origin 仍是 All。
- [ ] 只读插件行仍无 checkbox / 卸载。
- [ ] 虚拟化行高保持 204；`>40` 项网格无重叠。
- [ ] 中英文、`statusTone`、i18n、字号契约满足。
- [ ] Vitest 覆盖排序/名次、徽章合并、右下角、默认排序、Universal 插件 Tab。`pnpm typecheck`、`pnpm lint` 通过。改了 usage IPC 则补 Rust 测试，完成时跑 `just ci`。

## Out of Scope

- 改 Skill Usage 页排行、热力图、未使用报表。
- 新采集器、自定义日期范围、7/30/90 切换器。
- 启发式拆分 SkillPort copy 与手工 copy。
- 第二套技能卡片。
- 中央库网格的用量排序与右下角名次。
- Marketplace / Projects / Collections / 技能详情的同等装饰。
- 把默认 origin 改成 SkillPort 安装。
- 侧栏 Universal `37` 与页内 All `92` 计数口径。
