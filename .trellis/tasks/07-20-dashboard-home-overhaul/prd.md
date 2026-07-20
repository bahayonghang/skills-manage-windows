# PRD：Dashboard 首页优化（排版重组 / 冗余删减 / 数据修复 / 图表补强）

## Goal

把 Dashboard 从「营销横幅 + 重复统计」重组为「状态 + 待办优先」的驾驶舱：
首屏即可看到并直达待办工作；每个指标只出现一次；Activity / Top tags
展示真实数据；消除玻璃拟态与死代码带来的性能浪费。

已确认决策（2026-07-20 用户拍板）：**删除 Hero 营销块，改紧凑状态头**，
首页定位驾驶舱。技术选型：图表纯 SVG 手绘不引依赖；聚合下推 Rust IPC。

本版已并入 2026-07-20 Codex 评审的 4 个 P1 阻断项与 3 个 P2/P3 修订项，
每条均已对照代码验证属实（验证证据见各需求括号内锚点）。

## 背景与证据（截图 + 代码走查）

参考截图（已存档）：`research/screenshot-01-first-screen.png`（首屏）、
`research/screenshot-02-scrolled.png`（滚动后）。

现状结构：`src/pages/DashboardView.tsx` → `dashboardBindings.ts`（订阅 6 个
store、约 30 个 selector）→ `dashboardViewModel.ts`（36 字段 viewModel）→
`DashboardShell.tsx` → 8 个区块（HeroSection / HealthOrbit / MetricStrip /
ProgressBreakdown / AgentsPanel / WorkQueuePanel / LogsPanel / ActivityPanel）。

### 排版问题

- L1 Hero 营销块吃掉首屏：标题 2.35rem~3.25rem（`src/index.css:246-248`，
  `HeroSection.tsx:52`），1367px 高窗口下 WorkQueue 完全在折叠线外。
- L2 同一批数字重复 3 处：HealthOrbit mini stat（`HealthOrbit.tsx:186-200`）、
  MetricStrip（`MetricStrip.tsx:76-115`）、ProgressBreakdown
  （`ProgressBreakdown.tsx:32-61`）与 WorkQueue 计数互相重复。
- L3 三处 xl 双列栅格比例不一致（`DashboardShell.tsx:31,65,87`：
  1.35/0.85、1.18/0.82、1.25/0.85），右列边界逐行漂移。
- L4 WorkQueue 的 All/Review/Metadata tab 最多滤 4 项，count>0 过滤后仅剩
  2 项（`WorkQueuePanel.tsx:50-54`），tab 无存在价值。
- L5 Activity 把同一组 14 个 bucket 渲染两遍：热图 + sparkline
  （`ActivityPanel.tsx:57-94`），信息密度低。

### 样式问题

- S1 全部卡片 `.surface-glass` = `backdrop-filter: blur(22px) saturate(140%)`
  （`src/index.css:1368-1380`，确认仅 dashboard 7 个组件使用），另有 hero
  glow `blur-3xl`（`HeroSection.tsx:37`）与 `.bg-orbit` 双层径向渐变
  （`src/index.css:1396-1413`）。
- S2 盒中盒：FactorRail / MiniStat 在玻璃卡内再叠 border + 1px ring 阴影
  （`HealthOrbit.tsx:62,79`）。
- S3 全页约 20 处 `uppercase tracking-wide` 微标签 + 每行 ENABLED 徽章，
  暗色主题下 muted-foreground 对比度偏低。
- S4 display 字体营销大标题 +「COMMUNITY SKILL COCKPIT」eyebrow 是落地页
  语言，与桌面工具语境错位。
- S5 侧栏「Central Ski... 113」文本截断（截图 1）——本任务明确排除，见非目标。

### 性能问题

- F1 大面积 backdrop-filter（8 张卡 × blur 22px + blur-3xl 光斑）是 Windows
  WebView2 滚动掉帧的主因，与本仓库 Windows 打包定位直接冲突。
- F2 viewModel 36 字段中 11 个无人消费：activeQueueItems / centralPath /
  hasLoadError / healthSummary / isPlatformLoading / isPlatformRefreshing /
  lastScanLabel / registriesCount / resolvedTarget / targetDescription /
  targetLabel（`dashboardViewModel.ts:299-335` 对比
  `DashboardShell.tsx:24-101`）。
- F3 派生数据未 memo：每轮渲染对 centralSkills 全量 filter × 3
  （`dashboardViewModel.ts:175-195`）；DashboardShell 无 memo，任一 store
  slice 变化导致 8 个区块全部重渲染。
- F4 双数据源：后端 bootstrap 已聚合 `dashboardCentralSummary`
  （`src-tauri/src/commands/bootstrap.rs:239-293`），前端又在 store 已加载时
  用全量 skills 重算同一批计数（`dashboardViewModel.ts:167-195`），并用
  `repositories.length` 覆盖 summary 的来源计数（同段 196-199）——同面板
  两种口径，显示结果依赖用户是否先访问过 Central 页。
- F5 小项：`formatTime` 每行新建 Intl.DateTimeFormat
  （`DashboardPanels.tsx:41-48`）。

### 功能缺陷（数据真实性）

- B1 Activity「last 14 days」数据失真：bootstrap 只取 5 条日志
  （`dashboardBindings.ts:166-171`，`RECENT_LOG_LIMIT=5`，
  `dashboardUtils.ts:3`），`buildActivitySummary` 用这 5 条填 14 天桶
  （`dashboardViewModel.ts:239`）。截图 2 佐证：「5 OPS」、14 格热图仅
  1 格有色、日期标签却写「Jul 07 - Jul 20」。后端现有日志 IPC 仅
  list/get/clear/export，无按天聚合。
- B2 Top tags 永远为空：Dashboard 不加载 centralSkillsStore，
  `buildTopTags([])` →「No Central tags yet.」（截图 2），实际 38/113
  技能有 tag。
- B3 `lastScanLabel`（上次扫描时间）已计算但未在任何区块渲染（见 F2）。
- B4 ProgressBreakdown 进度条语义错误：Uncategorized 75/113 的条形看起来像
  完成度，实际是待办积压量。

### 评审验证出的硬事实（修订依据）

- V1 `refreshCounts()` 只调 `get_skill_counts_summary` + `applyScanSummary`，
  后者仅更新 skillsByAgent/lastScanAt/scanState，**不碰**
  dashboardCentralSummary（`src/stores/platformStore.ts:294-310,381-387`）；
  `get_dashboard_central_summary_impl` 存在但未暴露为 command
  （`bootstrap.rs:239`，`lib.rs:355` 仅注册 get_skill_counts_summary）。
- V2 日志 `created_at` 为 UTC RFC3339（`operation_logs_repo.rs:126`
  `Utc::now().to_rfc3339()`），`date(created_at)` 只能得到 UTC 日期；
  本机无 sqlite3 CLI（`which sqlite3` 失败），对账不能依赖外部工具。
- V3 `skills.is_central` 存在（`db/types.rs:130`），central 限定惯例为
  `WHERE s.is_central = 1`（`skills_repo.rs:725-731`）；
  `skill_tag_links` 无主外键约束（`schema/metadata.rs:173-181`），
  孤儿 link 可能存在。
- V4 target 切换时 AppShell 仅 reset platform/central/skills/marketplace
  四个 store 并触发全局重扫（`AppShell.tsx:103-110`）；skills 表无
  target_id 列（`schema/core.rs:19-32`），其内容代表当前 active target，
  重扫后重建 —— target 切换后 topTags/summary 必须重载。
- V5 `buildActivitySummary` / `heatCellClass` / `ACTIVITY_DAY_COUNT` 被
  `src/components/logs/LogsActivityCard.tsx:7-62` 复用，**不能删**；
  `buildTopTags` / `TOP_TAG_LIMIT` / `buildSparklinePath` 仅 dashboard
  使用，可删。
- V6 `just ci` 依赖 `sync-version`（`justfile:9,19`），会执行
  `node scripts/sync-version.mjs`，可能写入版本文件。

## Requirements

- R1 首屏重组（L1/S4/B3）：删 Hero 营销块，新 StatusHeader = 扫描状态
  pill + 上次扫描时间 + 汇总句 + CTA 行（Browse / Marketplace /
  Quick migrate）；工作队列上移至首屏。
- R2 指标去重（L2/B4）：每个指标全页只出现一次；删除 MetricStrip 与
  ProgressBreakdown 组件；HealthOrbit 删 MiniStat ×3。
- R3 队列直读（L4）：WorkQueuePanel 删 tab，4 项固定平铺，0 值 muted
  显示而非隐藏；activeJob 进度保留。
- R4 数据契约修复（B1/B2/F4 + V1/V2/V3）：
  - R4a 暴露 `get_dashboard_central_summary` 为 tauri command（复用
    `bootstrap.rs:239` 的 impl），platformStore 新增
    `refreshDashboardSummary()`；调用点：Dashboard 挂载、scanGeneration
    变化、更新检查完成回调。不得再用 `refreshCounts()` 充当此用途（V1）。
  - R4b 新增 `get_daily_operation_counts(days)`：按**本地日历日**分桶，
    窗口 = 本地今天向前 days-1 天；后端补齐零值日，**恰好返回 days 个
    桶**；created_at 为 UTC RFC3339（V2），本地日换算在 SQL
    `date(created_at,'localtime')` 或 Rust chrono::Local 边界中实现，
    repo 函数注入时间源以便确定性测试。
  - R4c 新增 `get_central_top_tags(limit)`：必须 JOIN skills 并限定
    `s.is_central = 1`（V3），排除 `uncategorized`；孤儿 link 不得计入。
  - R4d Dashboard 计数统一只读 `dashboardCentralSummary`，退订
    centralSkillsStore 的 skills/aiTagReviews/updateStatuses/repositories
    （保留 aiTagJob/updateJob/error 订阅）。
- R5 store 生命周期与并发（V4）：
  - topTags 放 platformStore（随 target reset；挂载 + scanGeneration
    变化时重载）。
  - dailyCounts 放 operationLogStore，**有意保持跨 target 语义**（与
    Recent logs 面板一致，日志条目自带 target 标记）；挂载时加载，
    scanGeneration 变化时也随 summary/topTags 一并重载（扫描是最高频
    操作，重扫后活动图应反映最新流水；单次查询极小，成本可忽略）。
  - 两个加载器都要有 latest-wins 防护（模块级请求 token，旧响应丢弃，
    参照 platformStore 的 refreshToken 模式）。
  - 两个 IPC 独立发起、独立 error 态：单个失败时另一图表正常渲染，
    失败图表显示面板级错误占位 + 重试。
- R6 图表补强与可访问性（L5）：Activity 改单一 SVG 柱状图（14 天真实
  数据）；新 TopTagsPanel 横向条形 Top 6；AgentsPanel 数量列升级迷你
  条形。纯 SVG 不引图表库。可访问性：SVG 带 `role="img"` +
  `aria-label`（或 `<title>`/`<desc>`），每根柱有文本等价（`<title>`
  或 aria-label），「今日」高亮不得只靠颜色（描边/纹理 +
  `aria-current="date"`）。
- R7 样式减负（S1/S2/S3/L3）：`.surface-glass` 去 backdrop-filter（含
  latte/claude-light 变体）；删 hero glow；FactorRail 去盒中盒；行内徽章
  仅未启用平台显示；双列栅格统一为
  `xl:grid-cols-[minmax(0,1.25fr)_minmax(20rem,0.85fr)]`。
- R8 性能与完整清理链（F2/F3/F5 + 评审 #5）：viewModel 删除全部死字段
  （现状 11 个 + 删除区块后失去消费者的 resolvedCollectionCount、
  activity/sparkline、buildTopTags 版 topTags 等）；移除 collections /
  registries 的 preload 与全部相关订阅；删除 repositories 覆盖分支；
  删除 `buildTopTags`/`TOP_TAG_LIMIT`/`buildSparklinePath`（保留
  `buildActivitySummary`/`heatCellClass`/`ACTIVITY_DAY_COUNT`，V5）；
  保留派生数据 useMemo 使 props 稳定；React.memo 只按实际 profiling
  应用于热点 section，**不作固定验收项**。
- R9 i18n 与测试：全部新增/改动文案 en + zh 双语同步（AGENTS.md 约束）；
  重写 `src/test/DashboardView.test.tsx`；新增 Rust 查询单测。

## Acceptance Criteria

- [ ] AC1 首屏：1440×900 窗口、DPI 1.0、暗色、默认字号下，无需滚动可见
      全部 4 个工作队列项（含 0 值）。
- [ ] AC2 任一计数指标在页面上只出现一次；无 MetricStrip /
      ProgressBreakdown 残留（含 testid、i18n 键）。
- [ ] AC3 Activity 图表渲染后端返回的恰好 14 个桶（本地日，含零值日）；
      Rust 定向测试覆盖：UTC 已跨日但本地未跨日、本地已跨日但 UTC 未
      跨日、空表、零值日填充、窗口起止边界。（对账用 Rust 测试与定向
      查询完成，不依赖外部 sqlite3 CLI，V2。）
- [ ] AC4 Top tags 只统计 is_central=1 技能；Rust 测试含非 central 技能
      与孤儿 link 反例；前端空/非空两态渲染正确。
- [ ] AC5 首页展示上次扫描时间与扫描状态。
- [ ] AC6 更新检查完成后 dashboardCentralSummary 立即刷新（Vitest 验证
      updates 计数变化），且 Dashboard 挂载与 scanGeneration 变化时触发
      refreshDashboardSummary。
- [ ] AC7 target 切换（local → WSL/SSH）：topTags/summary 重置并按新
      target 重载；慢响应旧请求结果被丢弃（latest-wins）；单个图表 IPC
      失败时另一图表正常显示且失败方可重试。
- [ ] AC8 Dashboard 样式不再使用 backdrop-filter（`.surface-glass` 定义
      中移除，暗/亮两主题生效）。
- [ ] AC9 viewModel 无死字段；Dashboard 不再订阅 centralSkillsStore 的
      skills/aiTagReviews/updateStatuses/repositories，不再 preload
      collections/registries。
- [ ] AC10 图表 SVG 满足 R6 可访问性条款（role/aria/文本等价/今日高亮
      非纯颜色）。
- [ ] AC11 视觉验证矩阵截图存档至本任务 `research/`：1440×900 与
      1280×800 两种窗口 × 暗/亮主题 × 三档字号，关键区域（首屏、
      Activity、TopTags）齐全。
- [ ] AC12 `just ci` 全绿（含重写的 Dashboard Vitest 与新增 Rust 单测）；
      提交前 `git status` 核对，sync-version 产生的版本文件漂移不混入
      本任务提交（V6）。

## 非目标

- 不改路由结构与其他页面；不改后端 readiness 评分公式本身。
- 不引入 recharts / d3 等第三方图表库。
- 侧栏截断（S5）**明确排除**，不在本任务做任何修复；如需处理另开任务。
- `LogsActivityCard`（Logs 页）继续复用现有 `buildActivitySummary`
  前端聚合，其数据源改造不在本任务范围（V5）。
- 本阶段只产出分析与方案；实现待评审通过后 `task.py start`。
