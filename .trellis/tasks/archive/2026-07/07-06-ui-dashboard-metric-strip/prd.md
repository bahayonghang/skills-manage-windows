# MetricStrip 弱化为单行紧凑统计条

## Goal

把 Dashboard 的 4 张等权「图标 + 大数字 + 描述」卡——全应用最接近 DESIGN.md 反面清单（无差别 SaaS metric 网格）的元素——弱化为一条单行紧凑统计条。保留全部既有能力（点击跳转、i18n、tabular-nums、emphasized 语义、testId），只降视觉权重，不减信息。

## Confirmed Facts

- `src/components/dashboard/sections/MetricStrip.tsx:23-61`：`grid gap-3 sm:grid-cols-2 xl:grid-cols-4` 布 4 个等权 `StatButton`——中央技能→`/central`、AI 审查→`/central?filter=ai-review`、来源仓库→`/marketplace`、集合→`/collections`；各含 icon + label + value + description + `emphasized`。
- `StatButton` 实现在 `src/components/dashboard/DashboardPanels.tsx`（大数字磁贴样式，`active:scale-[0.96]`）；MetricStrip 是其唯一网格化用法。
- 现有 testId：`dashboard-metric-central` / `dashboard-metric-ai-stat` / `dashboard-metric-collections`（sources 项无 testId）；section 有 `aria-label`（`dashboard.metricStrip.ariaLabel`）。
- 2026-07-06 设计评审判定该区"靠整卡可点 + tabular-nums 勉强站在线内"；产品决策（2026-07-06）：弱化为单行紧凑统计条。
- 父任务决策 ②：Dashboard 密度维持现状——本任务腾出的纵向空间**不新增内容**。

## Requirements

- MetricStrip 改为一条水平紧凑统计条：4 个统计项（小图标 + 数值 + 标签）并排一行，项间用间距/细分隔，而非 4 个独立卡片容器；每项仍可点击跳转（4 个目标路由不变）。
- 数值保持 `tabular-nums`；`emphasized` 语义保留（数值前景强调即可，不做大数字磁贴）；description 不再平铺展示，可降级为 `title` 提示或移除用法。
- 明确窄窗口折叠行为（`<sm` 换行为 2×2 或横向滚动，二选一写死在组件里）。
- 保留 section `aria-label` 与现有 testId；每项键盘可聚焦且有可见焦点环（与 `07-06-ui-keyboard-focus-a11y` 的共享工具类对齐）。
- 样式遵守卡片签名语言：如需容器，用 `rounded-xl + ring-1 ring-border + bg-card`；禁止新的即兴圆角/描边。

## Acceptance Criteria

- [ ] Dashboard 该区渲染为单行统计条，不再是 4 张等权大数字卡（4 套代表主题目视抽查）。
- [ ] 4 个跳转行为不变；引用 testId 的现有测试通过（如有断言布局的测试则同步更新）。
- [ ] 数值 `tabular-nums`；Tab 可聚焦每一项且焦点环可见。
- [ ] `pnpm test`（dashboard 相关）、`pnpm typecheck && pnpm lint` 通过；收尾跑 `just ci`。

## Out Of Scope

- Dashboard 其他区块（hero/orbit/progress/agents/work-queue/logs/activity）的任何改动。
- 新增统计指标或改变统计口径。
- StatButton 在其他场景（若有）的样式。
