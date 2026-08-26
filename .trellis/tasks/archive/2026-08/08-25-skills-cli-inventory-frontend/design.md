# Design — Skills CLI 库存优先页面前端落地与 doctor 非阻塞

## 边界

- 后端 IPC 契约不变：`skills_cli_doctor` / `skills_cli_list_global` / `skills_cli_install_targets` / `skills_cli_preview_source` / `skills_cli_add_global` / `skills_cli_remove_global` / `cancel_skills_cli_job`（`.trellis/spec/backend/skills-cli-global.md` §2）。`skills_cli_list_global` 已返回 snapshot `{ skills, canonicalRoot, lockPath }`（`2d8a529e` 已适配）。
- 触点：`src/stores/skillsCliStore.ts`、`src/pages/SkillsCliView.tsx`、新 `src/components/skillsCli/`、`src/i18n/locales/{en,zh}.json`、对应 Vitest、`src-tauri/src/services/skills_cli/mod.rs`（仅 doctor 日志）。

## Store 契约（R2/R5）

```ts
interface SkillsCliState {
  skills: SkillsCliGlobalSkill[];
  targets: SkillsCliInstallTarget[];
  preview: SkillsCliSourcePreview | null;
  doctor: SkillsCliDoctorReport | null;
  isLoading: boolean;        // 首次加载（skills 为空时）
  isRefreshing: boolean;     // 已有数据时的后台刷新
  isPreviewing / isMutating / isCancelling / jobId 不变;
  runtimeError: string | null;    // doctor 拒绝（写路径降级）
  inventoryError: string | null;  // list/targets 读取失败
}
```

`loadAll()` 分轨 settle，替代 `Promise.all` 整体拒绝：

- 库存轨：`Promise.all([list_global, install_targets])` → 成功写入 `skills`/`targets` 并清 `inventoryError`；失败写 `inventoryError` 且**保留旧 `skills`**。
- 运行时轨：`skills_cli_doctor` 独立 catch → 成功写 `doctor` + 清 `runtimeError`；失败 `doctor = null` + `runtimeError = code`。
- 首次（`skills.length === 0`）`isLoading`，否则 `isRefreshing`；两轨完成后统一落 false。刷新不清空任何已有字段。
- `addGlobal` / `removeGlobal` 成功后仍调 `loadAll()` 重读 lock snapshot；`resetForTargetChange()` 维持 `emptyState` 形状（新增字段加入 `emptyState`）。
- 错误值延续 `backendErrorStateValue(error)` 信封格式，`formatBackendError` 渲染。

## View 布局（R1/R4，DOM 顺序）

```
页头（标题 + Refresh：isRefreshing 时按钮 spinner，列表不拆）
├ 错误区
│  ├ inventoryError → <p data-testid="skills-cli-inventory-error" role="alert"> + 重试按钮
│  └ doctor 状态行 data-testid="skills-cli-doctor"（保留现有 testid）：
│    doctor OK → 现有 doctorOk 文案；runtimeError → 该句只在此出现一次
├ KPI strip + 两张 SVG 图（InventoryCensus）
├ 库存列表 <section data-testid="skills-cli-inventory">
│    UnifiedSkillCard variant="skillsCli"（现有字段：name/path/agents/source）
├ 安装区 <details data-testid="skills-cli-install">
│    open 默认值 = skills.length === 0 && !inventoryError
│    内部为现有 source/preview/skill+platform 选择/add/cancel 表单，逻辑不变
└ 底部 <p data-testid="skills-cli-paths" class="text-ui-meta"> canonicalRoot · lockPath
```

- `cli_unavailable`（R5）：`runtimeError` 存在时 `Install` / `Uninstall` 按钮 `disabled`，安装区顶部一行原因（`t("skillsCli.runtimeBlocked", { error })`）；库存卡片区不受影响。
- 空态：`skills.length === 0 && !inventoryError && !isLoading` → `skillsCli.empty`（AC3）；`inventoryError` 且无数据 → 库存错误（AC5），不渲染 empty。
- 卸载确认 Dialog 逻辑不变。

## 图表组件（R3）

新 `src/components/skillsCli/InventoryCensus.tsx`，props：`{ skills, targets }`，纯派生：

```
kpi: { installed, linked, sourceKinds }
platformCounts: targets 顺序映射 → skills.flatMap(s => s.agents) 计数（零值桶保留）
bucketCounts: 8 个固定桶 github|gitlab|git|mintlify|huggingface|local|well-known|unknown
```

- 视觉复用 `Dashboard/ActivityPanel` 手绘 SVG 柱图模式（`role="img"` + `aria-label` + 每根 `<title>`），KPI 形态对齐 `UsageMetricStrip`（其 props 绑定 `UsageKpis` 类型，不直接复用组件，按同形态实现轻量 strip）。
- 空 snapshot：容器渲染 `data-testid="skills-cli-census-empty"`，不画轴。
- 不引入图表库、不加 `backdrop-filter`。

## doctor 失败可诊断（R6）

`mod.rs` `doctor_with_launcher` probe 非零分支（现 `mod.rs:246-248`）：

```rust
if !probe.status_success {
    tracing::warn!(
        status = ?probe.status_success,
        stderr = %String::from_utf8_lossy(&probe.stderr).chars().take(400).collect::<String>(),
        "Skills CLI doctor probe failed");
    return Err(SkillsCliError::CliUnavailable);
}
```

- stderr 摘要只进 tracing（redaction-policy：不进 IPC / operation log 明细）；`CliUnavailable` 的公开句维持 `ipc_error.rs:415-417` 不变。
- `CliOutput.stderr` 已捕获（runner.rs:31-38），无需改 runner。

## i18n（R7）

新增 `skillsCli.kpi.*`（installed/linked/sourceKinds）、`skillsCli.chart.*`（平台图/来源图 aria 与 `<title>` 文案、census empty）、`skillsCli.runtimeBlocked`、`skillsCli.inventoryErrorRetry`、`skillsCli.refreshing`；调整 `skillsCli.subtitle` 为短句（PIN 信息移入 doctor 行/paths）。中英同步，键名与现文件风格一致。

## 兼容与风险

- `AppShell.tsx:59` 依赖 `resetForTargetChange` — 字段新增不破坏调用方。
- `SkillsCliView.test.tsx` 现有 mock 走 `mockIpcCommands`，doctor-error 用例（:170-207）按 AC2/AC11 改写；`skillsCliStore.test.ts` 增加 settle 矩阵用例。
- 回滚点：本任务全部为前端文件 + mod.rs 一处 warn，`git revert` 单提交可回滚。

## 验证

- `pnpm vitest run src/stores/skillsCliStore.test.ts src/pages/SkillsCliView.test.tsx src/components/skillsCli`
- `cargo test --manifest-path src-tauri/Cargo.toml skills_cli`
- `pnpm i18n:check`（如存在）或现有 i18n 校验入口；`just ci` 收尾。
- 人工 Windows 检查（不作为自动化门禁）：`pnpm tauri dev` 打开页面，确认无控制台闪窗、51 条库存与图表渲染、断网刷新时库存保留且出现一行 runtime 原因。

## Redesign v2 — census 紧凑化 + 饼图（用户反馈 2026-08-25）

首版图表以 `w-full` + viewBox 等比缩放，柱形被拉满容器宽度，平台行高过大，整卡占据首屏过高。v2 收敛为一张紧凑卡片：

```
┌ section rounded-lg border p-4 ────────────────────────────┐
│ KPI 行（border-b pb-3）：已装 51 · 已链接 40 · 来源 1      │
│   label text-xs muted，value text-lg semibold tabular     │
│ grid pt-4 md:grid-cols-[auto_minmax(0,1fr)] gap-6:        │
│ ┌ 来源饼图 ┐   ┌ 平台紧凑条形图（固定像素，不随宽缩放）┐      │
│ │ donut 120 │   │ 行高 16px / 条高 8px / 字号 9      │      │
│ │ 中心总数   │   │ 宽 ≈242px；零值桶 1px + opacity .3 │      │
│ │ 图例列    │   └────────────────────────────────────┘     │
└───────────────────────────────────────────────────────────┘
```

- 饼图取代来源类型条形图：来源分布是构成关系，饼图语义更准确；donut 中心显示总技能数。
- donut：viewBox 0 0 120 120，r=44，strokeWidth=14，`stroke="var(--chart-N)"`（theme token，禁 arbitrary hex）；分段 `strokeDasharray/Dashoffset`，`rotate(-90)` 从 12 点起；每段 `<title>` 含数量与百分比（`donutTitle`）；中心 `total` + `donutCenterLabel`。`role="img"` + `donutAria`。
- 桶→颜色映射表：github/gitlab/git/mintlify/huggingface 依次 chart-1..5，local/well-known 复用 chart-2/chart-4，unknown 固定 `var(--muted-foreground)`；图例色点与分段同源。
- 平台条形图改为固定像素 SVG（`width`/`height` 属性，无 viewBox 缩放），1:1 渲染；零值桶保留（延续 AC7）。
- 排版契约：label 一律 `text-xs`，计数 `tabular-nums`；不引入 `text-[...]`；SVG `fontSize` 为几何属性不受 no-growth 约束（typographyContract 现行口径）。
- i18n：新增 `census.donutAria` / `census.donutTitle` / `census.donutCenterLabel`，移除 `census.sourceAria`；中英同步。
- 测试：census 用例改为断言 donut 分段 title（含百分比）、平台零值桶 title 仍含 0、rect 数 = 平台行数、circle 数 = 非零桶数；KPI 与 empty 断言不变。
