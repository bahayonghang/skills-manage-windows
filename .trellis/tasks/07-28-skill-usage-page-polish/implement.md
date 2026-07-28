# Implement — Skill Usage 页面体验优化

执行顺序自上而下；每步后跑对应校验。全程只动 design.md §7 列出的文件。

## Step 1 store：target 切换清空页面数据

- [x] `src/stores/usageStore.ts` `subscribeTargetChanged` 的 `set({...})` 追加
      `overview: null, recent: [], providers: []`
- [x] `src/test/stores/usageStore.test.ts`：找到 target-changed 相关用例，断言三字段被清空
      （若无现成用例则新增一条：预置 overview → 触发 `usage://target-changed` → 三字段清空
      且 `refresh` 以 force=true 被调用）

## Step 2 i18n：新增键

- [x] `src/i18n/locales/en.json` + `zh.json` 的 `skillUsage` 下新增
      `scanning` / `matchFilter.{label,all,installed,unlinked}` / `rankingFiltered` /
      `empty.filteredTitle` / `empty.filteredHint`（文案见 design.md §5）
- [x] 两文件键结构保持一致（zh 全部中文化）

## Step 3 骨架与扫描态

- [x] `SkillUsageView.tsx`：
  - `initialLoading` 时不渲染 `UsageMetricStrip`，由 `UsageSkeleton` 顶部补 KPI 形态骨架条
  - `UsageSkeleton` 加 `relative` 容器 + 居中 `role="status"` 层：spin 图标 +
    `t("skillUsage.scanning")`
- [x] 组件测试：mock 初始态（overview=null, refreshing=true）断言
      `role="status"` + 扫描文案出现、KPI 数字 0 不出现

## Step 4 布局空白修复

- [x] `UsageSection` 加 `fill` prop（`flex flex-col` + children 包 `min-h-0 flex-1`），
      Top skills 卡启用
- [x] Top skills 卡：`min-h-[32rem] xl:row-span-3` → 按 design.md §2.2 调整 row-span；
      `SkillUsageTable` 的 `className` 从 `h-[32rem]` 改为 `h-full min-h-[24rem]`
- [x] 两处 `RecentCallsFeed` 外包 `max-h-[26rem] overflow-y-auto scrollbar-subtle` 容器
- [x] `UsageMetricStrip`：`grid-cols-4` → flex 聚拢排布；删除脚注 `<p>`；
      Top skills 卡头 range 文案删除（页头已有）
- [x] 手工核对（浏览器 fixtures 或 tauri dev）：1280 与 1920 宽下无成片空白、无横向滚动

## Step 5 安装状态筛选 + 状态点

- [x] `SkillUsageTable.tsx`：
  - `MatchFilter` 本地 state + `useMemo` 过滤（排序前）
  - 表头 segmented（复用 sort 按钮样式 + `aria-pressed`，group `aria-label` 用
    `matchFilter.label`）
  - 计数行：过滤激活时 `rankingFiltered`，否则 `rankingCount`
  - 筛选空态：`skills.length > 0 && filtered.length === 0` 时展示
    `filteredTitle/filteredHint`；完全无数据仍走原空态
  - `MatchStatus` 加状态点：matched=`statusFillClass.success`、
    ambiguous=`statusFillClass.warning`、unmatched=`bg-muted-foreground/40`
- [x] `SkillUsageDetailPanel.tsx`：匹配状态副标题复用状态点展示
- [x] 组件测试：
  - 三种 matchStatus 数据下：installed 只剩 matched 行；unlinked 只剩其余行；
    all 恢复全量
  - 计数行文案随过滤变化
  - 筛选空态出现（如全 unmatched 数据 + installed 过滤）
  - 既有断言（排序、键盘、open-skill 按钮仅 resolvedSkillId 行）保持通过

## Step 6 全量校验

- [x] `pnpm typecheck`
- [x] `pnpm lint`
- [x] `pnpm test`（全量；重点 `src/test/components/usage/` 与 `src/test/stores/usageStore.test.ts`）

## 校验命令

```bash
pnpm typecheck
pnpm lint
pnpm test -- src/test/components/usage/SkillUsage.components.test.tsx
pnpm test -- src/test/stores/usageStore.test.ts
pnpm test
```

## 回滚点

- 每个 Step 独立可 revert；整任务回滚 = revert 单个前端 commit，无迁移、无后端耦合。

## Review gates

- Step 4 后：确认「全部历史 / 最近 20 / 16 周」范围标签仍可见（spec 契约）
- Step 5 后：确认未新建场景专用卡片组件、statusTone 之外无原生调色板类名
- Step 6 后：trellis-check 全量复核
