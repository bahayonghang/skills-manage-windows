# Central 技能网格虚拟化卡片重叠修复

## Goal

修复 Central Skills 页 grid 视图下技能卡片纵向互相重叠/内容被下一行裁切的问题，使虚拟化网格的行高从"估算值"变为"真实契约"，卡片在任何字号缩放下都不再溢出所在行。

## 现象（来自用户截图）

- 页面：Central Skills，grid 视图，143 个技能（> 40，触发虚拟化）。
- 红框区域内：上一行卡片的描述文字第 3 行、`+` 加标签按钮、footer 仓库行与下一行卡片顶部互相压叠；描述末行被渐变/下一行卡片背景遮掉一半，看起来像两行卡片叠在一起。

## 根因分析

1. **固定估算行高 + 绝对定位**。`CentralSkillListContent.tsx:193-205` 在 `sortedSkills.length > 40` 时走 `VirtualizedGrid`；`virtualized-grid.tsx:107-137` 把每行 `absolute` 放在 `top = rowIndex * (itemHeight + rowGap)`，`itemHeight` 来自 `centralSkillGrid.ts:8-31` 的 `centralVirtualItemHeight()`：compact/fontScale=1 时为 184px，comfortable 为 192px。这只是估算，从不测量真实卡片高度。
2. **真实卡片高度由内容决定，且稳定超过估算值**。compact 卡片内容实测约 205-230px：`p-3.5` 上下 28 + 标题行 `min-h-7` 28 + 描述 line-clamp-2（`text-xs leading-relaxed` ≈ 19.5px/行）39 + `SkillCardMeta` 徽标行（`flex-wrap`，徽标多时会折成两行）20-40 + 可编辑标签行 `h-5` 20 + footer（`border-t pt-2` + `size-8` 图标）40 + 各处 `gap-2`。comfortable（line-clamp-3）还要再多一行约 20px。即卡片几乎必然比 `itemHeight` 高 20-40px。
3. **溢出为什么会"画"到下一行**。卡片 shell 是 `h-full`（`UnifiedSkillCard.tsx:285` 只给了 `min-h-[168px/188px]` 下限，没有上限），它的百分比高度挂在 gridcell（`virtualized-grid.tsx:129` 的 `div.min-w-0`）上；gridcell 自身高度来自 grid 行 stretch，形成循环百分比解析，浏览器按内容高度布局，于是卡片按真实内容长高，越过本行边界压到下一行（下一行 DOM 靠后、且是实色 `bg-card`，会把上一行溢出内容遮掉一部分——截图中描述第 3 行"变暗被切"即此效果，叠加 `SkillCardSummary` 截断渐变，视觉上更像两行卡叠在一起）。
4. **次要问题**。
   - 可编辑标签行固定 `h-5 overflow-hidden`（`UnifiedSkillCard.tsx:511`），但 `CardTagEditor` 的 `+` 按钮是 `size-8`（32px），被裁成只剩一小截，截图中每行之间孤立的 `+` 就是它。
   - `centralVirtualItemHeight` 的 fontScale 分段补偿是拍脑袋系数，与卡片真实内容（行数、换行）无关联，列宽变化引起描述换行时高度也会变。
   - `VirtualizedList` 同样的固定高问题（compact 168px），只是列表卡没有 footer，溢出量小。

## Requirements

- 卡片高度成为单一数据源的显式契约：grid/list 的 `itemHeight` 与卡片 shell 的精确高度（不再是 `min-h-*` 下限）由同一个常量/机制给出，两种密度、任意 fontScale 下一致。
- 卡片内部各行高度收敛为确定值：描述保持 line-clamp（compact 2 行 / comfortable 3 行）；`SkillCardMeta` 徽标行收敛为单行（nowrap + 溢出隐藏/截断），不再因 `flex-wrap` 折行撑高卡片；footer 用 `mt-auto` 钉在底部。
- 修复标签行 `+` 按钮被 `h-5` 裁切的问题（行高与按钮尺寸对齐）。
- 溢出宁可裁在卡片内部（配已有截断渐变），绝不允许画出行边界压到下一行。
- 分组视图（`CentralGroupedSkillList`，非虚拟化 plain grid）共用同一卡片，修复后不得引入回归；marketplace/projects/collections 等其他 `UnifiedSkillCard` 变体场景保持现状不破坏。
- 所有用户可见文本改动走 i18n（本任务预期无新增文案，如有必须同步中英文资源）。

## Acceptance Criteria

- [ ] Central 页 grid 视图（>40 技能，触发 VirtualizedGrid）在 compact 与 comfortable 两种密度下，任意滚动位置都不再有卡片互相重叠/文字被下一行裁切。
- [ ] fontScale（显示字号缩放）调整后仍无重叠。
- [ ] list 视图（搜索触发或手动切换）同样无重叠。
- [ ] 标签 `+` 按钮完整可见、可点击。
- [ ] 非虚拟化路径（<40 技能 plain grid、分组视图）渲染不回归。
- [ ] `pnpm typecheck && pnpm lint` 通过；相关 Vitest 用例（`src/test/components/skill/UnifiedSkillCard.test.tsx`、`unifiedSkillCardVariants.test.tsx` 及 central 页相关用例）通过并补充覆盖新高度契约的断言（如适用）。
- [ ] `just ci` 通过。

## Notes

- 关键文件：
  - `src/components/central/CentralSkillListContent.tsx`（虚拟化入口、itemHeight 来源）
  - `src/lib/centralSkillGrid.ts`（`centralVirtualItemHeight` / grid 常量）
  - `src/components/ui/virtualized-grid.tsx` / `virtualized-list.tsx`（绝对定位行）
  - `src/components/skill/UnifiedSkillCard.tsx`（卡片 shell、`min-h`、标签行）
  - `src/components/skill/SkillCardMeta.tsx`（flex-wrap 折行）
  - `src/components/skill/UnifiedSkillCardParts.tsx`（SkillCardSummary line-clamp + 截断渐变）
  - `src/components/skill/CardTagEditor.tsx`（`+` 按钮尺寸）
- 设计取舍见 `design.md`。
