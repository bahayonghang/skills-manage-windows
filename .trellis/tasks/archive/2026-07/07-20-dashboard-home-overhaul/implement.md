# Implement：Dashboard 首页优化

按依赖序执行，每步独立可提交、可回滚。验收标准见 prd.md AC1-AC12；
契约细节见 design.md（R 编号与 prd.md 一致）。

## Step 1 — 后端三个 IPC（src-tauri）✅ 已完成

- [x] `get_dashboard_central_summary`：为 `bootstrap.rs:239` 的 impl 加
      `#[tauri::command]` 薄封装并在 `lib.rs` 注册（lib.rs:356）。
- [x] `get_daily_operation_counts(days)`：`db/repos/operation_logs_repo.rs`
      `list_daily_operation_counts(pool, today: NaiveDate, days)`（时间源
      注入），command 在 `commands/logs.rs`（lib.rs:378）。本地日历日分桶 +
      后端零值填充 + 恰好 days 个桶 + 升序；days clamp 1..=60。
- [x] `get_central_top_tags(limit)`：`db/repos/tags_repo.rs`，JOIN skills
      限定 `s.is_central = 1`、排除 uncategorized；command 在
      `commands/central_metadata.rs`（lib.rs:420）；limit clamp 1..=50。
- [x] Rust 单测 11 条：AC3 七条（空表零填充、UTC/本地双向跨日、窗口零
      填充、cutoff ±1s、days clamp）、AC4 四条（非 central+孤儿 link、
      uncategorized 排除、排序+共享 tag、limit+clamp）。
- [x] 验证：`cargo fmt --check` / `clippy --all-targets --locked -D warnings`
      零警告 / `cargo test --locked` 全量 897+ 通过（子代理执行）。

## Step 2 — 前端数据接入与生命周期（R4/R5）✅ 已完成

- [x] platformStore：`refreshDashboardSummary()`（只 set
      dashboardCentralSummary，静默后台刷新）+ topTags 状态与
      `loadTopTags(limit=6)`（模块级 token，latest-wins）；
      `resetForTargetChange` 清空 topTags 并使在途请求失效。
- [x] operationLogStore：dailyCounts 状态与 `loadDailyCounts(days)`
      （独立 token，latest-wins；跨 target 语义不随 target reset）。
- [x] 两个 IPC 分别 invoke，各自 `{ data, isLoading, error }`，面板级
      错误占位 + 重试（DashboardPanels.ChartStateRow）。
- [x] useDashboardBootstrap：挂载时三个加载各一次（ref 防重入）；
      scanGeneration 变化时三者全部重载（实现偏差说明：dailyCounts 也
      随 scanGeneration 重载，保持重扫后活动图新鲜，prd/design 已同步）。
- [x] centralSkillsStore.updateSlice：`checkSkillUpdates` 与 `updateSkills`
      成功收尾处 `invalidateDashboardSummary()`（禁止用 refreshCounts
      替代，已落实）。
- [x] `src/types`（DailyOperationCount / CentralTopTag）、commandMap 三个
      类型化条目、浏览器 fixture（platform.ts + operationLogs.ts）。

## Step 3 — 区块重组与完整清理 ✅ 已完成

- [x] 新 `StatusHeader`（扫描 pill + 上次扫描 + 汇总句 + 3 个 CTA，
      复用 dashboard.hero.cta* 键与既有 testid）。
- [x] `WorkQueuePanel`：删 tab，4 项横排平铺（grid-cols-2 xl:grid-cols-4，
      0 值 muted 显示）；activeJob 进度保留。
- [x] `HealthOrbit`：删 MiniStat ×3，FactorRail 去盒中盒（同步删
      `.readiness-rail` CSS）。
- [x] `AgentsPanel`：数量列迷你条形（scaleX，相对列表最大值）；徽章仅
      未启用时显示。
- [x] `ActivityPanel`：删热图/图例/sparkline，改 SVG 柱状图（role=img +
      aria-label + 每柱 `<title>`，今日描边 + aria-current="date"）。
- [x] 新 `TopTagsPanel`：复用 ProgressRow 横向条形 + 空态。
- [x] 删除 `HeroSection` / `MetricStrip` / `ProgressBreakdown`（含
      MetricStrip.test.tsx）；`DashboardShell` 按 5 行重排，双列栅格统一
      `xl:grid-cols-[minmax(0,1.25fr)_minmax(20rem,0.85fr)]`。
- [x] viewModel：删全部死字段（lastScanLabel 接入 StatusHeader）；
      删 centralSkills 重算分支与 repositories 覆盖分支；派生数据
      useMemo（queueItems/recentLogs/lastScanLabel/enabledTargetsCount）。
- [x] bindings/bootstrap：退订 centralSkillsStore 的
      skills/aiTagReviews/updateStatuses/repositories（保留 job/error/
      subscribe）；移除 collections/registries 的 preload 与全部相关
      订阅；loadError 收缩为 centralError ?? logsError。
- [x] dashboardUtils：删 `buildTopTags`（保留 TOP_TAG_LIMIT 供
      bindings/viewModel 复用）与 `buildSparklinePath`；保留
      `buildActivitySummary`/`heatCellClass`/`ACTIVITY_DAY_COUNT`
      （LogsActivityCard 复用）；DashboardPanels 删未使用的 QueueRow。
- [x] 验证：`pnpm typecheck && pnpm lint` 绿；grep 无残留引用。

## Step 4 — 样式 token ✅ 已完成

- [x] `src/index.css` `.surface-glass` 去 backdrop-filter（含 light 变体）；
      删除死 CSS `.surface-glass-strong`（无任何消费方）与
      `.dashboard-hero-glow`、`.readiness-rail`；注释同步更新。
- [x] 验证：见 Step 7 实测。

## Step 5 — i18n 同步 ✅ 已完成

- [x] en.json / zh.json 同步：删除 78 个无消费键（hero.title/description/
      eyebrow/ctaReview、metricStrip、sparkline、health、metrics、
      platforms、quickActions*、scanStateDetail、queue.tabs/queue.empty、
      activity.less/more/topTags/noTags、agents.enabled 及其它 mockup
      遗留）；新增 statusHeader.*/topTags.*/activity.chartAria/barTitle/
      empty/loading/chartError/retry。键结构 parity 校验通过。

## Step 6 — 测试重写 ✅ 已完成

- [x] 重写 `src/test/DashboardView.test.tsx`（7 用例：状态头/队列平铺含
      0 值/无 h1/挂载加载/导航/scanGeneration 重载/部分失败重试/空态）；
      断言按仓库惯例用双语正则。
- [x] `operationLogStore.test.ts` +3（加载/失败保留旧数据/latest-wins）。
- [x] `platformStore.test.ts` +3（refreshDashboardSummary 只动 summary/
      loadTopTags 失败可重试/reset 清空+在途丢弃，AC7）。
- [x] `centralSkillsStore.test.ts` +2（checkSkillUpdates/updateSkills
      完成后刷新 summary，AC6）。
- [x] 全量 `pnpm vitest run`：127 文件 / 1411 通过 / 1 skipped。
- [x] 附带修复：CollectionView/SettingsView 测试的 PlatformState 补新
      字段；`pnpm typecheck && pnpm lint` 绿。

## Step 7 — 门禁与实测 ✅ 已完成

- [x] Activity 对账：以 Step 1 的 Rust 定向测试为准（本机无 sqlite3
      CLI）；cutoff/零填充/跨日矩阵已在 repo 层锁定。
- [x] `just ci` 全绿（web: typecheck→lint→sizecheck→test→build；
      rust: entrypointcheck→fmt→clippy→897 tests）；`git status` 核对
      无 sync-version 版本文件漂移（AC12）。
- [x] 视觉验证矩阵（AC1/AC11）：vite dev + fixture 数据，agent-browser
      截取 1440×900 / 1280×800 × mocha(暗)/latte(亮) × 0.875/1/1.125
      三档字号共 12 张 + 图表区特写 1 张，存档 `research/shot-*`；
      逐张目检：首屏无滚动可见 4 个队列项（AC1 ✓），1280×800+1.125
      极端档下状态头换行、队列标签截断但四项齐全。
- [x] AC6：逻辑由 centralSkillsStore.test.ts 两条用例覆盖
      （checkSkillUpdates/updateSkills → get_dashboard_central_summary
      被调用且 summary 更新）；浏览器 fixture 态无法执行真实更新，
      人工实机确认留给用户。
- [x] spec 更新（Phase 3.3）：新增
      `.trellis/spec/frontend/dashboard-data-contract.md` 并登记进
      frontend index。

## 风险与回滚点

- Step 3 范围最大：testid/i18n 键/utils 删除前逐项 grep 引用面（已执行）。
- summary 新鲜度：三个调用点（挂载 / scanGeneration / 更新完成回调）
  缺一不可，AC6 单测 + 实测兜底。
- `.surface-glass` 为全局 CSS（虽仅 dashboard 使用），如需回滚单独
  revert Step 4 对应提交即可。
- 本地日契约：repo 函数注入时间源是确定性测试的前提（已落实）。

## Start 前检查

- [x] prd.md / design.md / implement.md 齐备（已并入评审修订）
- [x] implement.jsonl / check.jsonl 已配置真实条目（validate 通过：
      7 + 4 条）
- [x] 用户评审通过，`task.py start` 已执行（2026-07-20）
