# Skills CLI 技能详情抽屉

父任务：`08-26-skills-cli-redesign`。

## Goal

提供一个可键盘操作、placement-aware 且错误语义完整的技能详情抽屉，展示元信息、平台 placement、
真实 `SKILL.md`，并通过既有受控 surface 与 canonical store actions 完成更新、Reveal 和安全卸载入口。

## Dependencies

- `08-26-backend-contract`：必须提供 bounded `skills_cli_read_skill_md(skillName)`、placement-aware inventory，
  以及只按 lock ownership 解析路径的 `skills_cli_reveal_skill_folder(skillName)`；renderer 不传任意路径。
- `08-26-page-shell`：必须提供 `activeSurface`/`openDetail`/`closeSurface` coordinator、
  content-width signal、卡片 `onDetail`/`onManageLinks` 扩展点与唯一共享的 `skillsCliActionToast` helper。
- `08-26-batch-actions`：是本任务的硬前置，唯一拥有 store link/unlink actions、
  `openUninstall(names)` 与共用卸载对话框。本任务不得建立占位回调或第二套 shared actions。
- `08-26-update-center` 可后置接入 `onUpdate` 与 update state；未接入时不显示不可用的 Update 按钮。

## Requirements

- R1: 抽屉使用受控 Base UI Dialog 从右侧滑出，带遮罩和关闭按钮；Skills CLI 内容宽度
  `>=720px` 时面板为 460px，`<720px` 时占满内容宽度，不能用默认 `md=768px` 代替。
- R2: 头部显示技能名、完整 canonical path 和可选 `Update available` 徽标；元信息 pill 分别显示
  source、`folderHash` 前 7 位、本地 `updatedAt ?? installedAt` 相对时间，空字段完全不渲染。
- R3: 平台区按 install target 顺序展示每个 placement 的 icon、名称、路径/原因和开关。
  `managed_link` 为开且可 unlink，`missing` 为关且可 link；`direct_copy` 为关联但不可 toggle，
  `conflict`/`unavailable` 禁用并显示原因。
- R4: 摘要中的已关联数只计 `managed_link + direct_copy`，总数计 enabled candidates。
  Link all 只处理 missing，Unlink all 只处理 managed links；direct copy/conflict/unavailable 永不调用 mutation IPC。
- R5: `SKILL.md` 区显示 bounded 后端返回的原始内容（含 frontmatter）与 byte size，使用 `<pre>`；
  loading 显示 skeleton，空文件显示明确 empty state，读取失败显示可重试内联错误，不渲染伪空内容。
- R6: 底部动作包含可选 Update、Reveal folder 和 Uninstall。Update 仅在 update available 且回调已接入时显示；
  Reveal 只调用 name-based CLI-safe backend command；Uninstall 原子切换到 batch-actions 的共用确认 surface。
- R7: 普通卡片入口总以 `focusSection=null` 打开；Manage Links 入口以 `focusSection="links"` 打开并滚入链接区。
  关闭或切换技能时重置 focus，普通打开不能继承上一次 links focus。
- R8: 单项 link/unlink 复用 batch-actions 建立的 canonical store actions。乐观更新失败必须回滚；
  抽屉和背后卡片从同一 store snapshot 渲染，并显示经 `formatBackendError` 处理的内联错误；
  补充 toast 只消费 page-shell 的 `skillsCliActionToast`，不得直接调用 `sonner` 或复制 helper。
- R9: 支持 doc loading/error/empty、mutation busy、offline/non-Local、disabled、hover、focus 与 narrow states；
  打开时聚焦标题或首个动作，关闭后焦点回到原触发器，Escape 只由 Base UI topmost dismissal 处理。
- R10: 所有新增字符串 en/zh 成对；小图标按钮满足 40px 热区，Switch/按钮有可读 label，
  元信息和 path 溢出可读取完整值，不依赖颜色传达 placement/error。

## Out of Scope

- 在 renderer 中接受或打开任意文件系统路径。
- 转换/删除 `direct_copy`，覆盖 `conflict`，或在详情内重建 link implementation。
- Markdown 语法高亮、编辑 `SKILL.md` 或缓存跨会话文档内容。
- 在 update-center 合入前伪造 update action。

## Acceptance Criteria

- [ ] AC1 (R1,R9): 普通入口打开受控 drawer，标题可被辅助技术识别；遮罩、关闭按钮和 Escape 均只关闭该 topmost surface，并把焦点返回触发器。
- [ ] AC2 (R1): 内容宽度 719px 时 drawer 为 full width，720px 时为 460px；不存在 720–767px 的误判区间。
- [ ] AC3 (R2): 名称、canonical path、source、hash 前 7 位与本地相对时间正确；任一元字段为空时对应 pill 不渲染。
- [ ] AC4 (R3,R4): 平台按 target 顺序呈现；managed_link/missing 可 toggle，direct_copy/conflict/unavailable disabled 且有可读原因。
- [ ] AC5 (R4): 关联数等于 managed_link + direct_copy；Link all 只向 missing 发 IPC，Unlink all 只向 managed_link 发 IPC。
- [ ] AC6 (R8): 单项或 all link/unlink 成功后抽屉与背后卡片同步；失败项回滚，成功项保留，内联错误与 toast 不含 raw details/path。
- [ ] AC7 (R5): doc loading 时显示 skeleton；成功显示原始 frontmatter/content 与精确 byte size；0-byte 文件显示 empty state。
- [ ] AC8 (R5): doc 失败显示可重试内联错误；切换技能时旧请求结果不会覆盖新技能，关闭后清除当前 doc/error。
- [ ] AC9 (R6): Reveal 只调用 `skills_cli_reveal_skill_folder({ skillName })`，不从 renderer 传 path；
  missing/not-owned/non-Local 错误被本地化并留在可见 surface。
- [ ] AC10 (R6): Uninstall 关闭 detail 并通过 coordinator 打开 batch-actions 共用确认对话框；没有占位回调或第二套 dialog state。
- [ ] AC11 (R6): Update 只有在 `updateAvailable === true` 且 `onUpdate` 已接入时出现；其余状态不显示假按钮。
- [ ] AC12 (R7): Manage Links 入口滚到链接区；关闭或普通入口重开后 focus 为 null，不自动再次滚动。
- [ ] AC13 (R9,R10): 键盘可操作所有开关/按钮，busy 时相关控件 disabled + `aria-busy`；en/zh parity、40px 热区与 focus-visible 检查通过。
- [ ] AC14 (R1,R2,R3,R4,R5,R6,R7,R8,R9,R10): 定向 Vitest、`pnpm typecheck`、`pnpm lint` 与最终 `just ci` 通过；
  Windows/WebView2 原生 drawer 动画、720px content breakpoint、焦点、Reveal 和视觉验收在执行前保持 `UNVERIFIED`。
