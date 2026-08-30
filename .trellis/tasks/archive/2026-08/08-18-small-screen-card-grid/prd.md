# 小屏窗口下 Central Skills 卡片网格响应式优化

## Goal

修复 Central Skills 页面在中小有效宽度窗口下卡片网格的可用性问题：卡片标题被截断到只剩 5–7 个字符，用户无法辨认技能名称。让布局在各档窗口宽度下平滑降级（4 列 → 3 列 → 2 列 → 1 列），并保证卡片标题行始终优先显示技能名。

## 现象（来自用户截图 2026-08-18）

- 大屏（窗口 CSS 宽度 ≈ 2550px，4 列 × ≈460px 卡片）：显示正常。
- 小屏（2560×1527 物理像素笔记本、150% DPI 缩放，有效 CSS 宽度 ≈ 1700px）：
  网格仍排 4 列，卡片仅 ≈276px 宽，技能名被截断为 `ask-ma…`、`batch-…`、
  `code-r…`，卡片可用性严重下降。

## 根因分析（已定位）

1. **网格最小列宽与卡片内容开销不匹配**（主因）。
   `src/lib/centralSkillGrid.ts` 中 `CENTRAL_SKILL_CARD_MIN_WIDTH = 220`、
   `MAX_COLUMNS = 4`。而 Central 场景卡片标题行的固定开销为：
   卡片 padding ≈32 + checkbox ≈30 + 4 个常驻操作图标（`shrink-0`，含 gap ≈140），
   合计 ≈200px。276px 宽的卡片留给技能名的只有 ≈74px（约 7 个字符）。
   即 220–300px 区间的卡片虽然"放得下"，但名字已经没法看了。

2. **标题行图标不随宽度降级**。
   `src/components/skill/UnifiedSkillCard.tsx:399`：comfortable 密度下 4 个操作图标
   常驻（`shrink-0`）；只有 compact 密度才 hover 才显示
   （`opacity-0 group-hover/skill-card:opacity-100`）。窄卡片没有触发任何降级。

3. **两侧固定面板挤压内容区**。
   左侧 app 导航 + Central 过滤侧栏（默认 286px，`centralSkillsLayoutSizing.ts`）
   合计固定占去 ≈500px。过滤侧栏已有折叠 rail 模式
   （`CentralSidebar.tsx` 的 `SIDEBAR_COLLAPSED_RAIL_PX` / pin 切换），
   但不会随窗口宽度自动收起。

4. **列数计算有两条路径需保持一致**：
   `>40` 项走 `VirtualizedGrid`（JS 按 `minColumnWidth`/`maxColumns` 计算列数），
   `≤40` 项走 CSS `minmax(auto-fill)`（`centralSkillCardGridTemplateColumns()`）。
   两条路径共用同一组常量，改常量时需同步验证两侧（现有测试
   `src/test/lib/centralSkillGrid.test.ts` 锁定了 220/4，需随方案更新）。

## Requirements

- 调整网格列宽策略，使有效窗口宽度 ≈1700px 时 Central 网格降为 3 列、
  更窄时进一步降为 2/1 列；卡片在任何列宽下技能名至少保留可用辨识度
  （目标：常见名称如 `ask-matt`、`code-review` 完整显示，或至少 ≥12 字符）。
- 窄卡片标题行降级策略：宽度不足时操作图标改为 hover/focus 显示
  （复用 compact 模式既有模式），或收进 `⋯` 溢出菜单；技能名优先于图标。
- 窗口较窄时（建议阈值 ≈1400px CSS）过滤侧栏自动从 pin 展开态退到 rail 态
  （或至少提供明显的一键收起），用户手动展开后尊重用户选择。
- 两条列数计算路径（VirtualizedGrid / CSS minmax）行为保持一致。
- 检查同页顶部工具栏（Add Skill / Update Center / Update mode /
  Check current results）在窗口最小宽度 900px（`tauri.conf.json` minWidth）
  下不溢出、不换行错乱。
- 所有用户可见文案走 `src/i18n/`。

## Acceptance Criteria

- [ ] 有效宽度 ≈1700px（150% 缩放的 2560 宽笔记本）下 Central 网格 ≤3 列，
      卡片技能名完整或仅轻微截断。（代码已保证：MIN_WIDTH=320 时该宽度
      内容区 ≈1150px → 3 列 × ≈375px；**待真实窗口人工确认**）
- [ ] 有效宽度 ≈1280px 下 ≤2 列（或 3 列但标题可读），无横向滚动条。（**待人工确认**）
- [ ] 窗口最小宽度 900px 下：单列/双列正常，顶部工具栏不溢出，侧栏可收起。
      （侧栏 <1400px 自动收纳轨已由测试覆盖；工具栏换行已有 flex-wrap，**待人工确认**）
- [x] >40 项（虚拟化）与 ≤40 项（CSS grid）两种路径在相同宽度下列数一致。
      （两条路径共用 CENTRAL_SKILL_CARD_MIN_WIDTH / MAX_COLUMNS / GAP 常量）
- [x] 大屏（≥1900px 内容区）仍保持 4 列，无回归。（公式行为不变：4 列下限
      由 max(320px, (100%-48px)/4) 保证，宽屏取后者）
- [x] 更新 `src/test/lib/centralSkillGrid.test.ts` 及相关卡片/网格测试；
      `pnpm test` 相关用例通过。（65 文件 / 751 用例全绿）
- [x] `just check` 通过；声明完成前跑 `just ci`。（2026-08-18 just ci 全部通过，
      含 Rust 平台测试）

## Notes

- 涉及文件（分析时已确认）：
  - `src/lib/centralSkillGrid.ts`（列宽/列数常量与模板函数）
  - `src/components/central/CentralSkillListContent.tsx`（两条渲染路径）
  - `src/components/ui/virtualized-grid.tsx`（JS 列数计算）
  - `src/components/skill/UnifiedSkillCard.tsx:353-470`（标题行与操作图标）
  - `src/pages/centralSkillsLayoutSizing.ts`、`src/components/central/CentralSidebar.tsx`（侧栏宽度/折叠）
- 遵守 spec：`.trellis/spec/frontend/skill-card-scenarios.md`（卡片改动走
  `toModel` 单一映射，勿在渲染层加场景分支）、
  `.trellis/spec/frontend/typography-tokens.md`（尺寸走 rem，随 fontScale 缩放——
  若提高 min 列宽，注意它是 px 常量，需评估 fontScale>1 时的行为）。
- 验收以真实窗口缩放（含 125%/150% DPI）人工确认为准；截图对比大屏/小屏。
