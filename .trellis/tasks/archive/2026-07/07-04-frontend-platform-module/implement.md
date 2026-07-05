# Implement：前端 Platform management module

按序执行；每步末尾的验证命令必须通过才进入下一步。全程不改后端、不改 i18n key 集合。

## Step 1：建 `src/lib/platformRegistry.ts` + 行为锁测试

- [x] 新建 `src/lib/platformRegistry.ts`：`UNIVERSAL_PLATFORM_REGISTRY` 声明式表（13 行，顺序 = 现 `UNIVERSAL_PROJECT_AGENT_ID_ORDER`；`globalGroup` 标记 10 项；`installPreference` 标记 7 项，序 = 现 `UNIVERSAL_INSTALL_AGENT_ORDER`）
- [x] 从表导出推导结果：`UNIVERSAL_AGENT_ID_ORDER` / `UNIVERSAL_PROJECT_AGENT_ID_ORDER` / `UNIVERSAL_INSTALL_AGENT_ORDER`（导出名保持）
- [x] `DEFAULT_ENABLED_PLATFORM_IDS` 迁入本文件；`platformVisibility.ts` 改为 re-export（其余调用方不动）
- [x] 新建 `src/test/platformRegistry.test.ts`：三条推导列表逐项等于迁移前字面量；不变量（id 唯一、installPreference 唯一且其成员 ⊆ 全集）
- [x] `platformTargetGroups.ts` 三份本地列表删除，改 import registry 推导值；对外 API 不动

验证：`pnpm test -- src/test/platformRegistry.test.ts src/test/platformTargetGroups.test.ts src/test/platformVisibility.test.ts` + `pnpm typecheck`

## Step 2：`platformTargetGroups.ts` 增加 3 个展示 helper + 单测

- [x] `getPlatformTargetLabel(target, t, variant: "full" | "short")`（universal → `platformTargets.universalLabel` / `universalShortLabel`；否则 `display_name`）
- [x] `getPlatformTargetTitleHint(target)`（universal → memberNames join；否则 `global_skills_dir`）
- [x] `getPlatformTargetCountAgentId(target)`（universal → `install_agent_id`；否则 `id`）
- [x] `platformTargetGroups.test.ts` 补三个 helper 的用例（universal 组 + 普通 agent 两分支）

验证：`pnpm test -- src/test/platformTargetGroups.test.ts` + `pnpm typecheck`

## Step 3：建共享多选模块 `src/components/platform/PlatformMultiSelect.tsx`

- [x] `usePlatformTargetSelection(options)`：选中 Set 状态、`toggle`（disabled 守卫）、`reset`（默认勾选语义）、`selectedInstallAgentIds` 推导（per design D2a）
- [x] `PlatformMultiSelectGrid`：两列网格 + Checkbox 行 + universal 副标题 + tooltip + 空态 + `renderBadges` / `showIcon`（per design D2b）
- [x] `InstallFailureList`：`failures: Array<{ key; label }>` → 现 `<ul>` 样式
- [x] 新建 `src/test/PlatformMultiSelect.test.tsx`：hook 的默认全选/守卫/推导去重 + Grid 的行渲染/badge/toggle 回调

验证：`pnpm test -- src/test/PlatformMultiSelect.test.tsx` + `pnpm typecheck`

## Step 4：4 个对话框逐个切到共享实现（一个一验，行为锁不改断言）

- [x] `CollectionInstallDialog.tsx`（最小，先切）：hook（disabled=locked、默认全选）+ Grid（badges: alwaysIncluded/notDetected）+ FailureList → `pnpm test -- src/test/CollectionInstallDialog.test.tsx`
- [x] `BatchInstallCentralSkillsDialog.tsx`：hook（disabled=project-unsupported、默认全选减 excluded）+ Grid + FailureList → `pnpm test -- src/test/BatchInstallCentralSkillsDialog.test.tsx`
- [x] `InstallDialog.tsx`：hook（disabled=sharedRoot|projectUnsupported 按 targetMode、默认全选非 disabled）+ Grid（badges: alwaysIncluded/linked/projectUnsupported/notDetected；sharedRoot 强制显示勾选）+ FailureList → `pnpm test -- src/test/InstallDialog.test.tsx`
- [x] `ProjectInstallDialog.tsx`：hook（无 disabled、默认全选 eligible）+ Grid（showIcon、labelVariant="short"、badges: willReplace/willCreate）→ `pnpm test -- src/test/ProjectInstallDialog.test.tsx`

验证（步末全量）：`pnpm test -- src/test/InstallDialog.test.tsx src/test/CollectionInstallDialog.test.tsx src/test/BatchInstallCentralSkillsDialog.test.tsx src/test/ProjectInstallDialog.test.tsx src/test/ModalInstallButton.test.tsx`

## Step 5：~12 个组件文件的展示分支替换为 helper

- [x] 标签类 → `getPlatformTargetLabel`：Sidebar、TopBar、AgentsPanel、ProjectsShell、CentralSearchBar、CentralInstalledSkillsQuickFilter、GlobalSearchDialog（+对话框内残留）
- [x] title 类 → `getPlatformTargetTitleHint`；计数类 → `getPlatformTargetCountAgentId`（TopBar、AgentsPanel）
- [x] design D3 列出的保留分支不动（Sidebar.platformCount、结构性分支）

验证：`pnpm test`（全量）+ `pnpm typecheck`

## Step 6：文档勘误 + 收尾门禁

- [x] CLAUDE.md InstallDialog 描述改为与实现一致（默认勾选全部 enabled/visible 目标；linked 仅徽标）
- [x] grep 验收：`selectedInstallAgentIds` 定义仅存在于共享模块；`UNIVERSAL_.*_ORDER` 字面量列表仅存在于 `platformRegistry.ts`
- [x] 全量门禁：`pnpm test` + `pnpm typecheck` + `pnpm lint`

## 回滚点

- 每 Step 一个 commit 粒度；任一步行为锁测试无法原样通过且无明确修复路径 → revert 该步，回 design 修订。
