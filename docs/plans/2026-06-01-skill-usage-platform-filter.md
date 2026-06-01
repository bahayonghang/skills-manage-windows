# 技能使用页「按平台筛选」实现计划

- 状态：待实现（已通过 `/think` 评审）
- 日期：2026-06-01
- 影响页面：`/usage`（`SkillUsageView`，标题「技能使用」）
- 形态：顶部平台分段选择器，整页按所选平台重新作用域

---

## 1. 背景与核心结论

「skill 统计界面」指 `SkillUsageView`（技能**使用**统计：调用次数 / 频次柱图 / 16 周热力图 / 最近调用 / 数据源状态）。它当前把所有平台的调用**混在一起**聚合。

关键事实：**平台维度的数据早已存在**——`skill_calls` 表的 `source` 列就是 provider 的 display name（如 `"Claude Code"`、`"Codex CLI"`）。每条调用记录都已带「来自哪个 AI 工具」。当前 `build_overview()` 的三个聚合查询只按 `target_id`（本机 / 远程机器）过滤，跨所有 `source` 合并。

因此本需求是**切片问题，不是建模问题**：给聚合查询加一个可选 `source` 过滤参数，前端加一个平台选择器即可。**无需改 schema、无需新表、无需新命令。**

```
skill_calls( target_id, skill, timestamp_ms, project, session_id, source )
                  │                                                  └─ ★「平台」= provider display name
                  └─ 机器作用域（本机 / SSH / WSL），已有顶部 target 切换器
```

两个正交的作用域：**机器（target_id，已有）× 平台（source，本次新增）**。

## 2. 词义对齐（避免误解）

本仓库「平台」有两套，互不相同：

|                | 安装侧                                       | 使用统计侧（本计划）                                                                    |
| -------------- | -------------------------------------------- | --------------------------------------------------------------------------------------- |
| 是什么         | 27 个安装目标（claude / cursor / windsurf…） | 8 个 usage provider                                                                     |
| 列表           | `agents` 表                                  | claude-code / codex / droid / opencode / grok（真实）+ antigravity / kiro / zed（stub） |
| 能否按它拆统计 | ❌ 安装目标不产生调用日志                    | ✅ 有日志扫描器才有 usage                                                               |

统计页能「按平台拆」的平台，**只能是 8 个 usage provider**。安装侧的 Cursor / Windsurf 等不会出现在这里（没有采集它们的会话日志）。

## 3. 方案总览

服务端过滤：给 overview / recent 的查询加可选 `source` 参数，前端选择器切换时重新 `invoke`。聚合逻辑保持在 Rust（单一真相），不把热力图网格等逻辑复制到 TS。本地 SQLite 查询，切换亚毫秒级。

含本次评审追加纳入核心的两项：

- **① 单平台时第 4 张 KPI 卡：数据源 → 会话数**。`uniqueSources` 在单平台下恒为 1（无信息量），换成 `uniqueSessions`（该平台用到技能的会话数）。
- **② 平台选择器固定列出全部 8 个 provider**，无数据者（`callCount === 0`）灰显 + 禁用，不隐藏。

### 选中态行为

- 默认 `selectedSource = null`（全部平台）= 现状全量视图。
- 选中某平台 → KPI / 频次 / 热力图 / 最近调用全部只含该 `source`。
- 切换 target（机器）→ 重置为「全部平台」（平台全集随机器变化）。
- 手动刷新 → **保留**当前所选平台（若刷新后该平台仍有数据）；若该平台变为无数据，回落「全部」。

## 4. 逐文件改动清单

> 调用点已用 `grep` 全量枚举，改动面收敛，无遗漏。

### 4.1 后端（Rust）

**`src-tauri/src/db/repos/usage_repo.rs`**

- 4 个查询函数新增 `source: Option<&str>` 参数，统一用 `AND (? IS NULL OR source = ?)`（保持静态 SQL；`None` → NULL → 条件恒真）：
  - `get_usage_kpis(pool, target_id, source)`
  - `list_top_skills(pool, target_id, source, limit)`
  - `list_daily_counts_since(pool, target_id, source, cutoff_ms)`
  - `list_recent_calls(pool, target_id, source, limit)`
- 绑定顺序示例（`get_usage_kpis`）：`bind(target_id).bind(source).bind(source)`。
- **会话数（优化①）**：`UsageKpisRow` 结构体新增 `unique_sessions: i64`；`get_usage_kpis` 的 SQL 增加一列：

```sql
SELECT
  COUNT(*)                    AS total_calls,
  COUNT(DISTINCT skill)       AS unique_skills,
  COUNT(DISTINCT project)     AS unique_projects,
  COUNT(DISTINCT source)      AS unique_sources,
  COUNT(DISTINCT session_id)  AS unique_sessions
FROM skill_calls
WHERE target_id = ? AND (? IS NULL OR source = ?)
```

**`src-tauri/src/services/usage/aggregate.rs`**

- `UsageKpis` 结构体新增 `unique_sessions: i64`。
- `kpis_from_rows()` 增加 `sessions: HashSet` 统计（与 SQL 路径保持一致）。
- 更新单测 `kpis_count_unique_dimensions` 断言 `unique_sessions`。

**`src-tauri/src/services/usage/mod.rs`**

- `build_overview(pool, target_id, source: Option<&str>, top_skills_limit)`：把 `source` 透传给上述 3 个查询；构造 `aggregate::UsageKpis` 时补 `unique_sessions: kpis_row.unique_sessions`。
- 同文件测试调用（`build_overview(&pool, "local", 50)`，约 :524）改为 `build_overview(&pool, "local", None, 50)`；新增「双 source fixture」断言过滤生效。

**`src-tauri/src/commands/usage.rs`**（3 处调用点）

- `usage_get_overview(.., top_skills_limit, source: Option<String>)` → `build_overview(&db, &target_id, source.as_deref(), limit)`。
- `usage_get_recent(.., limit, source: Option<String>)` → `list_recent_calls(&db, &target_id, source.as_deref(), n)`。
- `build_refresh_page`（:107 / :109）→ 两处均传 `None`：刷新永远返回**未过滤** base + **完整** providers 列表（providers 是选择器的数据源）。

### 4.2 前端（TypeScript）

**`src/types/usage.ts`**

- `UsageKpis` 新增 `uniqueSessions: number`。

**`src/stores/usageStore.ts`**

- 新增状态 `selectedSource: string | null`（`null` = 全部）。
- 新增 action `selectSource(source: string | null)`：写状态后调用 `loadOverview({ topSkillsLimit, source })` + `loadRecent({ limit, source })`。
- `loadOverview` / `loadRecent` 增加 `source?` 透传 `invoke`。
- 刷新保留所选平台：`applyRefreshResult` 后，若 `selectedSource` 非空且仍在新 `providers` 中 `callCount > 0`，再发一次带 `source` 的 overview/recent；否则置 `selectedSource = null`。
- `subscribeTargetChanged` 的事件回调里把 `selectedSource` 重置为 `null`（与既有 `detail: null` 一起）。
- `BROWSER_FIXTURE_OVERVIEW.kpis` 补 `uniqueSessions: 0`。

**`src/components/usage/PlatformFilterBar.tsx`**（新建，小组件）

- props：`{ providers: ProviderHealth[], selected: string | null, onSelect: (s: string | null) => void }`。
- 渲染「全部平台」pill（`selected === null` 时高亮）+ **全部** `providers`（不过滤）的 `displayName` pill，顺序沿用数组顺序（与 `ProviderHealthList` 对齐）。
- `callCount === 0` 的 pill：`disabled` + 灰显 + `title` 提示（i18n `notDetected`）；其余可点，点击 `onSelect(p.displayName)`。

**`src/components/usage/KpiStrip.tsx`**

- 新增 prop `singlePlatform?: boolean`（默认 `false`）。
- 第 4 张卡：`singlePlatform` 为真时显示「会话数」（`kpis.uniqueSessions`，换 session 类 lucide 图标如 `MessagesSquare`）；否则维持「数据源」（`kpis.uniqueSources`）。前 3 张不变。

**`src/pages/SkillUsageView.tsx`**

- 在标题行下方、`KpiStrip` 上方渲染 `<PlatformFilterBar providers selected={selectedSource} onSelect={selectSource} />`。
- `KpiStrip` 传 `singlePlatform={selectedSource !== null}`。
- 顶部 `kpis` 兜底字面量（约 :25）补 `uniqueSessions: 0`。

**`src/pages/skillUsageBindings.ts`**

- `useUsageBindings` 暴露 `selectedSource`、`selectSource`、`providers`。

**`src/components/usage/ProviderHealthList.tsx`**（可选，与选定线框图一致）

- 加可选 `activeSource?` / `onSelect?`；行变可点 + 高亮 active；`callCount === 0` 行不可点。与 `PlatformFilterBar` 共同驱动同一个 `selectedSource`。

### 4.3 i18n（`src/i18n/locales/{zh,en}.json`）

- `skillUsage.platformFilter.all` = 「全部平台」/ `All platforms`
- `skillUsage.kpi.sessions` = 「会话数」/ `Sessions`

### 4.4 测试随改（typecheck 会强制）

- `src/test/SkillUsage.components.test.tsx`：3 处 `UsageKpis` 字面量（约 :40、:264、:300）补 `uniqueSessions`；新增 `KpiStrip` 在 `singlePlatform` 下显示会话数的用例；新增 `PlatformFilterBar` 渲染（全部 + 灰显禁用）与点击回调用例。
- `usageStore` 测试：`selectSource("Claude Code")` 以 `{ source }` 调 overview/recent；刷新保留 / 回落逻辑。
- Rust：`usage_repo` / `services::usage` 加双 source fixture，断言 `Some(src)` 过滤后 KPI / topSkills / heatmap / recent 只含该源，且 `unique_sessions` 正确。
- 其余任何引用 `UsageKpis` 字面量的测试按 typecheck 报错补齐。

## 5. 关键决策

1. **服务端 SQL 过滤**，不在前端内存重算——避免把 Rust 热力图网格逻辑复制到 TS，聚合保持单一真相。
2. **过滤值 = provider `displayName`**（即 `skill_calls.source` 实际存的值）。
3. **`uniqueSessions` 设为常驻字段**（查询零成本），仅在单平台时替换第 4 张卡展示——展示逻辑收在 `KpiStrip`。
4. **选择器固定全 8 项**，无数据灰显禁用（用户明确要求；让支持的工具集一目了然，stub 永远灰显）。
5. **刷新保留所选平台、切 target 重置**：机器变则平台全集变，重置最直觉；同机刷新保留更顺手。
6. `usage_refresh` 始终返回未过滤 base + 完整 providers（它是选择器的数据源）。

## 6. 最脆弱的假设

整个方案押在 **`skill_calls.source` ≡ provider `display_name()`**。已核验 `providers/claude_code.rs`：单个 `const SOURCE = "Claude Code"` 同时用于 `display_name()` 与每条 `SkillCall.source`；`services/usage/mod.rs` 文档把「`display_name` 与 `source` 保持一致」列为 provider 约定。

> 风险：若未来某 provider 让 `source ≠ display_name`，该平台筛选会**静默查空**。落地时顺手核对另 4 个真实 provider（codex / droid / opencode / grok）的 `source:` 赋值是否等于其 `display_name()`。低风险。

## 7. 验证

```bash
# Rust
cd src-tauri && cargo test usage && cargo clippy -- -D warnings

# 前端
pnpm test -- src/test/SkillUsage.components.test.tsx
pnpm typecheck && pnpm lint

# 完整门禁
just ci
```

手动验收（`pnpm tauri dev` → `/usage`）：

- 切换平台 pill：KPI / 频次 / 热力图 / 最近调用四块同步只反映该平台。
- 单平台时第 4 张卡显示「会话数」；「全部平台」时显示「数据源」。
- 无数据 provider（含 antigravity / kiro / zed）灰显不可点。
- 底部「数据源状态」行点击与顶部 pill 同步高亮（若实现可选项）。
- 同机刷新保留所选平台；切换 target 回到「全部平台」。

## 8. 回滚

纯增量、无数据迁移、无 schema 变更。回滚 = 还原上述文件；DB `skill_calls` 不受影响。可安全 `git revert`。

## 9. 规模与阶段

- 约 12～13 个文件：后端 4（usage_repo / aggregate / services·mod / commands）+ 前端 6（types / store / bindings / View / KpiStrip / 新 PlatformFilterBar，可选 ProviderHealthList）+ i18n 2 + 测试随改。
- **无新服务、无新表、无新命令、无 schema 变更。**
- 单一可合并阶段；若要分两步，可先合「后端 source 参数 + 会话数字段」（对现有 UI 无副作用），再合「前端选择器 + KPI 切换」。
