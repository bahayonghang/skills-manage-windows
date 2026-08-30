# Skill Usage 未使用技能视图

## Goal

在 Skill Usage 页面中，除"用得最多"的 skills 外，让用户能发现**从未使用 / 长期未用**的 skills（覆盖 Central 库与各平台安装），以便清理和优化技能库。

## Background / Confirmed Facts（代码调查结论）

- 页面：`src/pages/SkillUsageView.tsx`（路由 `/usage`），面板含 `UsageMetricStrip`、`SkillUsageTable`（Top skills）、`RecentCallsFeed`、`ActivityHeatmap`、`SkillUsageDetailPanel`、`ProviderHealthList`；数据经 `useUsageStore`（Zustand，target 作用域，序列号防陈旧提交）。
- 数据流：8 个 UsageProvider 扫描日志 → `usage_refresh`（5 分钟缓存）→ SQLite `skill_calls` / `skill_usage_metadata` → `usage_get_overview` 等命令（`src-tauri/src/commands/usage.rs`）→ store → 页面。
- `skill_calls` 只记录"被调用过"的 skill 名称；**页面从不列出完整已安装清单**，故当前没有"零调用"概念。
- 已安装清单在另一条链路：`skills`（Central 库，约 144 个，`is_central=1`）+ `skill_installations` / `agent_skill_observations`（各平台安装）；现有命令 `get_central_skills` / `get_skills_by_agent`，**无跨 agent 一次性聚合查询**。
- 两条链路已有连接点：
  - `skill_usage_metadata.resolved_skill_id`（usage 名 → Central skill id，refresh 时写入；匹配规则 `enrichment.rs`：normalized id 精确 → 唯一 normalized name；多重匹配 `ambiguous`，无匹配 `unmatched`）；
  - `usage_get_skill_counts(skills, days)`（技能卡片 30 天调用徽标）。
- 现有 "Installed / Unlinked" 过滤为**视图本地**，且 "Installed" = "匹配到 Central skill"，不是"装在某平台上"（`.trellis/spec/frontend/skill-usage-state.md:48`）。
- spec 约束：
  - 组件不直接 `invoke`，仅 store 内经 `@/lib/ipc` 调用（`skill-usage-state.md:34`）；
  - `skill_calls` 只存日志事实，不回写解析结果/路径/估算（`skill-usage-analytics.md:33`）；
  - 固定时间口径：overview/排行 = 全部历史；热力图 = 112 天；recent = 最新 20；卡片徽标 = 30 天；
  - `source` 过滤须一致作用于 overview/recent/detail。
- usage 数据与技能库均按 **active target**（local / SSH / WSL remote）作用域隔离，差集计算必须限定同一 target。

## Requirements

- R1（零调用清单）：对已安装但 `skill_calls` 中无任何记录的 skills 给出"从未使用"清单。
- R2（长期未用）：对有调用记录但 `last_used_ms` 距今超过阈值的 skills 给出"长期未用"清单；阈值 30/60/90 天可切换，默认 90 天（用户已确认），视图本地切换。
- R3（双清单覆盖，用户已确认）：
  - Central 库维度：基于 `skills.is_central=1` LEFT JOIN usage 元数据/调用聚合；
  - 平台安装维度：基于 `agent_skill_observations` / `skill_installations` 的跨 agent 聚合，按平台分组展示。
- R4（清理决策信息）：每项展示名称、匹配状态（matched/ambiguous/unmatched）、安装位置/链接平台、调用次数、最后使用时间、Skill.md 体积估算（复用 `static_token_estimate` / `static_byte_count`）。
- R5（排序与筛选）：支持按未用时长/体积/平台排序或筛选；遵守"视图本地过滤不进 store、不改后端请求"的 spec 约束——后端返回全量未用候选，排序筛选在前端视图层完成。
- R6（i18n）：全部文案走 `src/i18n/`，中英文齐备。

## Acceptance Criteria

- [ ] AC1: Skill Usage 页面可看到"从未使用"skills 列表（数量汇总 + 明细），Central 与平台安装两个分组均覆盖。
- [ ] AC2: 可看到"长期未用"skills（默认 90 天），可在 30/60/90 天间切换，并按未用时长排序。
- [ ] AC3: 每项展示 R4 所列清理决策信息；点击已匹配 Central 的项可复用现有详情/跳转路径。
- [ ] AC4: 未使用视图与现有页面使用同一 active target 与 source 口径；target 切换后数据正确失效刷新。
- [ ] AC5: 不改动 `skill_calls` 表语义；组件无直接 `invoke`；`just check` 通过，i18n 中英文无缺失。

## Key Decisions（用户已确认）

- D1: 清理对象 = Central 库 + 各平台安装（两者都要）。
- D2: 长期未用阈值 = 默认 90 天，30/60/90 可切换。

## Out of Scope

- 实际删除/卸载动作（本期只出清单与入口，不做批量删除/卸载 UI）。
- remote target 语义变更。
- 修改 `skill_calls` 写入逻辑或 usage provider 扫描逻辑。

## Risks / Deferred

- 平台安装维度存在名称匹配噪声：只装在平台上、未入 Central 的散件无法经 `resolved_skill_id` 关联，需按 normalized name 直查 `skill_calls`，`unmatched` 项需明确标注"无使用记录来源"。
- `get_central_skills` 全量在 144 个 skill 量级可接受；平台聚合查询需验证大数据量下性能。
