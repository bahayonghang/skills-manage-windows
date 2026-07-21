# Design: Central Skills 刷新按钮与检查后自动刷新

## 总览

纯前端改动，涉及四层：store（listSlice 错误契约 + 刷新态）、actions hook（手动刷新编排）、controller（检查后自动刷新）、shell（按钮 UI）。后端不动。

## D1 错误传播契约：`loadCentralSkills` 可选 rethrow

**问题**：`loadCentralSkills()` 吞错只写 store error（listSlice.ts:66-70），约 15 个既有调用点（多为 fire-and-forget）依赖此语义；直接改成 rethrow 会产生大量 unhandled rejection。

**方案**：扩展签名，默认行为不变：

```ts
// centralSkillsStore.types.ts
loadCentralSkills: (options?: { throwOnError?: boolean }) => Promise<void>;
```

- catch 分支维持 `set({ error, isLoading/isRefreshingList: false })`；仅当 `options?.throwOnError === true` 时额外 `throw err`。
- 无参/无选项调用 = 既有行为，所有既有调用点与 mock 不受影响（`vi.fn()` 兼容多余实参）。
- 与 `.trellis/spec/frontend/async-error-feedback.md` 的"store action 负责 rethrow、可见 UI 调用方负责 toast"契约对齐，且由调用方显式选择。

## D2 刷新态契约：`isRefreshingList`，刷新保留旧内容

**问题**：`isLoading: true` 会让 `CentralSkillListContent.tsx:169` 把整个列表替换成加载空态；手动/自动刷新时闪空态不可接受。

**方案**：listSlice 新增状态 `isRefreshingList: boolean`（初始 false，加入 `CentralSkillsState` 与初始 state）。

`loadCentralSkills` 开始时的分流：

- store 中 `skills.length === 0`（首次/空库加载）→ `set({ isLoading: true, ... })`，维持现有空态行为。
- 已有数据 → `set({ isRefreshingList: true, error: null })`，`isLoading` 保持 false，列表内容保留。
- 完成/失败分支对应清除自己设置的那个标志。

按钮 spinner 绑定 `isRefreshingList`；`disabled = isRefreshingList || isLoading`。

## D3 手动刷新编排：并行 + 部分失败策略

**问题**：现有 `handleRefresh` 串行 `await refreshCounts(); await loadCentralSkills();`——计数失败（会 rethrow）阻断列表刷新；列表失败（不 rethrow）无反馈。

**方案**（实现在 `src/components/central/useCentralRefreshButton.ts` 的 hook 内；最初落在 `centralSkillsActions.ts` 的 `handleRefresh`，后因 D6 接线方式调整随按钮一起迁入 hook，actions 内不再保留）：

```ts
async function handleRefresh() {
  const [listResult, countsResult] = await Promise.allSettled([
    loadCentralSkills({ throwOnError: true }),
    refreshCounts(),
  ]);
  const failure = listResult.status === "rejected" ? listResult.reason
    : countsResult.status === "rejected" ? countsResult.reason
    : null;
  if (failure) {
    toast.error(t("central.refreshError", { error: String(failure) }));
  }
}
```

- 两者并行，互不阻断（解决计数阻断列表的问题）。
- 列表失败优先上报；仅计数失败也报 `central.refreshError`（对用户来说就是刷新失败）。
- 成功路径无 toast（与页面其他刷新惯例一致）。

## D4 控制器成功边界：检查成败只由 inventory 决定

**方案**（`centralUpdateCheckModeController.tsx` 的 `handleConfirm`）：新增 `loadCentralSkills` selector，嵌套 try：

```ts
try {
  const scope = buildUpdateCheckScope(mode, checkButtonState);
  const inventory = await refreshUpdateInventory(scope);   // 唯一决定检查成败
  try {
    await loadCentralSkills({ throwOnError: true });       // 后续同步步骤
  } catch (listErr) {
    toast.error(t("central.refreshError", { error: String(listErr) }));
  }
  openUpdateCenter(preferredUpdateCenterTab(inventory),   // 参数不变
    buildUpdateCheckRefreshContext(scope, checkButtonState));
  setOpen(false);
} catch (err) {
  // 既有分支不变：central.updateCheckError + inline error + 不打开
}
```

- 列表重取失败绝不进入外层 catch，不会误报 `central.updateCheckError`，不影响 `openUpdateCenter` 的参数与时机。
- 先重取列表再打开 Update Center：Update Center 从 centralSkillsStore 读数据，保证打开即见新状态；列表重取是本地 DB 读，延迟可忽略。

## D5 latest-wins 并发防护

**问题**：generation 只在 `updateSlice.ts:514` bump，重叠的 `loadCentralSkills` 共享 generation，后完成者胜出（可能是旧请求）。

**方案**：listSlice 模块级计数器：

```ts
let loadRequestId = 0;
// loadCentralSkills 内：
const requestId = ++loadRequestId;
// 每个 set 前判断：requestId === loadRequestId && generation === getGeneration()
```

- 仅最新请求可写 state；被覆盖请求的 set 全部丢弃。
- rethrow 不受 requestId 门控：调用方选了 `throwOnError` 就拿自己这次请求的真实结果（即使其 state 写入被丢弃）。极端情况下自动刷新被手动刷新覆盖、自动那次失败仍会 toast，可接受且不误导。
- 按钮 disabled 已防重复点击；本防护覆盖手动 vs 自动 vs 挂载加载的重叠。

## D6 按钮 UI

- 位置：`CentralSkillsShell.tsx` header 工具栏，"Update Center" 按钮（:373-383）之后、`{checkModeControl}` 之前。
- 形式：`variant="outline"` 图标按钮（`h-9 w-9 rounded-xl`，与工具栏按钮高度一致），`RefreshCw` 图标 + `cn("size-3.5", refreshing && "animate-spin")`（沿用本页 Check 按钮惯例）。
- `title` + `aria-label` = `t("central.refresh")`；`data-testid="central-refresh-skills"`。
- 传参：shell 组件内经 `useCentralRefreshButton` hook 装配（hook 内 `useTranslation()`，从 `useCentralSkillsStore` 选 `isRefreshingList/isLoading/loadCentralSkills`，从 `usePlatformStore` 选 `refreshCounts`，D3 编排除在 hook 的 `onClick` 中），返回 `{ refreshing, disabled, onClick }`；`CentralSkillsView.tsx` 与 `centralSkillsViewModel.ts` 零改动（sizecheck 冻结基线 865 约束：view 不得再增长）。shell 已有组件内直接用 store 的先例（`useUpdateCenterStore`）。

## 数据流

```
手动: 按钮 → useCentralRefreshButton.onClick → Promise.allSettled([loadCentralSkills({throwOnError}), refreshCounts()])
        → 失败 → toast central.refreshError
自动: Start check → refreshUpdateInventory(scope) ─失败→ updateCheckError（不变）
        └成功→ loadCentralSkills({throwOnError}) ─失败→ toast central.refreshError（仍打开）
               → openUpdateCenter(原参数) → setOpen(false)
```

## i18n

不新增 key。复用：`central.refresh`（en.json:648 / zh.json:648）、`central.refreshError`（en.json:790 / zh 对应行）、`central.updateCheckError`（en.json:1104）。实施时确认 zh.json 三个 key 均存在。

## 测试设计

- `src/test/centralSkillsStore.test.ts`：
  - 无参 `loadCentralSkills()` 失败仍只写 error 不抛（既有语义回归）。
  - `{ throwOnError: true }` 失败时 rethrow 且仍写 store error。
  - 已有数据时调用走 `isRefreshingList`（isLoading 保持 false，skills 在飞行中保留）；空数据走 `isLoading`。
  - 两个重叠请求，后到者结果生效（latest-wins）。
- `src/test/CentralSkillsView.shell.test.tsx`：按钮渲染、点击触发 `loadCentralSkills` + `refreshCounts`、刷新中 disabled。
- `src/test/CentralSkillsView.updates-and-search.test.tsx`：
  - 检查成功 → `loadCentralSkills` 在 `openDialog` 前被调用，列表/updateStatuses 可见更新（mock invoke 返回变化后的数据，断言 UI 结果而非仅断言调用）。
  - 检查成功 + 列表重取 reject → Update Center 仍打开、toast `central.refreshError`、无 `central.updateCheckError`。
  - 检查失败 → 既有行为不变（updateCheckError、不打开）。
  - 列表重取失败（手动按钮）→ toast `central.refreshError`；计数失败不阻断列表重取。
  - 刷新中再次点击不触发第二次请求。
- `src/test/centralSkillsViewTestSupport.tsx`：central store mock 补 `isRefreshingList` 字段；`mockLoadCentralSkills` 支持 resolve/reject 配置。
