# 实施计划：小屏窗口下 Central Skills 卡片网格响应式优化

## 变更边界

**最小行为差**：有效内容宽度 1100–1300px 时网格仍排 4 列 × ≈276px 卡片，
技能名被标题行固定开销（≈218px：padding 28 + checkbox 30 + 4 图标 152 + gap 8）
压到 ≈74px（约 7 字符），无法辨认。

**行为所在**：列宽策略在 `src/lib/centralSkillGrid.ts`（两条渲染路径共用常量）；
标题行空间竞争在 `UnifiedSkillCard.tsx` 渲染层；侧栏空间让渡在 `CentralSidebar.tsx`
（已有 rail/overlay 模式与 pin 持久化）。

**要改的文件**：
1. `src/lib/centralSkillGrid.ts` — `CENTRAL_SKILL_CARD_MIN_WIDTH` 220 → 320。
   320 = 标题行固定开销 ≈218 + 技能名最小可用 ≈100（≈12 字符 @14px semibold）。
   效果：1700px 有效窗口（150% 缩放笔记本）4 列 → 3 列 × ≈375px（名 ≈157px）；
   1280px → 2 列；大屏（内容区 ≥1352px）保持 4 列不回归。
2. `src/test/lib/centralSkillGrid.test.ts` — 同步锁定值与模板字符串。
3. `src/components/skill/UnifiedSkillCard.tsx` — 卡片根加 `@container`；
   动作图标行加容器查询降级：`<22rem` 时 `hidden`，
   `group-hover/skill-card:flex` + `group-focus-within/skill-card:flex` 揭示
   （复用 compact 既有模式，满足 icon-control-hit-area 的 hover/focus 成对约定）。
   22rem 随 fontScale 缩放，保护 Scale=1.125 档。其它卡片场景
   （Marketplace/Projects/Platform minColumnWidth=420）永不触发该区间，不受影响。
4. `src/hooks/useMediaQuery.ts`（新增）— `useSyncExternalStore` 封装 matchMedia；
   server snapshot = true（宽屏默认），jsdom polyfill 返回 false 时走窄分支。
5. `src/components/central/CentralSidebar.tsx` — `canPin = useMediaQuery("(min-width: 1400px)")`；
   生效 pin = 用户偏好 && canPin。窗口 <1400px 时强制 rail（hover/focus 仍 overlay 展开），
   并隐藏 pin 按钮（避免点了无可见效果）；偏好仍持久化，窗口拉宽后自动恢复 pin。
6. `src/test/components/central/CentralSidebar.test.tsx` — renderSidebar 增加
   matchMedia spy（matches: true，维持既有用例走 pinned 展开态）；
   新增窄窗口用例：pinned 偏好 + matches: false → `data-pinned="false"`、宽度 48、
   hover 后 overlay 展开且无 pin 按钮。

**明确不做**：
- 不改 app 主导航宽度、不动 Marketplace/Projects/Platform 页面网格常量（420 已够宽）。
- 不改虚拟化行高、overscan、40/60 条阈值（typography-tokens spec 明令禁止扩散）。
- 不把删除图标收进 ⋯ 菜单（comfortable 信息架构不变）。
- 顶部工具栏已有 `flex-wrap` + `min-[521px]` 断点（CentralSkillsShell.tsx:360,453），
  900px 最小窗口下可换行，不溢出，无需改动——实施后用真实窗口人工确认。

## 实施记录（2026-08-18）

已完成 1–6 全部改动，两处相对计划的偏差：

- **侧栏查询反转为 `(max-width: 1399px)`**：jsdom polyfill 恒 `matches:false`，
  按"窄屏"表述后 false 落在宽屏分支，`CentralSkillsView.*` 等页面级既有用例
  零改动通过；无需逐文件补 mock。
- **窄窗口测试用 `vi.stubGlobal` 而非 `vi.spyOn`**：spyOn+restore 会清掉 setup 的
  vi.fn polyfill 实现（后续 `matchMedia()` 返回 undefined）。教训已沉淀到
  `.trellis/spec/quality/jsdom-browser-api-stubs.md`。

额外验证：Tailwind 容器查询组合类已确认编译进产物 CSS
（`@container not (width>=22rem)` 内含 hidden / group-hover / group-focus-within 三条规则）。

## 验证

- 定向：`pnpm vitest run src/test/lib/centralSkillGrid.test.ts src/test/components/central/CentralSidebar.test.tsx src/test/components/skill/`
- `pnpm typecheck`、`pnpm lint`
- 完成前 `just ci`
- 人工：真实窗口拖宽/拖窄（含 150% DPI 笔记本）对照 PRD 验收项截图。
