# Skills CLI 多选、批量操作、导出与安全卸载

父任务：`08-26-skills-cli-redesign`。

## Goal

为 `/skills-cli` 提供可预测的多选、placement-aware 批量链接/解链、版本化导出和安全卸载确认，
同时让所有异步动作具备一致的 busy、partial failure、toast、焦点与 Escape 语义。

## Dependencies

- 必须先完成并合入 `08-26-backend-contract`：提供
  `managed_link | direct_copy | missing | conflict | unavailable` placement、受所有权校验保护的
  link/unlink/remove/export IPC，以及 removal 时“只删除 owned canonical/lock/managed links，保留
  independent direct copies，遇 conflict 拒绝”的契约。
- 必须先完成并合入 `08-26-page-shell`：提供选择模式入口、工具栏 `Export all` 回调、
  `SkillsCliView` 的受控 surface coordinator、卡片/组头扩展点，以及共享
  `skillsCliActionToast` helper（稳定 id、2800ms、replacement 与 semantic icon）。
- 本任务是 `08-26-detail-drawer` 的显式前置：本任务唯一建立共享的 store link/unlink/batch/remove/export
  actions 与卸载对话框入口；详情抽屉只复用，不得复制。
- 交付顺序在 `08-26-install-wizard` 之后。两者没有产品语义依赖，但都会追加共享 i18n/页面接线；本任务启动前先确认 install-wizard 已完成并合入 `dev`，禁止在同一工作树并行写入。

## Requirements

- R1: `Select` 打开后卡片显示复选框；关闭选择模式时清空选择集。选择模式下点击卡片切换选中，
  非选择模式下点击卡片打开详情。组头 `Select all` 把当前桶并入选择集并自动开启选择模式。
- R2: 批量操作栏在选择数大于零时浮于滚动区底部，提供 Link to platform、Unlink、
  Export selected、Uninstall 与清除选择；执行期间禁用会与当前 mutation 冲突的控件并显示可见 busy 态。
- R3: 批量 link 只对目标平台的 `missing` placement 发起；批量 unlink 只对 `managed_link` 发起。
  `direct_copy`、`conflict`、`unavailable` 不调用 mutation IPC，并在菜单/结果中显示本地化原因。
- R4: `skillsCliStore` 是共享写动作的唯一所有者，建立 `linkPlatform`、`unlinkPlatform`、
  `linkPlatformBatch`、`unlinkManagedBatch`、`removeGlobalBatch` 与 `exportInventory`；组件不得直接 invoke，
  详情任务不得建立第二套 action。
- R5: Export all 导出 store 中未经过搜索/分组过滤的完整库存；Export selected 只导出选择集，
  但保持权威库存顺序。两者使用稳定的 v1 JSON envelope、JSON 文件保存对话框和可预测默认文件名；
  用户取消不报错，写入失败显示本地化错误。
- R6: 单个与批量卸载共用确认对话框。影响预览消费 backend
  `skills_cli_preview_remove_global` 返回的不含 path/argv 的结构化 plan，分别显示 owned content、将删除的
  managed links、将保留的 independent direct copies 和 blocking conflicts；任何 conflict 或
  `confirmable=false` 都禁用确认。本任务不提供 `Keep platform link entries`，不在 remove 后重建链接，
  也不传未经验证的 CLI flag。
- R7: 卸载确认后端必须在 mutation 时重新校验 ownership/placement；前端逐技能串行执行，成功项保留、
  失败项继续收集，最终刷新库存并显示安全的 partial outcome。direct copies 保留且不得计入“将删除”。
- R8: 本任务所有反馈必须消费 page-shell 唯一提供的 `skillsCliActionToast` helper，不自建 toast id、
  duration 或 icon 映射。共享 helper 固定稳定 id、2800ms 与 replacement；本任务负责为各操作传正确的
  success/error/destructive semantic。错误文本经 `formatBackendError`，不暴露原始路径或 details。
- R9: Dialog/Menu 使用 Base UI 自身的 topmost Escape dismissal。页面不得注册第二个无条件全局 Escape handler；
  只有焦点位于页面且没有受控 surface/menu 处理 Escape 时，页面容器才清除选择并退出选择模式。
- R10: 所有新增文本 en/zh 成对；图标按钮满足 40px 热区与可见 focus ring；支持 loading、empty、error、
  disabled、hover、focus、selection 与窄宽度换行，不产生水平页面滚动。

## Export v1 Product Contract

- `schemaVersion: 1`
- `exportedAt`: ISO-8601 UTC
- `scope`: `"all" | "selected"`
- `skillCount`: `skills.length`
- `skills[]`: `name`、`source`、`sourceType`、`sourceUrl`、`installKind`、`canonicalPath`、`folderHash`、
  `installedAt`、`updatedAt`、以及按 install target 顺序排列的 `placements[]`
- `placements[]`: `agentId`、`displayName`、`state`；`state` 只能是
  `managed_link | direct_copy | missing | conflict | unavailable`
- v1 只允许上述字段；不得省略、改名、使用等价别名或加入未知字段
- 默认文件名：`skillport-skills-cli-all-YYYY-MM-DD.json` 或
  `skillport-skills-cli-selected-YYYY-MM-DD.json`

## Out of Scope

- 自动把 `direct_copy` 转换成 junction/symlink。
- 保留或重建指向已删除 canonical 的 platform link。
- 删除 independent direct copies 或覆盖 `conflict` 路径。
- CSV、Markdown 或导入该快照；v1 只承诺 JSON 导出。

## Acceptance Criteria

- [ ] AC1 (R1): 打开 `Select` 后卡片出现复选框；关闭后复选框消失、选择集清空。
- [ ] AC2 (R1): 选择模式下卡片点击只切换选中；普通模式打开详情；组头 `Select all` 合并桶内技能且不重复。
- [ ] AC3 (R2): 选择数大于零时批量栏出现并显示正确计数；清空后消失；busy 时冲突动作 disabled。
- [ ] AC4 (R3,R4): Link 只向 `missing` placement 发 IPC，Unlink 只向 `managed_link` 发 IPC；
  direct copy/conflict/unavailable 均不发 IPC，并显示原因。
- [ ] AC5 (R3,R7): 批量 link/unlink 的 partial failure 保留成功项、回滚失败项、刷新库存，
  结果展示 succeeded/failed/skipped 数及本地化安全错误。
- [ ] AC6 (R5): Export all 不受当前搜索、过滤或折叠影响；Export selected 只含选择集，且两者顺序稳定。
- [ ] AC7 (R5): 两种导出都生成符合 v1 contract 的 JSON；默认文件名区分 all/selected；
  保存对话框取消时不调用 export IPC、不显示错误或成功 toast。
- [ ] AC8 (R5,R8): 不可写路径或 export IPC 失败时保留当前 UI 状态，并以稳定 toast id 显示本地化错误。
- [ ] AC9 (R6,R7): 卸载对话框分别显示 backend plan 的 owned folders、managed links、retained copies、
  conflicts；direct copies 不计入删除数，存在 conflict 或 `confirmable=false` 时确认按钮 disabled，
  renderer 不显示或拼接 CLI argv。
- [ ] AC10 (R6,R7): 确认卸载不传 `--keep-links`、不执行 remove-then-relink；成功后 canonical/lock/managed links
  消失，独立 copy 保留，列表与选择集按真实刷新结果更新。
- [ ] AC11 (R7,R8): 批量卸载部分失败时对话框保留失败项与内联错误，成功项从库存/选择集中消失，
  toast 使用 review 过的错误码而非 raw details。
- [ ] AC12 (R8): page-shell 的 shared helper contract test 证明同 id replacement、2800ms 与 semantic icon；
  本任务测试证明 link/unlink/export 使用普通语义，remove success/failure 使用 destructive 语义，且没有直接 `sonner` 调用。
- [ ] AC13 (R9): Base UI 逐次只关闭 topmost surface/menu；页面没有无条件 window Escape listener；
  所有 surface 关闭后，页面内 Escape 才清除选择且不吞掉输入法/文本控件行为。
- [ ] AC14 (R10): 新增文本 en/zh parity 通过，小型图标热区与 focus-visible 符合 spec，键盘可完成选择、菜单、取消与确认。
- [ ] AC15 (R1,R2,R3,R4,R5,R6,R7,R8,R9,R10): 定向 Vitest、`pnpm typecheck`、`pnpm lint` 与最终 `just ci` 通过；
  Windows/WebView2 原生焦点、Escape、保存对话框和视觉验收在执行前保持 `UNVERIFIED`。
