# 执行计划：Skill Usage 最大化布局成本

前置：阅读 `.trellis/spec/frontend/index.md`、`skill-usage-state.md`、本任务 `prd.md` / `design.md` / `research/resize-jank-diagnosis.md`。只改前端与前端测试。

## 步骤

1. **热力图轨道**  
   改 `src/components/usage/ActivityHeatmap.tsx`：
   - 格子网格去掉 `auto-cols-fr`、`flex-1`、格子上的 `aspect-square` / `w-full`；
   - 改为固定 `grid-auto-columns` + 固定 `size-*`；
   - 月份行与格子列宽对齐；
   - `compact` 用更小的固定边长，规则相同。

2. **页面网格**  
   改 `src/pages/SkillUsageView.tsx`：
   - 去掉 Top skills 的 `xl:row-span-3`；
   - 保留 `min-h-[32rem]` + `fill` + 表体内部滚动；
   - `xl` 仍两栏；Unused 仍 `xl:col-span-2`；
   - 骨架 `UsageSkeleton` 同步去掉对应 `xl:row-span-*`，避免加载态与成页结构不一致。

3. **Containment**  
   Top skills 与 Heatmap 的 `UsageSection` 加 `contain-layout`。不要加到 Unused（弹窗 / 热区）。

4. **测试**  
   `src/test/components/usage/SkillUsage.components.test.tsx`：
   - 保留 112 格、漫游、`data-level`、图例；
   - 新增回归：热力图 markup 不含 `aspect-square` / `auto-cols-fr`；格子使用固定尺寸 class。
   - 若有页面测试覆盖 `SkillUsageView` 网格 class，更新 `row-span-3` 断言。

5. **目检**  
   `pnpm tauri dev` 打开 `/usage`，窗口先小于 1280px 再最大化；对照 Dashboard / Central。确认表格无卡片内空白、热力图格子不随最大化变大、unlink / 筛选仍可用。

## 验证命令（迭代期）

```bash
pnpm test -- --run src/test/components/usage
pnpm typecheck && pnpm lint
```

## 完成门

```bash
just ci
```

## 审查点 / 回滚

- 步骤 1 完成后，热力图源码不得再出现 `aspect-square` 与 `auto-cols-fr`。
- 步骤 2 完成后，`SkillUsageView.tsx` 不得再给 Top skills 加 `xl:row-span-3`。
- 回滚单元 = 整个任务单 commit。

## 收尾

- Phase 3.3：在 `.trellis/spec/frontend/skill-usage-state.md` 补一条：热力图格子为固定边长，禁止 `aspect-square` + `1fr`；`xl` 双栏不得用 `row-span` 把表格高度绑到热力图。
- Phase 3.4：单 commit，前缀 `perf(usage):`。
