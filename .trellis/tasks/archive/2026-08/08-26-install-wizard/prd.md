# Skills CLI 三步安装弹窗

父任务：`08-26-skills-cli-redesign`。

## Goal

用页头 `Install skills` 打开 Source → Skills → Platforms 三步安装弹窗。手工源和最近源都必须先完成
真实 preview 才能进入步骤 2；安装、库存刷新和最近源持久化各自报告真实结果，不能由 stale preview、
未验证 CLI flag 或共享页面/store 文件的并行覆盖制造假成功。

## Dependencies and Ownership

- `08-26-backend-contract` 必须先完成并合入：提供 recent-source exact settings policy、generated Skills CLI
  IPC 类型、reviewed error code 和真实 argv 契约。
- `08-26-page-shell` 必须先完成并合入：提供受控 `{ kind: "install" }` surface、`openInstall`/close/focus
  协议、独立 `SkillsCliInstallSurface` mount seam、siblings 共用的 Skills CLI toast helper，以及唯一
  `skillsCliStore.addGlobal(input): Promise<SkillsCliAddResult>` mutation-only action（内部不调用 `loadAll()`）。
- 本任务唯一拥有 install dialog、install surface adapter、install view model 和 recent-source Zustand store。
  它只消费现有 `skillsCliStore.previewSource/addGlobal/loadAll`，不修改 batch 所拥有的 link/unlink/remove/export
  actions，不重写 `SkillsCliView` 页面壳。
- `batch-actions` 与本任务可作为 page-shell 的 sibling 实施，但不得同时编辑彼此的模块；共享 i18n JSON
  由父协调非重叠 patch/rebase，不能据“可并行”推断共享文件所有权。

## Confirmed Evidence

- canonical `skillsCliStore` 已有 `previewSource` 与 `addGlobal` action（`src/stores/skillsCliStore.ts:114`、
  `src/stores/skillsCliStore.ts:130`），本任务无需建立第二套 add IPC owner；当前 action 在
  `src/stores/skillsCliStore.ts:151` 内联 refresh 的缺陷由 page-shell 前置任务修复为上述 mutation-only 契约。
- backend argv 的权威函数在 `src-tauri/src/services/skills_cli/argv.rs:304`；真实顺序由
  `src-tauri/src/services/skills_cli/argv.rs:313` 至 `src-tauri/src/services/skills_cli/argv.rs:323` 固定为
  source、重复 `-s`、`-g`、重复 `-a`、`-y`。
- 共享平台选择 hook/grid 已存在于 `src/components/platform/PlatformMultiSelect.tsx:42` 与
  `src/components/platform/PlatformMultiSelect.tsx:117`，不需要复制两列 checkbox 实现。
- page-shell 规划已冻结 install mount adapter（`.trellis/tasks/08-26-page-shell/design.md:55`）与统一 toast
  helper（`.trellis/tasks/08-26-page-shell/design.md:146`）；本任务只消费并填充该 seam。

## Requirements

- R1: 弹窗使用本地 Base UI Dialog 原语，标题、副标题、关闭按钮和三段式步骤条完整；打开时固定从
  Source 开始，关闭后再次打开不保留上次 source、selection、preview 或 inline error。
- R2: Source 步骤提供输入、`Preview skills`、支持源提示和最近源 pills。手工按钮与 recent pill 必须调用
  同一 `awaitPreviewAndAdvance(source)`；pending 时停在步骤 1，只有本次 preview 成功结果才进入步骤 2。
- R3: preview 失败、取消或返回空结果时停在步骤 1，保留可编辑 source，显示经
  `formatBackendError` 的 inline error 和稳定 Skills CLI toast；重试前及关闭时清除旧 inline error。
- R4: Skills 步骤显示 normalized source、发现数、已安装数、Select all/Clear all 和两列技能 checkbox；
  默认只选未安装项，已安装项有只读标记，空选择不能继续。
- R5: Platforms 步骤复用 `PlatformMultiSelectGrid`/`usePlatformTargetSelection`，不复制平台多选；默认选中
  `SkillsCliInstallTarget.defaultSelected`，显示每平台现有 placement 关联数，空选择不能安装。
- R6: 命令预览由纯函数按真实 argv 稳定渲染 source、每个 `-s <skill>`、`-g`、每个
  `-a <cliAgent>` 和 `-y`；SkillPort id 必须先映射到 backend 返回的 `cliAgent`。未持久化 help 证明前，
  不显示或执行 `--force`、`--keep-links` 或其它猜测 flag。
- R7: footer 在步骤 2–3 提供 Back；主按钮在 preview/install pending 时 disabled + spinner，选择为空时
  disabled。安装 pending 时忽略 close button、backdrop 和 Escape 的关闭请求，但仍由 Base UI 保持
  topmost dismissal/focus trap，不注册全局 keydown。
- R8: 安装只提交当前成功 preview 的 normalized source、当前技能选择和当前平台选择；旧 preview promise
  晚到或旧弹窗实例不能覆盖当前 session。`addGlobal` 继续使用 canonical store 的 jobId/correlation 约定。
- R9: 安装主 action 成功后关闭弹窗并显示成功 toast；随后库存 refresh 与 recent-source push 作为两个
  独立 follow-up settle。任一 follow-up 失败只报告自身，不把已成功安装改判失败或重新打开弹窗。
  普通 install 不掌握 pinned upstream identity，不调用或写入 update baseline；下一次 update check 可诚实进入
  `baseline_required`，由 update-center 的 Verify exact-match 或 Apply/Reinstall 建立基线。
- R10: recent sources 由独立 `skillsCliRecentSourcesStore` 经 generic settings IPC 读写；最多 8 条、最新在前、
  去重。加载失败不阻断手工 preview；安装成功后 push 失败显示辅助 warning，不能把不受信 persisted JSON
  直接渲染或送给 preview。
- R11: 页头 runtime/doctor 失败时 Install disabled 且无法打开；inventory stale/error 不阻断已有 runtime
  能力。弹窗只从 page-shell mount seam 消费 open/close，不建立第二套页面 surface 状态。
- R12: 所有新增字符串 en/zh 成对，关闭/图标控件满足 40px 热区与 visible focus，键盘可完成三步流程；
  窄宽度内容换行而不产生页面横向滚动。

## Out of Scope

- 修改 link/unlink/remove/export/update store actions或页面分组/卡片布局。
- 自动覆盖已安装技能、使用 `--force` 或假设 `--keep-links` 行为。
- 以 recent pill 的文本直接跳过 preview，或缓存网络结果替代本次 preview。
- 把 jsdom 结果当作 Windows WebView2 焦点、Escape、布局或视觉证据。

## Acceptance Criteria

- [ ] AC1 (R1,R11): runtime 正常时页头按钮通过 page-shell seam 打开步骤 1；runtime/doctor 失败时按钮 disabled 且 surface 不打开。
- [ ] AC2 (R1): 步骤条 current/completed/pending 三态、标题/副标题/关闭按钮可访问；关闭再开后 step/source/selections/preview/error 全部复位。
- [ ] AC3 (R2,R3): 手工 source preview pending 时仍在步骤 1；resolve 成功才进入步骤 2；reject/null/empty 留在步骤 1并显示 inline error + reviewed toast。
- [ ] AC4 (R2,R3): 点击 recent source 立即开始同一 preview 流程但不提前换步；成功使用该次 normalized preview 进入步骤 2，失败留在步骤 1且可编辑重试。
- [ ] AC5 (R2,R8): A preview pending 时手工按钮与 recent pills 均 disabled，不能发起 B；关闭/重开后 A
  晚到不会推进或覆盖新 dialog session，A settle 后才可发起下一次 preview。
- [ ] AC6 (R4): Skills 计数与 installed 标记正确；默认选中集合恰为未安装项；Select all/Clear all 与空选择 disabled 行为正确。
- [ ] AC7 (R5): Platforms 使用共享 multi-select，默认集合等于 `defaultSelected=true` 的目标；计数来自 placement，选择输出映射为去重 `cliAgent` 而非 display name/SkillPort id。
- [ ] AC8 (R6): 命令预览包含与 backend `build_add_global_argv` 同顺序/语义的 source、重复 `-s`、`-g`、重复 `-a`、`-y`；不含 `--force`/`--keep-links` 或不存在的 comma-list flag。
- [ ] AC9 (R7): preview/install pending 时重复提交无效；install pending 的 close/backdrop/Escape 不关闭 dialog，完成后 Base UI 恢复触发器焦点且无全局 Escape listener。
- [ ] AC10 (R8): install payload 只含最后一次成功 preview 的 source、当前 skill names 和映射后的 platform IDs；job correlation 测试证明旧 promise 不覆盖当前 mutation 状态。
- [ ] AC11 (R9): 主 install 成功立即关闭并显示成功；refresh 失败只显示 refresh 错误，recent push 失败只显示辅助 warning，两者都不改判 install 或重复提交；install flow 不调用 baseline IPC/DB write，后续 update check 对无基线技能返回 `baseline_required`。
- [ ] AC12 (R10): recent sources 重启读回、最新在前、去重、截断 8 条；非法 persisted JSON fail closed 为空且不触发 preview，加载失败仍可手工安装。
- [ ] AC13 (R10): recent pill 只在真实 preview 成功后进入步骤 2；成功安装才 push，preview 或 install 失败均不写 recent setting。
- [ ] AC14 (R12): en/zh key parity、40px 热区、focus-visible、Tab/Shift+Tab/Enter/Escape 和窄宽度换行测试通过，无远程资源或原型 hex。
- [ ] AC15 (R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12): focused dialog/surface/view-model/recent-store Vitest、`pnpm typecheck`、`pnpm lint`、`pnpm build` 与最终 `just ci` 通过；Windows installer/WebView2 原生焦点、Escape、中文排版和视觉未实测时保持 `UNVERIFIED`。
