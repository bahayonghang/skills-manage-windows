# Skill Usage 最大化卡顿诊断

日期：2026-08-16  
范围：静态代码对照。未在本机对最大化做 Performance 录制。

## 现象

- 页面：`/usage`（Skill Usage）
- 操作：窗口最大化
- 观感：卡一下
- 对照：其他页面同一操作不卡
- 截图窗口为未最大化；Top skills 表格铺满内容区，右侧无 Recent / Heatmap，判断当时视口未到 Tailwind `xl`（1280px）

## 排除

| 假设 | 结论 |
| --- | --- |
| 最大化触发 refresh / IPC | 否。`SkillUsageView` 与 usage 组件无 `resize` 监听 |
| 侧栏 `innerWidth` 切换 | 否。`Sidebar` 只在 mount 读一次，阈值 768px |
| 全局 `backdrop-filter` | 否。本页无 blur；有 blur 的 Central 侧栏不卡 |
| Tauri / WebView2 最大化本身 | 否。其他页同一操作不卡 |

## 主因链

1. 最大化使视口跨过 `xl`。
2. `SkillUsageView` 主网格从单栏变成 `xl:grid-cols-[minmax(0,1.55fr)_minmax(22rem,0.85fr)]`。
3. Top skills 卡片加上 `xl:row-span-3`，高度改由右栏块之和决定。
4. `ActivityHeatmap` 112 个 button 使用 `auto-cols-fr` + `aspect-square w-full`。列宽随容器变，格子高度跟宽度走。
5. 热力图卡片变高 → 右栏行高变 → 左栏 `row-span-3` 表格变高 → 表格内 50 行 subgrid 再排一次。

## 次因

- `SkillUsageTable` / `UnusedSkillsPanel` 每行独立 CSS Grid + 内层 `grid-cols-subgrid`，全量在 DOM。
- `usageStore` 传 `topSkillsLimit: 0`，后端 `limit == 0` 视为 `i64::MAX`。
- Unused 按 Central + 各平台小节展开，行数可超过 Top skills。
- 页面、表格、Unused、Recent 各自 `overflow-y-auto`。
- Central / Platform 用 `VirtualizedGrid`，最大化只重测视口。

## 锚点

- `src/pages/SkillUsageView.tsx`：`xl:grid-cols-...`、`xl:row-span-3`、`xl:col-span-2`
- `src/components/usage/ActivityHeatmap.tsx`：`auto-cols-fr`、`aspect-square w-full`、112 格
- `src/components/usage/SkillUsageTable.tsx`：行级 `grid` + `grid-cols-subgrid`
- `src/components/usage/UnusedSkillsPanel.tsx`：同上
- `src/stores/usageStore.ts`：`topSkillsLimit: 0`
- `src-tauri/src/db/repos/usage_repo.rs`：`limit == 0` → `i64::MAX`

## 建议顺序

1. 热力图固定格子边长，去掉 `aspect-square` + `1fr`。
2. 表格高度与热力图脱钩。
3. 给热力图 / 表格卡片加 `contain: layout`。
4. 若仍卡，另开任务虚拟化 Unused / Top skills。
