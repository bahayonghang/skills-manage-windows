# 按命令名类型化的 IPC adapter 与 fixture seam

## Goal

把前端两个互相竞争的 invoke adapter 收敛为一个按命令名类型化的 IPC adapter；真实 Tauri 与浏览器/测试 fixture 成为同一 interface 的两个 adapter（two adapters = real seam），让 store 与测试都按命令名对话，替代按调用顺序打桩。

## 背景与证据（2026-07-04 架构评审）

- **两个并行 adapter**：`src/lib/tauri.ts`（裸 `invoke`）vs `src/lib/ipc.ts:19-27`（`IpcCommandMap` 类型化，但仅覆盖 7/171 命令）。调用方必须知道该用哪个；类型安全只覆盖 <5% 的命令面。
- **154 处 `isTauriRuntime()` fixture guard / 41 文件**——浏览器演示 fixture 与真实调用的切换没有共享 seam，每个 store 自己写 guard。
- **测试按调用顺序打桩**：`src/test/setup.ts:168` 把 invoke mock 成裸 `vi.fn()` 无命令路由；11 个测试文件 `vi.mock("@tauri-apps/api/core")` 后用有序 `mockResolvedValueOnce` 链（如 `platformStore.test.ts:137-523` 约 18 连），store 里加/挪一次 invoke，无关断言全崩。全仓只有 1 个测试文件按命令名路由。
- **store seam 泄漏点**（组件/lib 直连 invoke）：`src/hooks/useSkillCallCounts.ts:67`、`src/hooks/useSkillExplanationSummaries.ts:46`、`src/lib/displayFont.ts:190,221`、`src/pages/ObsidianVaultView.tsx:226`；另记录 `marketplaceSkillDetailViewModel.ts:68` 的裸 `fetch()`。
- 合理例外：`src/lib/runtimeLogger.ts` 的 `invokeRaw`（日志自举，防循环）。

## Requirements

1. 收敛为单一 IPC adapter：按命令名类型化；`lib/tauri.ts` 与 `lib/ipc.ts` 二合一（保留哪个入口、迁移策略由 design 裁决）。
2. 类型覆盖允许**增量推进**（不强制一次补全 171 个命令类型），但新 adapter 必须让未类型化命令也走同一入口，design 给出覆盖推进策略与硬指标。
3. 浏览器演示 fixture 按命令名注册到 adapter 的 fixture 侧，替代 store 内散布的 `isTauriRuntime` guard（154 处可分批迁移，design 定批次）。
4. 测试基建：`src/test/setup.ts` 提供按命令名路由的 mock 助手，新测试不再按调用顺序打桩；存量测试迁移策略由 design 裁决（允许分批）。
5. 上述 4 个 store seam 泄漏点归位（进 store 或经新 adapter，按项目规则组件不得直接 invoke）。

## Constraints

- 「store 是唯一 invoke 层」的项目规则保持；`runtimeLogger.invokeRaw` 例外保留并注明理由。
- **snake_case→camelCase 载荷转换不在本任务范围**（评审标记为 Speculative 延伸；新 adapter 只需为未来转换留出自然落点，不实施）。
- 不改后端与 IPC 命令契约。

## Acceptance Criteria

- [ ] 全仓（除 `runtimeLogger` 例外）只剩一个 invoke adapter 入口；`lib/tauri.ts`/`lib/ipc.ts` 双轨消失（grep 验证）。
- [ ] fixture guard：store 内不再出现裸 `isTauriRuntime()` + 内联 fixture 的样板（目标数值由 design 定，验收时 grep 复核）。
- [ ] `src/test/setup.ts` 提供按命令名路由的 mock 助手，且至少已迁移的测试文件不再依赖 `mockResolvedValueOnce` 顺序链。
- [ ] 4 个泄漏点闭合：hooks/lib/pages 不再直接 import invoke（grep 验证）。
- [ ] `pnpm test`、`pnpm typecheck`、`pnpm lint` 全过。

## Notes

- 复杂度：complex（涉及 41 个文件的渐进迁移）→ 需 `design.md` + `implement.md`，design 必须定分批策略与每批验收线。
- 与 `07-04-frontend-platform-module` 相互独立，可并行；先做本任务会让对话框重构的测试更好写。
