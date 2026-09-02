# Implementation Plan

本文件只定义后续实施步骤；当前任务保持 `planning`，不在本规划阶段修改前端代码。

## Step 0 — 复现并冻结测试入口

- 在 `src/test/components/layout/GlobalSearchDialog.test.tsx` 增加当前错误 `/collection/:id` 的失败回归和冷启动 loader 断言。
- 在 `src/test/stores/collectionStore.test.ts` 建立可手动 resolve/reject 的 deferred helper，不按 invoke 调用次序串联 mock。

验证命令：

```powershell
pnpm exec vitest run src/test/components/layout/GlobalSearchDialog.test.tsx src/test/pages/CollectionsListView.test.tsx src/test/stores/collectionStore.test.ts
```

预期：新增回归在生产改动前失败；现有用例保持通过。回滚点：只移除新增失败测试。

## Step 1 — 修复唯一导航入口（R1）

- 修改 `src/components/layout/GlobalSearchDialog.tsx` 的 collection `SearchItem.onSelect`，使用 `/collections` 和 `collectionContext.collectionId`。
- 在 `src/test/pages/CollectionsListView.test.tsx` 断言 state 初始化的 `selectedId`，不添加新 route。

验证命令：

```powershell
pnpm exec vitest run src/test/components/layout/GlobalSearchDialog.test.tsx src/test/pages/CollectionsListView.test.tsx
```

通过 AC1-AC3 后建立回滚单元 A；失败时只回滚导航与对应断言。

## Step 2 — 建立列表就绪事实与搜索状态（R2、R3、R6）

- 在 `CollectionState`、`CentralSkillsState` 及各自 initial/reset/success 路径加入成功加载事实；失败不得伪装为 loaded。
- 在 `GlobalSearchDialog` 的 open effect 中按 `!hasLoaded && !isLoading` 调用 `loadCollections` / `loadCentralSkills`。
- 在 `src/i18n/locales/en.json`、`src/i18n/locales/zh.json` 增加状态和 Retry keys，并在 dialog 分来源渲染。
- 扩展 GlobalSearchDialog 测试覆盖冷启动、成功空数组、失败、Retry 成功和重开不重复加载。

验证命令：

```powershell
pnpm exec vitest run src/test/components/layout/GlobalSearchDialog.test.tsx src/test/stores/collectionStore.test.ts src/test/stores/centralSkillsStore.test.ts src/test/contracts/i18nLocales.test.ts
```

通过 AC4-AC8、AC15-AC16 后建立回滚单元 B；失败时整体回滚 loaded 字段、UI 和 locale keys，保留 Step 1。

## Step 3 — 收敛详情与 mutation refresh（R4、R5）

- 在 `src/stores/collectionStore.ts` 为 `loadCollectionDetail` 增加单调 request id 与 target 门控。
- 把 `addSkillToCollection`、`removeSkillFromCollection` 的直接详情 invoke 改为有目标检查的同一 loader 调用。
- 加入 A/B 成功乱序、A 迟到失败、loading 所有权、mutation A 与切换 B 并发用例。

验证命令：

```powershell
pnpm exec vitest run src/test/stores/collectionStore.test.ts src/test/pages/CollectionsListView.test.tsx
```

通过 AC9-AC14 后建立回滚单元 C；失败时整体回滚门控与对应并发测试，不恢复第二条 refresh 通道的一半状态。

## Step 4 — 总验证与人工边界

```powershell
pnpm typecheck
pnpm lint
pnpm exec vitest run src/test/components/layout/GlobalSearchDialog.test.tsx src/test/pages/CollectionsListView.test.tsx src/test/stores/collectionStore.test.ts src/test/stores/centralSkillsStore.test.ts src/test/contracts/i18nLocales.test.ts
just ci
```

- 在 Windows WebView2 手工从冷启动打开全局搜索，键盘选择 collection，确认进入 `/collections`、选中正确条目并保持焦点可用。
- 用真实 Tauri 数据分别检查 loading、真实空集合、失败后 Retry；未执行时把 AC21-AC22 明确记为 `UNVERIFIED`。
- 若总门禁失败，按 A、B、C 最小回滚单元定位；不得一次回滚全部任务或新增兼容 URL。
