# 技术设计：Skill Usage 最大化布局成本

## 边界

- 仅前端：`SkillUsageView`、`ActivityHeatmap`、相关组件测试。
- 不改 `usageStore`、IPC、Rust、i18n（无新文案）。
- 组件仍不直接 `invoke`。

## 方案

### 1. 热力图：固定格子，宽度不再驱动高度

现状（贵）：

```
grid flex-1 auto-cols-fr grid-flow-col grid-rows-7
  button.aspect-square.w-full  × 112
```

改为：

- 外层保留 `overflow-x-auto` 与 `min-w-[22rem]`。
- 格子网格改为 **固定轨道**：`grid-rows-7 grid-flow-col` + 固定列宽（例如 `grid-auto-columns: 0.75rem` 或 Tailwind `auto-cols-[0.75rem]`），格子用 `size-3` 一类固定边长，**删除** `aspect-square` 与 `w-full`。
- 月份行与格子列宽对齐：月份容器用同一列宽（`grid-cols-16` 改为与格子相同的 `auto-cols-[0.75rem]` + `grid-flow-col`），避免标签与格子错位。
- `compact` 模式（详情面板）可用更小的固定边长，规则仍是固定像素，不是 `1fr`。

边长选 Tailwind 间距档（`0.625rem` / `0.75rem`），与现有 `gap-1` 一起在 `min-w-[22rem]` 内放得下 16 列。

不采用 canvas / 单 SVG：会丢掉 112 `gridcell` 与方向键契约。

### 2. 双栏：表格高度独立

现状：`xl:row-span-3` + 表格 `fill` 使左栏高度 = 右栏 Recent + Heatmap（+ 可能的第三块）之和。热力图一变高，表格整列重排。

改为：

- 去掉 Top skills 的 `xl:row-span-3`。
- 主网格在 `xl` 仍两栏，但改为 **显式两行**：
  - 行 1：Top skills | Recent（或 Detail）
  - 行 2：Heatmap（右栏）或与 Recent 同列向下堆
  - Unused 仍 `xl:col-span-2`
- Top skills 卡片自己定高：保留 `min-h-[32rem]` + `fill` + 表体 `overflow-y-auto`。高度不再引用热力图。
- Recent 继续 `max-h-[26rem]` 内部滚动，避免再把行撑开。

选中技能后的 Detail / Recent 换位保持现有条件渲染，只是不再靠 `row-span-3` 去填满左栏。

视觉：宽屏下左栏表格可能比右栏堆叠更高或更矮，中间允许不对齐。这比高度耦合更稳，也符合 D3（卡片内无空白）。

### 3. Containment

给热力图 `UsageSection` 与 Top skills `UsageSection` 加 `contain-layout`（Tailwind `contain-layout`）。不要对 Unused 整表加 `contain: strict`，避免裁切 unlink 弹窗或 `::after` 热区。

## 数据流

无。resize 不进 store。无新 IPC。

## 测试

| 文件 | 变更 |
| --- | --- |
| `src/test/components/usage/SkillUsage.components.test.tsx` | 保留 112 格 / 键盘 / `data-level`；新增：两档容器宽度下格子 `getBoundingClientRect()` 宽高相等，且 class 不含 `aspect-square` |
| `src/test/pages/` 若有 usage 布局用例 | 断言 Top skills section 无 `xl:row-span-3`；或断言其高度不随 heatmap 容器宽度变化 |
| Unused / store 测试 | 预期零改动 |

jsdom 对 `getBoundingClientRect` 常返回 0。若量不到真实像素，改断言：

1. 格子 class 有固定 `size-*` / `h-* w-*`，没有 `aspect-square`、`w-full`、`auto-cols-fr`；
2. 给格子设 `style.width/height` 的测试辅助，或读 computed `grid-auto-columns`。

优先 class / 属性断言，不依赖 jsdom 排版。

## 权衡

| 选项 | 结果 | 决定 |
| --- | --- | --- |
| canvas 热力图 | 绘制便宜，键盘与单测重写 | 否 |
| 虚拟化 Unused | 降低次因，改动面大 | 本期不做 |
| 取消 `xl` 双栏 | 最大化不再换拓扑，但回退信息架构 | 否 |
| 固定格子 + 拆 `row-span-3` | 切断高度回传，契约可保留 | 采用 |

## 回滚

单 commit 可 `git revert`。无迁移、无数据格式变化。
