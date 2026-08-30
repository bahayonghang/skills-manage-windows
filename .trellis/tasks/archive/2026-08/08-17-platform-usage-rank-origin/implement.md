# 平台技能用量排序与来源区分 — 实现清单

## Order

1. **Repo + IPC**  
   `list_skill_usage_stats`（`cutoff_ms: Option<i64>`）→ `usage_get_skill_usage_stats` → `ipc_registry` → Rust 测试（空列表、预填 0、有/无 cutoff、target 隔离、`last_used_ms`）。

2. **前端合同**  
   `commandMap.ts` 类型、`fixtures/usage.ts` handler、`pnpm docs:gen`。`src/types/usage.ts` 增加 `SkillUsageStat`。

3. **Hook**  
   `useSkillUsageStats`：`{ stats, ready }`、5 分钟缓存、target key、失败 `ready=false`。单测对齐 `useSkillCallCounts.test.ts`。

4. **View-model**  
   `comparePlatformSkills` 吃 `{ count, lastUsedMs }`；未就绪走名称；`assignUsageRanks` 竞赛名次；分组内重算。扩 `platformSkillViewModel.test.ts`。

5. **徽章 + 卡片**  
   `SkillCardBadges` / `SkillCardMeta` 合并插件行、短中央/独立 chip、30 天 muted。平台卡片绝对定位右下角名次 + 左侧来源竖条。更新 `UnifiedSkillCard.types`、`toModel`、variant 负例、`UnifiedSkillCard.test.tsx`。

6. **PlatformView**  
   默认 `callCount` + `desc`；接入全历史 hook；`showSourceFilter = isClaudePage || pluginCount > 0`；i18n（排序窗口、名次、Tab 标签、无记录）。改 `PlatformView.test.tsx`：默认排序、Universal 有插件显示 Tab、无插件仍隐藏、插件行不再并排只读长文本。

7. **回归**  
   `pnpm test -- src/test/lib/platformSkillViewModel.test.ts src/test/hooks/useSkillCallCounts.test.ts src/test/hooks/useSkillUsageStats.test.ts src/test/components/skill/UnifiedSkillCard.test.tsx src/test/components/skill/unifiedSkillCardVariants.test.tsx src/test/pages/PlatformView.test.tsx`  
   `cd src-tauri && cargo test usage`  
   `pnpm typecheck`、`pnpm lint`、`pnpm docs:gen:check`。完成前 `just ci`。

## Validation

```bash
pnpm test -- src/test/lib/platformSkillViewModel.test.ts src/test/hooks/useSkillUsageStats.test.ts src/test/components/skill/UnifiedSkillCard.test.tsx src/test/pages/PlatformView.test.tsx
cd src-tauri && cargo test usage
pnpm typecheck
pnpm lint
pnpm docs:gen
pnpm docs:gen:check
just ci
```

浏览器/Tauri 桌面未在本规划验证。实现后若本机有桌面会话，在 Universal 上确认：默认用量降序、右下角名次、插件 Tab、徽章合并。无桌面时写明未验证。

## Risky files

| 文件 | 风险 |
| --- | --- |
| `src-tauri/src/commands/usage.rs` | 新命令必须走 `ipc_boundary!`，不得改旧 counts 签名 |
| `src-tauri/src/ipc_registry.rs` | 漏登记则 IPC 404 |
| `src/lib/ipc/commandMap.ts` / `src/fixtures/usage.ts` | 漏登记则 typecheck / 浏览器 fixture 失败 |
| `src/lib/platformSkillViewModel.ts` | 排序管线顺序（tab → origin → search → sort → group）不得打乱 |
| `src/pages/PlatformView.tsx` | Tab 可见性会打破两条旧测试，必须按「有插件才显示」改 |
| `src/components/skill/SkillCardBadges.tsx` / `SkillCardMeta.tsx` | 项目卡仍用 `ProjectSourceBadge`，不要误伤 |
| `src/components/skill/UnifiedSkillCard.tsx` | 行高 204；名次必须绝对定位 |
| `src/test/pages/PlatformView.test.tsx` | 多处按「只读」独立文案和 Claude-only Tab 断言 |

## Do not

- 改 `usage_get_skill_counts` 的 `days: u32` 或返回 `Record<string, number>`。
- 平台页 `useUsageStore`。
- 传 `days: 0` 当全历史。
- 用 `installed_at` / `repository` 重写 origin。
- 新建第二套技能卡片。
- 改中央库排序或中央卡片 footer 用量语义。
- 把默认 origin 改成 SkillPort 安装。

## Spec follow-up（实现后、归档前）

- `skill-usage-analytics.md`：登记 `usage_get_skill_usage_stats`；卡片 30 天徽标合同保留。
- `skill-card-scenarios.md`：平台场景增加 `lifetimeUsage`。
- `platform-origin-classification.md`：轴 B Tab 可见性（Claude 或 pluginCount>0），origin 导航仍只表达轴 A。
