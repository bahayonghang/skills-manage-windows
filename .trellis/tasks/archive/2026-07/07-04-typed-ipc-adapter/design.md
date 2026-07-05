# design: 按命令名类型化的 IPC adapter 与 fixture seam

> 依据 2026-07-05 代码勘查（数字均为当日 grep 实测）。prd.md 定义 What/验收，本文定 How。

## 0. 结论摘要

| 决策点               | 裁决                                                                                                                                                                                                                                                                                                                                        |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 存活入口             | 新建 `src/lib/ipc/` 目录，`index.ts` 统一 re-export；`src/lib/tauri.ts` 与旧 `src/lib/ipc.ts` 删除。import 说明符收敛为 `@/lib/ipc`                                                                                                                                                                                                         |
| 类型化方式           | `invoke` 双 overload：命令名 ∈ `IpcCommandMap` → 按名推导 args/result；其余命令走 `invoke<T>(command: string, args?)` 兼容 overload。存量 `invoke<T>("cmd")` 显式泛型调用自动落到兼容 overload，flip 时零调用点强改                                                                                                                         |
| fixture seam         | 真实 adapter（tauriInvoke）与 fixture adapter（命令名 → handler 注册表）实现同一条 `invoke` 路径，按 `isTauriRuntime()` 每次调用时路由。未注册命令在浏览器态 reject `IpcFixtureMissingError`（fail loud）                                                                                                                                   |
| fixture 数据位置     | 从 store 内联常量迁到 `src/fixtures/<domain>.ts`；`main.tsx` 静态 import + `installBrowserIpcFixtures()`（`!isTauriRuntime()` 守卫；静态 import 保证注册先于任何 store initialize，代价是桌面包多几 KB 常量，可接受）                                                                                                                       |
| 测试 seam            | `src/test/setup.ts` 把 `__TAURI_INTERNALS__.invoke` 换成命令路由 dispatcher（已验证 `@tauri-apps/api/core` 的 invoke 直通 `window.__TAURI_INTERNALS__.invoke`）。默认宽松（未注册命令 resolve `undefined`，与现状 `vi.fn()` 行为一致，存量测试零影响）；调用 `mockIpcCommands` 后该文件进入严格模式（未注册命令 reject 并列出已注册命令名） |
| guard 迁移批次       | 本任务批次 1 = 8 个 store + 2 个 hook + displayFont + ObsidianVaultView 归位；其余 guard 留批次 2/3（follow-up 任务，见 §7）                                                                                                                                                                                                                |
| snake→camel 载荷转换 | 不做（prd 约束）。自然落点：`invoke.ts` 的统一出入口处，未来加 per-command codec 即可                                                                                                                                                                                                                                                       |

## 1. 现状（2026-07-05 实测）

- 双 adapter：`src/lib/tauri.ts`（63 文件 import）vs `src/lib/ipc.ts`（5 文件 import，`IpcCommandMap` 仅 7 命令）。
- `isTauriRuntime()` 调用点 154 处 / 41 文件；fixture 数据（`BROWSER_FIXTURE*`）内联在 10 个 store 文件 + `platformPathPolicy.ts`。
- 测试：11 文件 `vi.mock("@tauri-apps/api/core")`，18 文件 `vi.mock("@/lib/tauri")`，`mockResolvedValueOnce` 376 处 / 30 文件；全仓仅 `OperationLogsView.test.tsx` 按命令名路由。
- `__TAURI_INTERNALS__` 直接触碰者仅 4 处：`setup.ts`（定义）、`tauri.test.ts` / `runtimeLogger.test.ts`（删除模拟浏览器）、`SkillDetailView.test.tsx`（临时置空）→ setup 换 dispatcher 影响面可控。
- 泄漏点确认：`useSkillCallCounts.ts:67`、`useSkillExplanationSummaries.ts:46`、`displayFont.ts:188-222`（get/set_setting）、`ObsidianVaultView.tsx:226`（组件直调 `invokeCommand("open_obsidian_path")`，唯一违反「组件不 invoke」规则者）。

## 2. 目标架构

```
src/lib/ipc/
  runtime.ts     isTauriRuntime + showMainWindowWhenReady + __resetMainWindowReadyForTest
  commandMap.ts  IpcCommandMap（按名类型规格，增量扩）+ UNTYPED_IPC_COMMANDS 允许清单
  fixtures.ts    fixture 注册表：registerIpcFixtures / registerUntypedIpcFixture /
                 clearIpcFixturesForTest / IpcFixtureMissingError
  invoke.ts      invoke（双 overload + failure recorder + real/fixture 路由）、invokeRaw、
                 listen（浏览器态 no-op unlisten）
  index.ts      公共面 re-export（唯一允许被外部 import 的路径）
src/fixtures/
  <domain>.ts    浏览器演示数据 + 该域命令 handler（从 store 搬出）
  index.ts       installBrowserIpcFixtures()：聚合注册
src/test/
  ipcMock.ts     mockIpcCommand / mockIpcCommands / ipcInvokeCalls / resetIpcMock
```

### invoke 签名（核心契约）

```ts
export function invoke<K extends keyof IpcCommandMap>(
  command: K,
  ...args: CommandArgs<K> extends undefined ? [] : [CommandArgs<K>]
): Promise<CommandResult<K>>;
export function invoke<T = unknown>(
  command: string,
  args?: unknown,
): Promise<T>;
```

- 兼容性：`invoke<BootstrapSnapshot>("get_bootstrap_snapshot")` 这类存量显式泛型调用不满足 `K extends keyof IpcCommandMap` 约束 → 落到第二 overload，行为不变。命令入 map 后，调用点去掉显式泛型即获得按名类型（迁移动作 = 删 `<T>`）。
- failure recorder 行为原样保留（`record_frontend_runtime_log` 除外），对 real 与 fixture 两条路径统一生效。
- `invokeRaw` 保留，仅供 `runtimeLogger.ts` 日志自举（防递归），在 `invoke.ts` 内注释注明例外理由。
- `listen`：包装 `tauriListen`，`!isTauriRuntime()` 时返回 `Promise.resolve(noop unlisten)`，消除调用方 listen guard 的必要性（批次外 store 的既有 guard 无害，后续批次顺手删）。

### 路由规则

`invoke` 每次调用时判定：`isTauriRuntime()` → `tauriInvoke`；否则查 fixture 注册表，命中执行 handler（支持异步），未命中 reject `IpcFixtureMissingError("no browser fixture for command X")`。store 的 catch 分支会把它落进 error state，浏览器演示中缺口可见、可定位。

### 调用方分层规则（写入 spec）

- pages/components：只许经 store（`ObsidianVaultView` 的 `open_obsidian_path` 移入 `obsidianStore`）。
- hooks / lib 工具：持有模块级缓存等不宜进 store 的语义时，允许直接用 `@/lib/ipc` 的类型化 `invoke`（本次的 2 个 hook 与 `displayFont` 即此类）。
- `runtimeLogger.invokeRaw`：唯一裸通道例外。

## 3. 入口收敛与迁移策略

一次性 flip（纯机械，sed 级）：

| 动作                                            | 面                                                                                     |
| ----------------------------------------------- | -------------------------------------------------------------------------------------- |
| `from "@/lib/tauri"` → `from "@/lib/ipc"`       | 63 文件（src + test）                                                                  |
| `vi.mock("@/lib/tauri"` → `vi.mock("@/lib/ipc"` | 18 测试文件（factory 形状不变：mock 的 export 名 invoke/isTauriRuntime/listen 均保留） |
| `invokeCommand(` → `invoke(`                    | 4 个 prod 文件 + `tauri.test.ts`（调用形状相同：同为按命令名 + 单参数对象）            |
| 删除 `src/lib/tauri.ts`、`src/lib/ipc.ts`       | —                                                                                      |

注意顺序：`src/lib/ipc.ts` 文件与 `src/lib/ipc/` 目录不能并存（resolver 歧义）。先建目录版并同步删旧文件、迁其 5 个 importer；`lib/tauri.ts` 可短暂并存至 flip 步删除。

`vi.mock("@tauri-apps/api/core")` 的 11 个测试文件不受影响（拦截在 adapter 之下，继续工作），按批次逐步退役。

## 4. 类型覆盖推进策略与硬指标

- **本任务硬指标**：`IpcCommandMap` ≥ 40 命令，必须覆盖：既有 7 命令 + 批次 1 全部 8 store 的真实路径命令 + 2 hook 命令 + `get_setting`/`set_setting` + `open_obsidian_path`。批次 1 迁移过的文件内不得再出现 `invoke<T>(` 显式泛型写法（grep 复核）。
- **ratchet 机制**：`src/test/ipcCommandCoverage.test.ts` 用 node fs 扫描 `src/**/*.{ts,tsx}` 中 `invoke("cmd"` / `invoke<`…`>("cmd"` 字面量，断言每个命令名 ∈ `IpcCommandMap` ∪ `UNTYPED_IPC_COMMANDS`。效果：新命令必须显式登记（入 map 或进允许清单），允许清单只减不增成为后续批次的可量化进度条；同时兜住命令名 typo。
- **后续批次目标**（不在本任务）：批次 2/3 随 guard 迁移把对应域命令入 map，终态 map 覆盖 171 命令、删除兼容 overload 与允许清单。

## 5. fixture seam

- `src/fixtures/<domain>.ts`：platform / skills / usage / targets / operationLog / runtimeLog / tagGroups / savedViews / settings（get_setting→null、set_setting→no-op，服务 displayFont 与各 store 的偏好读写）/ misc（usage_get_skill_counts→{}、get_skill_explanation_summaries→{} 等 hook 命令）。数据从 store 原样搬出（`BROWSER_FIXTURE_*`、`platformPathPolicy.BROWSER_PLATFORM_PATHS` 引用保留原语义）。
- store 改造后浏览器演示走**真实 store 逻辑 + fixture 命令响应**（如 platformStore 的 `hydrateShell` 由三条 fixture 命令组装，而非整块内联 state），演示保真度提升。
- 写类命令（set_setting、set_agent_enabled 等）fixture 返回成功值/回显，使乐观更新路径可演示。
- **安全网**：`src/test/browserFixtures.test.ts` —— 删除 `__TAURI_INTERNALS__`/`__TAURI__` 后 `installBrowserIpcFixtures()`，逐一驱动批次 1 store 的主加载动作，断言 resolve 且关键 state 非空、`error === null`。该测试同时吸收 usageStore.test 等文件中现存的「浏览器模式」describe 块（原位删除）。

## 6. 测试基建

`src/test/ipcMock.ts`：

```ts
mockIpcCommand(command, handler | value)   // 注册单命令；进入严格模式
mockIpcCommands({ cmd: handler | value })  // 批量注册
ipcInvokeCalls(command?)                   // [{ command, args }] 调用记录，供断言
resetIpcMock()                             // setup.ts 全局 afterEach 自动调用
```

- dispatcher 装在 `setup.ts` 的 `__TAURI_INTERNALS__.invoke`；宽松/严格语义见 §0。调用记录含宽松模式，`toHaveBeenNthCalledWith` 类断言迁移为 `ipcInvokeCalls("cmd")` 形式（次序无关，按命令名取）。
- **必迁测试**（验收：文件内 `mockResolvedValueOnce` = 0）：`platformStore.test.ts`（prd 点名的 ~18 连顺序链）、`skillStore.test.ts`、`usageStore.test.ts`、`useSkillCallCounts.test.ts`、`useSkillExplanationSummaries.test.tsx`；`tauri.test.ts` 改写为 `ipc.test.ts`（吸收 failure-recorder 用例 + 新增 fixture 路由 / 缺 fixture 报错 / listen no-op / 双 overload 编译期用例）。
- 其余批次 1 store 测试（target/operationLog/runtimeLog/tagGroups/savedViews）：guard 剥除不改真实路径 invoke 次序，存量顺序桩继续绿，**不强迁**（顺手迁为 stretch）；其余 25+ 文件留批次 2/3。

## 7. guard 迁移批次

**批次 1（本任务，~48 处 guard + 泄漏点归位）**：
`platformStore`(5)、`skillStore`(4)、`usageStore`(8, 含 listen)、`targetStore`(11)、`operationLogStore`(5)、`runtimeLogStore`(4)、`tagGroupsStore`(6)、`savedViewsStore`(5)；hooks×2、`displayFont`(2)、`ObsidianVaultView` 归位进 obsidianStore。

**批次 2/3（follow-up，完成后在父任务登记新子任务）**：settingsStore(+aiSlice)(17)、skillDetailStore(12)、projectsStore(11)、centralSkillsStore 4 slices(20)、marketplaceStore 4 slices(12)、collectionStore(3)、obsidianStore(2)、appUpdateStore(2)、localRemoteSyncStore(1)、updateCenterStore(8)、组件/viewmodel 层 guard（约 10）、剩余测试文件顺序桩退役、map 补全至 171。

## 8. 验收数值（grep 复核表）

| 检查                                                     | 目标                                                                                          |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `from "@/lib/tauri"` / `src/lib/tauri.ts`                | 0 处 / 文件不存在                                                                             |
| `invokeCommand`                                          | 0 处                                                                                          |
| `IpcCommandMap` 命令数                                   | ≥ 40                                                                                          |
| `isTauriRuntime()` 调用点                                | 全仓 ≤ 100（基线 154）；批次 1 的 12 个文件内 = 0（`src/lib/ipc/` 自身与 runtimeLogger 除外） |
| 必迁测试文件内 `mockResolvedValueOnce`                   | 0                                                                                             |
| `ipcCommandCoverage.test.ts` / `browserFixtures.test.ts` | 存在且绿                                                                                      |
| `pnpm test` / `pnpm typecheck` / `pnpm lint` / `just ci` | 全绿                                                                                          |

## 9. 风险与对策

| 风险                                                 | 对策                                                                            |
| ---------------------------------------------------- | ------------------------------------------------------------------------------- |
| setup dispatcher 改动破坏依赖裸 `vi.fn()` 的存量测试 | 宽松模式默认 resolve `undefined` 与现状一致；flip 步后立刻全量 `pnpm test` 定位 |
| sed 漏改 / vi.mock 路径漏改                          | 验收表 grep = 0 强校验；TS 编译对缺失模块报错兜底                               |
| fixture 覆盖缺口导致浏览器演示回归                   | fail-loud `IpcFixtureMissingError` + `browserFixtures.test.ts` 常驻安全网       |
| 双 overload 对 union 命令名/条件 spread 的类型边界   | `ipc.test.ts` 附编译期断言用例（`@ts-expect-error` 反例）先行验证               |
| usageStore listen 语义变化                           | listen 包装保持签名不变；浏览器 no-op 行为在 ipc.test 有专测                    |

## 10. 回滚

每步一提交（见 implement.md），任一步失败 `git revert` 该步即可；flip 步为纯路径替换，可逆。数据库/后端零改动，无运行时迁移，回滚无残留。

## 11. 兼容性

- 后端 IPC 命令契约零改动；前端对外行为唯一变化 = 浏览器演示由「store 内联分支」变为「fixture 命令响应 + 真实 store 逻辑」，桌面（Tauri）路径字节级等价。
- `@/lib/ipc` 说明符对既有 5 个 importer 保持可解析（目录 index 接管）。
