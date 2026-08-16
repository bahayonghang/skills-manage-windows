# Skill Usage 最大化卡顿：降低 resize 布局成本

## Goal

窗口最大化时，`/usage` 不再出现其他页面没有的一次性卡顿。页面在跨 `xl`（1280px）前后保持现有信息架构与可访问性，只降低同步布局成本。

## Background / Confirmed Facts

2026-08-16 用户在未最大化窗口下打开 Skill Usage（表格铺满内容区，右侧无 Recent / Heatmap），最大化时卡一下；Dashboard / Central / Platform 无此现象。

代码对照结论（详见 `research/resize-jank-diagnosis.md`）：

- 本页没有 `resize` / `ResizeObserver`，最大化不触发 React 重渲染或重新拉数。卡顿是浏览器同步 style → layout → paint。
- 截图布局是单栏，说明当时视口未到 `xl`。最大化到常见 1920 / 2560 会跨过 1280px。
- 跨 `xl` 时：`SkillUsageView` 从单栏变成 `1.55fr + 0.85fr`，表格 `xl:row-span-3`，Unused `xl:col-span-2`。
- `ActivityHeatmap` 用 `auto-cols-fr` + 112 个 `aspect-square w-full` button。格子高度随宽度变，再撑开右栏行高，再撑开左栏 `row-span-3` 表格，形成多轮 layout。
- Top skills 与 Unused 每行是独立 CSS Grid + 内层 `grid-cols-subgrid`，全量在 DOM，无虚拟化。`topSkillsLimit: 0` 不截断。
- Central / Platform 用 `VirtualizedGrid`，最大化只重测视口。Logs 热力只有 14 格，且不参与跨栏高度耦合。

## Requirements

- R1 热力图尺寸与宽度解耦：`ActivityHeatmap` 的格子宽高不得再由 `1fr` 列宽 + `aspect-square` 推导。格子在容器变宽时保持固定边长；过窄时横向滚动（已有 `overflow-x-auto`），不得把高度回传给外层网格。
- R2 拆开双栏高度耦合：`xl` 下 Top skills 卡片高度不得由右栏（Recent + Heatmap）之和决定。表格继续填满自己的卡片并内部滚动，不得回归 07-28 的卡片内空白。
- R3 隔离重排：热力图卡片与 Top skills 卡片使用 layout containment，避免一次轨道重算打穿整页。
- R4 契约保持：热力图仍是 16×7=112 个可聚焦 `gridcell`，分位数着色、月份标签、图例、方向键漫游不变。Unused unlink 弹窗、匹配筛选、排序、i18n、store/IPC 不变。
- R5 测试：现有 heatmap / usage 组件测试保持绿；新增「容器变宽时格子盒尺寸不变」的回归。

## Acceptance Criteria

- [ ] AC1：`ActivityHeatmap` 不再使用 `aspect-square` 与 `auto-cols-fr` 的组合。同一组 112 天数据在窄容器与宽容器下，格子的计算宽高相等。
- [ ] AC2：`xl` 双栏下，改变热力图卡片宽度不改变 Top skills 卡片高度（表格高度与热力图脱钩）。
- [ ] AC3：112 个 `gridcell`、单一 `tabIndex=0`、方向键漫游、`data-level`、图例与空态行为与现网一致。
- [ ] AC4：Unused 行右 unlink、匹配筛选、排序、打开技能、详情关闭回焦均保持现有测试绿。
- [ ] AC5：中英文案无新增硬编码；无新用户可见文案则 i18n 可不动。
- [ ] AC6：`pnpm typecheck`、`pnpm lint`、`pnpm test -- --run src/test/components/usage` 全绿；收尾跑 `just ci`。
- [ ] AC7：本机在 `/usage` 从低于 1280px 最大化到全屏，卡顿相对改前明显减轻；同操作下 Dashboard / Central 无回归。此项为手工验收，CI 不测窗口最大化。

## Out of Scope

- 不改 Rust / IPC / `usageStore` 刷新与 unused 报告契约。
- 不把热力图改成 canvas / 单张 SVG（会丢掉 112 格键盘契约）。
- 本期不虚拟化 Top skills / Unused。若 AC7 仍不足，另开任务。
- 不改 `xl` 断点本身，不取消双栏信息架构。
- 不改平台筛选条、KPI 条、Provider 折叠区。

## Key Decisions

- D1：优先拆热力图尺寸与 `row-span-3` 高度链，不先上虚拟化。50 行 subgrid 是次因；主因是 112 格 `aspect-square` 回传高度。
- D2：热力图保留 112 个 button `gridcell`，只改轨道与盒子尺寸模型。
- D3：表格继续 `fill` + 内部滚动，避免 07-28 卡片内空白回归。
- D4：任务复杂度按「多组件联动 + 布局契约」处理，需要 `design.md` 与 `implement.md`。

## Risks

- 固定格子边长后，宽屏热力图两侧会留空。可接受；用已有横向滚动处理窄屏。
- 去掉 `row-span-3` 后若未给表格独立高度，会回到卡片内空白。设计里必须写明替代高度来源。
- 本机最大化无法进 CI。AC1/AC2 用组件级尺寸断言锁结构，AC7 手工确认观感。
