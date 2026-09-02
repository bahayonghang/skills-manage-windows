# 前端 IPC adapter 与 fixture seam 约定

> 建立于 2026-07-05（任务 07-04-typed-ipc-adapter）。背景：曾并存两个 invoke adapter（`lib/tauri.ts` 63 文件 import / 旧 `lib/ipc.ts` 5 文件 import），浏览器演示态靠 154 处 `isTauriRuntime()` 内联分支返回假数据（绕过真实 store 逻辑），测试靠 376 处 `mockResolvedValueOnce` 顺序桩（对 invoke 次序脆弱）。本约定建立单一类型化入口 + 命令级 fixture + 命令路由测试 mock。

## 约定 1：`@/lib/ipc` 是唯一 IPC 入口

**What**：所有前端代码只从 `@/lib/ipc`（目录 `src/lib/ipc/`，`index.ts` re-export）import IPC 能力：`invoke` / `invokeRaw` / `listen` / `UnlistenFn` / `isTauriRuntime` / `registerIpcFailureRecorder` / `showMainWindowWhenReady` / fixture API。禁止直接 import `@tauri-apps/api/core` 或 `@tauri-apps/api/event`。

生产源码扫描 `@tauri-apps/api/event` 的匹配集合必须**精确等于** `src/lib/ipc/invoke.ts`（范围 `src/**/*.{ts,tsx}`，排除 `src/test/**`）。不得把 invariant 写成“生产为 0”——adapter 必须直连 Tauri。`UnlistenFn` 由 `invoke.ts` 再经 `index.ts` 公开；adapter 外的生产文件只从 `@/lib/ipc` 取该类型。`src/test/contracts/frontendArchitectureContract.test.ts` 锁定此 allowlist。

**例外（仅此一个）**：`src/lib/runtimeLogger.ts` 使用 `invokeRaw` 落盘前端运行时日志 —— 它是 failure recorder 本体，走 `invoke` 会在自身失败时递归记录。新代码不得再增加 `invokeRaw` 调用方。

**Wrong vs Correct**：

```ts
// ❌ Wrong：绕过 adapter 直连 Tauri
import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";

// ✅ Correct：统一入口（浏览器演示态自动路由 fixture；类型也走 adapter）
import { invoke, listen, type UnlistenFn } from "@/lib/ipc";
```

## 约定 2：命令按名类型化（无 untyped 回退）

**What**：`src/lib/ipc/commandMap.ts` 的 `IPC_COMMANDS` 由 generated map ∪ handwritten map 组成（`command<Args, Result>()` 幻影值）。`invoke` / `invokeRaw` 只接受 `keyof IpcCommandMap`：命令名、args、result 均按名推导。禁止生产 string/generic fallback，也禁止 `UNTYPED_IPC_COMMANDS` 允许清单。

**新增 IPC 命令的操作**：在 `ipc_registry.rs::__skillport_generated_commands` 登记既有 Rust command path，给实际 boundary DTO 补最小 `specta::Type`，运行 `pnpm ipc:codegen`。不得手写平行 schema，不得恢复 string overload。`src/test/contracts/ipcCommandCoverage.test.ts` 强制：全仓 invoke 字面量 ∈ generated ∪ handwritten、两 map 无重叠、runtime−frontend 精确等于 backend-only、generated args/result 不含 `unknown`、map ≥ 40。

**Wrong vs Correct**：

```ts
// ❌ Wrong：显式泛型或任意 string 掩盖漂移
const skills = await invoke<ScannedSkill[]>("get_skills_by_agent", { agentId });
await invoke("not_a_real_command" as string, {});

// ✅ Correct：按名推导
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

## Scenario: Rust-Derived Contracts And Structured Rejections

### 1. Scope / Trigger

- 修改 generated command、IPC rejection、fixture failure 或错误 code 分支时适用。

### 2. Signatures

```ts
type IpcErrorPayload = { code: string; message: string; retryable: boolean; correlationId?: string };
class IpcInvokeError extends Error { code: string; retryable: boolean; correlationId?: string }
normalizeIpcRejection(error: unknown): Error
failureRecorder(command, sanitizedArgs, normalizedError): void
```

typed、generated 与 backend-only command 的当前数量由 `ipcCommandCoverage.test.ts` 从 registry 和
command map 计算，不在本规范复制快照。生产路径不再有 untyped allowlist。

### 3. Contracts

- 已有 `IpcInvokeError` 必须原实例透传；有效对象载荷包装为 `IpcInvokeError`，合法 UUID correlationId 原样保留。
- strict legacy `code:message` 仅用于过渡；未知 rejection 使用固定
  `internal.unexpected`，不得暴露 raw transport 值。
- `String(error)` 只返回 public message；行为分支读取 `code`，不得嗅探 message。
- failure recorder 对每个 rejection 独立写入；一个 pending/失败的日志 IPC 不得吞掉并发或后续 rejection。
- 有合法 backend `correlationId` 时原样复用并标记 `correlationOrigin=backend`；legacy、fixture 或 unknown failure
  生成 frontend UUID 并标记 `correlationOrigin=frontend`，保证按 operation ID 可查。
- failure args 只保留数组/对象层级与非字符串 scalar；字符串全部替换，对象 key 改写为固定序号。window error /
  unhandled rejection 只保留 allowlisted Error name 与可选非负行列，不记录 raw message、filename、stack、reason
  或动态 error code。
- `record_frontend_runtime_log` 必须继续走 `invokeRaw`（绕过 recorder）并吞掉自身失败，禁止增加全局 busy 锁。
- Runtime IPC code 只能由生成的 `GENERATED_REVIEWED_IPC_ERROR_CODES` 判定；格式合法但未审核的 code 在 renderer
  payload 形成前降级为 `internal.unexpected`，新增稳定 code 必须先更新 Rust 清单并重新 codegen。
- `generatedCommandMap.ts` 只含 phantom args/result 元数据，不 import Tauri、不生成可调用 client。
- Serde serialize phase 是 result 权威，deserialize phase 是 args 权威；`Option` 的
  required-null/optional-null 差异必须在 Rust 或真实调用边界解决，禁止 cast to `unknown`。

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| existing `IpcInvokeError` | same object returned |
| valid structured payload | wrapper exposes code/message/retryable and optional correlationId |
| malformed correlationId | reject structured shape and fail closed |
| missing browser fixture | preserve `IpcFixtureMissingError` |
| unknown/raw sensitive rejection | fixed safe unexpected error |
| missing backend correlation | new frontend UUID + explicit frontend origin |
| recorder pending/fails | every other rejection still attempts one independent write |
| dynamic object key/message/stack/code | ordinal/fixed fields only; raw seed absent |
| generated/handwritten overlap | contract test fails |
| generated artifact contains runtime invoke or `unknown` | contract test fails |

### 5. Good / Base / Bad Cases

- Good: `await invoke("generated_command", args)` infers Rust-derived args/result.
- Base: a remaining allowlisted command uses the compatibility overload.
- Bad: explicit generic hides drift, message regex controls behavior, or generated code bypasses `@/lib/ipc`.

### 6. Tests Required

- Adapter tests cover object/coded/plain/unknown rejection, identity pass-through and `String(error)`.
- Runtime recorder tests cover backend/legacy/unknown correlation, concurrent pending writes, write failure recovery,
  key-and-value adversarial seeds, and global error allowlists.
- Fixture tests use `ipcFixtureError(code, message)` for expected backend failures.
- Coverage derives typed/untyped/frontend/generated/backend-only membership from authoritative sources and asserts parity.
- Run `pnpm ipc:codegen:check` twice after generation to prove determinism.
- `frontendArchitectureContract.test.ts` asserts the production `@tauri-apps/api/event` match set equals `src/lib/ipc/invoke.ts`.

### 7. Wrong vs Correct

```ts
// Wrong: hides generated contract and branches on prose.
await invoke<Result>("refresh_skill_update_inventory", args);
if (String(error).includes("cancel")) return;

// Correct
await invoke("refresh_skill_update_inventory", args);
if (parseBackendError(error).code === "operation.cancelled") return;
```
