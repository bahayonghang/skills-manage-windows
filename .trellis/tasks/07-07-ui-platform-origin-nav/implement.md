# 执行计划：平台视图安装来源快速导航

> 前置：`task.py start` 之后才动手。TDD 顺序：每步先补测试再实现。全程单分支单 commit 序列，任一步失败可 `git checkout -- <file>` 回退该步。

## Step 1 视图模型（纯函数层）

- [ ] `src/lib/platformSkillViewModel.ts`：
  - 新增 `PlatformOriginFilter` / `PlatformOriginNavModel` 类型；
  - 新增 `getPlatformSkillOrigin` / `getPlatformOriginRepoKey` / `derivePlatformOriginNav`；
  - `DerivePlatformSkillRowsInput` 增加 `originFilter`，管线在 tab 过滤后、搜索前 retain，新增输出 `originFilteredSkills`（`sourceFilteredSkills` 语义保持不变）。
- [ ] 先写 `src/test/platformSkillViewModel.test.ts` 新用例（见 design §6：origin 判定 / nav 聚合守恒与排序 / unassigned 归桶 / standalone 带 repo 不计入 / originFilter 各分支 / 与 search、sourceFilter 组合顺序）。

**验证**：`pnpm test -- src/test/platformSkillViewModel.test.ts` 全绿；`pnpm typecheck`（PlatformView 调用点会先报缺参，本步允许暂时以 `{kind:"all"}` 补上占位）。

## Step 2 i18n

- [ ] `src/i18n/locales/zh.json` / `en.json` 新增 `platform.originNav.*` 7 个 key（见 design §5 对照表），zh/en 同步。

**验证**：`pnpm typecheck`；grep 确认无遗漏 key 引用。

## Step 3 导航组件 + 页面接入

- [ ] 新建 `src/components/platform/PlatformOriginNav.tsx`（props 契约见 design §3；选中态样式对齐 claude 来源 tab；`<nav aria-label>` + `aria-current` + 共享焦点环；repo 子项 truncate + title + tabular-nums 计数）。
- [ ] `src/pages/PlatformView.tsx`：
  - 新增 `originFilter` state，与 `sourceFilter` 同一 effect 随 `agentId` 重置；
  - content 区改为 `flex`（aside w-56 + 原滚动区，`contentRef` 不动，见 design §3）；
  - `derivePlatformSkillRows` 传入 `originFilter`；nav model 用 `useMemo(derivePlatformOriginNav(platformRows.sourceFilteredSkills))`；
  - 空态链插入第 3 级：`originFilteredSkills.length === 0` → 空态 + 清除筛选（design §4 顺序）。
- [ ] 先写 `src/test/PlatformView.test.tsx` 新用例（渲染计数 / repo 子项过滤 / 路由切换重置 / 空态清除筛选）。

**验证**：`pnpm test -- src/test/PlatformView.test.tsx` 全绿。

**Review gate（本步自查）**：

- 未新建任何场景专用技能卡片（skill-card-scenarios 约定）；
- 无 `dark:` 二元色、无原生调色板状态色（statusTone 约定）；
- 所有用户可见文本走 i18n；
- 分类逻辑只存在于视图模型层，组件零判定。

## Step 4 全量校验（最后一轮全范围）

- [ ] `pnpm typecheck && pnpm lint && pnpm test`
- [ ] `pnpm tauri dev` 人工抽查：Universal 页三段计数守恒、repo 子项过滤、与搜索/分组/批量选择组合、切平台重置、4 套代表主题下选中态可读。
- [ ] `just ci`

## 回滚点

- 每 Step 一个 commit（`feat(platform): …` 拆分），revert 单个 commit 即回退对应步；整任务回滚 = revert 全部本任务 commit，无迁移/数据残留。
