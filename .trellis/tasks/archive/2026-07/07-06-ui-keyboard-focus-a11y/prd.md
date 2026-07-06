# 键盘焦点可见性与无障碍修复

## Goal

让纯键盘用户在**最高频操作路径**（装/卸技能到平台、卡片图标动作、就地确认）上处处看得见焦点；"已装/未装"状态不再靠颜色明暗单独承载；屏幕阅读器在中文 locale 下不再读到英文 "Close"；InlineConfirm 支持 Esc 撤退。

## Confirmed Facts

- `focus-visible:` 全仓仅 18 个 tsx 文件命中，集中在 `ui/` 原语（input/checkbox/switch/radio/textarea/input-group）与少数组件；`ui/Button` 原语已有完整焦点环（`src/components/ui/button-variants.ts:4`：`focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50`）。
- 以下**手写 `<button>` 无任何 focus-visible 类**，仅靠 base 层 outline-color 兜底（只改色不给宽度，浏览器默认细描边）：
  - 平台 toggle 图标 `src/components/skill/UnifiedSkillCardFooter.tsx:38-61`（全应用最高频操作）
  - 卡片图标动作 `src/components/skill/UnifiedSkillCard.tsx:603,786`
  - Dashboard 磁贴 `src/components/dashboard/DashboardPanels.tsx:14`
  - InlineConfirm 两键 `src/components/ui/inline-confirm-action.tsx:80-113`
  - `CentralSkillsShell` / `CentralSidebar` / `CentralSearchBar` / `CentralTopFilters` / `HeroSection` / `AgentsPanel` / `WorkQueuePanel` 等数十个 `active:scale-[0.96]` 手写按钮
- `UnifiedSkillCardFooter.tsx:40-45`：未装 = `text-muted-foreground/40`，已装 = `text-primary`，同一图标仅色相 + 透明度差异——违反 DESIGN.md Color-Is-Never-Alone；`/40` 透明度在卡片底上对比不足，低视力/色弱难辨装没装。
- `src/components/ui/dialog.tsx:79`（`<span className="sr-only">Close</span>`）与 `:117`（DialogFooter 默认文案 `Close`）硬编码英文，绕过 i18n（项目规定所有用户可见文本走 i18n）。
- `src/components/ui/inline-confirm-action.tsx:31-44`：armed 态仅监听 pointerdown 外点撤销，**无 Esc 处理**；armed 后焦点已移到确认键（`:52-56`），Esc 撤退是标准期望。

## Requirements

- 抽一个共享 focus ring 工具类（建议 `src/index.css` 中 `@utility` 或导出 cn 常量，形如 `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-card`），应用到上列全部手写 `<button>`；与 `ui/Button` 的 ring-3 视觉相容即可，不必逐像素一致。
- 平台 toggle 图标：未装态对比提级（去 `/40`，至少 `text-muted-foreground`）**并**增加非颜色差异（如已装态图标右下角小填充点 / 未装态描边处理 / 已装态 `ring-1 ring-primary/30` 底），hover/title/aria-label 行为保持不变。
- `dialog.tsx` 两处 "Close" 走 `t("common.close")`（zh/en 词条已有则复用，没有则补齐两语言）。
- `InlineConfirmAction`：armed 时按 Esc 复位到 idle（不触发确认），焦点回到 idle 触发键。

## Acceptance Criteria

- [ ] 纯键盘 Tab 走查中央技能库一张卡片：平台 toggle、图标动作、InlineConfirm 每一步均有清晰可见焦点环（人工走查，四套代表主题下抽查）。
- [ ] 已装/未装除颜色外存在形状差异；未装态图标对 `bg-card` 对比 ≥3:1（用主题 token 值论证或工具实测）。
- [ ] `Grep '>Close<|"sr-only">Close'` 在 `src/components/ui/dialog.tsx` 0 命中；中文 locale 下关闭键读中文。
- [ ] InlineConfirm armed 后按 Esc 复位且不触发 onConfirm——新增组件测试覆盖。
- [ ] `pnpm test`（相关文件）、`pnpm typecheck && pnpm lint` 通过；收尾跑 `just ci`。

## Out Of Scope

- 新增全局快捷键体系（Cmd+K 已有）。
- 触屏/移动端适配。
- aria 属性大盘点（现有覆盖已扎实：330 命中 / 119 文件）。
