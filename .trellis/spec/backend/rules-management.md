# Rules 文件库与恢复契约

## 1. Scope / Trigger

新增或修改 Rules IPC、共享规则格式、文件 symlink、导入备份或恢复时适用。
Rules 是独立文件领域，不使用 Skill uid、SQLite 表或 `ensure_centralized`。
前端交互契约见 `../frontend/rules-management.md`。

## 2. Signatures

命令参数使用 camelCase。每条命令包含 `targetId: string`。

| Command | 其他参数 | Result |
| --- | --- | --- |
| `list_rules` | 无 | `RulesSnapshot` |
| `read_rule` | `name` | `RuleDetail` |
| `preview_rules_import` | 无 | `RulesImportPreview` |
| `import_existing_rules` | `entries: {name, sourceFingerprints}[]` | `RulesImportResult` |
| `create_rule` | `name, description, body` | `RuleDetail` |
| `save_rule` | `name, body, expectedRevision` | `RuleDetail` |
| `set_rule_target_enabled` | `name, tool, enabled, expectedDestinationFingerprint: string \| null` | `RuleDetail` |
| `delete_rule` | `name, expectedRevision` | `RuleMutationResult` |
| `recover_rule_operation` | `operationId` | `RuleMutationResult` |

具体 Rust DTO 位于 `services/rules/types.rs`，签名位于 `commands/rules.rs`。
修改后运行 `pnpm ipc:codegen` 与 `pnpm docs:gen`。禁止手写 generated map。

## 3. Contracts

- `targetId` 必须是 `local`。命令在解析 home 或读取规则前拒绝其他目标。
- mutation 使用现有 Local Central mutation guard，阻塞 FS 在 `run_blocking_fs_with` 中运行。
- `RuleTool` 只接受 `claude-code`、`omp`。路径由 `paths.rs` 派生：中央库 `~/.skillport/rules`，工具目录 `~/.claude/rules`、`~/.omp/agent/rules`。
- Claude 设置 `CLAUDE_CONFIG_DIR`，或 OMP 设置 `PI_CODING_AGENT_DIR`、`PI_CONFIG_DIR`、`OMP_PROFILE`、`PI_PROFILE` 时，对应工具标记 unsupported 并拒绝 mutation。
- 共享文件使用 YAML `alwaysApply: true` 与 description。正文编辑不包含头部；`source` 返回完整文件。导入保留选定来源的原正文与换行，备份保留来源完整字节。
- Claude 来源允许无 YAML 的 Markdown。中央文件须含 `alwaysApply: true`；外部移除该字段后，正文只读并拒绝保存和启用。OMP 的 always-apply 桶不要求非空 description。
- 只管理平面、无条件 `.md`。条件字段、嵌套目录、其他格式与保留文件名不进入可写路径。
- 中央扫描跳过保留名与嵌套 `.md` 目录，单个排除项不使整页失败。一工具使用自定义根时，仍可从另一受支持工具导入；禁止读取被排除工具的默认目录。
- `revision` 与普通文件 `fingerprint` 是完整字节 SHA-256。保存须核对 expected revision；接管同内容普通文件须核对确认时的 destination fingerprint。
- `RuleTargetState` 为 `linked | absent | native_equivalent | conflict | broken | unreadable | unsupported | recovery_required`。界面据状态决定操作，服务重新验证实际 FS。
- 导入预览只读。导入保留工具原文件，启用链接是独立动作。两工具使用同一个中央文件，Windows 使用 `symlink_file`，不降级为 copy 或 junction。
- 单文件、扫描与批次预算复用 `ResourceBudget::default_skill()`。当前上限分别为 1 MiB、2048 entries、256 MiB。
- `.backups/<operation-id>/<owner>/<name>.md` 存原字节，`.state/operations/<operation-id>.json` 存恢复 receipt。导入记录在 `.state/import.json`。扫描排除隐藏状态目录。
- receipt 只包含校验后的 UUID、名称、工具、phase 与 fingerprints。恢复路径重新派生；不接受 receipt 提供的任意绝对路径。
- receipt 拒绝重复工具及 kind/phase 不一致。metadata 只有 `NotFound` 可视为缺失；权限或占用失败须停止恢复。
- 写入使用同目录临时文件与原子 persist。启用前完成并回读备份；停用、删除与恢复只操作验证归属的链接。外部变化导致停止并保留备份与未完成 receipt。
- 审计只包含 reviewed code、固定消息、operation correlation 与安全 metadata。禁止记录正文、description 或底层原始错误。

## 4. Validation & Error Matrix

| Condition | Code / behavior |
| --- | --- |
| 非 Local 请求 | `rules.local_only`，不读取规则 |
| 路径分隔符、控制符、`..`、保留名（含 Windows COM/LPT 上标数字）、非法字符或尾随点/空格 | `rules.invalid_name` |
| 大小写同名、越界/外部路径、异文目标或外部链接 | `rules.target_conflict`，保留原对象 |
| expected revision 不匹配 | `rules.revision_conflict`，保留旧内容 |
| 不支持格式或工具根/profile | `rules.unsupported` |
| Windows 权限、占用或 symlink 权限失败 | `rules.permission_denied` |
| mutation guard 超时 | `rules.lock_busy`，retryable |
| receipt 非法或恢复目标被外部修改 | `rules.recovery_blocked` |
| 超预算 | `rules.budget_exceeded` |
| 文件不存在 | `rules.not_found` |
| 其他 FS 失败 | `rules.io`，不透传原始错误 |

## 5. Good / Base / Bad Cases

- Good：同名 Claude/OMP 正文等价，显式导入中央库；两工具原文件仍存在。逐工具确认接管后，两个入口指向共享文件。
- Base：新建规则后工具状态 absent，保存不会自动启用工具。
- Bad：preview 后目标被编辑，用户提交旧 fingerprint；服务拒绝接管。
- Bad：恢复前用户用普通文件替换链接；服务保留用户文件和备份。

## 6. Tests Required

- 临时 home：原正文、备份字节、重复导入、异文与条件格式拒绝。
- Windows 真实文件链接：中文/空格名称、相对 target、双入口回读、编辑后更新、外部与断链归属。
- revision 冲突：磁盘旧内容与前端草稿均保留。
- 故障注入：backup、receipt、原子写入、link commit、部分停用/删除；恢复不得覆盖外部编辑。
- 非 Local、非法名称、目录/文件链接越界、malformed receipt、环境自定义根、资源预算。
- 命令注册、reviewed errors、i18n、generated IPC 与架构文档漂移检查。
- 定向测试不能替代完整 `just ci`。原生 Tauri、工具会话与安装包证据分别记录。

## 7. Wrong vs Correct

```rust
// Wrong: removes the destination without proving file ownership.
std::fs::remove_file(tool_root.join(name))?;

// Correct: validate the name, fingerprint, link destination and receipt first.
service.set_enabled(name, tool, false, None)?;
```
