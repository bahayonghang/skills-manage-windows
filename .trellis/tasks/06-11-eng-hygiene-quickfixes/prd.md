# PRD：工程卫生快修包（子任务 D）

> 父任务：`06-11-analysis-driven-fixes` ｜ 执行顺序：第 1 位（无前置依赖）
> 来源：分析报告条目 #4、#5、#6、#7、#8（部分）

## Goal

一次性清除分析报告中全部「低严重度、小规模」的工程卫生问题，为后续大改造（spawn_blocking、thiserror）提供干净的 lint 与代码基线。

## Requirements

1. **ESLint 配置清理**
   - 删除 legacy `.eslintrc.cjs`（ESLint 10 下为死配置）。
   - `eslint.config.cjs` 全局 `ignores` 补充：`src-tauri/target/**`、`tmp/**`、`outputs/**`、`node_modules/**`。
2. **Sidebar 订阅修复**：`src/components/layout/Sidebar.tsx:107` 的 `usePlatformStore()` 整 store 解构改为按字段 selector 订阅。
3. **data.json 搬迁**：根目录 `data.json`（skillport/state-export 测试夹具）移至测试夹具目录（建议 `src/test/fixtures/skillport-state-export.json`），更新引用它的测试，git 中删除根目录副本。
4. **set_setting 静默失败收口**：新增带 tracing 日志的 best-effort 辅助函数（如 `set_setting_best_effort`），替换 `commands/scanner.rs:49,66-67,92` 等处的裸 `let _ = db::set_setting(...)`；全仓搜索同模式一并替换。
5. **discover 废弃残留清理**：删除 `src/lib/discoverDeprecationPreference.ts` 及其引用（`/discover` 路由重定向本身保留）。
6. **遮罩 token 化**：3 处 `bg-black/20`（`CentralPlatformManageDrawer.tsx:112`、`CategorizeDrawer.tsx:37`、`TaskCenterDrawer.tsx:75`）抽成统一 overlay token（CSS 变量或共享类）。

## Acceptance Criteria

- [ ] 仓库中不存在 `.eslintrc.cjs`；`rtk proxy npx eslint . --format json` 对全仓运行错误数为 0（幻影错误消失）。
- [ ] `pnpm lint`、`pnpm typecheck`、`pnpm test` 全绿。
- [ ] `Sidebar.tsx` 不再有无 selector 的 store 调用（`grep "usePlatformStore()" -> 0 命中`）。
- [ ] 根目录无 `data.json`；引用该夹具的测试通过。
- [ ] `src-tauri` 全仓无裸 `let _ = db::set_setting`（grep 验证）；`cargo test` 全绿。
- [ ] 全仓无 `discoverDeprecationPreference` 引用。
- [ ] 3 个 Drawer 遮罩走统一 token，视觉不回归（人工核对三个 Drawer 打开效果）。

## Out of Scope

- 圆角规格清扫（父任务已裁定不做）。
- 任何 services 层错误类型改动（属 C 批次）。
