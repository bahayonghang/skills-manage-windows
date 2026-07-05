# implement: UnifiedSkillCard 显式场景 interface

> 执行顺序即本清单顺序。S1–S3 是一个原子重构批（中间态 typecheck 必红），S4 门禁过后才允许提交。

## S1 组件本体重构（`src/components/skill/UnifiedSkillCard.tsx`）

- [ ] 现有 `UnifiedSkillCardProps` 改名为模块私有 `SkillCardModel`，删除 `onClick` / `summaryLabel` / `isInstalled` 三字段与 `:256` 可点击分支（含其 JSX 与 `resolvedSummaryLabel` 的 summaryLabel 支路，保留 `t("common.aiSummaryLabel")` 固定标签）
- [ ] 新增 §3（design.md）的 6 个场景成员 interface + `UnifiedSkillCardProps` 判别联合（全部导出）
- [ ] 新增 `toModel(props): SkillCardModel`：`switch (props.variant)` 每分支只拷贝该场景合法字段
- [ ] `UnifiedSkillCardComponent` 首行归一化为 model，其余渲染代码零改动（对 install 按钮删除 isInstalled 支路：恒走可安装态渲染）
- [ ] 删除 zh/en locale 的 `platform.searchSkillLabel`（`src/i18n/locales/{zh,en}.json:539`）

→ verify: 本文件自洽（后续步骤消红）

## S2 builder 与 11 处调用点迁移

- [ ] `centralSkillCardProps.ts`：返回类型 `CentralSkillCardProps`，返回对象加 `variant: "central"`；context `density` 类型改引
- [ ] `CentralGroupedSkillList.tsx:139` / `CentralSkillListContent.tsx:144,149`：经 builder 自动获得 variant（grid spread + platformIcons/footer 附加保持）
- [ ] `PlatformView.tsx:496` 加 `variant="platform"`
- [ ] `ProjectsShell.tsx:520` 加 `variant="project"`
- [ ] `ObsidianVaultView.tsx:392,433` 加 `variant="import"`
- [ ] `MarketplaceShell.tsx:257,560` 加 `variant="marketplace"`
- [ ] `CollectionView.tsx:418` / `CollectionsListView.tsx:484` 加 `variant="collection"`
- [ ] requiredness 微调：以 tsc 实测为准，只紧不松（规则见 design §3）

→ verify: `pnpm typecheck` 绿

## S3 测试迁移与负面用例

- [ ] `UnifiedSkillCard.test.tsx`：13 用例按 design §5 场景归入，加 `centralBaseProps` 辅助
- [ ] 新增 `src/test/unifiedSkillCardVariants.test.tsx`：6 正例 + ≥4 组 `@ts-expect-error` 互斥负例（对象字面量为主 + 单行 JSX 各 1，全部带中文描述）
- [ ] 核查 `marketplaceViewTestSupport.tsx` mock 消费的 prop 名（预期零改动）

→ verify: `pnpm test -- src/test/UnifiedSkillCard.test.tsx src/test/unifiedSkillCardVariants.test.tsx`

## S4 全量门禁 + grep 复核（design §6 表）

- [ ] `pnpm test`（页面测试 PlatformView / ProjectsShell / Collection×2 / CentralSkillsView.* 为视觉锁定面）
- [ ] `pnpm typecheck`、`pnpm lint`
- [ ] grep：组件内 `onClick|summaryLabel|isInstalled` = 0；全仓 `searchSkillLabel` = 0；`variant=` 覆盖 11 调用点（用 Grep 工具，勿用 Bash grep 数方括号/尖括号）

## S5 spec 沉淀

- [ ] 新增 `.trellis/spec/frontend/skill-card-scenarios.md`（场景清单、成员形状约定、新增场景的流程：加成员→toModel 分支→负面用例）并登记 `frontend/index.md`

## S6 提交与收尾

- [ ] commit 1：`refactor(skill-card)` 重构主体 + 调用点 + 测试（[AI] 标注，Why 行）
- [ ] commit 2：`docs(spec)` spec + 任务工件
- [ ] 归档任务、journal 记录

## 回滚点

- S6 前任意失败：工作树 `git checkout -- .`（无中间提交）
- S6 后：按 commit 逐个 `git revert`；纯前端无迁移残留
