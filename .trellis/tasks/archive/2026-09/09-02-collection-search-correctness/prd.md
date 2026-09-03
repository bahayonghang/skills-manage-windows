# Collection 搜索与详情一致性

## Goal

修复全局搜索进入不存在路由、未加载数据导致漏搜，以及 collection 详情请求乱序覆盖的问题。

## Findings

- `FE-CORR-001`（High / S）：`src/components/layout/GlobalSearchDialog.tsx:142-159` 导航到 `/collection/:id`，但 `src/App.tsx:119-123` 仅注册 `/collections`，命中后得到空 Outlet。
- `FE-CORR-002`（High / M）：`src/stores/collectionStore.ts:177-203` 没有 latest-wins 保护，A/B 请求乱序会让旧详情覆盖新选择，mutation 后 refresh 也可覆盖。
- `FE-CORR-003`（Medium / M）：`src/components/layout/GlobalSearchDialog.tsx:66-120` 只搜索当前 store；`src/components/layout/AppShell.tsx:76-85` 启动时未加载 collections/Central，首次打开会静默漏结果。

## Requirements

- R1: Collection 搜索结果必须复用已注册的 `/collections` 路由，并通过现有 `location.state.collectionContext.collectionId` 传递选择；不得新增 `/collection/:id`、兼容跳转或第二个详情所有者。
- R2: `useCollectionStore` 与 `useCentralSkillsStore` 必须分别暴露可区分“从未成功加载”和“已成功加载为空”的列表加载事实；dialog 打开时仅对未加载且非加载中的来源调用既有 loader。
- R3: 搜索界面必须分别呈现 loading、load error、loaded-empty 三类可观察状态；加载失败不得退化为“无结果”。
- R4: `loadCollectionDetail` 必须以单一、单调请求所有权门控 `currentDetail`、详情 error 和 `isLoadingDetail`；只有当前请求可以写入或结束当前 loading。
- R5: `addSkillToCollection` 与 `removeSkillFromCollection` 的后续详情刷新必须复用 R4 的门控；当当前详情目标已切换到其他 collection 时，旧 mutation 不得发起会夺回所有权的刷新。
- R6: 搜索加载失败必须可重试，所有新增可见状态与错误提示必须通过 `src/i18n/locales/en.json` 和 `src/i18n/locales/zh.json`，组件不得直接调用 Tauri IPC。
- R7: 实现只修改现有路由、store、dialog 与其测试，不引入搜索依赖、索引服务或新 URL surface。

## Acceptance Criteria

- [ ] AC1 (R1): 选择任一 collection 搜索结果后，MemoryRouter 的当前位置是 `/collections`。
- [ ] AC2 (R1): AC1 的 history state 精确包含所选 `collectionId`。
- [ ] AC3 (R1): `CollectionsListView` 首次渲染选择 AC2 指定的 collection。
- [ ] AC4 (R2): 冷启动首次打开 dialog 时，collections 与 Central 两个 loader 对各自来源各调用一次。
- [ ] AC5 (R2): 已成功加载为空后关闭并重开 dialog，不因数组为空重复调用 loader。
- [ ] AC6 (R3, R6): 任一来源加载失败时显示其本地化错误和重试入口。
- [ ] AC7 (R3): 加载失败的来源不显示“无结果”状态。
- [ ] AC8 (R3): 两个来源均成功且结果确实为空时显示 loaded-empty，而不是 loading 或 error。
- [ ] AC9 (R4): deferred promise 回归证明先请求 A 再请求 B 且 B 先返回时，最终 `currentDetail` 为 B。
- [ ] AC10 (R4): A 的迟到成功不能改写 B 的详情数据。
- [ ] AC11 (R4): A 的迟到失败不能改写 B 的详情错误状态。
- [ ] AC12 (R4): A 的迟到完成不能结束 B 拥有的 `isLoadingDetail`。
- [ ] AC13 (R5): mutation A 的刷新与用户切换 B 并发时，最终 `currentDetail` 为 B。
- [ ] AC14 (R5): mutation A 在目标切换后不会重新夺取详情请求所有权。
- [ ] AC15 (R6): 英中 locale key parity 通过。
- [ ] AC16 (R6): 新增错误状态可从同一界面重试并成功进入 loaded 状态。
- [ ] AC17 (R1, R2, R3, R4, R5, R6, R7): 本任务列出的定向 Vitest 全部通过。
- [ ] AC18 (R1, R2, R3, R4, R5, R6, R7): `pnpm typecheck` 通过。
- [ ] AC19 (R1, R2, R3, R4, R5, R6, R7): `pnpm lint` 通过。
- [ ] AC20 (R1, R2, R3, R4, R5, R6, R7): `just ci` 通过。
- [ ] AC21 (R1, R3): Windows WebView2 人工检查覆盖 collection 搜索的键盘选择和焦点恢复。
- [ ] AC22 (R1, R3): Windows WebView2 的真实 Tauri 数据加载在人工执行前保持 `UNVERIFIED`，不得由 jsdom 测试替代。

## Out of Scope

- 新增 collection 详情 URL 或历史兼容跳转。
- 重做全局搜索索引或引入搜索依赖。
- 改变 collection 后端命令、数据库 schema 或 Central 数据语义。
