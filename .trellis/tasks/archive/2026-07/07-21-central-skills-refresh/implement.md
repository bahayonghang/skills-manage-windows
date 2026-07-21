# Implement: Central Skills 刷新按钮与检查后自动刷新

按顺序执行；每步完成后跑该步标注的验证。设计细节见 `design.md`（D1-D6）。

## Step 1 — store：错误契约 + 刷新态 + latest-wins

文件：

- `src/stores/centralSkillsStore.types.ts`
  - `loadCentralSkills` 签名改为 `(options?: { throwOnError?: boolean }) => Promise<void>`。
  - `CentralSkillsState` 增加 `isRefreshingList: boolean`。
- `src/stores/centralSkillsStore.shared.ts`（初始 state 所在）
  - 初始 state 增加 `isRefreshingList: false`；浏览器 fixture 分支（`createCentralBrowserFixtureState`）同步处理该标志。
- `src/stores/centralSkillsStore.listSlice.ts`
  - 模块级 `let loadRequestId = 0;`；`loadCentralSkills(options?)` 内 `const requestId = ++loadRequestId;`。
  - 入口分流：`get().skills.length > 0` → `set({ isRefreshingList: true, error: null })`；否则 `set({ isLoading: true, error: null })`（listSlice 的 context 解构需补上 `get`）。
  - 成功/失败两个分支的 set 都加 `requestId === loadRequestId && generation === getGeneration()` 门控，并对应清除 `isRefreshingList` 或 `isLoading`。
  - catch 分支末尾：`if (options?.throwOnError) throw err;`（rethrow 不做 requestId 门控，见 D5）。

验证：`pnpm vitest run src/test/centralSkillsStore.test.ts`（先补 Step 4 的 store 用例或至少保证既有用例全绿）。

## Step 2 — 手动刷新编排

文件：`src/components/central/useCentralRefreshButton.ts`（新建，无 JSX 用 .ts）

- hook 内 `useTranslation()`，从 `useCentralSkillsStore` 选 `isRefreshingList`、`isLoading`、`loadCentralSkills`，从 `usePlatformStore` 选 `refreshCounts`。
- 按 design D3 在 hook 内实现编排（`Promise.allSettled` 并行 + 失败 toast `central.refreshError`，列表失败优先上报），返回 `{ refreshing: isRefreshingList, disabled: isRefreshingList || isLoading, onClick }`。
- `centralSkillsActions.ts` 不保留 `handleRefresh`（避免两份 D3）。

验证：`pnpm typecheck`。

## Step 3 — 控制器与 Shell UI

文件：

- `src/pages/centralUpdateCheckModeController.tsx`
  - 新增 `const loadCentralSkills = useCentralSkillsStore((state) => state.loadCentralSkills);`。
  - `handleConfirm` 按 design D4 嵌套 try；外层 catch 保持原样。
- `src/components/central/CentralSkillsShell.tsx`
  - 组件内调用 `useCentralRefreshButton()` 驱动刷新按钮（沿用组件内直接用 store 的既有先例），不新增 prop。
  - header 工具栏在 Update Center 按钮后插入图标按钮（design D6），`data-testid="central-refresh-skills"`。
- `CentralSkillsView.tsx` 与 `centralSkillsViewModel.ts` 零改动（sizecheck 冻结基线 865 约束）。
- 确认 `zh.json` / `en.json` 中 `central.refresh`、`central.refreshError`、`central.updateCheckError` 均存在（预期已存在，不新增）。

验证：`pnpm typecheck && pnpm lint`。

## Step 4 — 测试

文件：`src/test/centralSkillsViewTestSupport.tsx`、`src/test/centralSkillsStore.test.ts`、`src/test/CentralSkillsView.shell.test.tsx`、`src/test/CentralSkillsView.updates-and-search.test.tsx`

按 design.md「测试设计」清单补齐用例。注意 `.trellis/spec/frontend/async-ui-test-stability.md` 的异步断言约定；mock reject 只用于 `throwOnError: true` 的调用路径（生产代码只有这里会抛）。

验证：`pnpm vitest run src/test/centralSkillsStore.test.ts src/test/CentralSkillsView.shell.test.tsx src/test/CentralSkillsView.updates-and-search.test.tsx`。

## Step 5 — 全量验证

```powershell
pnpm typecheck; pnpm lint; pnpm test
just ci
```

`just ci` 必须通过才算完成。

## 回滚点

- Step 1 若引发既有 store 测试大面积失败：检查初始 state / fixture 分支是否漏加 `isRefreshingList`，不要改既有断言语义。
- 所有改动均为新增式（可选参数、新增 prop、新增状态），`git checkout -- <file>` 即可整体回滚，无数据库/后端迁移。

## 风险与注意

- `loadCentralSkills` 调用点约 15 个，签名扩展是可选参数，理论上零影响；若 typecheck 报错多半在某处自定义了同名局部类型（如 `settingsViewActions.ts:123`、`centralStoreLocationView.ts:15`、`CentralSkillDialogs.tsx:202`），同步扩展即可，不改其行为。
- 不要顺手"修复" Update Center apply 后的刷新缺口（Out of Scope）。
- 按钮样式以 `CentralSkillsShell.tsx:373-407` 现有按钮为准，不引入新设计。
