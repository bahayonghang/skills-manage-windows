# 技术设计 — Skills CLI 失效条目清理与多选批量更新

对应 `prd.md` 的 D1 / D2 与 R1–R11。
**依赖 `08-27-skills-cli-doctor-gate` 已合入**：本任务假定 `runtimeBlocked` prop
已从 `SkillsCliBatchBar` / `SkillsCliUninstallDialog` / `SkillsCliDetailDrawer` 移除。

## 1. 现状结构

### 1.1 清理判据的唯一正确来源是 `reasonCode`

卡片徽章只表达「所有 placement 都是 `unavailable`」（`SkillCardDenseRow.tsx:30-60,87`），
它不区分四种成因。后端 `classify_absent`（`placement.rs:73-110`）按固定顺序产出：

| 顺序 | reason_code | canonical 是否存在 | 删除的实际后果 |
| --- | --- | --- | --- |
| 1 | `canonical_missing` | 否 | 清理 lock 记录与残留 managed link |
| 2 | `platform_unsupported` | 是 | **真实卸载**，删掉健康技能 |
| 3 | `platform_not_detected` | 是 | 同上 |
| 4 | `platform_disabled` | 是 | 同上 |

因为 `canonical_missing` 判定最优先且与平台无关，
「某技能属于失效组」等价于「其**任一** placement 的 `reasonCode` 为 `canonical_missing`」。

`reasonCode` 已暴露给前端（`generatedCommandMap.ts:1048-1055`），无需新增 IPC。

### 1.2 多选框架已存在，缺口只有三处

`SkillsCliView.tsx` 持有唯一选择状态 `selectedCardNames`（`:100`）与 `selectMode`（`:96`）。
批量栏 `SkillsCliBatchBar.tsx` 已有 Link 菜单（`:61-137`）、Unlink（`:138-146`）、
Export selected（`:147-155`）、Uninstall（`:156-164`）。
`ICON_HIT` 常量（`:25-26`，`size-8` + `after:size-10`）已实现 40px 热区。

缺口：批量栏无 Update；无清理入口；unlink 无按平台选择（link 侧有菜单，unlink 侧只有一个按钮）。

### 1.3 排版契约已被锁定，不可推翻

```49:50:src/pages/skillsCliViewModel.ts
export const SKILLS_CLI_GRID_CLASS =
  "grid grid-cols-2 gap-3 @min-[900px]/skills-cli:grid-cols-3 @min-[1180px]/skills-cli:grid-cols-4";
```

密度用**命名容器查询**（`@container/skills-cli`），不是 viewport 断点。
`src/test/contracts/skillsCliPageShell.test.ts:44-56` 显式禁止 `md:` / `lg:` 一类断点。
`deriveSkillsCliLayoutBands(contentWidthPx)` 把当前档位暴露到
`data-grid`（`SkillsCliView.tsx:272`），这是 AC10c 的可断言抓手。

**结论**：R10 的"卡片网格密度"不是重做栅格，而是在既有契约内调间距与卡片内部密度。

### 1.4 批量在后端是逐项的，且没有进度通道

`skills_cli_remove_global` / `link_platform` / `unlink_platform` 都是单项签名，
前端 `removeGlobalBatch` / `runPlacementBatch` 逐项循环。
每一项独立申请并释放一次 job lease 与一次 `acquire_target_mutation_guard`（默认 10s 超时）。

进度事件只有 update 子系统有（`skills-cli://update-progress`，`updates/mod.rs:32`）。
**install / link / unlink / remove 没有进度通道**——所以 R9 的进度只能由前端按已完成项数自己数。

`skills_cli_apply_updates` 每请求只接受一个 `repositoryKey`（`generatedCommandMap.ts:989-993`）。

## 2. 目标结构

### 2.1 候选集合与分组（R1、R2）

纯函数放进既有 `src/pages/skillsCliBatchModel.ts`
（已有 `summarizeLinkTargets`、`selectedHasManagedLink`，同类归属）：

```ts
type CleanupGroup = "stale" | "platformUnavailable";

interface CleanupCandidate {
  name: string;
  group: CleanupGroup;
  reasons: readonly { platform: string; reasonCode: string }[];
}

function deriveCleanupCandidates(
  skills: readonly SkillsCliGlobalSkill[],
): readonly CleanupCandidate[];
```

判定顺序（与 §1.1 严格对应）：

1. 过滤出「所有 placement 状态为 `unavailable`」的技能——**复用**
   `SkillCardDenseRow` 的同一判据，抽成共享函数，不复制一份。
2. 任一 placement 的 `reasonCode === "canonical_missing"` → `stale`，否则 → `platformUnavailable`。
3. `reasons` 逐条保留平台与原因，供对话框行内展示。

平台侧三种原因**不再细分**成三个可勾选组（同一技能可能同时有
`platform_not_detected` 与 `platform_disabled`，细分会让它归属不唯一），
合并为一组、行内展示各自原因。

默认勾选：`stale` 全选，`platformUnavailable` 全不选（D1/D2）。

### 2.2 清理入口与对话框（R1、R2、R3、R4）

- 入口放在工具栏、与 `Export all`（`SkillsCliView.tsx:321`）同排，
  **不占批量栏空间**——批量栏只在有选中项时出现（`SkillsCliBatchBar.tsx:44-46`），
  而清理不依赖选择。
- 候选为空时入口禁用，点击不发 IPC（AC2）。
- 新组件 `SkillsCliCleanupDialog`：两个分组区，各自组头带全选复选框与计数；
  `platformUnavailable` 组勾中任一项时才渲染风险提示（AC3）。
- 确认后**复用既有删除通道**：先 `skills_cli_preview_remove_global` 取影响预览，
  再走 `removeGlobalBatch`。不新建第二条删除路径（R4）。
- conflict：预览里带 conflict 的技能禁用确认并零写；
  independent direct copies 不计入删除数——这两条是后端既有语义
  （spec `skills-cli-global.md:99-102`），前端只需如实呈现，不放宽。

### 2.3 批量更新（R5、R6）

批量栏新增 Update 动作。选择集按 `repositoryKey` 分组：

- `repositoryKey` 的取法复用 `repositoryKeyForSkills`（`SkillsCliView.tsx:455` 已在用）。
- 每组一次 `skills_cli_apply_updates`，**各组独立 jobId**，串行执行。
- 单组失败不阻断其余组，结果汇总为一个 partial outcome。
- 无 update 元数据时不发 IPC，复用 `openUpdateSurface`
  （`skillsCliPageHandlers.ts:211-230`）已有的 `skillsCli.updates.checkFirst` 引导（R6）。

串行而非并发：并发会让多个 apply 争抢同一个 target mutation guard，
把可预测的顺序执行变成随机的 Busy 风暴。

### 2.4 按平台批量 unlink（R7）

把现有单按钮改成与 Link 对称的菜单（同样用 `MenuPrimitive`，
复用 `SkillsCliBatchBar.tsx:61-137` 的结构与 `SkillsCliLinkTargetSummary` 计数展示）：

- 菜单项 = 各平台 + 一个「解链所有平台」（保留现有行为）。
- 只对该平台下状态为 `managed_link` 的技能发 IPC。
- `direct_copy` / `conflict` / `unavailable` 计入 skipped 并显示本地化原因，不发 IPC。

### 2.5 前端侧进度与重复提交保护（R9）

后端不推进度（§1.4），所以状态放 store（组件不得直接 invoke，spec `:64`）：

```ts
// skillsCliStore 新增
batchProgress: { operation: "cleanup" | "update" | "unlink"; completed: number; total: number } | null;
```

- 逐项循环每完成一项就 `set` 一次 `completed`。
- `batchProgress !== null` 即"有批量在飞"，作为**唯一**的重复提交闸门：
  清理入口、批量 Update、批量 unlink 三处都读它来禁用（AC9、AC9c、AC9d）。
- 结束时置回 `null`，无论成功或部分失败。
- 不新增第二套选择状态、不新增第二个 toast helper（R8）——
  结果仍走 `skillsCliActionToast` 与既有 `PlacementMutationOutcome`。

**`skills_cli.busy` 的处理（AC9b）**：逐项路径下每项独立抢 guard，
中途被别的写操作插队就会拿到 `skills_cli.busy`。
该项计入 `failed` 并**继续下一项**，不中断整批，最终 partial outcome 包含它。
不做自动重试——重试会延长 guard 争用窗口，且用户可以直接对失败项再来一次。

### 2.6 排版（R10）

在 §1.3 的既有契约内做，四个子项各自的落点：

| 子项 | 落点 | 不做什么 |
| --- | --- | --- |
| 卡片网格密度 | 卡片内部信息密度与 `gap-3` 的沿用 | 不改 `SKILLS_CLI_GRID_CLASS` 的列数断点，不引入 viewport 断点 |
| 批量栏换行 | 新增按钮不带超容器 `min-width`；`flex-wrap` 已在 `:56` | 不给批量栏加横向滚动容器 |
| 图标热区与 focus ring | 新增图标按钮复用 `ICON_HIT`（`:25-26`） | 不另写一套热区实现 |
| 组头信息层级 | `SkillsCliGroupHeader` 内标题/计数/操作三区，用既有 typography token | 不引入页面私有字号 |

## 3. 数据流

```
清理
  工具栏入口 → deriveCleanupCandidates(skills)   纯函数，无 IPC
  → CleanupDialog（stale 默认全选 / platformUnavailable 默认全不选）
  → 确认 → skills_cli_preview_remove_global（逐项）→ 有 conflict 则禁用确认，零写
  → removeGlobalBatch（逐项）
       每项：lease → guard → 删除；busy → 计 failed，继续下一项
       每项完成 → batchProgress.completed += 1
  → PlacementMutationOutcome → skillsCliActionToast → batchProgress = null → 刷新库存

批量更新
  批量栏 Update → 按 repositoryKey 分组
  → 无 update 元数据 → 不发 IPC，显示 checkFirst 引导
  → 逐组：独立 jobId → skills_cli_apply_updates（该组一次调用）
  → 组间串行；单组失败不阻断
  → 汇总 partial outcome
```

## 4. 契约与兼容性

- **无后端改动**。不新增 IPC 命令、不改签名、不新增错误码 →
  不触发 `pnpm docs:gen` 与 `ipc_registry`。
- `skills_cli_apply_updates` 仍是单 `repositoryKey` 请求，分组在前端（父任务 F5）。
- `SkillsCliBatchBar` 新增两个 prop（Update 动作、unlink 菜单），
  Unlink 从按钮变菜单——内部组件契约变更，测试同步。
- `skillsCliStore` 新增 `batchProgress` 一个状态字段。
- 新增 i18n 键：清理入口与对话框、风险提示、批量 Update、按平台 unlink、进度文案。en/zh 成对。
- `SKILLS_CLI_GRID_CLASS` 与容器查询契约**不改**，
  `src/test/contracts/skillsCliPageShell.test.ts` 必须继续通过。

## 5. 权衡

- **清理覆盖全部 Unavailable 而非只删失效条目**：用户要的是"统一删除"，
  只做失效条目会答非所问。安全性靠默认勾选与风险提示保证，不靠缩小范围。
- **前端计数进度而非等后端事件**：给 link/unlink/remove 加后端进度通道是跨子系统改造，
  远超本任务。前端计数的代价是"已完成"只反映 IPC 返回，不反映远端实际进度——
  对逐项串行路径这两者等价。
- **平台侧三种原因合并成一组**：损失了按原因精细勾选的能力，
  换取"同一技能归属唯一"。行内逐条展示原因把信息补回来。
- **busy 不自动重试**：把控制权留给用户，避免加剧 guard 争用。

## 6. 回滚点

| 单元 | 内容 | 可否单独回滚 |
| --- | --- | --- |
| A | `deriveCleanupCandidates` + 共享的"全 unavailable"判据抽取（纯函数） | 可 |
| B | 清理入口 + `SkillsCliCleanupDialog` | 依赖 A |
| C | 批量 Update（分组 + 串行 apply） | 可 |
| D | 按平台批量 unlink（按钮→菜单） | 可 |
| E | `batchProgress` 与重复提交闸门 | B、C、D 都读它，应先于三者合入 |
| F | 排版优化 | 可 |

B / C / D 相互独立，可分批交付。E 若回滚，三处入口需退回"用 `busy` 粗粒度禁用"，
是可用的降级形态。F 纯样式，独立。
