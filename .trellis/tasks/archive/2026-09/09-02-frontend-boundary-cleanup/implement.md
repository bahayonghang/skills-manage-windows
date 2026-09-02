# Implementation Plan

本文件只定义后续实施步骤；当前任务保持 `planning`，不在本规划阶段删除或移动前端代码。

## Step 0 — 重测基线与 reachability（R3、R5、R7）

执行并记录完整输出，不使用审计中的 193：

```powershell
rtk rg -l "from\s+[`"']@/types[`"']" src --glob '*.ts' --glob '*.tsx' --glob '!src/test/**' --glob '!src/types/**' | Sort-Object -Unique
rtk rg -l '@tauri-apps/api/event' src --glob '*.ts' --glob '*.tsx' --glob '!src/test/**' | Sort-Object
rtk rg -n 'CollectionView|SkillPreviewDialog|DuplicatePlatformSkillsDialog|SkillDetailPanelShell' src --glob '*.ts' --glob '*.tsx'
```

- 把第一条的排序路径清单与行数记录为 AC16-AC18 证据。
- Step 0 实测基线：**199** 个生产 `@/types` importer（排除 `src/test/**`、`src/types/**`）。完整排序清单见 `research/root-types-barrel-baseline-199.txt`。审计中的 193 不得写入 contract。
- 实现后同一口径为 **194**（≤ 199）。相对基线减少的 5 条：删除 `src/pages/CollectionView.tsx`、`src/components/platform/DuplicatePlatformSkillsDialog.tsx`；窄入口迁移 `src/components/settings/AiSettingsSection.tsx`、`src/stores/settingsStore.aiSlice.ts`、`src/stores/marketplaceStore.githubImportHelpers.ts`。
- 将第三条逐项分类为 production、test-only、comment；若出现未纳入设计的真实 production/lazy/dynamic 入口，停止删除并返回 planning。
- 回滚点：此步只读，无工作树变更。

## Step 1 — 建立自动化边界契约（R1-R3、R5-R7）

- 新建 `src/test/contracts/frontendArchitectureContract.test.ts`。
- 先写失败断言：update-center lib/store 不反向 import page；Tauri event 生产匹配集合精确等于 `src/lib/ipc/invoke.ts`；4 个 dead path 不存在；root barrel 消费者不高于 Step 0 实测基线。
- 测试扫描明确排除 `src/test/**`；root barrel 额外排除 `src/types/**`。

验证命令：

```powershell
pnpm exec vitest run src/test/contracts/frontendArchitectureContract.test.ts
```

预期：生产改动前由真实 finding 导致相应断言失败。回滚点：只移除新 contract test。

## Step 2 — 移动 update mode canonical owner（R1、R6）

- 新建 `src/lib/updateCheckMode.ts`，移动 `UpdateCheckMode`、setting key、默认值和 normalize 函数。
- 修改 `src/pages/centralUpdateCheckMode.ts`、`src/lib/updateCenterRefreshScope.ts`、`src/stores/updateCenterStore.ts`、`src/stores/settingsStore.ts` 及相关 components/tests 的 import；page-specific builder 留在 page。
- 迁移这些触及文件的 `@/types` 根 import 时使用已有窄入口；不扩展到其他领域。

验证命令：

```powershell
pnpm exec vitest run src/test/stores/updateCenterStore.test.ts src/test/stores/settingsStore.test.ts src/test/components/central/UpdateCheckModeDialog.test.tsx src/test/components/central/updateCenter/updateCenterDecisionAggregation.test.ts src/test/contracts/frontendArchitectureContract.test.ts
pnpm typecheck
```

通过 AC1-AC4 后建立回滚单元 A；owner 和全部 import consumer 必须一起回滚。

## Step 3 — 收口 Tauri event adapter（R2、R7）

- 在 `src/lib/ipc/invoke.ts` 保留唯一 Tauri event import，在 `src/lib/ipc/index.ts` re-export `UnlistenFn`。
- 修改 `src/lib/explanationStream.ts`、`src/stores/marketplaceStore.githubImportHelpers.ts`、`src/stores/projectsStore.ts`、`src/stores/skillDetailStore.ts` 从 `@/lib/ipc` 获取 type。
- 更新 contract，断言生产匹配集合精确等于 `src/lib/ipc/invoke.ts`，不是空集合。

验证命令：

```powershell
pnpm exec vitest run src/test/runtime/ipc.test.ts src/test/stores/projectsStore.test.ts src/test/stores/skillDetailStore.test.ts src/test/contracts/frontendArchitectureContract.test.ts
pnpm typecheck
```

通过 AC5-AC7 后建立回滚单元 B；adapter export 与 4 个 consumer import 作为原子集合回滚。

## Step 4 — 删除已证不可达模块（R3）

- 删除 4 个完整路径：`src/pages/CollectionView.tsx`、`src/components/marketplace/SkillPreviewDialog.tsx`、`src/components/platform/DuplicatePlatformSkillsDialog.tsx`、`src/components/skill/SkillDetailPanelShell.tsx`。
- 删除仅测试废弃模块的 `src/test/pages/CollectionView.test.tsx` 和 `src/test/components/marketplace/SkillPreviewDialog.test.tsx`。
- 保留并运行 `CollectionsListView`、Marketplace drawer、Skill detail drawer 的现役入口测试；不要删除仅在断言说明中提到旧组件名的有效测试。

验证命令：

```powershell
pnpm exec vitest run src/test/pages/CollectionsListView.test.tsx src/test/components/marketplace/MarketplaceSkillDetailDrawer.test.tsx src/test/components/skill/SkillDetailDrawer.test.tsx src/test/contracts/frontendArchitectureContract.test.ts
pnpm typecheck
```

通过 AC8-AC11 后建立回滚单元 C；若发现真实入口，整体恢复 4 个候选及 2 个孤立测试并返回 planning。

## Step 5 — i18n 化 browser fixture 状态（R4）

- 在 `settingsStore.aiSlice.ts` 保留非 Tauri 状态事实，不把英文句子作为最终 UI 文案；`AiTestResultPanel` 在渲染边界用 `t(...)`。
- 把 `MarketplaceView.handlePreviewRepo` 的 browser fallback title/detail 改为 locale keys。
- 同步 `src/i18n/locales/en.json` 与 `src/i18n/locales/zh.json`，增加英语和简体中文回归。

验证命令：

```powershell
pnpm exec vitest run src/test/stores/settingsStore.test.ts src/test/pages/SettingsView.test.tsx src/test/pages/MarketplaceView.catalog-and-entry.test.tsx src/test/contracts/i18nLocales.test.ts
```

通过 AC12-AC15 后建立回滚单元 D；store 状态标记、renderer 分支和两个 locale 一起回滚。

## Step 6 — 复算 barrel ratchet（R5、R6）

- 再次执行与 Step 0 完全相同的 root barrel 命令，保存排序清单。
- 比较路径集合而非手写数字；实现后消费者不得高于启动基线，触及领域不得继续依赖根 barrel。

验证命令：

```powershell
rtk rg -l "from\s+[`"']@/types[`"']" src --glob '*.ts' --glob '*.tsx' --glob '!src/test/**' --glob '!src/types/**' | Sort-Object -Unique
pnpm exec vitest run src/test/contracts/frontendArchitectureContract.test.ts
```

通过 AC16-AC20 后建立回滚单元 E；若基线升高，回滚新增 consumer，不修改 baseline 掩盖回归。

## Step 7 — 总验证、人工边界与回滚纪律

```powershell
pnpm typecheck
pnpm lint
pnpm exec vitest run src/test/contracts/frontendArchitectureContract.test.ts src/test/contracts/i18nLocales.test.ts src/test/stores/updateCenterStore.test.ts src/test/stores/settingsStore.test.ts src/test/pages/CollectionsListView.test.tsx src/test/pages/MarketplaceView.catalog-and-entry.test.tsx
just ci
```

- Windows WebView2 人工 smoke：打开 Update Center 并切换 check mode；在 Settings 执行 AI 测试；在 Marketplace 打开 browser/desktop 可用的 preview 入口并确认无断链。
- WebView2 人工步骤未执行时，AC25-AC28 必须报告 `UNVERIFIED`；Vitest、typecheck 或 source scan 不得替代。
- 总门禁失败时按 A（update mode）、B（adapter）、C（dead code）、D（i18n）、E（barrel ratchet）逐个最小回滚；不得恢复 deprecated wrapper、放宽 event allowlist 或把启动 baseline 改成错误的固定数。
