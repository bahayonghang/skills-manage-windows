# Skills CLI 页面骨架与紧凑卡片网格

父任务：`08-26-skills-cli-redesign`。

## Goal

把 `/skills-cli` 重排为可访问、可响应的页面壳：页头、工具栏、分组紧凑卡片网格和页尾，
并把 `InventoryCensus` 迁到 `/dashboard`。页面必须保持 runtime 与 inventory 的独立失败语义，
并为后续批量操作、安装、详情、卸载和更新表面提供唯一的受控接线与内容宽度契约。

## Confirmed evidence and dependencies

- **硬前置**：`08-26-backend-contract` 必须完成并合入 `dev`；本任务消费其 placement-aware inventory、
  install targets 与稳定错误契约，不在前端重新推断 junction/copy/conflict。
- 规范性视觉与交互依据是父任务持久化的
  `../08-26-skills-cli-redesign/research/design-contract.md`。缺失的
  `design_handoff_skills_cli/README.md` 与 `support.js` 不是依赖；
  `research/skills-cli-redesign.dc.html` 仅是非规范静态构图灵感。
- 当前 `UnifiedSkillCard` 的 `density="compact"` 使用约 168px 最小高度，不能满足 76px dense
  卡片契约；本任务在 `variant: "skillsCli"` 内增加显式 dense-row 布局，仍由该组件唯一渲染。
- Tailwind 已有容器查询用法；本任务以 Skills CLI 内容容器而非 viewport 为断点所有者。

## In scope

- `/skills-cli` 页头、工具栏、分组网格、组头、加载/空/错误状态和页尾。
- `skillsCliViewModel.ts` 中的纯过滤、分桶、placement-aware 计数派生。
- page-level `activeSurface`、open/close 回调、内容宽度与 layout band 的受控契约；具体浮层由子任务实现。
- 可独立替换的 `SkillsCliInstallMount` adapter；安装子任务只在 adapter 后接入
  `SkillsCliInstallSurface`，不修改 `SkillsCliView` 或 surface controller。
- 共享 `skillsCliActionToast` helper，集中稳定 toast id、2800ms duration 和 reviewed semantic icons；
  batch/install/detail/update 只传本地化消息与受控语义，不复制常量。
- `skillsCliStore.addGlobal` 的唯一 mutation-only 契约：安装 mutation 与后续 inventory refresh 分离，
  供当前页面和后续 install surface 共用。
- `UnifiedSkillCard` 的 skillsCli 专属 dense-row 布局及类型互斥测试。
- `InventoryCensus` 从 Skills CLI 页移除并挂到 Local Dashboard，组件内部不变。
- Export all 按钮的呈现和回调边界；真实导出 controller/store/文件选择归
  `08-26-batch-actions`，写文件 IPC 归 `08-26-backend-contract`。
- en/zh i18n、主题 token、键盘、焦点、hover/focus、disabled、selection 与响应式测试。

## Out of scope

- 批量操作栏、Export selected、卸载确认、安装弹窗、详情抽屉、更新抽屉及其业务/store/backend。
- placement 创建、删除、识别或修复；页面只消费后端权威状态。
- 第二套技能卡片、远程字体/CDN/图片或原型 hex。
- 把静态 HTML、源码检查或 jsdom 类名断言当作 Windows WebView2 原生视觉证据。

## Requirements

- R1: 只能在 `08-26-backend-contract` 完成并合入 `dev` 后启动；若权威 DTO 尚无 placement，
  保持 planning，不用 `agentIds` 或目录存在性猜测替代。
- R2: 只能以父 `research/design-contract.md`、本 PRD 和本设计为规范性设计输入；静态 HTML 中的
  no-op、模板事件、CDN 和尺寸不进入实现或验收。
- R3: 页头显示标题、installed/linked/unlinked/repositories 四个全量派生计数、runtime 状态、
  Refresh 和 Install skills。runtime 失败时显示本地化安全摘要并禁用 Install，但成功或 stale inventory
  网格仍可用；inventory 失败不得被 runtime error 覆盖。
- R4: 计数基于未过滤 inventory：installed=技能总数；linked=至少一个 `managed_link` 的技能数；
  unlinked=至少一个 enabled target 为 `missing` 的技能数；repositories=非空 source identity 去重数。
  `direct_copy` 不计 linked，且不得用 `installed - linked` 推导 unlinked。
- R5: 工具栏包含可清除搜索、Repository/Platform/Status/None、单选平台 chip、Unlinked only、
  Select 和 Export all。Export all 始终表示未过滤的完整 inventory snapshot，不受 filter/group/selection
  影响；`onExportAll` 未接线时 disabled，禁止 no-op。
- R6: 搜索对技能名、来源仓库标签和 canonical path 做 Unicode 大小写无关子串匹配；平台 chip 只匹配
  该 target 的 `managed_link`/`direct_copy`；Unlinked only 只匹配 enabled target 的 `missing`，不把
  `direct_copy`、`conflict`、`unavailable` 冒充未链接。三者可叠加。
- R7: 分组支持 Repository、Platform、Status、None。Repository 未知来源末置；Platform 按 target
  顺序且允许多桶归属，并有 missing/unlinked 桶；Status 至少表达 linked、unlinked、copy-or-conflict；
  None 单桶。空桶不渲染，桶 id 稳定且折叠状态按 id 记忆。
- R8: 吸顶组头具有折叠、桶标签、技能数/managed-link 数、更新徽标占位、Select all 和 Update all
  接线边界。未提供后续回调时相应动作 disabled 或不渲染，不得执行 no-op。
- R9: `UnifiedSkillCard` 为 `variant: "skillsCli"` 新增显式 `layout: "denseRow"`（或等价专属字段），
  `font_scale=1` 使用 76px 目标最小高度和三行单行布局；1.125 字号偏好可按 token 增长，但不得裁切
  标题、路径或焦点环。不得把 168px compact 分支冒充交付。
- R10: dense-row 链接行最多显示 4 个由权威 placement/agent id 驱动的 `PlatformIcon` 和 `+n`；
  无 managed link 时显示本地化状态 pill。选择模式显示 checkbox；普通模式的主详情动作可键盘触发。
  hover 揭示动作在 `focus-within` 同样可见，内部动作不触发卡片主动作。
- R11: 主内容声明命名 container；网格按**内容宽度**精确为 `>=1180px: 4` 列、
  `900–1179px: 3` 列、`<900px: 2` 列。工具栏在 chip 溢出前换行，页面无水平滚动。
  非常规 container utility 必须由构建 CSS 证明，不能降级为 viewport 断点。
- R12: 无 stale inventory 的 loading 显示约 12 张 aria-hidden 骨架并标记 busy；inventory 空和过滤空
  使用不同空态，后者包含 query；刷新失败保留 stale 网格与可重试 inventory error；刷新中禁用重复刷新。
- R13: `InventoryCensus` 从 Skills CLI 页移除，在 Local Dashboard 作为独立 Skills CLI inventory
  区块挂载，沿用 `useSkillsCliStore`，组件内部不改，不重算或覆盖 `dashboardCentralSummary`；
  非 Local 不加载、不渲染。
- R14: 页面拥有唯一 `activeSurface` 判别联合与 `openInstall/openDetail/openUpdate/openUninstall/closeSurface`
  回调；详情普通入口的 focus 默认为 null，Manage Links 才为 links，关闭时复位。页面由
  `ResizeObserver` 测量内容宽度并派生 `<720` drawer band 与 2/3/4-column layout band，供后续浮层消费；
  CSS container 仍是网格布局权威。
- R15: Dialog/Menu/Drawer 继续由 Base UI topmost dismissal 处理 Escape，页面不得注册无条件全局 Escape。
  只在 `activeSurface === null` 且本次事件未被阻止时，页面根的冒泡 key handler 才清除 selection。
- R16: 搜索、分组、chip、Select、折叠和图标按钮有本地化 accessible name；分段/chip 用
  `aria-pressed`，折叠用 `aria-expanded/aria-controls`，小于 40px 的图标控件扩展热区并有
  `focus-visible`。选择态不能只靠颜色，所有交互支持 Tab/Enter/Space。
- R17: 所有文本进入 en/zh；颜色、字体、边框和状态只用主题 token、`statusTone.ts`、
  `displayFont.ts` 和已打包等宽字体；图标仅用 lucide-react 与 `PlatformIcon`。
- R18: 自动化测试覆盖纯函数、错误轨道、disabled/no-op 防线、surface/content-width 状态、
  键盘/焦点、类型互斥、Dashboard、container 类与构建 CSS。浏览器和 Windows WebView2 视觉证据
  在实际执行前标为 `UNVERIFIED`。
- R19: 页面必须通过独立 `SkillsCliInstallMount` adapter 挂载安装表面，稳定 props 至少包含
  `open`、`onOpenChange`、`returnFocusRef` 与 `contentWidthPx`。page-shell 初始 adapter 明确
  `available=false` 并渲染 null，因此 Install 按钮 disabled 而非可点击 no-op；install-wizard 只替换
  adapter 内部为独立 `SkillsCliInstallSurface` 并设 `available=true`，不得修改 `SkillsCliView`、
  `SkillsCliHeader`、`SkillsCliActiveSurface` 或 batch overlay 所有权。
- R20: page-shell 拥有 `skillsCliActionToast.tsx`，导出唯一稳定 id、`2800ms` duration 及
  success/error/destructive-success/destructive-error 的 reviewed icon/tone 映射。helper 只接受调用方已本地化、
  已审阅的 message，不格式化 backend details；同 id 的后续 toast 替换前一个。batch/install 等子任务
  必须消费该 helper，不得各自调用 sonner 重建 Skills CLI toast 参数。
- R21: page-shell 必须把现有 `skillsCliStore.addGlobal` 收敛为唯一 canonical mutation-only action：
  它校验选择、生成/相关联 job id、只调用 `skills_cli_add_global`、在成功时返回非空
  `SkillsCliAddResult` 并清理 mutation/preview 状态；selection failure 写稳定 `actionError` 并 reject，
  busy failure 不覆盖正在运行的 job state，当前 job 的 backend failure 写 `actionError` 后 rethrow，
  stale completion 不覆盖新 job/target state；
  它不得内联调用 `loadAll()`，也不得把 refresh failure 捕获成 add failure。不得再新增第二个同义 add action。
- R22: page-shell 的临时安装入口和后续 `SkillsCliInstallSurface` 都必须在 `addGlobal` 成功后单独调用
  `loadAll()`。若 mutation 成功而 inventory refresh 失败，仍保留 mutation success 结果/成功语义，
  另用本地化 follow-up refresh warning 显示 `inventoryError`，不得写 `actionError`、不得显示 add failed、
  不得重新提交 mutation；runtime-only refresh failure 只更新 runtime pill，也不否定安装成功。

## Acceptance Criteria

- [ ] AC1 (R1,R2): backend prerequisite 已完成并合入 `dev`；实现上下文引用持久化设计契约，
  仓库和实现不依赖缺失 README、`support.js` 或静态 HTML 事件。
- [ ] AC2 (R3,R4): doctor 失败时显示 runtime error、Install disabled，但网格仍渲染；inventory error
  独立显示且刷新失败保留 stale 数据。四计数覆盖 direct_copy/conflict/missing 混合 fixture。
- [ ] AC3 (R5): Export all 在 callback 未提供时 disabled；注入后点击只调用一次，不传 filter/selection
  子集，并有 accessible name；端到端保存由 `batch-actions` AC 验收。
- [ ] AC4 (R5,R6): 搜索三字段、平台单选/再次点击清除、Unlinked only 及叠加结果正确；
  direct_copy、conflict、unavailable 不进入 Unlinked only。
- [ ] AC5 (R7,R8): 四分组生成稳定桶；unknown 末置、Platform 多桶、Status 表达 copy/conflict、
  空桶剔除；折叠按 id 保留，未接线的 Select all/Update all 无 no-op。
- [ ] AC6 (R9,R10): skillsCli 类型具有专属 dense-row 字段，`toModel` 是唯一映射；测试锁定
  76px 目标类且拒绝回退 168px compact，卡片场景正/负类型测试全绿。
- [ ] AC7 (R10,R16): placement 图标最多 4 个并显示 `+n`；checkbox、详情和内部动作可键盘操作，
  hover/focus-within 等价，内部动作不会触发详情。
- [ ] AC8 (R11,R14): contract test 锁定命名 container 与 2/3/4 列；生产 CSS 有 900/1180px
  container rules，无 viewport 替代；ResizeObserver 派生 `<720` drawer band 并与 CSS band 边界测试一致。
- [ ] AC9 (R12): loading、inventory empty、filtered empty、stale+error、refreshing 五态有测试；
  骨架区域 busy，重复刷新 disabled，filtered empty 文案包含 query。
- [ ] AC10 (R13): Skills CLI 不渲染 census；Local Dashboard 渲染且不改变 central summary 数据源，
  非 Local 不调用 loader；既有 census 测试保持通过。
- [ ] AC11 (R14,R15): surface controller 测试覆盖普通详情 focus=null、links focus、关闭复位、
  uninstall payload 与内容宽度 bands；Base UI 浮层存在或事件已 prevented 时 Escape 不清 selection，
  无浮层且未处理时才清除。
- [ ] AC12 (R16,R17): 新控件可通过 role/name/pressed/expanded/disabled 查询；icon 热区和 focus-visible
  类存在；en/zh 成对且无原型 hex、远程字体/CDN/图片。
- [ ] AC13 (R18): focused Vitest、typecheck、lint、build、task validate 和 `just ci` 全部通过；
  未执行的浏览器/Windows WebView2 截图、中文排版、焦点环和断点检查报告 `UNVERIFIED`。
- [ ] AC14 (R14,R19): 页面测试证明 Install 经 `SkillsCliInstallMount` 消费同一个 activeSurface；
  adapter unavailable 时按钮 disabled 且没有 no-op，available 时 open/close/return-focus/contentWidth 原样传递；
  install-wizard 的 planned file set 不含 `SkillsCliView.tsx`、Header 或 surface controller。
- [ ] AC15 (R20): helper 单元测试锁定稳定 toast id、2800ms、同 id replacement 及四种 semantic icon/tone；
  类型/API 不接受 raw backend object，batch/install 集成测试 mock 该 helper 而非直接断言重复 sonner 参数。
- [ ] AC16 (R21): store 测试证明 add success 只调用一次 `skills_cli_add_global`、返回非空 result、
  清理当前 job/preview 且不调用 list/targets/doctor；selection/busy/backend rejection 设置正确状态并 reject，
  mutation 失败和 stale completion 仍遵守 job-correlation 契约。
- [ ] AC17 (R21,R22): 页面/controller 测试分别覆盖 mutation failure 与 mutation success + inventory refresh
  failure：后者只显示“安装成功但刷新失败”的 follow-up warning，保留成功结果，不发 add-error、不重试 add；
  install mount consumer 使用同一个 `addGlobal` + separate `loadAll` seam。
