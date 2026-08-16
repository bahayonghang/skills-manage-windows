# 技术设计:未使用技能 unlink 弹窗化

## 边界

- 仅前端(`src/`)与前端测试;后端 `uninstall_skill_from_agent`、unused 报告契约、Rust 层全部不动。
- 组件不直接 `invoke`,沿用 store 单向数据流(`skill-usage-state.md`)。

## 组件结构

### 1. 新组件 `src/components/usage/UnusedSkillUnlinkDialog.tsx`

单弹窗服务 Central 与平台两种条目,内部先把异构安装数据归一为统一 target 模型:

```ts
type UnlinkDisabledReason =
  | "disabledPendingRecovery"
  | "disabledSharedRoot"   // 仅 Central:central 自身或 linkType === "native"
  | "disabledReadOnly"     // 仅平台
  | "disabledSourceKind"   // 仅平台:sourceKind !== "user"
  | "disabledNoRow";       // 仅平台:rowId === null

interface UnlinkTarget {
  skillId: string;        // Central: entry.skillId;平台: install.skillId(逐行携带)
  agentId: string;
  rowId: string | null;   // Central: null;平台: install.rowId
  disabledReason: UnlinkDisabledReason | null;
}
```

- `centralTargets(entry)` / `platformTargets(entry)` 归一函数与禁用原因判定从现有
  `CentralAgentChip` / `platformUnlinkDisabledReason` 迁移合并,放本文件或 `usageFormat.ts` 旁的
  纯函数模块,便于单测。
- Props:`{ open, onOpenChange, entry, onUnlinkAgents, pendingUnlinkKeys }`;`entry` 为
  `VisibleEntry` 携带的 `UnusedSkillEntry` 原始数据。
- 弹窗布局沿用 `BatchUninstallCentralSkillsDialog` 范式(`Dialog/DialogHeader/DialogBody/DialogFooter`,
  `sm:max-w-md` 量级):
  - Header:标题(技能名)+ Description(Central 条目注明「保留 Central 副本」;平台条目注明跨 Agent 列表来源);
  - Body:「全选」`Checkbox` + Agent 列表(每行 `Checkbox` + agentId + 不可卸载原因文案,
    禁用行整行降透明度并带 `title` 提示);
  - Footer:取消(`DialogClose`)+ 确认按钮(destructive variant,文案含选中数,
    选中可卸载项 ≥1 才可用;执行中 loading 并禁用)。
- 默认不勾选任何项;全选只作用于 `disabledReason === null` 的项。
- 部分失败呈现:确认后 store 返回逐项结果,弹窗不关闭,失败行内联错误文案 + 重置勾选为
  仅失败项(便于直接重试),成功项从列表消失(随 `refreshUnused()` 报告更新自然移除)。
- data-testid:`unused-unlink-dialog`、`unused-unlink-select-all`、
  `unused-unlink-option-{agentId}`、`unused-unlink-option-disabled-{agentId}`、
  `unused-unlink-confirm`、`unused-unlink-error-{agentId}`。

### 2. `UnusedSkillsPanel.tsx` 行内改造

- 删除 `CentralAgentChip`、`PlatformUnlinkAction`、`preferredPlatformInstall` 及第二排 chip 容器。
- Central 条目第二排改为与平台行一致的弱化文本行:`agent1 · agent2 · …`(复用 `entryAgentIds`),
  仅作信息展示,无任何操作。
- 操作列(打开按钮之后、最右)新增 Unlink 图标按钮(ghost + icon-sm + `HIT_AREA_CLASS` +
  `Unlink` icon),点击设置 `dialogEntry` state 打开弹窗;整条目无可卸载项时禁用并给 title 原因。
- 触发器 testid:`unused-unlink-trigger-{origin}-{name}`。
- 弹窗 state 由 `UnusedSkillsPanel` 持有(`dialogEntry: UnusedSkillEntry | null`),避免每行挂弹窗。

### 3. 行为时序

```
点击行右 Unlink → dialogEntry 置位 → 弹窗渲染归一 targets(默认全不选)
→ 用户全选/单选 → 确认 → store.unlinkUnusedSkillFromAgents(entry targets 子集)
→ 逐 target:set pending key → invoke uninstall_skill_from_agent → clear pending key → 记录结果
→ 全部结束 → 一次 refreshUnused() → 返回逐项结果
→ 全成功:成功 toast + 关闭弹窗;部分失败:弹窗保留失败项呈现;刷新异常:错误 toast(formatBackendError)
```

## Store 契约(`src/stores/usageStore.ts`)

新增批量方法(替代该场景对 `unlinkUnusedSkill` 的使用):

```ts
unlinkUnusedSkillFromAgents(
  targets: Array<{ skillId: string; agentId: string; rowId?: string | null }>,
): Promise<Array<{ skillId: string; agentId: string; rowId: string | null; ok: boolean; error: string | null }>>
```

- 顺序执行(后端 Central mutation lock 本就串行化,N ≤ Agent 数,量级个位数);
- pending key 复用 `unlinkActionKey(agentId, skillId, rowId)` 与既有 `pendingUnlinkKeys` 通道,
  弹窗按行读取渲染 spinner;finally 语义保证无泄漏;
- `refreshUnused()` 只在整批结束后调用一次;
- 成功摘要 toast 与失败 toast 沿用现有 `toast` 通道;逐项失败明细由返回值交给弹窗呈现,不逐项 toast。
- 旧 `unlinkUnusedSkill` 若无剩余调用方(检查 `skillUsageBindings.ts` / 测试)则删除,避免双入口。

## 数据流与绑定

- `skillUsageBindings.ts`:`onUnlink` prop 换为 `onUnlinkAgents`(批量);`pendingUnlinkKeys` 不变。
- `SkillUsageView.tsx`:仅透传调整。

## i18n(`src/i18n/locales/{en,zh}.json`)

新增 `skillUsage.unused.unlink.dialog.*`:`title`、`descriptionCentral`、`descriptionPlatform`、
`selectAll`、`selectedCount`(插值 count)、`confirm`、`cancel`、`success`、`partialFailure`、
`triggerLabel`(行右按钮 aria/title,插值 skill)、`allDisabledTitle`。
删除不再引用的 `skillUsage.unused.unlink.actionLabel` / `.confirm`(先全局 grep 确认无他处引用);
五种禁用原因 key 原样保留。

## 测试策略

- `src/test/components/usage/UnusedSkillsPanel.test.tsx`:
  - 触发器位置与 testid;全禁用条目触发器禁用 + title;
  - 弹窗打开、Central/平台 targets 归一(平台跨 Agent 全列出)、禁用行原因;
  - 全选不含禁用项;确认按钮计数与禁用逻辑;
  - 确认调用 store 批量方法一次、参数为选中集;部分失败行呈现与重试勾选态。
- `src/test/stores/usageStore.test.ts`:批量方法全成功/部分失败/异常路径;pending key 生命周期
  (执行中存在、结束清除);`refreshUnused` 恰好一次。
- `src/fixtures/usage.ts`:补多 Agent Central 条目与跨 Agent 平台散件条目夹具。

## 权衡与备选

- **批量后端命令 vs 前端循环**:选前端顺序循环。避免新增 IPC 面与后端聚合事务;后端逐行守卫
  (pending recovery / shared-root / row 校验)与 mutation lock 天然复用。备选的
  `uninstall_skill_from_agents` 批量命令留待出现性能或原子性诉求再做。
- **Central 第二排保留只读 chip vs 文本行**:选文本行,与平台行的 agent 副标题形态一致,
  降低面板视觉噪音;per-agent 操作统一收敛到弹窗。
- **默认全选 vs 默认不选**:选默认不选。destructive 批量操作让用户显式选择,防误触;
  「全选」勾选框满足一键场景。

## 兼容与回滚

- 纯前端变更,单 commit 可整体 revert;无数据迁移、无 IPC 契约变更。
- 行为兼容风险点:旧测试依赖 `unlink-chip-*` / `unlink-action-*` testid,需同步重写而非保留兼容层。
