# Design

## Change List

| 文件 / 符号 | 计划变更 | 追溯 |
| --- | --- | --- |
| `src/components/layout/GlobalSearchDialog.tsx` / collection `SearchItem.onSelect` | 导航到 `/collections`，传入 `{ collectionContext: { collectionId } }`；订阅两个来源的 loaded/loading/error 并在打开时触发缺失加载 | R1-R3, R6 |
| `src/pages/CollectionsListView.tsx` / `locationCollectionContext`、`selectedId` initializer | 保持现有 collection context 为唯一入口；只补回归，不新增路由分支 | R1 |
| `src/stores/collectionStore.ts` / `CollectionState`、`loadCollections` | 增加明确的列表成功加载事实；成功（包括空数组）置为 loaded，失败保留可重试状态 | R2-R3 |
| `src/stores/collectionStore.ts` / `loadCollectionDetail`、`addSkillToCollection`、`removeSkillFromCollection` | 用一个闭包级单调 `detailRequestId` 与当前 `detailTargetId` 门控详情写入；mutation 仅在目标仍是自己的 collection 时复用同一详情 loader | R4-R5 |
| `src/stores/centralSkillsStore.types.ts`、`src/stores/centralSkillsStore.shared.ts`、`src/stores/centralSkillsStore.listSlice.ts` / Central list state | 增加与既有 latest-wins loader 同步的“已成功加载”事实，并在 target reset 时复位 | R2-R3 |
| `src/i18n/locales/en.json`、`src/i18n/locales/zh.json` / `globalSearch.*` | 增加 loading/error/empty/retry 文案并保持 key parity | R3, R6 |
| `src/test/components/layout/GlobalSearchDialog.test.tsx`、`src/test/pages/CollectionsListView.test.tsx` | 覆盖 registered route、history state、冷启动、空/错/重试 | AC1-AC8, AC15-AC16 |
| `src/test/stores/collectionStore.test.ts` | 用按命令名 mock 与 deferred promise 覆盖详情及 mutation 竞争 | AC9-AC14 |

## Navigation Contract

`GlobalSearchDialog` 只产生现有 `/collections` history entry；`CollectionsListView` 继续是 collection 选择和详情渲染的唯一所有者。搜索不读取 URL 参数，也不增加旧 `/collection/:id` alias。该契约直接满足 R1，并由 AC1-AC3 验证。

## Loading Contract

- 每个来源增加一个“至少成功完成一次列表加载”的布尔事实；数组长度不能替代该事实。
- dialog 从 closed 变为 open 时，仅当 `!hasLoaded && !isLoading` 才调用既有 loader；React StrictMode 或重复 effect 不得并发重复加载。
- loader 成功返回空数组仍置 loaded；失败保持未加载并记录 error，因此 Retry 可再次调用。
- UI 按来源判定：loading 优先，未加载且 error 显示 error/retry，loaded 且过滤结果为空才显示 empty。
- 组件继续只调用 Zustand action，不直接 `invoke`。

这组机制覆盖 R2、R3、R6；AC4-AC8、AC15-AC16 分别验证调用次数和可见状态。

## Concurrency Contract

- `loadCollectionDetail(id)` 先把 `detailTargetId` 设为 `id`，再分配递增的 `detailRequestId`；data、error、loading completion 都要求 request id 与 target 同时仍匹配。
- A 后 B、B 先返回时，A 的成功和失败都被丢弃，且不能把 B 的 `isLoadingDetail` 置 false。
- mutation 主命令完成后先比较 `detailTargetId`：不再等于 mutation collection 时跳过详情刷新；仍相等时调用同一个 `loadCollectionDetail`，不保留第二条直接 `get_collection_detail` 通道。
- mutation 主命令本身的失败仍按现有 action 语义 rethrow；本任务只门控后续详情响应，不隐藏真实 mutation 失败。

该契约覆盖 R4、R5，并由 AC9-AC14 验证。

## Compatibility

- `/collections`、`location.state.collectionContext`、Zustand action 名称和 Tauri 命令不变。
- 不提供旧 `/collection/:id` 兼容层；该路径从未注册为生产路由。
- 新 loaded 字段只属于内存态，不涉及持久化或迁移。

## Verification Boundary

- Vitest/MemoryRouter 能证明路由 state、loader 调用、可见状态和 Promise 时序。
- `pnpm typecheck` 与 `pnpm lint` 证明类型和静态规则；`just ci` 是总门禁。
- jsdom 不能证明 Windows WebView2 的键盘导航、焦点恢复、真实 IPC 时序或人工可用性；这些在实际桌面 smoke 前必须报告 `UNVERIFIED`。

## Rollback

| 回滚单元 | 包含内容 | 回滚点 |
| --- | --- | --- |
| A | 路由 state 修复及 MemoryRouter 测试 | AC1-AC3 定向测试通过后形成独立点 |
| B | 两个列表 loaded 状态、dialog 加载 UI、i18n | AC4-AC8、AC15-AC16 通过后形成独立点；不与 C 混合回滚 |
| C | collection 详情单通道 latest-wins 与 mutation refresh | AC9-AC14 通过后形成独立点 |

若任一单元失败，只回滚该单元及其测试；不得为恢复旧行为新增 route alias 或第二条 refresh 通道。

## Considered but Not Chosen

- 不新增 `/collection/:id`：会制造两个详情所有者和兼容面。
- 不在 AppShell 启动时无条件预加载：搜索打开时按 loaded 状态加载足以满足需求，也避免扩大启动成本。
- 不引入 AbortController 或请求库：当前 Tauri promise 只需提交门控，单调 id 是更小的完整方案。
- 不以 `array.length` 推断 loaded：真实空集合必须与未加载区分。
