# Rules 前端状态契约

## 1. Scope / Trigger

修改 `/rules`、Rules store、规则切换或工具链接交互时适用。
后端命令与错误矩阵见 `../backend/rules-management.md`。

## 2. Signatures

`useRulesStore` 位于 `src/stores/rulesStore.ts`，领域类型位于 `src/types/rules.ts`。
主要动作：`load()`、`select(name)`、`editBody(body)`、`discardDraft()`、`save()`、
`previewImport()`、`importRules(names)`、`createRule(name, description, body)`、
`setTargetEnabled(tool, enabled, fingerprint?)`、`deleteRule()`、`recover(operationId)`。

## 3. Contracts

- `/rules` 使用 lazy route；左侧入口在 Central Skills 后。沿用 NavItem、字体与主题 token。
- 组件调用领域 store。只有 store 调用 typed IPC；fixture 在命令边界提供合成数据。
- Rules drafts 只存会话内存，按文件名记录 `body, baseBody, revision`。离开页面或切换 target 后保留；关闭应用后不持久化。
- 切换规则遇到 dirty draft，提供保存并切换、放弃、取消。保存失败时保持当前规则和草稿。
- 保存使用草稿的 expected revision。保存进行中再次编辑时，成功响应更新 base/revision 并保留新正文。
- 请求捕获 target、目标切换 generation 与 request ID。读写响应及 cleanup 均须核对归属；Local→远程→Local 后，旧 Local mutation 也不得覆盖新状态。
- 非 Local 页面禁用 mutation 并说明限制，不调用规则 FS 读取。
- 导入和工具启用分别确认。同内容普通文件的接管确认展示工具路径及备份位置，并提交当前 fingerprint。冲突/不支持状态没有启用操作。
- 启停按工具 pending；失败显示 reviewed localized error，必要时要求重新扫描。恢复按钮先展示恢复说明和备份路径。
- body 为可编辑正文；preview 使用 compact Markdown renderer；source 为只读完整文件。compact 模式没有 `SKILL.md` 标记。
- 规则列表与正文分别滚动。文件名和关键路径可完整读取；最小原生窗口为 900×600。
- 新可见文本进入中英文 i18n。用户正文不会随界面语言切换而翻译。

## 4. Validation & Error Matrix

| Condition | UI behavior |
| --- | --- |
| 保存 revision 冲突 | 显示错误，保留草稿与原 revision |
| 旧 read 返回 | 丢弃响应 |
| 切换到 SSH/WSL | 禁用 Rules 操作，保留 Local 草稿 |
| 接管异文/外部链接 | 无接管入口，服务仍重新验证 |
| 接管或恢复失败 | 确认框保留并显示 localized error |
| 未完成 receipt | 页面显示待恢复操作，独立确认 |
| 浏览器 fixture | 显示合成数据提示，不能声称真实磁盘操作通过 |

## 5. Good / Base / Bad Cases

- Good：编辑后切页再返回，草稿仍在；保存成功后清除对应 dirty 标记。
- Base：搜索只过滤本地列表，不改变正文。
- Bad：target 或选中规则变化后，晚到 read 覆盖新详情。

## 6. Tests Required

- store：草稿保留、保存冲突、保存期间继续编辑、旧响应、per-tool pending、恢复失败。
- page：搜索、预览/source、导入、逐工具启停、dirty 确认、非 Local、错误反馈。
- 浏览器渲染：中英文，1200×800、900×800、1200×600、900×600，导航展开/收起，最大字体与边界尺寸。
- 原生 WebView/IPC 与 browser fixture 证据单独记录。完整门槛为 `just ci`。

## 7. Wrong vs Correct

```ts
// Wrong: a component performs the filesystem operation through raw IPC.
await invoke("set_rule_target_enabled", payload);

// Correct: a component uses the domain action; the store owns request state.
await useRulesStore.getState().setTargetEnabled(tool, true, fingerprint);
```
