# 执行计划 — Skills CLI 失效条目清理与多选批量更新

依据 `prd.md` 与 `design.md`。按段执行，每段结束跑该段验证命令再进入下一段。

**前置**：`08-27-skills-cli-doctor-gate` 已合入 `dev`。
本计划假定 `runtimeBlocked` prop 已从批量栏、卸载对话框、详情抽屉移除。
若尚未合入，段 3 与段 5 会与其产生冲突面。

## 段 1 — 候选集合与分组（回滚单元 A，纯函数）

- [ ] 1.1 抽取"该技能所有 placement 都是 `unavailable`"的判据为共享函数。
      现在这段逻辑在 `SkillCardDenseRow.tsx:30-60`，**抽出而非复制**——
      两处判据分叉就是 PAC2 的失败模式。
- [ ] 1.2 `src/pages/skillsCliBatchModel.ts` 新增 `deriveCleanupCandidates(skills)`
      （design §2.1）。判定：任一 placement 的 `reasonCode === "canonical_missing"`
      → `stale`，否则 → `platformUnavailable`。
- [ ] 1.3 `reasons` 逐条保留 `{ platform, reasonCode }`，供对话框行内展示。
      平台侧三种原因**不拆成三组**。
- [ ] 1.4 单元测试（AC1）：canonical 缺失 → `stale`；
      canonical 健康但全部未检测/禁用 → `platformUnavailable`；
      同一技能混合 `platform_not_detected` 与 `platform_disabled` → 单一
      `platformUnavailable` 组且 `reasons` 有两条。

验证：`pnpm vitest run src/test/lib/skillsCliViewModel.test.ts src/test/pages`

## 段 2 — `batchProgress` 与重复提交闸门（回滚单元 E，先于 B/C/D）

- [ ] 2.1 `src/stores/skillsCliStore.ts` 新增状态
      `batchProgress: { operation, completed, total } | null`。
- [ ] 2.2 `removeGlobalBatch` 与 `runPlacementBatch` 的逐项循环中，
      每完成一项 `set` 一次 `completed`；进入前置 `total`，结束置 `null`
      （成功与部分失败都要置回）。
- [ ] 2.3 `skills_cli.busy` 的处理：该项计入 `failed` 并**继续下一项**，
      不中断整批，不自动重试（design §2.5）。确认既有 `PlacementMutationOutcome`
      的 failed 语义能承载它，不新增结构。
- [ ] 2.4 组件层不新增任何 `invoke`——进度只读 store（spec `skills-cli-global.md:64`）。

验证：`pnpm vitest run src/test/stores/skillsCliStore.test.ts`

## 段 3 — 清理入口与对话框（回滚单元 B）

- [ ] 3.1 `SkillsCliView.tsx` 工具栏在 `Export all`（`:321`）同排新增清理入口。
      **不放批量栏**——批量栏仅在有选中项时渲染（`SkillsCliBatchBar.tsx:44-46`），
      而清理不依赖选择。
- [ ] 3.2 候选为空时入口禁用，点击不发 IPC（AC2）。
- [ ] 3.3 新建 `src/components/skillsCli/SkillsCliCleanupDialog.tsx`：
      两个分组区，各带组头全选与计数；`stale` 默认全选，
      `platformUnavailable` 默认全不选。
- [ ] 3.4 `platformUnavailable` 勾中任一项时渲染风险提示，未勾选时不渲染（AC3）。
- [ ] 3.5 确认路径**只**调用 `skills_cli_preview_remove_global` 与 `removeGlobalBatch`，
      不新建第二条删除通道（AC4）。
- [ ] 3.6 预览里带 conflict 的技能禁用确认且零写；
      independent direct copies 不计入删除数——如实呈现后端语义，不放宽。
- [ ] 3.7 Escape 走 Base UI topmost dismissal，**不注册**第二个无条件全局 handler
      （归档 `08-26-batch-actions` R9）。

验证：`pnpm vitest run src/test/components/skillsCli src/test/pages/SkillsCliView.test.tsx`

## 段 4 — 批量更新（回滚单元 C）

- [ ] 4.1 `SkillsCliBatchBar` 新增 Update 动作，复用既有 `Button` + `min-h-10` 形态。
- [ ] 4.2 选择集按 `repositoryKey` 分组，取法复用
      `repositoryKeyForSkills`（`SkillsCliView.tsx:455` 已在用）。
- [ ] 4.3 逐组一次 `skills_cli_apply_updates`，**各组独立 jobId**，组间**串行**。
      不并发——并发会让多个 apply 争抢同一 target mutation guard。
- [ ] 4.4 单组失败不阻断其余组；结果汇总为一个 partial outcome（AC5）。
- [ ] 4.5 无 update 元数据时不发 IPC，复用
      `openUpdateSurface`（`skillsCliPageHandlers.ts:211-230`）已有的
      `skillsCli.updates.checkFirst` 引导（AC6）。
- [ ] 4.6 执行中读段 2 的 `batchProgress` 禁用该动作（AC9c）。

验证：`pnpm vitest run src/test/pages/SkillsCliView.test.tsx`

## 段 5 — 按平台批量 unlink（回滚单元 D）

- [ ] 5.1 `SkillsCliBatchBar.tsx:138-146` 的 Unlink 按钮改为菜单，
      结构与计数展示复用 Link 菜单（`:61-137`）与 `SkillsCliLinkTargetSummary`。
- [ ] 5.2 菜单项 = 各平台 + 一个「解链所有平台」（保留现有行为，不移除）。
- [ ] 5.3 只对该平台下 `managed_link` 的技能发 IPC；
      `direct_copy` / `conflict` / `unavailable` 计入 skipped 并显示本地化原因，
      不发 IPC（AC7）。
- [ ] 5.4 执行中读 `batchProgress` 禁用该动作（AC9d）。

验证：`pnpm vitest run src/test/components/skillsCli/SkillsCliBatchBar.test.tsx`

## 段 6 — 排版（回滚单元 F）

- [ ] 6.1 **不改** `SKILLS_CLI_GRID_CLASS`（`skillsCliViewModel.ts:49-50`）的列数断点。
      密度优化只在卡片内部信息层级与既有 `gap-3` 范围内做。
- [ ] 6.2 **不引入** `md:` / `lg:` 等 viewport 断点——
      `src/test/contracts/skillsCliPageShell.test.ts:44-56` 会拦截。
- [ ] 6.3 新增图标按钮复用 `ICON_HIT`（`SkillsCliBatchBar.tsx:25-26`），不另写热区实现。
- [ ] 6.4 新增按钮不带超过容器的固定 `min-width`，保持批量栏 `flex-wrap`（`:56`）有效。
- [ ] 6.5 `SkillsCliGroupHeader` 内确保「标题 / 计数 / 组级操作」三区顺序稳定，
      标题与计数用既有 typography token，不引入页面私有字号。

验证：`pnpm vitest run src/test/contracts/skillsCliPageShell.test.ts && pnpm lint`

## 段 7 — 测试补齐

- [ ] 7.1 AC8：断言页面无直接 `invoke`、无直接 `sonner` 调用；
      选择状态仍是 `SkillsCliView` 单一 `selectedCardNames`（`:100`），
      未出现第二套选择状态。
- [ ] 7.2 AC9：清理 N 个技能时展示已完成 / 总数；执行中重复点击不发起第二批。
- [ ] 7.3 AC9b：某项返回 `skills_cli.busy` 时计入 failed、不中断整批、
      最终 partial outcome 包含它。
- [ ] 7.4 AC9c：跨 2 仓库的批量更新展示 1/2 → 2/2；
      第一组在飞时再次点击不产生第三次 `skills_cli_apply_updates`。
- [ ] 7.5 AC9d：按平台 unlink 在飞时该动作禁用，重复点击不发起第二批。
- [ ] 7.6 AC10a / AC10c / AC10e / AC10f：类名与 DOM 结构契约断言；
      AC10c 用 `skills-cli-layout-bands` 的 `data-grid`（`SkillsCliView.tsx:272`）
      在容器宽 720 / 1000 / 1280 三档取样。
- [ ] 7.7 异步断言不依赖固定延时，遵循 `async-ui-test-stability`。
- [ ] 7.8 AC10b / AC10d / AC10g 是原生视觉检查，**不写自动化测试**，
      在任务记录中标记 `UNVERIFIED` 直至在 Windows x64 bundle 上人工确认。

验证：`pnpm vitest run src/test`

## 段 8 — 收尾

- [ ] 8.1 AC11：新增文案 en/zh 成对，i18n parity 通过。
- [ ] 8.2 确认无后端改动 → `pnpm docs:gen:check` 应无 diff。
- [ ] 8.3 AC12：定向 Vitest、`pnpm typecheck`、`pnpm lint` 与 `just ci` 通过。

## 风险文件与回滚点

回滚单元见 `design.md` §6。

| 文件 | 风险 | 回滚单元 |
| --- | --- | --- |
| `components/skill/SkillCardDenseRow.tsx` | 判据抽取若留下第二份实现，清理集合会与徽章不一致（PAC2 失败） | A |
| `pages/skillsCliBatchModel.ts` | 分组判定错会让健康技能落进默认勾选组——这是本任务最高危的错误 | A |
| `stores/skillsCliStore.ts` | `batchProgress` 未在部分失败路径置回 `null` 会永久锁住三个入口 | E |
| `pages/SkillsCliView.tsx` | 与远端子树共同的冲突面，需错开工作树 | B、C、D |
| `pages/skillsCliViewModel.ts` | 动到 `SKILLS_CLI_GRID_CLASS` 会打破已锁定的契约测试 | F（应保持不动） |

## 前置检查

- [ ] `08-27-skills-cli-doctor-gate` 已合入 `dev`，`runtimeBlocked` prop 已移除。
- [ ] 确认远端子树未在同一工作树改 `SkillsCliView.tsx`。
- [ ] 工作树干净。
