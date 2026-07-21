# Central Skills 刷新按钮与检查后自动刷新

## Goal

Central Skills 页面目前缺少手动刷新技能列表的入口；且执行"检查更新"（Choose update check mode → Start check）完成后，后端数据库中的技能更新状态已经变化，但页面列表仍显示旧数据。本任务为页面补充手动刷新按钮，并在更新检查完成后自动刷新技能列表。

## Background / Root Cause

- 技能列表由 `src/stores/centralSkillsStore.ts` 的 `loadCentralSkills()`（`centralSkillsStore.listSlice.ts:37-71`）加载，一次性拉取 7 个 invoke 的结果。
- 更新检查链路：`UpdateCheckModeDialog` → `handleConfirm`（`src/pages/centralUpdateCheckModeController.tsx:66-87`）→ `updateCenterStore.refresh`（`src/stores/updateCenterStore.ts:218-284`，失败会 rethrow）→ Rust `refresh_skill_update_inventory`。
- Rust 侧 refresh 会把最新状态持久化到数据库，但前端 `updateCenterStore.refresh` 只回写自己的 inventory，完全不触碰 centralSkillsStore——这是列表不刷新的根因。
- 已核实的代码事实（审阅校验结论）：
  - `loadCentralSkills()` 捕获异常后只写 store `error`，**不 rethrow**（listSlice.ts:66-70）；全仓库约 15 个调用点依赖此语义，不能直接改。
  - `refreshCounts()` 失败**会 rethrow**（`platformStore.ts:322-325`）。
  - 现有 `handleRefresh`（`centralSkillsActions.ts:602-609`）先 `await refreshCounts()` 再 `loadCentralSkills()`：计数失败会阻断列表刷新，且其 catch 实际只能捕获计数失败。
  - `loadCentralSkills` 走页面级 `isLoading`，为 true 时整个列表被替换成加载空态（`CentralSkillListContent.tsx:169-170`），不适合刷新场景。
  - central store 的 generation 仅在 `updateSlice.ts:514` bump；并发的 `loadCentralSkills` 调用共享 generation，后到者胜出，没有 latest-wins 防护。

## Requirements

- R1 手动刷新按钮：在 Central Skills 工具栏（`CentralSkillsShell.tsx` header 区）添加刷新按钮。
  - 文案复用已有 i18n key `central.refresh`（en "Refresh central skills" / zh "刷新中央技能库"），不新增 key；失败 toast 复用 `central.refreshError`。
  - 刷新期间保留现有列表内容（不出现整页加载空态），按钮自身显示 spinner 并禁用。
  - 列表刷新与计数刷新并行执行，互不阻断；任一失败都要给出 toast 反馈（详见 design.md D3）。
- R2 检查后自动刷新：`handleConfirm` 中 inventory 检查成功后、打开 Update Center 前，自动重取技能列表。
  - 成功边界：**只有 inventory 检查决定检查的成败**；列表重取失败不得误报为 `central.updateCheckError`，不得阻止 Update Center 按原参数打开，只能报 `central.refreshError`（遵循 `.trellis/spec/frontend/async-error-feedback.md` 成功路径契约）。
- R3 错误传播契约：不改动 `loadCentralSkills()` 既有默认语义（吞错写 store），通过显式可选机制让调用方拿到失败（详见 design.md D1）。
- R4 并发规则：按钮刷新中禁用防重复点击；store 层增加 latest-wins 防护，覆盖手动/自动/挂载加载重叠的场景（详见 design.md D4）。
- R5 纯前端改动，不修改 Rust 后端。所有用户可见文本走 i18n。

## Out of Scope

- Update Center 内"应用更新"（apply）后的列表刷新（`UpdateCenterDialog.tsx:190,238` 同类缺口，可另行立项）。
- 后端事件推送式刷新（tauri event listen）。
- 其他页面（Platform/Marketplace 等）的刷新语义统一。

## Acceptance Criteria

- [ ] 工具栏出现刷新按钮，点击后并行触发列表重取与计数刷新；刷新中列表内容保留、按钮 spinner + 禁用，重复点击不触发第二次请求。
- [ ] 列表重取失败显示 `central.refreshError` toast；计数刷新失败不阻断列表重取，也有失败反馈。
- [ ] Start check 成功后：列表自动重取并生效（`updateStatuses` 等可见更新），随后 Update Center 按原参数打开；若列表重取失败，Update Center 仍正常打开，仅报 `central.refreshError`。
- [ ] Start check 本身失败时行为不变：报 `central.updateCheckError`、保留弹窗、不打开 Update Center。
- [ ] `loadCentralSkills()` 无参调用的既有行为（吞错、写 store error）对所有既有调用点保持不变，有 store 回归测试证明。
- [ ] store 层 latest-wins：两个重叠的列表加载，后到请求的结果生效。
- [ ] `pnpm typecheck && pnpm lint` 通过，相关 Vitest 用例通过，最终 `just ci` 通过。

## Notes

- 技术设计与执行计划见同目录 `design.md`、`implement.md`。
