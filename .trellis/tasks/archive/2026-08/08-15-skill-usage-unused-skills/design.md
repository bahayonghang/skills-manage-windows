# Design — Skill Usage 未使用技能视图

## Architecture / Boundaries

```
skills (is_central=1) ──┐
agent_skill_observations ├─ LEFT JOIN ── skill_calls (+ skill_usage_metadata)
skill_installations ────┘        │
                                 ▼
              usage service: build_unused_report(target, source?, threshold_days)
                                 │
                    新命令 usage_get_unused_skills
                                 ▼
              useUsageStore.unused slice（序列号防陈旧）
                                 ▼
       SkillUsageView → UnusedSkillsPanel（排序/阈值/分组 = 视图本地）
```

- 后端负责**事实计算**（调用次数、最后使用时间、归属分组）；前端负责**呈现与筛选**（阈值切换、排序、Never/Stale 过滤均为视图本地，符合 `skill-usage-state.md:48` 约束的延伸）。
- 不改 `skill_calls` 语义、不改 provider 扫描、不写新表——本期为只读派生查询，无需 DB migration。

## Backend（Rust）

新 repo 查询（`src-tauri/src/db/repos/usage_repo.rs` 或新 `unused` 段落）：

1. `list_central_unused_candidates(target_id)`：
   `skills(is_central=1)` LEFT JOIN `skill_usage_metadata.resolved_skill_id` → `skill_calls` 聚合（`COUNT(*)`, `MAX(timestamp_ms)`），得出每个 Central skill 的调用次数与最后使用时间；带 `linked_agents`（复用 `SkillWithLinks` 的组装逻辑或 `skill_installations`）。
2. `list_platform_unused_candidates(target_id)`：
   `agent_skill_observations`（或 `skill_installations`，实现时以现有扫描写入方为准）按 `(agent_id, name)` 聚合，名称 normalize 后直查 `skill_calls`，并 LEFT JOIN metadata 取 `match_status` / `resolved_skill_id`。

Service（`src-tauri/src/services/usage/mod.rs`）：
- `build_unused_report(source: Option<&str>, threshold_days: u32) -> UnusedSkillsReport`
- 分类规则：`call_count == 0` → `never_used`；`call_count > 0 && last_used_ms < now - threshold_days` → `stale`；其余不返回。
- `source` 过滤与现有命令一致（`skill-usage-analytics.md:51`）：作用于 calls 聚合。
- target 作用域：与现有 usage 命令一致，内部解析 active target；技能库侧走 `state.active_db()`，保证同一 target。

新命令（`src-tauri/src/commands/usage.rs` + `ipc_registry.rs` + `src/lib/ipc/commandMap.ts` + `src/types/usage.ts`）：

```ts
usage_get_unused_skills(source?: string, thresholdDays?: number) -> UnusedSkillsReport

interface UnusedSkillEntry {
  skillId: string | null;        // Central id，平台散件为 null
  name: string;
  matchStatus: "matched" | "ambiguous" | "unmatched";
  origin: "central" | "platform";
  agents: string[];              // 安装/链接的平台
  installedPath: string | null;
  callCount: number;             // 0 = 从未使用
  lastUsedMs: number | null;
  staticTokenEstimate: number | null;
  staticByteCount: number | null;
  status: "never_used" | "stale";
}
interface UnusedSkillsReport { central: UnusedSkillEntry[]; platforms: UnusedSkillEntry[]; }
```

## Frontend

- `useUsageStore` 新增 `unused` 子状态 + `refreshUnused()`，沿用三序列号模式（target 切换失效重建，见 `skill-usage-state.md:42`）；接入 `useUsageBootstrap` 的刷新与 `usage://target-changed` 订阅。
- 新组件 `src/components/usage/UnusedSkillsPanel.tsx`：
  - 分组切换：Central / 平台（平台分组内按 agent 分节）；
  - 状态过滤：全部 / 从未使用 / 长期未用；阈值 30/60/90 天（默认 90）——视图本地；
  - 排序：未用时长 / 体积估算 / 名称——视图本地；
  - 行信息：名称、匹配状态、平台、调用次数、最后使用、体积估算；`skillId` 非空时复用现有"选中打开详情"路径（`SkillUsageDetailPanel` 已依赖 `resolvedSkillId`）。
- 放置：`SkillUsageView.tsx` 主网格，Top skills 之下新增面板（不动 Recent calls / Heatmap 布局）。
- i18n：`skillUsage.unused.*`（en + zh）。

## Trade-offs

- **一次性返回全量候选，阈值在前端切换**：一次查询覆盖三种阈值，避免后端往返；代价是首包略大（144 量级，可忽略）。
- **平台维度按 normalized name 直查 skill_calls**：简单但存在同名噪声；`unmatched` 项在 UI 明确标注，不强行关联 Central。
- **不做删除动作**：清理操作仍在 Central/平台页现有入口完成，本期只提供识别与跳转。

## Operational / Rollback

- 纯增量：新命令 + 新面板，不改既有命令签名；回滚 = 移除面板与命令注册。
- 生成物：新增 Tauri 命令后须跑 `pnpm docs:gen` 并提交生成文件。
