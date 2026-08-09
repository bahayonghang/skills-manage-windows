# Dashboard 与 Central Skills 界面打磨

> 来源：2026-08-05 基于两张运行截图（Dashboard 首页、Central Skills 页）+ 源码静态核查的界面走查（make-interfaces-feel-better 原则，full 模式）。

## Goal

修复 Dashboard 与 Central Skills 两个高频界面上一组相互独立、各自可验收的界面打磨缺陷：不可区分的图标身份/状态、过小的点击区域、键盘不可见的焦点控件、截断文本无补救途径、以及产生浑浊中间色的多色相渐变。提升可访问性与一致性，不改变任何信息架构、布局结构与交互流程。

## Constraints

- 全部改动沿用现有 Tailwind CSS 4 + 语义 token 体系，禁止引入第二套样式方案、arbitrary `text-[...]` 字号/颜色或 inline `fontSize`（见 `.trellis/spec/frontend/typography-tokens.md`）。
- Body 默认字体 JetBrains Mono 是项目既定决策（`.trellis/spec/frontend/font-preferences.md`），本次不得当作缺陷改动。
- 所有用户可见文案走 i18n，新增/修改文案同步 `en.json` / `zh.json`。
- 技能卡片改动必须通过 `UnifiedSkillCard` 唯一实现收口，不为单一场景开分支（见 `.trellis/spec/frontend/skill-card-scenarios.md`）。
- 任何 `title` tooltip 不得替代可访问名称；涉及焦点可见性的修复必须对键盘与指针同样生效。

## Requirements

### R1（HIGH）标签移除按钮键盘焦点不可见

- 位置：`src/components/skill/CardTagEditor.tsx:66`
- 现状：标签上的移除 `X` 按钮 `opacity-0`，仅 `group-hover/tag:opacity-100` 显示；键盘 Tab 聚焦时按钮仍完全不可见，移除操作对键盘用户不可达。
- 要求：增加 `focus-visible` 显示路径（如 `focus-visible:opacity-100`），保证指针与键盘均可感知；保持 `transition-opacity` 既有过渡。

### R2（MEDIUM）Antigravity 与 Antigravity CLI 图标无法区分

- 位置：`src/components/platform/PlatformIcon.tsx:61-62`
- 现状：`"antigravity"` 与 `"antigravity-cli"` 映射到同一个 `AntigravityIcon`，卡片底栏两个相同 "A" 图标并排，只能靠 tooltip 区分。
- 要求：为 `antigravity-cli` 提供可区分的视觉（如叠加 CLI/终端角标或独立 glyph），不新增图标库依赖；Dashboard Agents 面板与卡片底栏同时生效。

### R3（MEDIUM）锁定平台与已链接平台视觉相同

- 位置：`src/components/skill/UnifiedSkillCardFooter.tsx:41-45`
- 现状：`isLocked` 与 `isLinked` 都是 `text-primary ring-1 ring-primary/30`，仅 linked 多一个 hover 背景；"始终包含、不可切换" 与 "已安装、可切换" 两种语义在静态下无法区分。
- 要求：为 locked 给出差异化静态表达（如更低对比度 + 锁定标识或 dashed ring），并保留既有 aria/title 语义。

### R4（MEDIUM）卡片图标按钮点击区域低于桌面密度下限

- 位置：`src/components/skill/UnifiedSkillCardParts.tsx:31`（`CardActionButton` `h-8 w-8`）、`src/components/skill/UnifiedSkillCardFooter.tsx:40`（`PlatformToggleIcon` `size-8`）
- 现状：可见按钮 32×32px，低于 dense desktop 40×40px 下限；卡片头部 4 个动作按钮 `gap-0.5`。
- 要求：按代码库既有模式（`UnifiedSkillCard.tsx:298` checkbox 的 `after:size-10` 伪元素）扩大有效点击区域；同步调整间距，保证相邻按钮点击区域不重叠。

### R5（MEDIUM）截断的技能名称没有补救途径

- 位置：`src/components/skill/UnifiedSkillCard.tsx:307-320`
- 现状：名称 `truncate` 后不展示完整文本（按钮与 `h3` 两个分支都无 `title`），如 "animation-vocabula…"；描述文本已通过 `SkillCardSummary` 的 `title={text}`（`UnifiedSkillCardParts.tsx:62`）提供全文，名称反而没有。
- 要求：为名称提供悬停可见的全文（`title={name}` 或与代码库一致的方式），两个分支都覆盖。

### R6（MEDIUM）Readiness 因子条多色相渐变产生浑浊中间色

- 位置：`src/components/dashboard/sections/HealthOrbit.tsx:20-25`（`FACTOR_TONE_CLASS`）
- 现状：`accent` 为 `from-chart-1 to-primary/45`（六主题 chart-1 均为绿色、primary 为粉色，见 `src/index.css:322` 起），绿→粉线性插值在截图中呈绿→黄→粉"彩虹"，中间色浑浊；四条 rail 四种色相，面板色彩噪声大。
- 要求：改为单色相（或同色系）渐变；如需区分因子，用同一色相的不同明度/透明度表达，不出现跨色相插值。

### R7（LOW）标签编辑器 "+" 按钮过小且看似禁用

- 位置：`src/components/skill/CardTagEditor.tsx:83-91`
- 现状：`size-5`（20×20px），无标签时 `opacity-40`，易被误认为装饰或禁用态。
- 要求：放大到与卡片其他控件一致的量级（含伪元素扩区），无标签时不使用"看似禁用"的透明度；保留 dashed 样式。

### R8（LOW）Work queue 卡片截断描述无 tooltip

- 位置：`src/components/dashboard/sections/WorkQueuePanel.tsx:69-76`
- 现状：label 与 description 均 `truncate`，无 `title`，描述恒被截断（"…apply updat…"）。
- 要求：为截断文本提供全文 tooltip 或等效补救。

### R9（LOW）侧栏分组开关 hint 截断失去信息价值

- 位置：`src/components/central/CentralSidebar.tsx:383-387`
- 现状：`sidebarBulkExpansionHint`（"Saved views, tag groups, repositories, and tags"）在侧栏宽度下恒被截断为 "Saved views, tag groups, r…"。
- 要求：缩短 hint 文案或移除副行改用 tooltip；中英文案同步。

## Acceptance Criteria

- [ ] 键盘 Tab 至标签移除按钮时按钮可见（R1），可触发移除。
- [ ] 卡片底栏与 Agents 面板中 Antigravity / Antigravity CLI 图标静态可区分（R2）；locked 与 linked 平台图标静态可区分（R3）。
- [ ] 卡片头部动作按钮与底栏平台开关的有效点击区域 ≥40×40px 且互不重叠（R4）；标签 "+" 不再以 20px/opacity-40 形态出现（R7）。
- [ ] 截断的技能名称与 work queue 描述悬停可见全文（R5、R8）；侧栏 hint 不再出现截断残句（R9）。
- [ ] Readiness 四条因子条均为单色相渐变，无绿→粉跨色相插值（R6）。
- [ ] 无新增 arbitrary 字号/颜色与 inline `fontSize`；`pnpm typecheck && pnpm lint` 通过；涉及交互的改动补跑对应 Vitest；收尾 `just ci` 通过。
- [ ] 新增/修改文案同时更新 `en.json` / `zh.json`，并检查 README 文案是否受影响。

## Out of Scope

- 字体预设、主题色板、布局栅格、信息架构的调整。
- 卡片底栏 PNG 平台图标（kiro 等真实应用图标）改为单色 SVG —— 已知权衡，本次仅记录不改。
- Agents 面板迷你条形的最小可见宽度地板 —— 保持比例真实性，不改。
