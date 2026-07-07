# 提取前端 Platform management module：分组与多选

## Goal

把「Universal Agents 是一个虚拟分组」这条领域知识收进一个前端 platform management module（分组、可见性、多选行为一个小 interface），4 个安装对话框与约 18 处组件分支降级为薄调用方；顺带修正 CLAUDE.md 与实现不符的 InstallDialog 描述。

## 背景与证据（2026-07-04 架构评审）

分组知识目前没有 module 承载，散布为：

- `src/lib/platformTargetGroups.ts:10,23,39` — 三份有序 ID 列表（`UNIVERSAL_AGENT_ID_ORDER` 10 项、`UNIVERSAL_PROJECT_AGENT_ID_ORDER` 13 项、`UNIVERSAL_INSTALL_AGENT_ORDER` 7 项）。
- `src/lib/platformVisibility.ts:12` — `DEFAULT_ENABLED_PLATFORM_IDS`（7 项）+ coding/lobster 分类逻辑。
- `isUniversalPlatformTarget(agent) ? … : …` 分支散布 ≈18 个组件文件（Sidebar、TopBar、ProjectsShell、GlobalSearchDialog、AgentsPanel、SkillDetailSidebar、UnifiedSkillCardFooter、CentralSearchBar、CentralSkillsShellMenus、4 个安装对话框等）。
- 平台多选网格 + 选中推导在 4 个对话框各自重实现：`InstallDialog.tsx:81,298`、`CollectionInstallDialog.tsx:49,136`、`ProjectInstallDialog.tsx:84,248`、`BatchInstallCentralSkillsDialog.tsx:91,275`（`selectedInstallAgentIds()` 推导 4 份；网格类名、重置逻辑、部分失败汇总近乎相同；CollectionInstallDialog 263 行 ≈ InstallDialog 487 行减去模式单选/项目选择器/方式单选）。

后果：新增 Platform #37 最多要改 4 个前端列表，漏一处即静默渲染为独立平台。

**文档勘误（归本任务）**：CLAUDE.md 称 InstallDialog「默认勾选已链接平台」，实际实现（`InstallDialog.tsx:112-152`）默认勾选**全部** enabled/visible 目标，linked 只是徽标（`:315-319`）。

## Requirements

1. 一个 platform 分组/选择 module：Universal Agents 虚拟分组、平台可见性、多选行为（含选中推导、重置语义、部分失败汇总）收进一个小 interface。
2. 4 个安装对话框改为共用同一多选实现；各自默认勾选语义**保持现状**（Install=全部 enabled/visible；CollectionInstall=全部 detected），本任务只收敛实现不改行为。
3. ≈18 处 `isUniversalPlatformTarget` 分支尽量改为消费新 module 的分组结果；哪些纯展示分支可保留由 design 裁决。
4. 新平台注册收敛到一处前端登记点（或完全由后端数据驱动，design 裁决）。
5. 修正 CLAUDE.md 的 InstallDialog 描述，以实现为准。

## Constraints

- `UnifiedSkillCard` 唯一卡片约束不变；不新建场景专用卡片。
- 所有用户可见文本走 i18n；状态色走 `statusTone.ts`。
- 不改后端；`AgentWithStatus.icon_name` 后端驱动图标的现状保持。

## Acceptance Criteria

- [ ] grep 验证：选中推导（`selectedInstallAgentIds` 类逻辑）在 4 个对话框中不再各自定义，指向共享实现。
- [ ] 新平台加入的前端登记点唯一，漏登记会显式失败（类型约束或测试）而非静默渲染错误。
- [ ] 4 个对话框既有交互行为不变（默认勾选、部分失败汇总等有测试锁定）。
- [ ] CLAUDE.md 中 InstallDialog 描述与实现一致。
- [ ] `pnpm test`、`pnpm typecheck`、`pnpm lint` 全过。

## Notes

- 复杂度：complex → 需 `design.md` + `implement.md`。
- 呼应 CONTEXT.md 优先方向 #2——评审确认后端平台定义已集中于 `db/seed.rs`，真实摩擦在前端。
