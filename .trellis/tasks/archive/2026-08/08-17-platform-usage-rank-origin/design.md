# 平台技能用量排序与来源区分 — 设计

## Architecture

```
skill_calls (facts)
  → usage_repo::list_skill_usage_stats(target, names, cutoff?)
  → commands::usage_get_skill_usage_stats
  → @/lib/ipc + fixtures/usage.ts
  → useSkillUsageStats(names, { days: null })
  → derivePlatformSkillRows + assignUsageRanks
  → UnifiedSkillCard variant="platform"
```

30 天徽标继续走现有 `usage_get_skill_counts` + `useSkillCallCounts(..., 30)`。两条读取窗口分离，互不改对方合同。

平台页不订阅 `usageStore`。`usageStore` 的 source 筛选、骨架、扫描中状态属于 `/usage`。

## Contracts

### 全历史读取

新增，不改 `usage_get_skill_counts`。

```rust
// db/repos/usage_repo.rs
list_skill_usage_stats(
    pool, target_id, skills: &[String], cutoff_ms: Option<i64>
) -> Result<Vec<SkillUsageStatRow>, sqlx::Error>

struct SkillUsageStatRow { skill: String, count: i64, last_used_ms: Option<i64> }
```

`cutoff_ms = None` 时 SQL 不加 `timestamp_ms >= ?`。空 `skills` 返回空 vec。命令层对请求里每个名字预填 `{ count: 0, last_used_ms: None }`，再覆盖查到的行。与现有 counts 命令同一预填策略。

```ts
// IPC
usage_get_skill_usage_stats: {
  skills: string[];
  days: number | null; // null = 全部已记录历史
} -> Record<string, { count: number; lastUsedMs: number | null }>
```

`days: number` 保留给将来复用，本任务平台排序只传 `null`。命令层 `days = Some(n)` 时 cutoff 与现有 counts 相同。禁止调用方传 `0` 当全历史。

服务层：若命令继续直调 repo（与 `usage_get_skill_counts` 一致）则保持该形状；新增 repo 函数返回 `sqlx::Error`，命令 `.map_err(|e| e.to_string())`。不新增 `Result<T, String>` 的 service API。

登记：`ipc_registry.rs`、`commandMap.ts`、`src/fixtures/usage.ts`、`pnpm docs:gen`。

### 前端 hook

`useSkillUsageStats(skillNames, { days: number | null })`：

- 返回 `{ stats, ready }`。`ready === false` 表示加载中或失败。
- 失败静默（与 30 天 hook 相同），`ready` 仍为 false，避免把空 map 当成「全部零次」。
- 5 分钟模块缓存，key = `targetId::days|all::sortedNames`。target 切换不复用。
- 只在 hook 内 `invoke`，组件不调用。

### 排序与名次

扩展 `derivePlatformSkillRows` 输入：

```ts
usageStats?: Record<string, { count: number; lastUsedMs: number | null }>;
usageReady?: boolean;
```

`sort.field === "callCount"`：

- `usageReady !== true`：按名称升序（忽略 direction），不写名次。
- 就绪后：`count desc`（若 direction=desc）→ `lastUsedMs` 同向，缺省当 0 → 名称。默认 direction = `desc`。
- `count === 0` 视为零次，沉在有次数之后（降序时自然成立）。

`assignUsageRanks(sortedSkills, stats)`：

- 只在 `usageReady` 时调用。
- 竞赛名次：次数与 `lastUsedMs` 都相同则同名次，下一个跳号（1,2,2,4）。
- 零次：`rank = null`。
- 匹配键 `skill.name`。同名两行共享 count，若排在一起则同名次。
- `groupBy === "repository"` 时对每个 `group.skills` 分别调用。
- 输出挂到卡片的 `lifetimeUsage: { rank: number | null; count: number }`。

行键仍用 `getPlatformSkillRowKey`，名次 map 用行键，避免同名两行抢同一个 DOM key。

`PlatformView` 默认：`sortField = "callCount"`，`sortDirection = "desc"`。

### 卡片

`PlatformSkillCardProps` 增加：

```ts
lifetimeUsage?: { rank: number | null; count: number };
```

`undefined` = 未就绪，不渲染右下角。`rank === null` = 无记录。`rank >= 1` = `#N · count`。

右下角：`absolute bottom-3.5 right-3.5`，`tabular-nums`，`data-testid="usage-rank"`。`title` / `aria-label` 用 i18n 写窗口与次数。不进 `SkillCardMeta`，不加 footer 行，保持 `itemHeight={204}`。

外壳由 `toModel` 从已有 `originKind` + `sourceType` 派生，不新开跨场景 prop：

- plugin → 左侧 3px `bg-warning` 竖条 + `bg-warning/5`
- symlink 且非 plugin → 左侧 3px `bg-info` 竖条
- 其余不加条

竖条是卡片 `relative` 上的绝对元素，不占用 central `statusAccent`。

### 徽章

只改 `SkillCardBadges` + `SkillCardMeta` 的平台消费路径。

`SkillCardMeta` 平台组合：

1. `originKind === "plugin"` → 一枚 warning chip（Lock +「插件来源」）。不渲染 `ReadOnlyBadge`，不渲染独立安装指示。
2. 否则若 `sourceType === "symlink"` → info chip（Link2 +「中央技能库」），title 含符号链接。
3. 否则若有 `sourceType` → muted chip（FolderOpen +「独立安装」），title 含 copy/native。
4. `isReadOnly && originKind !== "plugin"` → 仍渲染 `ReadOnlyBadge`。
5. 不再为 `originKind === "user"` 单独渲染用户来源 pill。
6. 30 天 `usageBadge`：改 `statusChipClass` 的中性/muted 形态（`border-border bg-muted/40 text-muted-foreground`），文案不变。
7. `SourceChip` 仓库名不变。

chip 几何统一：`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium`。颜色走 `statusChipClass` / muted。禁止手写 amber。

### 来源 Tab

```ts
const showSourceFilter = isClaudePage || sourceCounts.plugin > 0;
```

`derivePlatformSkillRows` 的 `sourceFilter`：`showSourceFilter ? sourceFilter : "all"`。

Tab UI 从 `isClaudePage &&` 改为 `showSourceFilter &&`。aria-label 用 `platform.sourceFilterTabsLabel`（中「来源筛选」/ 英 "Source filters"）。

无插件的 Cursor / 现有 Universal fixture（`universal-helper` 无 `source_kind`）继续不显示 Tab。有插件的 Universal 必须显示。默认 origin 仍是 `{ kind: "all" }`。

## Compatibility

- `usage_get_skill_counts` 参数和返回不变。中央库 30 天徽标不改数据源。
- `PlatformSortField` 字面量 `callCount` 保留，避免 URL/测试大面积改名。
- 改 Universal Tab 可见性：更新 `PlatformView.test.tsx` 里「有插件才显示」的断言；无插件用例保持隐藏。
- 插件行不再出现「只读」独立文案：更新用 `getCardBadgeMatches(readOnlyText)` 断言插件行的测试。
- 新增命令后必须 `pnpm docs:gen`，提交 `docs/architecture/_generated/`。

## Trade-offs

| 选项 | 结论 |
| --- | --- |
| 扩展旧 counts 命令 vs 新命令 | 新命令。旧返回是 `Record<string, number>`，塞 `lastUsedMs` 会破坏 30 天调用方。 |
| 复用 `usage_get_overview` | 拒绝。会拉热力图，并诱使平台页挂上 `usageStore`。 |
| 名次画进 meta 行 | 拒绝。meta nowrap 已满；用户指定右下角。 |
| 名次占新 footer 行 | 拒绝。会顶破 204 行轨。绝对定位。 |
| 竞赛名次 vs 稠密名次 | 竞赛（1,2,2,4）。与「并列」直觉一致。 |
| 默认改 origin=SkillPort | 拒绝（Q2 不是 C）。 |

## Rollback

删除新命令与 hook，恢复 `PlatformView` 默认 `name/asc` 和 `isClaudePage` Tab 守卫，回退徽章组件。无 schema 迁移。
