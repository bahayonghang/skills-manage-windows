# 微交互与细节打磨

## Goal

统一按压/过渡的微交互语言，修掉细节纪律漂移：两套按压反馈并存、3 处 `transition-all`、低于可读下限的 10px 信息标签、瞬态弹窗里的 accent 过量。单项都小，合起来决定"手感是否始终如一"。

## Confirmed Facts

- **两套按压语言并存**：`ui/Button` 用 `active:translate-y-px`（`src/components/ui/button-variants.ts:4`），而全仓 35+ 处手写按钮用 `active:scale-[0.96]`（遵守度很高，是纪律好的表现）。用 `<Button>` 还是裸 `<button>` 决定了按下去是"下沉"还是"缩放"。
- **scale 漂移值**：`active:scale-95` ×4 + `active:scale-[0.99]` ×1（`AppearanceSettingsSection.tsx:250,279,321,437,577`）、`active:scale-[0.98]` ×2（`Sidebar.tsx:59`、`SettingsSideNav.tsx:40`）。基准应为 `0.96`（低于 0.95 显夸张，0.98/0.99 几乎不可感知）。
- **`transition-all` ×3**（违反"永不 transition:all"）：`SettingsSideNav.tsx:40`（`transition-all active:scale-[0.98]`）、`AboutSettingsSection.tsx:323`（进度条宽度）、`CentralStatePortabilityDialog.tsx:423`（进度条宽度）。
- **子规格小标签**：Label 规格 0.72rem(≈11.5px)/650/0.12em；实际大量 `text-[10px]`/`text-[0.65rem]` 大写标签承载信息（`Sidebar.tsx:306,332`、`UnifiedSkillCard.tsx:478,691,812`、`HealthOrbit.tsx:64`、`SourceMeta.tsx:111` 等）。10px 大写 + 宽字距对低视力吃力，已触地板。
- **HealthOrbit 字距即兴**：`HealthOrbit.tsx:141,157` `tracking-[0.22em]/[0.2em]`，超 Label 规格 0.12em。
- **GlobalSearchDialog accent 过量**：`GlobalSearchDialog.tsx:129,148,172,195,212,226` 每行结果图标都 `text-primary/70`，列表级 accent 触碰 Accent-Is-Rare（瞬态弹窗内可容忍，属降噪项）。

## Requirements

- **决策并统一按压语言**：二选一——(a) 手写交互图标/磁贴类保持 `scale-[0.96]`，`ui/Button` 改为同语言；(b) 维持 Button 的 `translate-y-px`，把它明文写进 DESIGN.md 作为「表单/主按钮 vs 图标/磁贴」的双轨规则。无论哪种，漂移值（95/0.98/0.99）全部归一到 0.96。
- `transition-all` 三处改为指定属性：进度条 `transition-[width]`；SettingsSideNav 改 `transition-[background-color,border-color,color,scale]`（按实际动画属性）。
- 信息承载型标签字号托底 ≥11px（`text-[11px]` 或 0.68rem+）；`text-[10px]` 仅保留给纯装饰性计数/角标。
- HealthOrbit tracking 对齐 0.12em，或在 DESIGN.md 登记「readiness 面板 display 例外」。
- GlobalSearchDialog 行图标降为 `text-muted-foreground`，仅选中/高亮行升 primary。

## Acceptance Criteria

- [ ] `Grep 'transition-all'` 在 `src/` 0 命中。
- [ ] `Grep 'active:scale'` 仅剩统一值（0.96），无 95/0.98/0.99 漂移；按压语言决策已写入 DESIGN.md。
- [ ] 抽查 Sidebar/UnifiedSkillCard/HealthOrbit：信息标签 ≥11px。
- [ ] 目视抽查全局搜索弹窗（Cmd+K）：未选中行图标为中性色。
- [ ] `pnpm typecheck && pnpm lint`、相关测试通过；收尾跑 `just ci`。

## Out Of Scope

- MetricStrip 重构（已决策弱化为单行统计条，由兄弟任务 `07-06-ui-dashboard-metric-strip` 承载）。
- 新增任何动效/编排。
- UnifiedSkillCard 冻结基线 840 行的结构性改动（仅类名级微调）。
