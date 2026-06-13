# 设置页双组件改为上下布局

## Goal

把“Integrations & Keys”设置页中的 GitHub PAT 和 AI Provider 两个区块改为上下堆叠展示，减少右侧空白并提升内容密度。

## Confirmed Facts

- 目标页面是 `src/pages/settingsPageSections.tsx` 里的 `SettingsIntegrationsPage`。
- 当前布局使用 `div className="grid gap-4 xl:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]"`，导致两个 section 在大屏下左右并排。
- 两个主要区块分别是 `GitHubPatSettingsSection` 和 `AiSettingsSection`。
- 这次只涉及设置页的展示布局，不改输入逻辑、保存逻辑或数据结构。

## Requirements

- 两个设置区块在该页面中改为纵向排列。
- 页面在宽屏下也不要回到左右并排的两列布局。
- 其他设置分页保持不变。
- 不引入额外的交互变化。

## Acceptance Criteria

- [ ] Integrations 页的 GitHub PAT 和 AI Provider 区块上下排列。
- [ ] 宽屏下不再显示这两个区块的两列并排布局。
- [ ] 页面其余设置页的布局和行为不受影响。

## Out of Scope

- 不调整两个区块内部的表单结构。
- 不修改文案、逻辑、存储、校验或测试流程。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.