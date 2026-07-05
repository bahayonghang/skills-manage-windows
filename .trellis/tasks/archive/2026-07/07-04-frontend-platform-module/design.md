# Design：前端 Platform management module

## 调研结论（2026-07-04 实读代码）

### 现状盘点

**分组数据（3 份有序列表 + 1 份默认可见列表，分居 2 个文件）**

- `platformTargetGroups.ts`：`UNIVERSAL_AGENT_ID_ORDER`（10 项）、`UNIVERSAL_PROJECT_AGENT_ID_ORDER`（13 项）、`UNIVERSAL_INSTALL_AGENT_ORDER`（7 项）
- `platformVisibility.ts`：`DEFAULT_ENABLED_PLATFORM_IDS`（7 项）

**关键事实（决定登记表形态）**：

1. `UNIVERSAL_AGENT_ID_ORDER` 恰好等于 `UNIVERSAL_PROJECT_AGENT_ID_ORDER` 过滤掉 `antigravity` / `antigravity-cli` / `gemini-cli` 三项后的结果——**两列表共享同一 curated 顺序**，global 列表可由 project 列表 + 布尔标记推导。
2. `UNIVERSAL_INSTALL_AGENT_ORDER` 顺序独立（codex 优先），是**偏好序**而非展示序，7 项均 ⊆ project 列表。
3. `DEFAULT_ENABLED_PLATFORM_IDS` 语义独立（默认启用，与 Universal 分组正交），但同属「新增平台要记得改的前端清单」。

**4 个对话框的重复实现**

| 关注点                         | InstallDialog (487 行)                                                                              | CollectionInstallDialog (263 行)               | BatchInstallCentralSkillsDialog (465 行)                    | ProjectInstallDialog (350 行)                           |
| ------------------------------ | --------------------------------------------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------- |
| `selectedInstallAgentIds` 推导 | filter selected → 排 shared-root/project-unsupported → flatMap install ids → dedupe                 | filter selected → 排 locked → flatMap → dedupe | filter selected → 排 project-unsupported → flatMap → dedupe | selectedTargets → flatMap → dedupe                      |
| 两列网格行渲染                 | Checkbox + 名称块 + universal 副标题 + 徽标（alwaysIncluded/linked/projectUnsupported/notDetected） | 同左（alwaysIncluded/notDetected）             | 同左（projectUnsupported）                                  | label 包裹 + PlatformIcon + willReplace/willCreate 徽标 |
| 默认勾选（重置语义）           | 全部 enabled/visible（排 disabled）                                                                 | 全部                                           | 全部减 `defaultExcludedAgentIds`                            | 全部 eligible                                           |
| toggle 守卫                    | disabled 目标 no-op                                                                                 | locked 目标 no-op                              | disabled 目标 no-op                                         | 无守卫                                                  |
| 部分失败列表                   | `failed.map` → `agent_id: error`                                                                    | 同左                                           | 同左（key 含 skill_id）+ skipped 汇总                       | 无（throw 走 error）                                    |

**~18 处 `isUniversalPlatformTarget` 组件分支**（grep 实测 20 文件，扣除 lib/test/4 对话框后约 12 个组件文件），归为 3 类：

- **标签类**（最多）：`isUniversal ? t("platformTargets.universalLabel"|"universalShortLabel") : display_name` — Sidebar、TopBar、AgentsPanel、ProjectsShell、CentralSearchBar、CentralInstalledSkillsQuickFilter、GlobalSearchDialog、4 对话框内部
- **title/tooltip 类**：`isUniversal ? memberNames.join(", ") : global_skills_dir` — 同上多处
- **计数代表类**：`isUniversal ? install_agent_id : id` — TopBar、AgentsPanel；Sidebar 是变体（对 member 求和，保留）

### 文档勘误证实

CLAUDE.md 称 InstallDialog「默认勾选已链接平台（反映当前状态）」；实测 `InstallDialog.tsx:112-152` 默认勾选**全部 enabled/visible 目标**（排 shared-root/project-unsupported），linked 仅在 `:315-319` 渲染徽标。需修正。

## 设计决策

### D1：唯一登记点 = `src/lib/platformRegistry.ts`（前端登记表，不走后端驱动）

约束「不改后端」排除了后端数据驱动方案。新建 registry 模块，一张声明式表承载全部分组知识：

```ts
// 一行一个 universal 成员，数组顺序即展示顺序（= 现 project 列表顺序）
interface UniversalPlatformRegistration {
  id: string;
  /** 是否属于全局 Universal 分组（false = 仅项目场景成组） */
  globalGroup: boolean;
  /** install 代表选择偏好序（1 最优先）；undefined = 不作候选 */
  installPreference?: number;
}
export const UNIVERSAL_PLATFORM_REGISTRY: readonly UniversalPlatformRegistration[];
```

三份旧列表全部改为**从表推导**并保持导出名（`platformTargetGroups.ts` 内部消费，不动其对外 API）：

- project 顺序 = 表顺序全集
- global 顺序 = 表顺序过滤 `globalGroup`
- install 偏好序 = 按 `installPreference` 升序

`DEFAULT_ENABLED_PLATFORM_IDS` 同步迁入 registry 文件（`platformVisibility.ts` 改为 re-export，调用方不动）。至此「新增平台 #37」前端只碰 `platformRegistry.ts` 一个文件。

**漏登记显式失败**：新增 `src/test/platformRegistry.test.ts` 锁三条推导结果逐项等于现字面量（行为锁），并锁不变量：install 候选 ⊆ 全集、`installPreference` 无重复、id 无重复。类型层面 registry 用 `satisfies` + `as const` 保字面量类型。

### D2：多选行为收敛 = hook + 共享网格组件

新建 `src/components/platform/PlatformMultiSelect.tsx`（与 `PlatformIcon` 同目录，即「platform management module」的组件半边）：

**(a) `usePlatformTargetSelection(options)` hook** — 选中状态 + 推导 + 重置语义：

```ts
interface UsePlatformTargetSelectionOptions {
  targets: PlatformTarget[]; // 调用方已滤 central
  isTargetDisabled?: (t: PlatformTarget) => boolean; // 勾选守卫（shared-root / project-unsupported / locked）
  isTargetDefaultSelected?: (t: PlatformTarget) => boolean; // 默认勾选语义，缺省 = 非 disabled 全选
}
// 返回: { selectedIds, isSelected, toggle, reset, selectedInstallAgentIds }
```

- `toggle(id, checked)` 内置 disabled 守卫（现 4 处手写 find + no-op 收敛于此）
- `reset()` 按 `isTargetDefaultSelected` 重建选中集；对话框 open 时调用（重置时机留在调用方 useEffect，因各对话框还要重置 method/path 等私有状态）
- `selectedInstallAgentIds` = filter selected → 排 disabled → flatMap `getPlatformTargetInstallAgentIds` → dedupe（4 份推导收敛于此；InstallDialog 的 targetMode 过滤通过 `isTargetDisabled` 表达——与其现有 disabled 逻辑本就一致）

**(b) `PlatformMultiSelectGrid` 组件** — 两列网格 + 行渲染：

```ts
interface PlatformMultiSelectGridProps {
  targets: PlatformTarget[];
  isSelected: (t) => boolean;
  isDisabled?: (t) => boolean;
  onToggle: (id: string, checked: boolean) => void;
  renderBadges?: (t: PlatformTarget) => ReactNode; // 行尾徽标（linked/alwaysIncluded/willReplace…）
  showIcon?: boolean; // ProjectInstallDialog 需要 PlatformIcon
  emptyMessage: string;
  ariaLabel: string;
}
```

统一承载：`grid grid-cols-2 gap-x-4 gap-y-2`、Checkbox、可点击名称块、universal 标签 + memberNames 副标题、title tooltip、空态。徽标差异走 `renderBadges`（各对话框保留自己的 i18n 文案与显示条件）。

**部分失败汇总**：InstallDialog / Collection / Batch 三处的 `failed.map → li` 列表收敛为同文件导出的 `InstallFailureList`（props: `failures: Array<{ key: string; label: string }>`）；headline 文案与 skipped 汇总语义各异（i18n key 不同、Batch 有 reason 分组），**保留在各对话框**——收敛渲染重复，不强并语义。

### D3：展示分支收敛 = `platformTargetGroups.ts` 增加 3 个标签 helper

```ts
getPlatformTargetLabel(target, t, variant: "full" | "short"): string   // 标签类
getPlatformTargetTitleHint(target): string                             // title/tooltip 类（member 名单 vs skills dir）
getPlatformTargetCountAgentId(target): string                          // 计数代表类
```

替换范围：Sidebar、TopBar、AgentsPanel、ProjectsShell、CentralSearchBar、CentralInstalledSkillsQuickFilter、GlobalSearchDialog + 4 对话框内部的同型分支。

**保留的 `isUniversalPlatformTarget` 直接调用**（design 裁决为合理展示分支，不硬塞进 helper）：

- `Sidebar.platformCount`（member 求和，语义独特）
- `flattenPlatformTargets` / `getPlatformTargetMemberIds` 等 lib 内部实现
- 各对话框「universal 才渲染 memberNames 副标题」的条件（已收进 Grid 组件，组件内部用）
- `SkillDetailViewShared` / `UnifiedSkillCardFooter` / `PlatformView` / `platformCleanupGroups` 等处若属结构性分支（分组展开、成员遍历）保留；纯标签分支替换

### D4：行为不变约束

- 4 个对话框默认勾选语义逐一保持：Install=全部 enabled/visible（排 shared-root/project-unsupported）；Collection=全部（universal 锁定常选）；Batch=全部减 excluded；Project=全部 eligible。现有 4 份 `*.test.tsx` 是行为锁，重构后必须原样通过（不改断言，只允许改 render 用的 wrapper/props 装配）。
- `platformTargetGroups.ts` 对外 API（函数签名、排序结果）不变，`platformTargetGroups.test.ts` 原样通过。
- CollectionInstallDialog 的 universal「锁定常选」是**行为**不是 bug，保留。

## 权衡与放弃项

- **放弃后端驱动分组**：需改 `db/seed.rs` + IPC 契约，违反本任务「不改后端」约束；registry 表已把风险从「4 处漏一」降到「1 处 + 测试锁」。
- **放弃统一 4 对话框为单一组件**：模式单选/项目选择器/技能选择器差异是真实业务差异（Install 487 行 vs Collection 263 行的差 ≈ 这些块），强并会造出 props 开关地狱；只收敛真重复（选中推导、网格、失败列表）。
- **放弃把重置时机收进 hook**：各对话框 open-effect 还重置 method/projectPath/search 等私有状态，收进 hook 反而拆散一处 effect。

## 兼容性 / 回滚

- 纯前端重构，无 IPC / DB / 后端影响；i18n key 不增不删（新组件复用调用方传入文案）。
- 回滚 = revert 前端 commit，无数据迁移。
