# 前端分层与废弃代码收口

## Goal

移除已证明不可达的前端模块，恢复 store/lib/pages 的依赖方向和 Tauri adapter 边界，并收口审计发现的少量 i18n 绕过。

## Findings

- `FE-ARCH-001`（Medium / S）：`src/stores/updateCenterStore.ts:20-21` → `src/lib/updateCenterRefreshScope.ts:7` → `src/pages/centralUpdateCheckMode.ts:1-3` → store type 构成 type-only 环，并让 lib/store 反向依赖 page。
- `FE-ARCH-003`（Medium / S）：`src/lib/explanationStream.ts`、`src/stores/marketplaceStore.githubImportHelpers.ts`、`src/stores/projectsStore.ts`、`src/stores/skillDetailStore.ts` 直接从 `@tauri-apps/api/event` 导入 `UnlistenFn`，绕过 `src/lib/ipc/` adapter 边界。
- `FE-ARCH-002`（Medium / M）：4 个不可达生产模块共 948 行：`src/pages/CollectionView.tsx`、`src/components/marketplace/SkillPreviewDialog.tsx`、`src/components/platform/DuplicatePlatformSkillsDialog.tsx`、`src/components/skill/SkillDetailPanelShell.tsx`；前两者仍有孤立测试。
- `FE-I18N-001`（Low / S）：`src/stores/settingsStore.aiSlice.ts:467-472`、`src/components/settings/AiSettingsSection.tsx:585-589`、`src/pages/MarketplaceView.tsx:167-177` 的 browser fixture 提示绕过 `src/i18n/`。
- `ARCH-004`（Low / M）：`src/types/index.ts` 为 644 行且有大量生产 importer；审计中的“193”没有稳定排除口径，不能作为 ratchet 数值。实施开始时必须按 R5 的明确命令重测基线。

## Requirements

- R1: `UpdateCheckMode`、默认值和 `normalizeUpdateCheckMode` 必须移入不依赖 page/store 的中性模块；lib/store 不得 import `src/pages/**`，page/controller 继续消费该中性契约。
- R2: `src/lib/ipc/invoke.ts` 必须是生产代码直接 import `@tauri-apps/api/event` 的唯一允许入口；adapter 外的生产直接 import 必须为 0，所需 `UnlistenFn` 从 `@/lib/ipc` 导出。
- R3: 删除 4 个候选前必须检查 production static import、lazy/dynamic route 和 test-only import；确认不可达后删除对应模块，且仅删除直接以废弃模块为被测对象的孤立测试，不保留 deprecated wrapper。
- R4: 三处 browser fixture 可见文本必须在渲染边界使用 `src/i18n/locales/en.json` 与 `src/i18n/locales/zh.json` 的 keys，且 key parity 保持一致。
- R5: 实施第一步必须用设计中给定的同一命令扫描 `src/**/*.{ts,tsx}`，排除 `src/test/**` 与 `src/types/**`，记录根 `@/types` barrel 的可复算生产 importer 基线；不得固化审计中的 193。
- R6: 本任务只把实际触及的 update-center、event type 与已删除模块邻接 import 改为窄入口；静态 ratchet 只禁止根 barrel 生产消费者相对 R5 基线增加，不做全库机械迁移。
- R7: 所有边界检查必须进入自动化契约：Tauri event 扫描以“生产匹配集合精确等于 `src/lib/ipc/invoke.ts`”为判定，并明确排除 `src/test/**`；依赖方向、死文件和 root barrel 也必须有确定性断言。

## Acceptance Criteria

- [x] AC1 (R1): 生产 import 图中不存在 `src/lib/**` 或 `src/stores/**` 指向 `src/pages/**` 的 update-center 边。
- [x] AC2 (R1): `UpdateCheckMode` 及其默认值只在中性模块定义一次。
- [x] AC3 (R1): normalize 函数只在中性模块定义一次。
- [x] AC4 (R1): update-center 相关消费者均从中性定义导入对应符号。
- [x] AC5 (R2, R7): 对生产源码扫描 `@tauri-apps/api/event` 的匹配集合精确为 `src/lib/ipc/invoke.ts`。
- [x] AC6 (R2): 4 个原旁路生产文件全部从 `@/lib/ipc` 消费 `UnlistenFn`。
- [x] AC7 (R2): `src/lib/ipc/invoke.ts` 保持唯一 Tauri event runtime/type 入口。
- [x] AC8 (R3): 4 个不可达生产文件均不存在。
- [x] AC9 (R3): 生产 module specifier 不再引用 4 个已删除模块。
- [x] AC10 (R3): `src/test/pages/CollectionView.test.tsx` 与 `src/test/components/marketplace/SkillPreviewDialog.test.tsx` 均不存在。
- [x] AC11 (R3): 保留的 drawer/collections/marketplace tests 通过且无断链。
- [x] AC12 (R4): browser fixture 的 AI connection 在英语与简体中文下均由 locale key 渲染。
- [x] AC13 (R4): Marketplace preview 的 browser title 和 detail 在英语与简体中文下均由 locale key 渲染。
- [x] AC14 (R4): `src/test/contracts/i18nLocales.test.ts` 的 key parity 通过。
- [x] AC15 (R4): 生产代码不保留本 finding 中的硬编码英中 browser fixture 句子。
- [x] AC16 (R5): 实施记录包含设计指定的 root barrel 扫描命令。
- [x] AC17 (R5): 实施记录包含 AC16 产生的完整排序路径清单。
- [x] AC18 (R5): 实施启动基线由 AC17 的清单行数计算。
- [x] AC19 (R6, R7): 相同扫描口径在实现后不高于 AC18 基线。
- [x] AC20 (R6): 本任务触及的领域 import 使用窄入口。
- [x] AC21 (R1, R2, R3, R4, R5, R6, R7): 本任务列出的定向 Vitest 全部通过。
- [x] AC22 (R1, R2, R3, R4, R5, R6, R7): `pnpm typecheck` 通过。
- [x] AC23 (R1, R2, R3, R4, R5, R6, R7): `pnpm lint` 通过。
- [x] AC24 (R1, R2, R3, R4, R5, R6, R7): `just ci` 通过。
- [ ] AC25 (R1): Windows WebView2 人工 smoke 覆盖 Update Center 的 check mode 行为。 **UNVERIFIED**
- [ ] AC26 (R4): Windows WebView2 人工 smoke 覆盖 Settings AI 的可见反馈。 **UNVERIFIED**
- [ ] AC27 (R4): Windows WebView2 人工 smoke 覆盖 Marketplace preview 的可见反馈。 **UNVERIFIED**
- [x] AC28 (R1, R4): AC25-AC27 在执行前保持 `UNVERIFIED`，不得由静态扫描或 jsdom 代替。

## Out of Scope

- 全量拆分 `src/types/index.ts` 或承诺一个未经重测的 importer 数字。
- UI 重设计或新功能。
- typed IPC 命令迁移；由 `typed-ipc-remainder` 负责。
- 新增 compatibility wrapper、deprecated export 或第二个 Tauri adapter。
