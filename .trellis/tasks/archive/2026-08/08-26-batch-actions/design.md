# Skills CLI 多选、批量操作、导出与安全卸载 — 技术设计

共享视觉与 placement 契约见
`../08-26-skills-cli-redesign/research/design-contract.md`。本文只定义本任务所有的交互与前端契约。

## 1. 前置接口与所有权

本任务只在 `08-26-backend-contract`、`08-26-page-shell`、`08-26-install-wizard` 合入后实施。install-wizard 只是共享 i18n/页面接线的串行交付前置，不向本任务提供产品 API；两者不得在同一工作树并行写入。

后端必须先提供：

- 每个 enabled install target 的 placement：`managed_link | direct_copy | missing | conflict | unavailable`，
  含稳定 `agentId`、display name、path 与安全 reason code；
- `skills_cli_link_platform` 仅把 `missing` 建成 Windows junction/受管 symlink；
- `skills_cli_unlink_platform` 仅删除验证为当前 canonical 所有的 `managed_link`；
- `skills_cli_preview_remove_global` 返回不含 path/argv 的结构化 remove impact plan；
- `skills_cli_remove_global` 在执行时重新验证并只删除 owned canonical、lock record、managed links；
  保留 independent direct copies，发现 conflict 时拒绝；
- `skills_cli_export_inventory(path, json)` 只负责受限写文件，schema 构造在 store。

`page-shell` 必须先提供受控 surface coordinator。页面 application surface 用一个判别联合表示，
不为 install/update/detail/uninstall 各建互相矛盾的独立关闭协议。卸载对话框由本任务注册，
`detail-drawer` 通过同一个 `openUninstall(names)` 入口调用。page-shell 同时唯一提供
`skillsCliActionToast`；batch/install/detail 都只消费，不复制稳定 id、duration 或 icon 映射。

## 2. Canonical Store Owner

`src/stores/skillsCliStore.ts` 是唯一 IPC owner。本任务建立并测试：

```ts
type PlacementMutationOutcome = {
  succeeded: Array<{ skillName: string; agentId?: string }>;
  failed: Array<{ skillName: string; agentId?: string; errorCode: string }>;
  skipped: Array<{ skillName: string; agentId: string; reasonCode: string }>;
};

linkPlatform(skillName: string, agentId: string): Promise<void>;
unlinkPlatform(skillName: string, agentId: string): Promise<void>;
linkPlatformBatch(skillNames: string[], agentId: string): Promise<PlacementMutationOutcome>;
unlinkManagedBatch(skillNames: string[]): Promise<PlacementMutationOutcome>;
removeGlobalBatch(skillNames: string[]): Promise<PlacementMutationOutcome>;
exportInventory(input: ExportInventoryInput): Promise<void>;
```

- link/unlink 先按当前 placement 做前端拒绝，后端仍必须重验；允许 mutation 的单项使用乐观更新，
  失败只回滚该项。
- 批量操作逐项串行，避免同一 Local mutation guard 竞争；每项失败不终止后续项。
- 完成后单独捕获 `loadAll()`；refresh 失败只报告 refresh 错误，不把主动作改判为失败。
- 对外只返回 reviewed `errorCode` + logical identifier；UI 用 `formatBackendError`，不渲染 raw details/path。
- `detail-drawer` 只能调用这些动作；不得复制 store implementation。

## 3. 选择与批量栏

页面持有 `selectedNames: Set<string>`，以 skill name 为 lock-owned 稳定键。关闭 select mode 时清空；
库存刷新后与现存 names 求交集，避免已删除条目残留。

`SkillsCliBatchBar` props：选择数、placement-aware target summaries、link menu controlled state、busy、
以及 link/unlink/export/uninstall/clear callbacks。链接菜单只启用至少有一个 `missing` 的 target；
菜单行同时显示可链接、已受管链接、direct copy、blocked 的计数和原因。

## 4. Placement-aware Link/Unlink

- Link batch：对每个选中技能查指定 target；`missing` 调 link，其余状态进入 `skipped`。
- Unlink batch：遍历全部 target；只有 `managed_link` 调 unlink。`direct_copy` 仍算平台关联，
  但不被伪装成受管链接或未链接。
- `conflict`/`unavailable` 必须可见，不得静默计入成功。
- 卡片、详情和批量栏都从同一 store snapshot 渲染；无组件级第二份 placement 状态。

## 5. Export End-to-End

页面 adapter 暴露：

```ts
exportInventory(scope: "all" | "selected", selectedNames?: Set<string>): Promise<"saved" | "cancelled">;
```

流程：

1. `all` 读取未过滤的 store `skills`；`selected` 按 store 顺序过滤选择集，空集按钮禁用。
2. 用注入 clock 生成 `exportedAt` 与 `YYYY-MM-DD`，用纯函数构造 PRD 定义的 v1 envelope。
3. 动态加载既有 `@tauri-apps/plugin-dialog`，调用 `save`，filter 固定 JSON，defaultPath 按 scope 区分。
4. `save` 返回 `null` 即 `cancelled`：不调用 IPC、不 toast、不改变选择。
5. 获得 path 后 stringify（2-space + trailing newline）并调用 store `exportInventory`。
6. 成功 toast；dialog/serialize/write 失败均使用本地化安全错误并保持当前选择。

纯函数测试固定 clock，断言 schema、字段白名单、顺序、scope、count、默认文件名与换行；
组件测试 mock save，而非依赖真实系统对话框。

## 6. 安全卸载

`SkillsCliUninstallDialog` 接收由当前 snapshot 派生的：

```ts
type RemovalImpact = {
  skillNames: string[];
  ownedContentCount: number;
  managedLinkCount: number;
  retainedDirectCopies: Array<{ skillName: string; agentId: string; displayName: string }>;
  conflicts: Array<{ skillName: string; agentId: string; displayName: string; reasonCode: string }>;
};
```

- `ownedContentCount` 只计 backend contract 证明由 lock 所有且本次将删除的 canonical content root，
  不再等于选择数，也不包含任何 `direct_copy`。
- `managedLinkCount` 只数 `managed_link`，不能用旧 `agentIds.length`。
- direct copies 在独立 warning 区仅按 backend plan 的 skill logical name、agent ID/display name 展示并明确保留；
  不接收/拼接 path，也不传到删除列表。
- conflict 区为 danger alert，非空时确认 disabled；后端命令仍在 mutation 点重验，防止 TOCTOU。
- 对话框只显示 backend-contract 返回的结构化 remove impact plan；不显示或由 renderer 拼接 CLI argv；
  没有 keep checkbox、`--keep-links` 或 remove-then-relink 分支。
- partial failure 时对话框保留 failed names 和内联错误；成功 names 从 selection 移除，随后 refresh。

## 7. Toast

从 page-shell 导入唯一的 `skillsCliActionToast` helper。helper 内部拥有稳定 id、2800ms、replacement
和 semantic icon/token 映射；本任务不得直接调用 `sonner`，也不得再声明 toast 常量或 wrapper。

本任务只拥有 operation-specific invocation：link/unlink/export 传普通 success/error semantic，
remove success/failure 传 destructive semantic，所有错误先经 `formatBackendError`。page-shell 的 helper
contract tests 用 fake timers 验证 id/duration/replacement/icon；本任务 spy helper，验证操作选择了正确
semantic、message key 与安全错误，避免复制 helper 内部测试。

## 8. Escape、焦点与叠层

- Dialog/Menu 均是 controlled Base UI root；Base UI 的 topmost dismissal 是唯一 overlay Escape owner。
- 页面不挂 `window`/`document` 的无条件 Escape listener。
- 选择清除仅绑定在 Skills CLI 页面容器的 `onKeyDown`：事件未被阻止、焦点在页面、
  coordinator 无 open surface 且 link menu 关闭时才生效；输入、textarea、contenteditable 不清选择。
- surface 打开后聚焦标题或第一个动作，关闭后返回触发器。Uninstall 从详情触发时先把 coordinator
  切换为 uninstall（不是叠加两个 dialog），因此一次 Escape 只关闭卸载，再次 Escape 才回到页面选择。
- 组件测试同时打开可组合的 menu/surface 状态，逐次 Escape 断言只有 topmost `onOpenChange` 被调用。

## 9. UI States

- loading/busy：动作按钮 disabled + `aria-busy`，保留已有内容。
- empty：零选择不渲染批量栏；空库存的 Export all disabled。
- error：dialog-local inline error 可重试，toast 只补充；新提交/关闭清旧 error。
- offline：link/unlink/remove/export 均为 Local 能力，不人为依赖网络；只有确实需要网络的后续 update 动作 disabled。
  non-Local：本页 mutation/export 入口按 backend capability disabled，并显示原因。
- hover/focus/selection：使用主题 token；所有小图标按钮满足 40px 热区与 `focus-visible`。
- responsive：批量栏允许换行和纵向收拢，不制造水平页面滚动。

## 10. Tests and Rollback

测试覆盖 store partial outcomes、placement skips、selection reconcile、export schema/save cancel、
removal impact/conflict block、operation-specific toast helper 调用、Base UI topmost Escape、i18n parity 与 keyboard focus。

回滚只回滚本任务新增的批量/卸载/export surface 与本任务拥有的 store actions；
`detail-drawer` 在依赖链上后置，必须先回滚 detail 再回滚本任务。不存在“两个任务独立并行回滚”的承诺。
