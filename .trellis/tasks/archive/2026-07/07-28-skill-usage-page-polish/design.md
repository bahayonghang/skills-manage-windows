# Design — Skill Usage 页面体验优化

纯前端改动。后端 IPC 契约（`usage_refresh` / `usage_get_overview` / `usage_get_recent` /
`usage_get_skill_detail`）不动。遵循 `.trellis/spec/frontend/skill-usage-state.md`。

## 1. 扫描加载态（R1）

### 1.1 视图层（`SkillUsageView.tsx`）

- `initialLoading = overview === null && refreshing` 判定保留；
- **KPI 条纳入骨架**：`initialLoading` 时不渲染 `UsageMetricStrip`（当前会显示 4 个 0），
  改由 `UsageSkeleton` 内部补一块与 KPI 条同高的骨架条，保证 stable final-layout；
- `UsageSkeleton` 增强：
  - 顶部：KPI 条形态骨架（一行 4 个短块）；
  - 主体：保持现有两列灰块结构（与最终 grid 同形）；
  - 居中覆盖：`Loader2 animate-spin`（或 `RefreshCw`）+ 文案
    `t("skillUsage.scanning")`（zh：「正在扫描各平台会话日志…」，en：
    "Scanning session logs across platforms…"），置于骨架上层
    （`relative` 容器 + `absolute inset-0 flex items-center justify-center`），
    加 `role="status"` 供测试与读屏定位。

### 1.2 store 层（`usageStore.ts`）

`subscribeTargetChanged` 回调的 `set({...})` 中追加：

```ts
overview: null,
recent: [],
providers: [],
```

效果：target 切换 → 数据清空 → `refresh(true)` 期间 `overview === null && refreshing`
→ 复用同一骨架态。这修复了「重扫期间展示上一台机器数据」的 spec 违背项
（所有可见面板必须来自同一 target）。首次进入与 target 切换共用一条加载路径，无新增状态字段。

风险：`providers` 清空后 `PlatformFilterBar` 短暂无 pill 可渲染——可接受，
骨架期整个 main 区域都是骨架；filter bar 在 header 内，短暂只剩「全部平台」pill，
刷新完成即恢复（与首装首扫的表现一致）。

## 2. 排版与空白（R2）

### 2.1 根因

- 左卡 `xl:row-span-3` 的高度被右列内容（20 条 RecentCallsFeed，无内部滚动）撑开，
  而卡内 `SkillUsageTable` 固定 `h-[32rem]` → 卡片内下方整片空白；
- KPI 条 `grid grid-cols-4` 均分全宽 → 宽屏大段留白。

### 2.2 方案（保持现有 grid 骨架，最小改动）

- **`UsageSection` 支持 fill 模式**：加 `fill?: boolean` prop —— true 时
  `flex flex-col`，children 容器 `min-h-0 flex-1`。Top skills 卡启用 fill；
- **`SkillUsageTable`**：去掉调用方传入的固定 `h-[32rem]`，改为 `h-full`
  （组件根已是 `flex min-h-0 flex-col`，列表 `flex-1 overflow-y-auto` 已就绪），
  卡片被右列撑到多高，表格就填多高 → 空白消除；保留 `min-h-[24rem]` 兜底，
  防右列极短时表格塌缩；
- **`RecentCallsFeed`**：调用方包一层 `max-h-[26rem] overflow-y-auto`
  （`scrollbar-subtle`），20 条不再把整行拉到 1000px+；
  详情打开时的第二个 Recent 卡同样处理；
- 左卡 `min-h-[32rem] xl:row-span-3` 调整为 `xl:row-span-2`：默认态右列只有
  Recent + Heatmap 两张卡（详情态第三张 Recent 出现在第三行，跨行数不需要 3——
  详情态下第三行让左卡自然结束即可；若视觉上左卡在详情态偏短，保留 row-span-3 亦可，
  实现时以实际渲染取舍，两者都消除了空白根因）；
- **`UsageMetricStrip`**：`grid-cols-4` → `flex flex-wrap gap-x-10 gap-y-2`
  （数字聚拢左侧，去掉 divide 均分），删除条内脚注
  `<p>…allRecorded…</p>`（页头副标题已有同文案）；分隔改用相邻 border-l + padding；
- 「固定范围可见」契约：页头副标题（全部历史）、Recent 卡头（最近 20）、
  Heatmap 卡头（16 周）保留 —— Top skills 卡头的 range 文案与页头重复，删除。

## 3. 安装状态筛选（R3）

### 3.1 状态与数据流

- `SkillUsageTable` 内部新增本地 state：
  `type MatchFilter = "all" | "installed" | "unlinked"`，默认 `"all"`；
- 过滤谓词：`installed → matchStatus === "matched"`；
  `unlinked → matchStatus !== "matched"`（ambiguous + unmatched 都无 resolvedSkillId，
  统称「未关联」，避免把 ambiguous 误标成「未安装」）；
- 过滤在 `useMemo` 中于排序前应用：`sortSkills(filtered, sortMode)`；
- 不进 store、不发请求 —— 符合 spec「store 持有字段清单」不扩张；平台维度筛选
  （selectedSource）仍走 store，两者正交。

### 3.2 UI

- 表头控件行（现有 sort segmented 左侧）加同款 segmented group：
  `全部 / 已安装 / 未关联`，复用 sort 按钮的样式类与 `aria-pressed` 模式；
- 计数行：filter 激活时显示 `t("skillUsage.rankingFiltered", { count, total })`
  （「{{count}} / {{total}} skills」），全部时保持现有 `rankingCount`；
- 过滤后空集：复用现有空态排版，文案
  `t("skillUsage.empty.filteredTitle")` + `t("skillUsage.empty.filteredHint")`
  （提示切回「全部」）。注意与「无任何数据」空态区分：
  `skills.length === 0` 走原空态；`skills.length > 0 && filtered.length === 0` 走筛选空态。

## 4. 匹配状态可视化（R4）

- `MatchStatus` 子组件（`SkillUsageTable.tsx`）：文本前加状态点
  `<span class="size-1.5 rounded-full">`：
  - matched → `statusFillClass.success`
  - ambiguous → `statusFillClass.warning`
  - unmatched → `bg-muted-foreground/40`（中性，无语义 token，不进 statusTone）
- 文本色保持 `text-muted-foreground`（低噪声，点已足够扫读）；
- `SkillUsageDetailPanel` 头部的 matchStatus 副标题复用同一子组件（从 table 导出或
  提为 `components/usage/UsageMatchDot.tsx` 共享小组件，二选一以实现简洁为准）。

## 5. i18n 新增键

`skillUsage.*`（en / zh 同步）：

| key | en | zh |
| --- | --- | --- |
| `scanning` | Scanning session logs across platforms… | 正在扫描各平台会话日志… |
| `matchFilter.label` | Filter by install state | 按安装状态筛选 |
| `matchFilter.all` | All | 全部 |
| `matchFilter.installed` | Installed | 已安装 |
| `matchFilter.unlinked` | Unlinked | 未关联 |
| `rankingFiltered` | {{count} } / {{total}} skills | {{count}} / {{total}} 个技能 |
| `empty.filteredTitle` | No skills match this filter | 没有符合筛选条件的技能 |
| `empty.filteredHint` | Switch back to “All” to see every recorded skill. | 切回「全部」可查看所有已记录技能。 |

（`rankingFiltered` 实际写法无空格，表格内为转义展示。）

## 6. 兼容与回滚

- 全部为渲染层与视图本地状态改动 + 一处 store reset 扩展；无 schema/IPC 变更；
- 回滚 = revert 前端 commit 即可，无数据迁移；
- 现有测试影响面：`SkillUsage.components.test.tsx`（布局/文案断言）、
  `usageStore.test.ts`（target-changed 重置断言需补三字段）。

## 7. 涉及文件

| 文件 | 变更 |
| --- | --- |
| `src/pages/SkillUsageView.tsx` | 骨架增强、KPI 条条件渲染、grid/row-span 与 Recent 包裹、range 文案去重 |
| `src/components/usage/UsageMetricStrip.tsx` | 紧凑排布、删脚注 |
| `src/components/usage/SkillUsageTable.tsx` | h-full 填充、match filter、计数行、筛选空态、状态点 |
| `src/components/usage/SkillUsageDetailPanel.tsx` | 匹配状态点复用 |
| `src/stores/usageStore.ts` | target-changed 清 overview/recent/providers |
| `src/i18n/locales/en.json` / `zh.json` | 新增键 |
| `src/test/components/usage/SkillUsage.components.test.tsx` | 新增筛选/骨架/空态用例，修正受影响断言 |
| `src/test/stores/usageStore.test.ts` | target-changed 重置断言补字段 |
