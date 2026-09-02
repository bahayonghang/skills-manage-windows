# Implementation Plan

本文件只定义后续实施步骤；当前任务保持 `planning`，不修改 IPC 代码或生成物。

## Step 0 — Freeze the exact baseline [R1]

- Files/symbols：`src/lib/ipc/commandMap.ts::UNTYPED_IPC_COMMANDS`、`src/test/contracts/ipcCommandCoverage.test.ts::scanInvokedCommands`、本 design 的六批清单。
- 运行 coverage test 和只读扫描，记录 count、command names、调用文件；验证 47 项恰好分区为 `6+8+7+8+12+6`。若 HEAD 已漂移，先更新清单与 R/AC 追溯，不直接实施。
- 定向命令：

```powershell
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts
rtk rg -n '^  "[a-z0-9_]+",$' src/lib/ipc/commandMap.ts
```

- Rollback point：本步只记录 evidence，不改产品文件。

## Steps 1–6 — Graduate one domain batch at a time [R2][R3][R5][R6]

每批严格使用同一顺序：

1. 在该批 Rust command/DTO owner 上补最小 `specta::Type` metadata，并把 command 加入 `ipc_registry.rs::__skillport_generated_commands`；不改 runtime policy。
2. 运行 `pnpm ipc:codegen`，检查 `generatedCommandMap.ts` args/result 无 `unknown`，再运行 `pnpm ipc:codegen:check`。
3. 在该批 store 调用点删除 `invoke<T>` 显式泛型，修正由真实 Rust signature 暴露的类型漂移；不改 store 状态机。
4. 从 `UNTYPED_IPC_COMMANDS` 删除且只删除该批名称，运行 coverage/parity 与该域行为测试。
5. 提交该批前运行定向命令；失败则成组回退本批 Rust/生成物/调用点/allowlist/tests，不影响前批。

各批定向测试命令：

```powershell
# Batch 1 Collections
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/stores/collectionStore.test.ts

# Batch 2 Projects
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/stores/projectsStore.test.ts src/test/stores/skillDetailStore.test.ts

# Batch 3 Settings/runtime/scanner
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/runtime/runtimeLogger.test.ts src/test/stores/settingsStore.test.ts src/test/stores/appUpdateStore.test.ts

# Batch 4 Marketplace/skills.sh/agents
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/stores/marketplaceStore.test.ts src/test/stores/centralSkillsStore.test.ts

# Batch 5 Central repositories/tags/reviews
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/stores/centralSkillsStore.test.ts

# Batch 6 AI explanation/jobs
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/stores/centralSkillsStore.test.ts src/test/stores/skillDetailStore.test.ts src/test/stores/marketplaceStore.test.ts
```

每批共同命令：

```powershell
pnpm ipc:codegen:check
pnpm typecheck
cargo test --manifest-path src-tauri/Cargo.toml --locked --features ipc-codegen ipc_codegen
```

## Step 7 — Remove the final fallback [R4][R5][R7]

- Files/symbols：`commandMap.ts::UNTYPED_IPC_COMMANDS`、`invoke.ts::{invoke,invokeRaw}`、`ipcCommandCoverage.test.ts`、`src/test/runtime/ipc.test.ts::compileTimeTypedUsage` 和 untyped fixture cases。
- 确认 allowlist count 为 0 后，在同一变更删除 allowlist export与任意 string overload；增加 `@ts-expect-error` fixtures，分别锁定错误 name、参数数量/形状和返回赋值。
- 保留 `dispatch`、fixture routing、failure recorder、`invokeRaw` 自举语义；只收紧其 command key/args/result 类型。
- 定向验证：`pnpm typecheck`；`pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/runtime/runtimeLogger.test.ts`。
- Rollback point：fallback removal 单独提交；漏项时 revert 此提交，不恢复任何已毕业 command 的 allowlist 项。

## Step 8 — Close generated and repository evidence [R5][R6]

- 运行 `pnpm ipc:codegen` 与 `pnpm docs:gen` 后执行只读 checks，确认第二次 generation 无 diff。
- 搜索 production `invoke<`、任意 string overload、`UNTYPED_IPC_COMMANDS`、第二 adapter 与 component/page 直接 Tauri imports；逐项记录 0 或既有明确例外。
- 运行总验证块，人工检查六批 wire names；真实 WebView2/SSH/WSL/provider 行为记录 `UNVERIFIED`。

## Total Verification

```powershell
pnpm ipc:codegen
pnpm ipc:codegen:check
pnpm docs:gen
pnpm docs:gen:check
pnpm typecheck
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts src/test/runtime/runtimeLogger.test.ts src/test/stores/collectionStore.test.ts src/test/stores/projectsStore.test.ts src/test/stores/settingsStore.test.ts src/test/stores/appUpdateStore.test.ts src/test/stores/marketplaceStore.test.ts src/test/stores/centralSkillsStore.test.ts src/test/stores/skillDetailStore.test.ts
cargo test --manifest-path src-tauri/Cargo.toml --locked --features ipc-codegen ipc_codegen
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked
just ci
```

## Human and External Evidence

- 人工：逐批检查 generated args/result、lowerCamel wire names 与 store 调用，拒绝 `any`/`unknown` cast 绕过。
- 外部：Windows WebView2、真实 SSH/WSL target、AI/provider 与 marketplace network commands 未由本地 contract 证明；必须分别报告 `UNVERIFIED`。

## Final Rollback Points

- Batch 1–6 各自一个独立 commit/revert 单元。
- Step 7 fallback removal 单独一个 commit/revert 单元。
- docs 生成物与导致它变化的 registry batch 同进退；禁止手工编辑或单独回退生成物。
