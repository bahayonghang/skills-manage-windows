# Dashboard 数据契约

> 建立于 2026-07-20（任务 07-20-dashboard-home-overhaul）。背景：Dashboard 曾并存
> 「后端 bootstrap summary」与「前端全量 centralSkills 重算」两套计数口径，显示
> 结果依赖用户导航历史；Activity「近 14 天」实际只有 5 条日志；viewModel 36 字段
> 中 11 个无人消费。本契约冻结单一数据源与刷新触发点。

## 契约 1：计数唯一来源 = `dashboardCentralSummary`

- Dashboard 的全部计数（central 总数、updates、aiReview、uncategorized、
  unassigned、readiness、sources）只读 platformStore 的
  `dashboardCentralSummary`（后端 `get_dashboard_central_summary_impl` 一次
  SQL 聚合）。**禁止**在前端用全量 `centralSkills`/`repositories` 重算同批
  计数或覆盖 summary 字段。
- Dashboard 不订阅 centralSkillsStore 的 skills/aiTagReviews/updateStatuses/
  repositories；activeJob 进度与 error 除外。

## 契约 2：summary 的三个刷新触发点（缺一不可）

1. Dashboard 挂载（bootstrap ref 防重入）；
2. `scanGeneration` 变化（重扫完成 = 数据必然过期）；
3. 更新类操作完成回调（`centralSkillsStore.updateSlice` 的
   `checkSkillUpdates` / `updateSkills` 成功收尾调
   `refreshDashboardSummary()`）。

禁止用 `refreshCounts()` 充当 summary 刷新——它只更新
skillsByAgent/lastScanAt/scanState（实证：`applyScanSummary` 不碰
dashboardCentralSummary）。

## 契约 3：图表数据 = 专用聚合 IPC，不做前端假窗口

- 需要窗口化统计（N 天趋势、Top N）时，后端提供恰好窗口大小的聚合结果
  （零值填充、时间源注入以便确定性测试），前端不得用小样本列表假装大窗口。
- 现有：`get_daily_operation_counts(days)`（本地日历日分桶，恰好 days 桶）、
  `get_central_top_tags(limit)`（JOIN skills 限定 `is_central = 1`，
  `skill_tag_links` 无 FK，必须防孤儿 link）。
- 生命周期：target 作用域数据放 platformStore（随 reset + scanGeneration
  重载）；跨 target 流水放 operationLogStore（不随 target reset）。加载器
  一律模块级 token latest-wins；多 IPC 独立 invoke、独立 error 态 + 面板级
  重试。

## 契约 4：Dashboard 视觉与性能

- `.surface-glass` 不使用 `backdrop-filter`（8 卡 × blur(22px) 是 Windows
  WebView2 滚动掉帧主因）；新增 Dashboard 卡片沿用该 token，不得回退加
  blur。装饰性光斑（blur-3xl）禁止出现在常驻面板。
- 图表一律纯 SVG 手绘（不引入 recharts/d3）；SVG 必须 `role="img"` +
  aria 概述 + 数据点文本等价（`<title>`），「今日」类高亮不得只靠颜色
  （描边/纹理 + `aria-current`）。
- viewModel 字段必须全部被 Shell 消费；新增字段先确认消费者，删除区块时
  同步删除其数据加载与订阅。
