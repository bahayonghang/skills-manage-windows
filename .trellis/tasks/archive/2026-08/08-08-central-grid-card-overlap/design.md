# Design — Central 技能网格虚拟化卡片重叠修复

## 方案对比

### 方案 A（推荐）：固定高度契约 + 卡片内部行收敛

让"行高"成为真实契约，而不是估算：

1. **单一数据源**：在 `centralSkillGrid.ts` 导出每个 (view, density) 的精确卡片高度常量（取代 `centralVirtualItemHeight` 的估算公式；fontScale 通过 CSS 变量/重算处理，或保持常量但把补偿系数校准到真实内容）。`CentralSkillListContent` 把它同时传给 `VirtualizedGrid.itemHeight` 和卡片（CSS 变量如 `--central-card-height`，或 className）。
2. **gridcell 高度链打通**：`virtualized-grid.tsx` 的 gridcell 加 `h-full min-h-0`（或显式 `style.height = itemHeight`），卡片 shell 用 `h-full`（去掉 `min-h-[168px/188px]` 下限思维，改为精确高度），`overflow-hidden` 保证内容裁在卡内。
3. **内部行收敛**：
   - `SkillCardMeta`：`flex-wrap` → `flex-nowrap overflow-hidden`（单行，多余徽标隐藏或截断）。
   - 描述：保持 line-clamp-2/3 不变。
   - 标签行：`h-5` 与 `CardTagEditor` 的 `+` 按钮尺寸对齐（如行高改 `h-6` 且按钮 `size-5`，或按钮改 `size-5`），保证完整可见。
   - footer：`mt-auto` 钉底（现状已是）。
4. `VirtualizedList` 走同一常量机制。

优点：虚拟化保持 O(1) 均匀行高，滚动数学不变，改动小、可预测。
代价：卡片内容被严格约束，个别内容多的卡片会在卡内截断（有渐变兜底，交互可接受）。

### 方案 B（备选）：动态测量行高

`VirtualizedGrid` 为每行挂 ResizeObserver，维护实测高度缓存 + 前缀和偏移，替代固定 stride。

优点：卡片高度自由，内容完整。
代价：虚拟化核心重写（可见范围计算、overscan、滚动锚定、首屏未测量行的回退高度），复杂度和回归风险明显高；为这个问题收益不成比例。

**结论：选 A。** 若未来确实需要内容自适应高度，再单独立项做 B。

## 实施要点

- `src/lib/centralSkillGrid.ts`：导出精确高度常量（按 view × density），删除/替换 `centralVirtualItemHeight` 的估算公式；校准 compact/comfortable 常量到行收敛后的真实像素（预计 compact ≈ 176-184，comfortable ≈ 200-208，以实测为准）。

## 实施记录（2026-08-08 落地）

实际改动与最终取值：

- `centralSkillGrid.ts`：`CENTRAL_GRID_CARD_HEIGHT = { comfortable: 240, compact: 216 }`、`CENTRAL_LIST_CARD_HEIGHT = { comfortable: 196, compact: 168 }`；`centralVirtualItemHeight = ceil(base × fontScale)`（全站 rem 随根字号 `--font-scale` 线性缩放，行高与卡片内容天然同步）。
- `virtualized-grid.tsx`：行容器加 `gridTemplateRows: "100%"` —— 这是重叠的直接修复点：隐式行轨道默认 `auto` 会按卡片内容撑高，钉成 100% 后 gridcell 高度恒等于 `itemHeight`；gridcell 加 `min-h-0`。
- `UnifiedSkillCard.tsx`：标签行 `h-5` → `h-6`（配合 `CardTagEditor` 的 `+` 按钮 `size-8` → `size-6`，按钮不再被裁）。
- `SkillCardMeta.tsx`：`flex-wrap` → `flex-nowrap overflow-hidden`，meta 恒单行。
- `UnifiedSkillCardParts.tsx`（SkillCardSummary）：AI label 移入被 clamp 的 `<p>` 内部，去掉 `label && "inline"` —— 原先 `inline` 与 line-clamp 的 `display:-webkit-box` 层叠冲突，clamp 可能整段失效。
- `PlatformView.tsx`：`itemHeight` 196 → 204（行轨道钉死后，platform 卡内容上限 ≈ 200 必须被覆盖）。遗留：PlatformView 行高未接 fontScale，后续如需可单独对齐。
- 测试：`centralSkillGrid.test.ts` 期望值更新；新增 `src/test/components/ui/virtualized-grid.test.tsx`（行高 100% 钉死 + 固定 stride 两条回归防线）。
- 验证：`pnpm typecheck` / `pnpm lint` / 相关 Vitest（centralSkillGrid、skill 组件 132、pages 366、PlatformView 61）全绿；`just ci` 通过。
- `src/components/ui/virtualized-grid.tsx`：gridcell `h-full min-h-0`。
- `src/components/skill/UnifiedSkillCard.tsx`：shell 精确高度（替换 `min-h-[168px]`/`min-h-[188px]`），标签行高度与按钮对齐。
- `src/components/skill/SkillCardMeta.tsx`：单行 nowrap + overflow 处理。
- `src/components/skill/CardTagEditor.tsx`：`+` 按钮尺寸与标签行对齐。
- `src/components/central/CentralSkillListContent.tsx`：接线新常量。
- 测试：更新/补充 `src/test/components/skill/unifiedSkillCardVariants.test.tsx` 等断言；如已有 `centralSkillGrid` 相关快照需同步。

## 验证

- `pnpm typecheck && pnpm lint`
- 相关 Vitest 用例
- `pnpm tauri dev` 手工验证：Central 页 grid/list × compact/comfortable × fontScale 缩放，143+ 技能滚动无重叠
- 收尾 `just ci`
