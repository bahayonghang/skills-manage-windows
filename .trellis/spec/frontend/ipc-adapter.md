# 前端 IPC adapter 与 fixture seam 约定

> 建立于 2026-07-05（任务 07-04-typed-ipc-adapter）。背景：曾并存两个 invoke adapter（`lib/tauri.ts` 63 文件 import / 旧 `lib/ipc.ts` 5 文件 import），浏览器演示态靠 154 处 `isTauriRuntime()` 内联分支返回假数据（绕过真实 store 逻辑），测试靠 376 处 `mockResolvedValueOnce` 顺序桩（对 invoke 次序脆弱）。本约定建立单一类型化入口 + 命令级 fixture + 命令路由测试 mock。

## 约定 1：`@/lib/ipc` 是唯一 IPC 入口

**What**：所有前端代码只从 `@/lib/ipc`（目录 `src/lib/ipc/`，`index.ts` re-export）import IPC 能力：`invoke` / `invokeRaw` / `listen` / `isTauriRuntime` / `registerIpcFailureRecorder` / `showMainWindowWhenReady` / fixture API。禁止直接 import `@tauri-apps/api/core` 或 `@tauri-apps/api/event`（adapter 内部除外）。

**例外（仅此一个）**：`src/lib/runtimeLogger.ts` 使用 `invokeRaw` 落盘前端运行时日志 —— 它是 failure recorder 本体，走 `invoke` 会在自身失败时递归记录。新代码不得再增加 `invokeRaw` 调用方。

**Wrong vs Correct**：

```ts
// ❌ Wrong：绕过 adapter 直连 Tauri
import { invoke } from "@tauri-apps/api/core";

// ✅ Correct：统一入口（浏览器演示态自动路由 fixture）
import { invoke, listen } from "@/lib/ipc";
```

## 约定 2：命令按名类型化（双 overload + 覆盖率 ratchet）

**What**：`src/lib/ipc/commandMap.ts` 的 `IPC_COMMANDS` 按命令名登记 args/result 类型（`command<Args, Result>()` 幻影值模式，单一来源同时供类型推导与运行时枚举）。`invoke` 双 overload：命令 ∈ map → 按名推导，无需写泛型；未类型化命令走 `invoke<T>(command, args?)` 兼容 overload。

**新增 IPC 命令的操作**：优先在 `IPC_COMMANDS` 加一行类型化条目；确有理由暂缓时登记进 `UNTYPED_IPC_COMMANDS` 允许清单。`src/test/contracts/ipcCommandCoverage.test.ts` 强制：全仓 invoke 字面量 ∈ map ∪ 清单、清单零僵尸条目、命令类型化后必须离开清单（只减不增）、map ≥ 40。

**Wrong vs Correct**：

```ts
// ❌ Wrong：已入 map 的命令还写显式泛型（掩盖类型漂移）
const skills = await invoke<ScannedSkill[]>("get_skills_by_agent", { agentId });

// ✅ Correct：按名推导（map 中 get_skills_by_agent 已定义返回类型）
const skills = await invoke("get_skills_by_agent", { agentId });
```

## 约定 3：浏览器演示态 = fixture 命令响应 + 真实 store 逻辑

**What**：浏览器（非 Tauri）运行时，`invoke` 按命令名路由到 `src/fixtures/<domain>.ts` 注册的 handler；store/hook 内**禁止**写 `isTauriRuntime()` 分支返回内联假数据。`main.tsx` 在 `!isTauriRuntime()` 时渲染前调用 `installBrowserIpcFixtures()`（静态 import，保证注册先于任何 store initialize）。

**行为语义**：

- 未注册命令 reject `IpcFixtureMissingError`（fail loud，缺口可定位；`src/test/fixtures/browserFixtures.test.ts` 安全网驱动各 store 主加载路径常驻验证）。
- 桌面限定操作（卸载、SSH/WSL 目标创建等）在 fixture 侧 reject **原字符串**（Tauri 命令错误即 string），store `String(err)` 后 error 文案与桌面一致。
- 有状态域（tagGroups / savedViews）用模块级内存数据集实现 CRUD，store 仍按真实路径 refetch。

**遗留（批次 2/3）**：存量 guard 仍留在 settingsStore、skillDetailStore、projectsStore、centralSkillsStore、marketplaceStore、collectionStore、updateCenterStore、appUpdateStore、localRemoteSyncStore 及组件/viewmodel 层（合计约 100 处）；迁移时逐 store 走「fixture 注册 → 剥 guard → 测试迁命令路由」三步，禁止新增内联 fixture 分支。

## 约定 4：测试按命令名 mock，不按调用次序打桩

**What**：`src/test/support/setup.ts` 已把 `__TAURI_INTERNALS__.invoke` 换成命令路由 dispatcher。测试用 `src/test/support/ipcMock.ts` 的 `mockIpcCommand(command, handlerOrValue)` / `mockIpcCommands(map)` 注册响应，用 `ipcInvokeCalls(command)` / `ipcInvokedCommands()` 断言调用 —— 与 invoke 发生次序解耦。

**模式语义**：注册过任一 handler 的测试进入严格模式（未注册命令 reject 并列出已注册命令名）；未注册任何 handler 时宽松（resolve `undefined`，兼容存量顺序桩文件）。`afterEach` 全局自动 `resetIpcMock()`。

**Wrong vs Correct**：

```ts
// ❌ Wrong：顺序桩链（invoke 次序一变全链错位）
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mocked(invoke).mockResolvedValueOnce(agents).mockResolvedValueOnce(counts);

// ✅ Correct：命令路由
mockIpcCommands({ get_bootstrap_snapshot: snapshot, scan_all_skills: counts });
await usePlatformStore.getState().initialize();
expect(ipcInvokeCalls("scan_all_skills")).toHaveLength(1);
```

## 约定 5：调用方分层不变

store 是唯一 invoke 层（`src/stores/` 与少数 lib/hook 基础设施）；组件/页面不得直接 `invoke`，需要 IPC 动作时下沉为 store action（先例：`ObsidianVaultView` 的 `open_obsidian_path` → `obsidianStore.openObsidianPath`）。`listen` 包装在浏览器态返回 no-op unlisten，调用方无需再包 runtime guard。
