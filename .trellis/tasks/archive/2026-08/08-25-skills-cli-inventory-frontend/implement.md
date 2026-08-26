# Implement — Skills CLI 库存优先页面前端落地与 doctor 非阻塞

## 顺序清单

1. **store 分轨**（`src/stores/skillsCliStore.ts`）
   - 拆 `error` → `runtimeError` + `inventoryError`；`loadAll()` 改为库存轨 / 运行时轨独立 settle；新增 `isRefreshing`；`emptyState` 补新字段。
   - `previewSource` / `addGlobal` / `removeGlobal` 的错误写入改到对应轨（preview 失败仍走 toast + 内联，不污染库存错误）。
2. **store 测试**（`src/stores/skillsCliStore.test.ts`）
   - doctor 失败 + list 成功 → skills 保留、runtimeError 置位；list 失败 → 旧 skills 保留、inventoryError 置位；刷新不拆数据；空字段默认值。
3. **图表组件**（新 `src/components/skillsCli/InventoryCensus.tsx` + `InventoryCensus.test.tsx`）
   - KPI 派生、平台零值桶、桶计数、`role="img"`/aria/`<title>`、census empty 测试 id（AC7/AC8）。
4. **View 重构**（`src/pages/SkillsCliView.tsx`）
   - 按 design 布局重排 DOM；`<details>` 折叠规则；`skills-cli-inventory` / `skills-cli-install` / `skills-cli-paths` 锚点；runtimeBlocked 禁用安装/卸载；Refresh 后台化。
5. **i18n**（`src/i18n/locales/en.json`、`zh.json`）
   - 新增 kpi/chart/runtimeBlocked/refreshing/重试等键；缩短 subtitle；中英对齐。
6. **View 测试改写**（`src/pages/SkillsCliView.test.tsx`）
   - doctor-error 用例改为 AC2（库存仍渲染 + 错误一次 + 按钮禁用）；新增 AC1/AC3/AC4/AC5/AC6/AC9 断言；保留 preview/add/uninstall 既有断言不削弱（AC11）。
7. **doctor 日志**（`src-tauri/src/services/skills_cli/mod.rs` probe 分支 + `tests.rs`）
   - probe 非零 → `tracing::warn!`（status + stderr 截断）；测试断言 error payload 仍为公开句、log sink 收到摘要（AC10）。
8. **收尾门禁**
   - `pnpm vitest run src/stores/skillsCliStore.test.ts src/pages/SkillsCliView.test.tsx src/components/skillsCli`
   - `cargo test --manifest-path src-tauri/Cargo.toml skills_cli`
   - `just ci`（含 lint/format/i18n 校验；无 IPC 形状变化，`ipc:codegen` 不需重跑，确认 `docs:gen:check` 干净）。
   - 人工 smoke：`pnpm tauri dev` 验证截图两问题消失（库存 + 图表在首屏；错误不再整页阻断）。

## 验证命令

```bash
pnpm vitest run src/stores/skillsCliStore.test.ts src/pages/SkillsCliView.test.tsx src/components/skillsCli
cargo test --manifest-path src-tauri/Cargo.toml skills_cli
just ci
```

## 风险文件与回滚

- `src/stores/skillsCliStore.ts`：`AppShell.tsx` 消费 `resetForTargetChange`，字段只增不删。
- `src/pages/SkillsCliView.tsx`：整页重排，测试同步改写；回滚 = revert 单提交。
- `mod.rs`：仅 probe 分支加日志，不动错误矩阵。

## task.py start 前检查

- [ ] prd/design/implement 三件套就绪（本任务 inline 工作流，JSONL 门禁跳过）。
- [ ] 用户已对最新规划摘要明确批准。
