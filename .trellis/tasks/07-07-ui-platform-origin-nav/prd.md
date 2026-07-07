# 平台技能视图左侧安装来源快速导航栏

## Goal

在 `/platform/:agentId`（含 Universal 等平台目标页）技能列表左侧新增一条快速导航栏，把当前平台的技能按安装来源分为「SkillPort 安装（symlink 到中央技能库）」与「独立安装（用户自行放置/拷贝）」两大类，SkillPort 类下再按来源仓库（repo）细分；点击导航项即过滤右侧卡片列表，帮助用户快速定位某一来源的技能。

## Confirmed Facts（代码调研结论，2026-07-07）

- **数据已足够，纯前端可实现。**`ScannedSkill.link_type` 由扫描器按文件系统事实生成（`src-tauri/src/services/scanner/mod.rs:124` `detect_link_type`：symlink → `"symlink"`，平台目录普通目录 → `"copy"`，中央目录 → `"native"`）；卡片 footer 的「中央技能库 / 独立安装」正是按 `link_type === "symlink"` 判定（`src/components/skill/SkillCardBadges.tsx:15` `SourceIndicator`）。
- **仓库信息已随行返回。**`ScannedSkill.repository`（`SkillRepository`，含 `is_unknown` 标记"未指派"伪仓库）来自中央技能的仓库指派（`skill_repository_members` → `skill_repositories`，见 `src-tauri/src/db/repos/skills_repo.rs:213` `get_skills_for_agent` 的 JOIN）。
- **`installed_at` 不能作为"SkillPort 装的"信号。**扫描器对平台目录里发现的所有技能（含用户手工拷贝的）都会 upsert `skill_installations` 行（`src-tauri/src/services/scanner/persistence.rs:126-143`），`installed_at` 实际是"首次被扫描到"的时间。
- **copy 方式经 SkillPort 安装的技能，落盘后与用户手工拷贝目录不可区分**（同为普通目录），现有卡片对二者统一展示「独立安装 — copy」。因此 v1 分类规则与卡片徽标严格同语义，用户在界面上可自洽验证。
- 页面既有筛选管线集中在 `derivePlatformSkillRows`（`src/lib/platformSkillViewModel.ts`）：claude 来源 tab 过滤 → 搜索 → 排序 → 分组；`getPlatformRepositoryGroupInfo` 已有 repo 分组 key/label 推导可参照。
- 批量选择（`PlatformTransferRail` 的「选择当前结果」）作用于 `filteredSkills` 下游；导航过滤放进管线上游即可自动正确组合。

## Requirements

1. **导航结构**（固定三段 + 动态子项）：
   - 全部（计数）
   - SkillPort 安装（计数）—— 子项按来源仓库列出（repo 显示名 + 计数，按名称排序）；若存在 symlink 但仓库未指派（`repository` 缺失或 `is_unknown`）的技能，末尾追加「未指派来源」子项
   - 独立安装（计数）
2. **分类规则**：`link_type === "symlink"` → SkillPort 安装；否则 → 独立安装。与卡片 `SourceIndicator` 完全同语义。
3. 点击导航项过滤右侧列表；与搜索、排序、分组、claude 来源 tab、批量选择**正交组合**（导航过滤在搜索之前生效）。
4. **计数口径**：claude 来源 tab 过滤后、搜索过滤前的技能集（导航计数不随搜索词变化，随 tab 变化）。
5. 切换平台路由时导航选中态重置为「全部」（与现有 `sourceFilter` 重置行为一致）。
6. 可访问性：导航容器为 `<nav>` 带 `aria-label`；当前选中项有明显选中态与 `aria-current`；键盘可聚焦、焦点环可见（对齐 07-06-ui-keyboard-focus-a11y 的共享焦点环约定）。
7. 导航过滤后无结果时展示空态，并提供「清除筛选」动作（重置导航到「全部」）。
8. 文案走 i18n（zh/en 同步），词汇与既有 `platform.sourceCentral`（中央技能库）/ `platform.sourceStandalone`（独立安装）对齐，不引入同一概念的第二套词。
9. 适用于所有 `/platform/:agentId` 页面（Universal 目标与普通平台行为一致）。

## Acceptance Criteria

- [ ] Universal 页左侧出现导航栏，三段结构与计数正确（symlink/copy 混合数据下核对：计数之和 = 全部）。
- [ ] 点击「SkillPort 安装」下某 repo 子项 → 右侧仅剩该 repo 的 symlink 技能；点击「独立安装」→ 仅剩非 symlink 技能；点击「全部」→ 恢复完整列表。
- [ ] 搜索、排序、按仓库分组、批量「选择当前结果」均在导航过滤后的集合上正确工作。
- [ ] 切换到另一平台后导航选中态自动回到「全部」。
- [ ] 导航过滤 + 搜索组合出零结果时出现空态与「清除筛选」。
- [ ] `pnpm test -- src/test/platformSkillViewModel.test.ts src/test/PlatformView.test.tsx`、`pnpm typecheck`、`pnpm lint` 通过；收尾跑 `just ci`。

## Out Of Scope

- 后端 / DB 任何改动（不新增 install-origin 标记；copy 方式经 SkillPort 安装的技能 v1 归入「独立安装」，与卡片徽标一致——如需精确区分，另立任务在安装链路落 origin 标记）。
- 中央技能库页、项目页、市场页的类似导航（仅平台视图）。
- `UnifiedSkillCard` 本身的任何改动。
- claude-code 页现有 user/plugin 来源 tab 的语义调整（导航与其正交并存）。
