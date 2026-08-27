# Skills CLI 失效条目清理与多选批量更新

父任务：`08-27-skills-cli-availability-remote`
源需求：U2、U3、U4

## Goal

为 `/skills-cli` 提供覆盖全部 `Unavailable` 技能、但以安全默认保护健康技能的统一清理入口，
把多选批量能力补齐到「更新」与「按平台解链」，并优化卡片区与批量栏的排版密度。

## Background

用户诉求三条：

1. 「下面的 unavailable 帮我设计个统一删除的功能和对应的按钮」
2. 「还得做一个 skills 多选功能，支持多选更新、删除、unlink 等功能」
3. 「优化相关样式和排版」

仓库证据（详见父任务 `research/current-state-evidence.md`）显示第 2 条大部分已存在，第 1 条存在语义陷阱。

## Decisions

- **D1（Q2 = 方案 B）**：清理入口**覆盖所有徽章为 `Unavailable` 的技能**，
  但确认对话框按 `reasonCode` 分组，**默认只勾选 `canonical_missing`**；
  其余分组默认不勾选，手动勾选时显示风险提示。
- **D2**：分组采用两层，理由见下方 Confirmed Facts 的「删除语义差异」：
  - **失效条目组**（`canonical_missing`）：canonical 目录已不存在，删除只清理 lock 记录与残留 managed link。默认勾选。
  - **平台不可用组**（`platform_unsupported` / `platform_not_detected` / `platform_disabled`）：
    技能本体健康，勾选后执行的是**真实卸载**，会删除 canonical 内容。默认不勾选并显示风险提示。

## Confirmed Facts

### `Unavailable` 徽章的真实语义

- 徽章只在「该技能在所有平台都没有 managed_link」且聚合状态为 `unavailable` 时出现
  （`SkillCardDenseRow.tsx:30-60,87`）。`missing` 优先级高于 `unavailable`，
  所以实际条件是**所有 placement 都是 `unavailable`**。
- 后端 `classify_absent`（`placement.rs:73-110`）按顺序产生 4 种 `reason_code`：

  | 顺序 | reason_code | 语义 |
  | --- | --- | --- |
  | 1 | `canonical_missing` | canonical 目录已不存在 → 真正的失效/幽灵条目 |
  | 2 | `platform_unsupported` | 平台不支持本地放置 |
  | 3 | `platform_not_detected` | 本机未检测到该平台 |
  | 4 | `platform_disabled` | 平台被用户禁用 |

- `canonical_missing` 与平台无关且判定最优先：canonical 缺失时**每个** placement 都是该 reason。
  因此「某技能属于失效组」等价于「其任一 placement 的 reasonCode 为 `canonical_missing`」。
- 反之，2/3/4 是平台侧原因，技能本体健康。同一技能的不同 placement 可能分别是
  `platform_not_detected` 与 `platform_disabled`，因此平台侧不再细分为三个可勾选组，
  合并为一个「平台不可用」组，在行内展示各自原因。
- **删除语义差异（D2 的依据）**：失效组删除时 canonical 本就不存在，属于清理记录；
  平台不可用组删除时 canonical 存在，属于真实卸载。二者风险不同，不能共用同一默认值。
- `reasonCode` 已暴露给前端（`generatedCommandMap.ts:1048-1055`）。

### 多选批量现状

归档任务 `08-26-batch-actions` 已交付：选择模式、卡片复选框、组头 `Select all`、
批量栏（Link to platform / Unlink / Export selected / Uninstall）、
`reconcileSelectedNames` 对账、`PlacementMutationOutcome` 的 succeeded/failed/skipped 语义、
`skillsCliActionToast` 共享 helper。

真实缺口三项：

1. 批量栏无 Update 按钮。更新入口只有组头 `onUpdateAll`（按仓库）与详情抽屉单技能。
2. 无失效条目清理入口。
3. `unlinkManagedBatch` 只能「解链所有平台」，link 侧有 agent 菜单而 unlink 侧不对称。

### 批量操作在后端是逐项的（决定清理入口的规模上限与反馈形态）

| 操作 | 后端是否原生批量 | 形态 |
| --- | --- | --- |
| `skills_cli_add_global` | **是** | 一次 CLI 调用，argv 重复 `-s` / `-a` |
| `skills_cli_apply_updates` | **是**（单仓库内多技能） | 一次 mutation guard 覆盖整个 apply |
| `skills_cli_verify_update_baseline` | **是** | 一个 job lease 下循环 |
| `skills_cli_remove_global` | **否**，签名是单个 `skill_name` | 前端 `removeGlobalBatch` 逐项循环 |
| `skills_cli_link_platform` / `unlink_platform` | **否** | 前端 `runPlacementBatch` 逐项循环 |

逐项路径的每一项都会**独立**申请并释放一次 exclusive job lease 与一次
`acquire_target_mutation_guard`（默认 10s 超时）。因此清理 N 个技能 = N 轮加解锁，
期间任何其他写操作都会间歇性撞上 Busy。

**进度事件只存在于 update 子系统**（`skills-cli://update-progress`，
`services/skills_cli/updates/mod.rs:32`）。install / link / unlink / remove **没有**进度通道。
所以批量清理的进行中反馈只能由前端按已完成项数自行渲染，不能指望后端推送。

### 批量更新的硬约束

`skills_cli_apply_updates` 每请求只接受一个 `repositoryKey`（`generatedCommandMap.ts:989-993`）。
`openUpdateSurface`（`skillsCliPageHandlers.ts:211-230`）在 `repositoryKey` 为空时 toast 拒绝。
跨仓库选择集必须按 repositoryKey 分组串行 apply（各自独立 jobId）。

### 必须遵守的既有契约

- 组件不得直接 `invoke`；只有 `src/stores/skillsCliStore.ts` 可以（spec `skills-cli-global.md:64`）。
- 卸载共用 `SkillsCliUninstallDialog`，影响预览消费 `skills_cli_preview_remove_global`
  返回的不含 path/argv 的结构化 plan（归档 `08-26-batch-actions` R6）。
- 删除路径：只删 owned canonical / lock / managed links，保留 independent direct copies，
  遇 conflict 拒绝且零写（spec `skills-cli-global.md:99-102`）。
- 反馈必须走 `skillsCliActionToast`，不得直接调 `sonner`（归档 R8）。
- Escape 走 Base UI topmost dismissal，页面不注册第二个无条件全局 handler（归档 R9）。

## Requirements

- R1：新增「清理 Unavailable 条目」入口，候选集合 = 所有徽章为 `Unavailable` 的技能。
  入口位置与 `Export all` 同排（工具栏），不占用批量栏空间。
- R2：清理确认对话框按 D2 的两层分组渲染，每组显示技能数与逐条原因；
  失效条目组默认全选，平台不可用组默认全不选。
- R3：平台不可用组被勾选时显示风险提示，明确说明这些技能本体健康、删除等同真实卸载。
- R4：清理执行复用既有 `skills_cli_preview_remove_global` 影响预览与 `removeGlobalBatch` 路径，
  不新建第二条删除通道；conflict 仍然拒绝且零写，independent direct copies 仍然保留。
- R5：批量栏新增 Update 动作。选择集按 `repositoryKey` 分组，各组独立 jobId 串行 apply；
  单组失败不阻断其余组，结果汇总为 partial outcome。
- R6：未 Check updates（无 update 元数据）时点击批量更新不发 IPC，
  给出与现有 `skillsCli.updates.checkFirst` 一致的引导。
- R7：批量 unlink 提供与 link 对称的按平台选择能力，同时保留「解链所有平台」。
  只对 `managed_link` 发 IPC；`direct_copy` / `conflict` / `unavailable` 计入 skipped 并显示本地化原因。
- R8：所有新增写动作归属 `skillsCliStore`，复用既有 `PlacementMutationOutcome` partial-failure 语义，
  不新建第二套选择状态或 toast helper。
- R9：清理与批量操作执行期间提供前端侧的进行中反馈（已完成 / 总数），
  因为后端在这些路径上不推送进度事件。执行中禁止重复触发同一批操作。
- R10：样式与排版优化限定本页，覆盖四个子项：卡片网格密度、批量栏换行且不产生横向滚动、
  徽章与图标按钮的 40px 热区与 focus ring、组头信息层级。不改全局设计 token。
  为使验收可重复，前置条件是**内容容器宽度**（不是 viewport）：
  本页已用命名容器查询 `@container/skills-cli`，档位由既有常量定义——
  `SKILLS_CLI_THREE_COLUMN_MIN_PX = 900`、`SKILLS_CLI_FOUR_COLUMN_MIN_PX = 1180`
  （`src/pages/skillsCliViewModel.ts:44-50`）。
  固定三档取样：**720（2 列）/ 1000（3 列）/ 1280（4 列）**。
  所有 R10 子项的验收都必须声明自己在哪一档下断言（TPR-08）。
  **既有密度契约不得推翻**：`SKILLS_CLI_GRID_CLASS` 的取值与「禁止 viewport 断点」
  已被 `src/test/contracts/skillsCliPageShell.test.ts:44-56` 锁定，
  本任务的排版优化只能在该契约内做，不得引入 `md:` / `lg:` 一类断点。
- R11：新增文案 en/zh 成对。

## Acceptance Criteria

- [ ] AC1 (R1,R2)：候选集合与分组计算有单元测试，覆盖：
      canonical 缺失 → 落入失效组且默认勾选；
      canonical 健康但所有平台未检测/禁用 → 落入平台不可用组且默认不勾选；
      同一技能混合 `platform_not_detected` 与 `platform_disabled` → 落入单一平台不可用组并逐条展示原因。
- [ ] AC2 (R2)：库存中不存在 `Unavailable` 技能时，清理入口不可用或隐藏，点击不发 IPC。
- [ ] AC3 (R3)：勾选平台不可用组后风险提示可见；未勾选时不可见。
- [ ] AC4 (R4)：清理确认只调用 `skills_cli_preview_remove_global` 与 `removeGlobalBatch`；
      存在 conflict 的技能禁用确认且零写；independent direct copies 不计入删除数。
- [ ] AC5 (R5)：选择集跨 2 个仓库时发出 2 次 `skills_cli_apply_updates`，jobId 各不相同；
      第一组失败不阻断第二组，结果汇总为 partial outcome。
- [ ] AC6 (R6)：未 Check updates 时点击批量更新不发 IPC，显示引导文案。
- [ ] AC7 (R7)：按平台批量 unlink 只对该平台 `managed_link` 发 IPC；
      其余状态计入 skipped 且展示本地化原因，不发 IPC。
- [ ] AC8 (R8)：测试断言页面无直接 `invoke` 与直接 `sonner` 调用，
      选择状态仍是 `SkillsCliView` 单一 `selectedCardNames`。
- [ ] AC9 (R9)：清理 N 个技能时展示已完成 / 总数进度；执行中重复点击不发起第二批。
- [ ] AC9b (R9)：某一项因 mutation guard 争用返回 `skills_cli.busy` 时，
      该项计入 failed 而不中断整批，最终 partial outcome 包含它。
- [ ] AC9c (R9)：**非 cleanup 的批量操作同样有进度与重复提交保护**（TPR-08）。
      以批量更新为场景：选择集跨 2 个仓库时展示「已完成 1/2 → 2/2」，
      第一组进行中再次点击批量更新不发起第二批（不产生第三次 `skills_cli_apply_updates`）。
- [ ] AC9d (R9)：按平台批量 unlink 进行中，批量栏的该动作处于禁用态，
      重复点击不发起第二批。

R10 的四个子项分开验收，并按「自动化可断言」与「Windows 原生视觉检查」划界（TPR-08）。
自动化部分在 jsdom 下运行，只断言 DOM 结构与类名契约；
真实布局（列数、换行位置、是否出现横向滚动条、焦点环观感）属原生检查。

- [ ] AC10a (R10) — 批量栏换行 · 自动化：批量栏容器保留 `flex-wrap`
      （`SkillsCliBatchBar.tsx:56` 已有），且**新增**的动作按钮不引入超过容器的固定
      `min-width`。断言对象是类名契约，不是 `scrollWidth`（jsdom 不做真实布局）。
- [ ] AC10b (R10) — 批量栏换行 · 原生：Windows x64 bundle 下于容器宽 1280 / 1000 / 720
      三档逐一目视确认批量栏换行且页面无横向滚动条。执行前标记 `UNVERIFIED`。
- [ ] AC10c (R10) — 卡片网格密度 · 自动化：`skills-cli-layout-bands` 的 `data-grid`
      在容器宽 720 / 1000 / 1280 下分别为两列 / 三列 / 四列档；
      卡片间距沿用 `SKILLS_CLI_GRID_CLASS` 的 `gap-3`，不引入页面私有像素值。
      同时断言既有契约测试仍通过——不得出现 `md:` / `lg:` 等 viewport 断点。
- [ ] AC10d (R10) — 卡片网格密度 · 原生：三档容器宽度下目视确认实际列数与间距
      符合 AC10c 声明。执行前标记 `UNVERIFIED`。
- [ ] AC10e (R10) — 组头信息层级 · 自动化：组头渲染出「标题 / 技能计数 / 组级操作」
      三个区域，标题使用既有 typography token 的标题层级类，计数为次级文本层级，
      三者顺序稳定。不引入页面私有字号。
- [ ] AC10f (R10) — 图标按钮 · 自动化：新增图标按钮复用 `SkillsCliBatchBar.tsx:25-26`
      既有的 `ICON_HIT` 常量（`size-8` + `after:size-10` = 40px 热区），
      不另写一套热区实现；且具备 `focus-visible` 样式类；键盘 Tab 可达。
- [ ] AC10g (R10) — 图标按钮 · 原生：Windows/WebView2 下确认焦点环可见且热区实测 ≥ 40px。
      执行前标记 `UNVERIFIED`。
- [ ] AC11 (R11)：i18n en/zh parity 通过。
- [ ] AC12 (Completion Gate)：定向 Vitest、`pnpm typecheck`、`pnpm lint` 与 `just ci` 通过。
      来源是 `AGENTS.md` 的 Completion Gate 一节，不隶属本任务的任一 R（TPR-09）。
      原先此处与上一条重复编号为 AC11，已改为 AC12 以与 `check.jsonl` 的引用一致。

## Out of Scope

- 自动把 `direct_copy` 转换为 junction/symlink。
- 删除 independent direct copies 或覆盖 conflict 路径。
- 重建指向已删除 canonical 的 platform link。
- 更新流程的后端契约变更（`skills_cli_apply_updates` 仍是单仓库请求）。
- 全局设计 token 与其他页面的排版。

## Ordering

在 `08-27-skills-cli-doctor-gate` 合入 `dev` 之后开发：
两者都会改 `SkillsCliView.tsx` 与 `SkillsCliBatchBar.tsx` 的禁用逻辑，同一工作树并行写会冲突。
