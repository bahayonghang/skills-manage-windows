# 卡片表面语言收敛（圆角/描边/玻璃）

## Goal

全应用回到单一卡片语言：`rounded-xl(14px) + ring-1 ring-border + bg-card + shadow-sm`（DESIGN.md 签名）。越界者（22-25.6px 圆角、border 代 ring 的第二套惯用法、dashboard 外的装饰性玻璃）要么收敛、要么在 DESIGN.md 明文登记为例外——不允许无登记的偏离继续存在。

## Confirmed Facts

- DESIGN.md 约定：卡片圆角封顶 `rounded-xl`(14px)、禁止 24px+；签名描边是 `ring-1 ring-border`（非 border）；Glass-Is-Earned（玻璃只用于 dashboard 控制室场景）。`ui/card.tsx:15` 是标准答案。
- **`rounded-3xl`(22px) ×7**，全部 dashboard 玻璃面板：`src/components/dashboard/sections/` 下 `AgentsPanel.tsx:34`、`HeroSection.tsx:34`、`HealthOrbit.tsx:135`、`ActivityPanel.tsx:32`、`LogsPanel.tsx:25`、`ProgressBreakdown.tsx:24`、`WorkQueuePanel.tsx:57`。
- **超 24px 硬红线**：`AppearanceSettingsSection.tsx:141` `rounded-[1.6rem]`(25.6px)、`AboutSettingsSection.tsx:103,477` `rounded-[1.5rem]`(24px)；另有 `AppearanceSettingsSection.tsx:169,483` `rounded-[1.35rem]`(21.6px)。
- **`rounded-2xl` + `border`（非 ring）第二套惯用法成片**：`AppearanceSettingsSection.tsx:182,191,239,268,296,347,383,425,510,577`、`AboutSettingsSection.tsx` 多处、`CentralSkillAiTagPanel` / `CentralSkillCategorizePanel` / `CentralRepositorySyncPanels:105`、`DashboardPanels.tsx:183`。
- **dashboard 外装饰性玻璃**：`AppearanceSettingsSection.tsx:169` 静态主题预览卡叠 `backdrop-blur + shadow-2xl + shadow-background/30`（非 sticky、纯装饰，明确越界）；`BulkActionBar.tsx:64` 浮动操作条 `rounded-2xl + shadow-2xl + backdrop-blur`（浮层悬于滚动内容之上，可读性论证可辩护，需二选一决策）。sticky 头尾的 `bg-*/90 backdrop-blur`（FacetSection/CentralSidebar 等）判定为可辩护，不在本任务范围。
- **启动闪屏（检测器唯一强命中）**：`src/index.css:59` `.startup-card` `border-radius: 1.5rem`(24px)；`:1067` inline-code chip `0.45rem`(7.2px) 近 token 漂移；`:79` 闪屏渐变含未登记进 DESIGN.md 调色板的 `#89b4fa`（Catppuccin Mocha 标准蓝）。

## Requirements

- **决策一（dashboard 玻璃面板圆角）**：推荐收敛到 `rounded-2xl`(18px) 并在 DESIGN.md「Elevation/Components」补记「dashboard 玻璃面板 = 2xl 例外」；若产品倾向完全统一则全部回 `rounded-xl`。二选一，落决策进 DESIGN.md。
- Settings 的 24px+ 与 21.6px 任意值圆角全部收敛：普通卡 `rounded-xl`，确有层级需要的外层容器至多 `rounded-2xl`（若保留须与 dashboard 例外同条登记）。
- `rounded-2xl + border` 子面板统一为 `rounded-xl + ring-1 ring-border`；嵌套内层遵守同心圆角（外 14px、内边距 16px → 内层用 `rounded-lg`(10px) 近似同心，不与外层同值）。
- `AppearanceSettingsSection.tsx:169`：去 `backdrop-blur` 与 `shadow-2xl`，回落 `bg-card ring-1 ring-border shadow-sm`。
- `BulkActionBar.tsx:64`：二选一——保留玻璃并在 DESIGN.md 登记「浮动操作条 overlay 例外」，或降为实底 `bg-popover ring-1 ring-border shadow-md`。
- 启动闪屏：`.startup-card` 圆角 24px→18px；inline-code chip 对齐 token（6px 或 8px）；`#89b4fa` 登记进 DESIGN.md 调色板清单或替换为已登记色。
- DESIGN.md 补记「Dashboard 密度定位」条目：单屏多区高密度是有意的调度台张力（产品决策 2026-07-06），密度类反馈以此为基准判断，不因"超出单一焦点"而重排。

## Acceptance Criteria

- [ ] `Grep 'rounded-3xl'` 与 `Grep 'rounded-\[1\.\d+rem\]'` 在 `src/**/*.tsx` 0 命中（若保留 2xl 例外，DESIGN.md 有对应登记条目）。
- [ ] Settings 与 Central 子面板卡片统一 `ring-1 ring-border`：`Grep 'rounded-2xl border'` 相关命中清零或仅剩登记过的例外。
- [ ] DESIGN.md 已更新（圆角阶梯口径 + 保留例外的明文登记 + Dashboard 密度定位条目）。
- [ ] 4 套代表主题目视抽查 Dashboard 与 Settings：层次不塌、无回归。
- [ ] `pnpm typecheck && pnpm lint` 通过；收尾跑 `just ci`。

## Out Of Scope

- Dashboard 信息架构/密度调整（父任务 open question）。
- `.central-skill-card-surface` 的 box-shadow 描边实现重写（另行观察）。
- sticky 头尾的 `bg-*/90 backdrop-blur`（判定可辩护）。
