# implement: 按命令名类型化的 IPC adapter 与 fixture seam

> 每步一个提交（git-commit skill，`[AI]` 头 + Why）。任一步红灯：先修复；无法当步修复则 revert 该步提交回滚。步序不可调换（step 1 消除 `src/lib/ipc.ts` 文件/目录歧义是 step 2 的前置）。

## Step 1 — 新建 `src/lib/ipc/` 目录 adapter（吸收旧 ipc.ts）

- [ ] 建 `src/lib/ipc/runtime.ts`：从 `lib/tauri.ts` 迁 `isTauriRuntime`、`showMainWindowWhenReady`、`__resetMainWindowReadyForTest`（含 TauriWindow 类型）。
- [ ] 建 `src/lib/ipc/commandMap.ts`：`IpcCommandSpec`/`IpcCommandMap`（先收编旧 ipc.ts 的 7 命令）+ `CommandArgs`/`CommandResult` 工具类型 + `UNTYPED_IPC_COMMANDS: readonly string[]`（暂空，step 6 填）。
- [ ] 建 `src/lib/ipc/fixtures.ts`：注册表 Map、`registerIpcFixtures`（typed mapped-type 参数）、`registerUntypedIpcFixture`、`clearIpcFixturesForTest`、`hasIpcFixture`、`IpcFixtureMissingError`。
- [ ] 建 `src/lib/ipc/invoke.ts`：双 overload `invoke`（见 design §2 签名）+ failure recorder（原 `registerIpcFailureRecorder` 语义 + `record_frontend_runtime_log` 豁免）+ real/fixture 路由 + `invokeRaw`（注释注明 runtimeLogger 例外）+ `listen` 包装（浏览器 no-op unlisten）。内部直接 import `@tauri-apps/api/core`/`event`，不依赖 `lib/tauri.ts`。
- [ ] 建 `src/lib/ipc/index.ts`：re-export 公共面（invoke/invokeRaw/listen/isTauriRuntime/registerIpcFailureRecorder/showMainWindowWhenReady/__resetMainWindowReadyForTest/fixture API/类型）。
- [ ] **删除 `src/lib/ipc.ts`**（避免文件/目录 resolver 歧义），其 5 个 importer（`centralSkillsStore.installSlice.ts`、`obsidianStore.ts`、`skillDetailStore.ts`、`ObsidianVaultView.tsx`、`tauri.test.ts`）的 `invokeCommand(` → `invoke(`，import 改自 `@/lib/ipc`。
- [ ] 新建 `src/test/ipc.test.ts`：迁移 `tauri.test.ts` 的 failure-recorder 用例；新增 fixture 命中/未注册 reject/`listen` 浏览器 no-op/双 overload 编译期断言（含 `@ts-expect-error` 反例）。旧 `tauri.test.ts` 删除。
- [ ] 验证：`pnpm test -- src/test/ipc.test.ts` + `pnpm typecheck`（此步 `lib/tauri.ts` 仍在，全仓应绿）。
- [ ] 提交（回滚点 1）。

## Step 2 — 全仓 flip 到单一入口

- [ ] `from "@/lib/tauri"` → `from "@/lib/ipc"`：全仓替换（勘查基线 63 文件，src + test）。
- [ ] `vi.mock("@/lib/tauri"` → `vi.mock("@/lib/ipc"`：18 个测试文件（factory 内容不动）。
- [ ] 删除 `src/lib/tauri.ts`。
- [ ] 验证：`pnpm typecheck` + `pnpm test`（全量）+ grep：`from "@/lib/tauri"` = 0、`invokeCommand` = 0。
- [ ] 提交（回滚点 2；本步纯机械可逆）。

## Step 3 — 测试基建：命令路由 mock

- [ ] `src/test/setup.ts`：`__TAURI_INTERNALS__.invoke` 换 dispatcher（宽松默认：未注册 resolve `undefined` + 记录调用；严格模式由注册动作触发：未注册 reject 并列出已注册命令）。全局 `afterEach` 调 `resetIpcMock()`。
- [ ] 新建 `src/test/ipcMock.ts`：`mockIpcCommand` / `mockIpcCommands` / `ipcInvokeCalls` / `resetIpcMock`（API 见 design §6）。
- [ ] 范例迁移 `platformStore.test.ts`：删 `vi.mock("@tauri-apps/api/core")` 与全部 `mockResolvedValueOnce` 顺序链（基线 35 处），改 `mockIpcCommands` + `ipcInvokeCalls` 断言；保留原用例语义（含 pending-promise 的 isLoading 用例，用 handler 返回可控 Promise 实现）。
- [ ] 范例迁移 `skillStore.test.ts`（基线 11 处顺序桩）。
- [ ] 验证：`pnpm test`（全量，确认 dispatcher 未破坏依赖裸 vi.fn 的存量文件）。
- [ ] 提交（回滚点 3）。

## Step 4 — fixture seam 接线 + 批次 1 store 剥 guard

顺序（由简到繁，每完成 1-2 个 store 跑一次相关测试）：

- [ ] `src/fixtures/` 骨架 + `index.ts` 的 `installBrowserIpcFixtures()`；`main.tsx` 静态 import，`!isTauriRuntime()` 时调用（先于 render）。
- [ ] `settings` fixture（`get_setting`→null、`set_setting`→no-op）→ 剥 `displayFont.ts` 2 处 guard（类型化 get/set_setting 入 map）。
- [ ] `misc` fixture（`usage_get_skill_counts`→{}、`get_skill_explanation_summaries`→{}）→ 剥 2 个 hook 的 guard；两 hook 的命令入 map；同步迁移 `useSkillCallCounts.test.ts`、`useSkillExplanationSummaries.test.tsx`（必迁）。
- [ ] `platformStore`：`BROWSER_FIXTURE_AGENTS/COUNTS/DASHBOARD_CENTRAL_SUMMARY` 搬 `src/fixtures/platform.ts`，注册 `get_bootstrap_snapshot`/`get_setting`（已有）/`list_platform_paths`/`scan_all_skills`/`get_skill_counts_summary`/`set_agent_enabled` 等；剥 5 处 guard；相关命令入 map。
- [ ] `skillStore`（4 处）→ `src/fixtures/skills.ts`。
- [ ] `usageStore`（8 处 + listen guard）→ `src/fixtures/usage.ts`；`usageStore.test.ts` 必迁（浏览器分支 describe 并入 browserFixtures.test）。
- [ ] `targetStore`（11 处）→ `src/fixtures/targets.ts`。
- [ ] `operationLogStore`（5 处）/`runtimeLogStore`（4 处）→ 对应 fixture。
- [ ] `tagGroupsStore`（6 处）/`savedViewsStore`（5 处）→ 对应 fixture。
- [ ] `ObsidianVaultView.tsx:226` 归位：`open_obsidian_path` 动作移入 `obsidianStore`，组件改调 store action。
- [ ] 新建 `src/test/browserFixtures.test.ts` 安全网（删 `__TAURI_INTERNALS__`/`__TAURI__` → install → 驱动 8 store 主加载 → 断言 state 非空且 `error === null`）。
- [ ] 验证：逐 store `pnpm test -- src/test/<store>.test.ts`；步末全量 `pnpm test` + `pnpm typecheck`。
- [ ] 提交（回滚点 4；可按 store 拆多个提交）。

## Step 5 — 类型覆盖 ratchet

- [ ] 盘点全仓 `invoke("…"` 字面量命令名清单；批次 1 相关命令确认已入 map（含 step 4 新增），其余填入 `UNTYPED_IPC_COMMANDS`。
- [ ] 新建 `src/test/ipcCommandCoverage.test.ts`：node fs 递归扫 `src/**/*.{ts,tsx}`（排除 `src/test/`），正则提取 invoke 命令字面量，断言 ∈ map ∪ 允许清单。
- [ ] 硬指标复核：map ≥ 40；批次 1 迁移文件内 `invoke<`（显式泛型）= 0。
- [ ] 验证：`pnpm test -- src/test/ipcCommandCoverage.test.ts`。
- [ ] 提交（回滚点 5）。

## Step 6 — 全量门禁 + 验收复核

- [ ] `pnpm lint`、`pnpm typecheck`、`pnpm test`、`just ci`。
- [ ] design §8 验收表逐项 grep 复核，结果记入任务 notes（数值留档）。
- [ ] 浏览器演示人工冒烟（`pnpm dev` 走 dashboard/platform/usage 页）——可选，安全网测试已覆盖逻辑面。
- [ ] 提交遗留（如有）。

## Step 7 — Phase 3 收尾（对应 workflow 3.3/3.4）

- [ ] 新增 `.trellis/spec/frontend/ipc-adapter.md`（入口唯一性、双 overload 约定、fixture 注册、测试 mock 规范、调用方分层规则、runtimeLogger 例外），挂入 `.trellis/spec/frontend/index.md`。
- [ ] 批次 2/3 遗留清单登记到父任务 `07-04-architecture-deepening` notes。
- [ ] Journal + 归档按 finish-work 流程。

## 验证命令速查

```bash
pnpm test               # 全量
pnpm test -- src/test/ipc.test.ts
pnpm typecheck && pnpm lint
just ci                 # 最终门禁
```
