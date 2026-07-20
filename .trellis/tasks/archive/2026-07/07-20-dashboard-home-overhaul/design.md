# Design：Dashboard 首页优化

Q1 已确认：删除 Hero 营销块，首页定位「状态 + 待办优先」的驾驶舱。
本版已并入评审修订（PRD 的 V1-V6 硬事实为设计依据，R 编号与 prd.md
最终版一致）。

## 目标信息架构

自上而下 5 行（`DashboardShell.tsx` 重排）：

1. **StatusHeader（新组件）** — 一行紧凑状态条，替代 HeroSection：
   - 左：扫描状态 pill（复用 `HeroSection.tsx:26-31` 的 pillTone 逻辑）+
     「上次扫描 {lastScanLabel}」（激活原死字段）+ 汇总句
     （中央库 N 技能 · M 来源 · K 平台已启用，全部取自 summary）。
   - 右：CTA —— Browse Central（primary）/ Marketplace（outline）/
     Quick migrate（ghost，`title=quickMigrateDescription`）。
   - 删除：eyebrow、营销 h1、描述段、Review CTA（由工作队列承接）、
     hero glow 光斑。
2. **工作队列（改造 WorkQueuePanel）** — 上移至第二行：
   - 删除 All/Review/Metadata tab；4 个队列项固定平铺为横排卡
     （`grid-cols-2 xl:grid-cols-4`），count=0 以 muted 样式显示 0 而不再
     隐藏。
   - activeJob 进度条保留在卡片底部，逻辑不变。
3. **Readiness（瘦身 HealthOrbit）+ 平台覆盖（改造 AgentsPanel）**：
   - HealthOrbit：删除 3 个 MiniStat；FactorRail 去掉内层 border + ring
     阴影，只留 label / 百分比 / 轨道条。
   - AgentsPanel：数量列升级为「迷你条形 + 数字」（相对列表最大值的
     scaleX 条，复用 `DashboardPanels.tsx:143-147` 的 transform 方案）；
     行内徽章仅在平台**未启用**时显示。
4. **Activity（单图表）+ TopTagsPanel（新组件）**：
   - ActivityPanel：删除 14 格热图与 less/more 图例，改为 SVG 柱状图
     （14 根 rect，每根带 `<title>` 文本等价，「今日」描边高亮 +
     `aria-current="date"`，不只靠颜色）；总数与日期范围保留。
   - TopTagsPanel：横向条形 Top 6（名称 truncate + scaleX 条 + 计数），
     空态沿用现有 `dashboard.activity.noTags` 文案。
   - 两个 SVG 均 `role="img"` + `aria-label` 概述。
5. **Recent logs** — 保留 5 条预览，整行单卡；`formatTime` 的
   Intl.DateTimeFormat 提为模块级常量。

栅格统一：所有双列行统一
`xl:grid-cols-[minmax(0,1.25fr)_minmax(20rem,0.85fr)]`。

## 数据流与契约（R4/R5）

### IPC 1：`get_dashboard_central_summary`（修 V1）

- 后端：`bootstrap.rs:239` 的 `get_dashboard_central_summary_impl` 已存在，
  只需加 `#[tauri::command]` 薄封装并在 `lib.rs` 注册（仿
  `get_skill_counts_summary`，`bootstrap.rs:206-211`）。
- 前端：platformStore 新增 `refreshDashboardSummary()`，只 set
  `dashboardCentralSummary` 一个字段。
- 调用点（三处，缺一不可）：
  1. Dashboard 挂载（useDashboardBootstrap 现有 ref 防重入模式）；
  2. `scanGeneration` 变化（重扫完成后 summary 必然过期）；
  3. 更新检查完成回调（centralSkillsStore 的 update 工作流收尾处
     fire-and-forget 调一次）——这是「检查更新后 updates 计数立即刷新」
     （AC6）的保障；**禁止**用 `refreshCounts()` 充当此用途（V1 实证其
     不更新 summary）。
- 注意：Dashboard 挂载刷新依赖路由切换会重建 DashboardView（React
  Router 行为），从 Central 页检查完更新返回时自然刷新；第 3 个调用点
  覆盖「不离开当前页」的场景。

### IPC 2：`get_daily_operation_counts(days)`（修 V2）

- 语义契约：**本地日历日**分桶；窗口 = 本地今天 00:00 起向前 days-1 天；
  后端用 Rust 侧时间源生成完整天数序列并**零值填充**，恰好返回 days 个
  桶（前端不再补桶）。
- 实现：`created_at` 为 UTC RFC3339（`operation_logs_repo.rs:126`）。
  repo 函数签名注入时间源（如 `today: NaiveDate` 或 `now: DateTime<Local>`
  参数）以便确定性测试；command 封装用 `chrono::Local` 取本机今天。
  分组用 `date(created_at, 'localtime')`，cutoff 由注入的本地日换算为
  UTC 边界后作为 `WHERE created_at >= :cutoff_utc` 参数传入。
- DST 说明：按本地日历日切分，DST 当天 23/25 小时仍计为一天，属于预期
  语义，写入测试注释。
- 测试用例（AC3）：UTC 已跨日但本地未跨日、本地已跨日但 UTC 未跨日、
  空表、全零窗口零填充、窗口起止边界（cutoff 前 1 秒不计入）。

### IPC 3：`get_central_top_tags(limit)`（修 V3）

- SQL 形态（对齐 `skills_repo.rs:725-731` 的 central 限定惯例）：
  `SELECT t.id, t.name, COUNT(*) AS count FROM skill_tag_links l
   JOIN skills s ON s.id = l.skill_id AND s.is_central = 1
   JOIN skill_tags t ON t.id = l.tag_id
   WHERE l.tag_id != 'uncategorized'
   GROUP BY t.id, t.name ORDER BY count DESC, t.name ASC LIMIT ?`
- JOIN skills 同时解决孤儿 link（`skill_tag_links` 无 FK，
  `schema/metadata.rs:173-181`）与非 central 技能计入两个问题。
- 测试用例（AC4）：非 central 技能的 link 不计入、孤儿 link（skill_id
  在 skills 表不存在）不计入、uncategorized 排除、limit 截断、
  count 并列时按 name 排序。

### store 放置与生命周期（R5，修 V4）

- **topTags → platformStore**：skills 表无 target_id 列，内容代表当前
  active target（V4），AppShell 在 target 切换时 reset platformStore 并
  触发全局重扫（`AppShell.tsx:103-110`）→ topTags 随
  `resetForTargetChange` 清空，重扫后 `scanGeneration` 递增触发重载。
- **dailyCounts → operationLogStore**：有意保持**跨 target 语义**——
  operationLogStore 本来就不随 target 切换 reset，日志条目自带
  target_kind/target_id，Activity 展示「本工具全部操作流水」与 Recent
  logs 面板口径一致。挂载时加载；scanGeneration 变化时与 summary /
  topTags 一并重载（重扫是最高频操作，活动图应保持新鲜；单次聚合
  查询极小）。
- **latest-wins**：两个加载器各用模块级请求 token（自增，仅最新一次
  响应允许写入），仿 platformStore `refreshToken` 模式
  （`platformStore.ts:46-47,224-267`）。
- **部分失败**：两个 IPC 分别 invoke（不用 Promise.all），各自维护
  `{ data, isLoading, error }`；失败图表渲染面板级错误占位 + 重试按钮，
  另一图表不受影响（AC7）。
- **计数口径统一**：viewModel 删除 centralSkills 全量重算分支
  （`dashboardViewModel.ts:167-195`）与 repositories 覆盖分支
  （同段 196-199）；Dashboard 退订 centralSkillsStore 的
  skills/aiTagReviews/updateStatuses/repositories，保留
  aiTagJob/updateJob/error（activeJob 与 loadError 需要）。

### 订阅与加载移除清单（R8，评审 #5）

- bootstrap 移除：loadCollections、loadRegistries 两个 effect 及其全部
  bindings（collections/isCollectionsLoading/collectionsError、
  registries/isMarketplaceLoading/marketplaceError、
  loadCollections/loadRegistries）——MetricStrip 删除后它们失去唯一
  消费者；loadError 聚合相应收缩为 centralError ?? logsError。
  （行为变化：Marketplace 首访自行加载 registries，该页本就有自己的
  加载路径，可接受。）
- viewModel 移除：原 11 个死字段（activeQueueItems/centralPath/
  hasLoadError/healthSummary/isPlatformLoading/isPlatformRefreshing/
  registriesCount/resolvedTarget/targetDescription/targetLabel；
  lastScanLabel 保留接入 StatusHeader）+ 删除区块后失去消费者的
  resolvedCollectionCount、activity、sparkline、buildTopTags 版 topTags。
- dashboardUtils 删除：`buildTopTags`、`TOP_TAG_LIMIT`、
  `buildSparklinePath`；**保留** `buildActivitySummary`、`heatCellClass`、
  `ACTIVITY_DAY_COUNT`（LogsActivityCard 复用，V5）。
- React.memo：先用 useMemo 稳定 props；memo 仅按 profiling 应用于热点
  section，不作固定验收项。

## 样式改造（R7）

- `.surface-glass`（`src/index.css:1368-1380`）确认仅 dashboard 7 个组件
  使用 → 直接改定义：删除 `backdrop-filter`（含 -webkit 前缀），保留细
  边框 + 浅渐变表面 + 轻阴影；latte/claude-light 变体
  （`src/index.css:1423-1451`）同步。单独 commit，回滚 = revert。
- 删除 `.dashboard-hero-glow`（`src/index.css:1415-1421`）与 HeroSection；
  删除 MetricStrip / ProgressBreakdown 组件及其 testid。
- 微标签降噪：uppercase eyebrow 仅保留 readiness 卡一处；行内 ENABLED
  徽章仅未启用时显示。

## i18n（R9）

- en.json + zh.json 同步增删：删 `dashboard.hero.title/description/eyebrow`、
  `dashboard.metricStrip.*`、`dashboard.health.*`、`dashboard.metrics.*`、
  `dashboard.queue.tabs.*`、`dashboard.sparkline.*`、`dashboard.activity.less/more`；
  新增 `dashboard.statusHeader.*`、`dashboard.topTags.*`、Activity 新键。
- 删键前全局 grep 确认无其他页面引用。

## 兼容与回滚

- 纯增量 IPC + 前端重组，无 DB migration；前后端同包发布，无版本错位。
- 提交按「后端 IPC / 前端重组 / 样式 token」分开，支持分段 revert。
- `just ci` 依赖 sync-version（V6），提交前 `git status` 核对版本文件
  漂移不混入。

## 测试设计

- Rust（参照 `bootstrap.rs:411+` readiness 测试风格）：
  `get_daily_operation_counts` 的 AC3 用例矩阵；`get_central_top_tags`
  的 AC4 用例矩阵；`get_dashboard_central_summary` command 封装冒烟
  （impl 已有测试）。
- Vitest：重写 `src/test/DashboardView.test.tsx`（现 523 行 / 5 用例）——
  状态头渲染（含 lastScan）、队列 4 项含 0 值平铺、无 tab、Activity 以
  mock dailyCounts 渲染 14 柱、TopTags 空/非空、无 hero 文案；新增
  AC6（更新完成 → summary 刷新）与 AC7（target 切换重载、latest-wins、
  部分失败）用例；保留「导航不触发 registry sync」类断言。
- 视觉验证（AC11）：1440×900 与 1280×800 × 暗/亮 × 三档字号的
  Tauri 截图，存档至本任务 `research/`。
- 最终门禁：`just ci` 全绿（`.trellis/spec/quality/ci-quality-gate.md`）。
