# 修复 Central 更新应用失败并完善诊断日志

## Goal

允许 Update Center 在存在无关待恢复 Central 操作时继续更新或删除其他技能，减少批量仓库快照中的瞬时传输失败，并让每个失败项在 Update Center、Operation Logs 和 Runtime Logs 中保留足以定位阶段、错误族和自动重试结果的稳定诊断信息。

## Background And Confirmed Facts

- 2026-08-09 10:28 的两次 Local apply 均为 6 项更新、0 项成功、6 项失败。第一次耗时 14.432 秒，第二次耗时 324 毫秒。
- 六个 inventory 条目均存在 `repository_id`，本地 hash 等于 baseline hash，远端 hash 不同。更新清单本身满足可更新条件。
- 六个选中技能在两次 apply 期间均未创建新的 `central_update` journal。唯一变化的是无关的 `yao-meta` `central_delete/prepared` 行，其 `delete_restore_collision` 在每次 apply 时被再次记录。
- `update_skills_batch` 在创建任何单项 journal 前调用 target 级 `recover_pending_update_operations`。该函数先恢复 target 下全部 pending delete；任意一行失败后，batch 将同一个字符串错误复制给所有计划。
- 错误随后两次丢失结构：`CentralUpdatesError` 先转为 `String`，`SkillUpdateApplyFailure::new` 再将所有 update 错误改写为同一个 `central_updates.update_failed / central_updates.item_failure`。
- 最新 Operation Log 只保存总数和两个通用聚合码；Runtime Log 只保存 `success_count`、`failure_count` 与耗时。现有日志无法确认失败技能、失败阶段或底层错误族。
- 只读现场证据、假设排除过程和红灯命令保存在 `research/diagnosis.md`。
- 2026-08-09 05:13 的 `deleteMissing=1` 只删除 `claude-md-improver`，但唯一非终态 journal 属于 `yao-meta`。共享 Local/remote delete 单项入口仍在持有 guard 后执行全 target、fail-fast recovery，因此在创建目标删除 journal 前被无关 `delete_restore_collision` 阻断。
- 2026-08-09 05:16 的 bypass refresh 用时 69.272 秒并持久化三个 `github_import.transport_failed`：`bahayonghang/my-ai-cli-toolkit`、`anthropics/skills`、`tw93/kami`。相邻 refresh 的失败数在 2 到 4 之间，且这些仓库此前有成功记录；仓库、branch、显式 redirect rejection 和完全不可达不是当前首要解释。
- snapshot 首轮固定并发 4；codeload 终端请求及 body 读取没有批后自动重试。`GithubImportError::Http` 同时承载 connect/timeout/request/body/普通 HTTP 状态，并统一映射为 `github_import.transport_failed`，现有持久化日志无法区分实际子类。

## Requirements

### R1. 按技能隔离待恢复操作

- `update_skills_batch` 必须继续持有 target mutation guard，且不得增加未加锁的写入路径。
- apply 只检查并尝试恢复本次选中技能对应的非终态 journal。无关技能的 pending 行不得被重试、更新时间或阻断当前更新。
- 选中技能的恢复成功后，正常进入 prepare、stage、commit 和 copy refresh。
- 选中技能的恢复失败时，只将该技能标记为失败；同一批次中没有 pending 行或恢复成功的其他技能必须继续执行。
- target guard 获取失败、pending inventory 查询失败等无法归属到单个技能的前置失败，可以使整批失败，但必须保留稳定阶段和错误分类。
- 启动恢复、Operation Logs 显式 Retry 和显式 Reconcile 的既有全 target 语义保持不变。本任务不得自动核销或删除 pending 行。

### R2. 保留结构化单项诊断

- 从 `CentralUpdatesError` 到 batch outcome、`CentralSkillUpdateFailure` 和 `SkillUpdateApplyFailure` 的转换不得依赖 `Display` 文本重建错误码或分类。
- 每个 apply 失败项必须包含非空的 `step`、`identifier`、`phase`、`errorCode` 和 `errorCategory`。字段值必须来自受控枚举或静态映射。
- `identifier` 必须是安全逻辑标识：技能 ID、仓库 ID、`agent_id::skill_id` 或固定 `batch`。不得使用完整路径、repository source path、逗号拼接的 ID 列表或其他动态错误内容。
- `CentralOperationError` 使用 `central_operation.<code>` 命名空间；无法提供更具体 reviewed code 的更新错误使用动作级 fallback code，同时保留具体 `errorCategory` 和 `phase`。
- `error` 继续是固定 public message。URL、仓库地址、完整路径、manifest、fingerprint、token、凭据、数据库错误文本、命令输出和原始 `Display` 不得进入 IPC、Operation Logs、Runtime Logs、toast 或导出。
- 阶段至少区分 mutation lock、recovery、prepare、stage、database commit、copy refresh、decision apply 和 result finalization；实现可使用更细的稳定阶段，但不得输出动态标签。

### R3. 让日志可用于诊断

- `update_center.apply` Operation Log 在保留现有计数、状态和聚合码的同时，增加有界 `failureItems` 数组。每项只包含 `step`、`identifier`、`phase`、`errorCode` 和 `errorCategory`。
- `failureItems` 按 apply 结果顺序保留前 50 项，并记录 `failureItemsTruncated`。历史日志和超过上限的批次仍可读取和导出。
- Runtime Log 的 partial/failed apply 事件增加排序去重后的 `failure_codes`、`failure_categories` 和 `phase_counts`。Runtime Log 不记录单项动态错误文本。
- 全失败、部分失败和全成功的 Operation Log 状态语义保持 `failed`、`partial` 和 `succeeded`。

### R4. 改善当前界面反馈

- Update Center 失败 toast 使用具体 `errorCode` 的 i18n 文案，并显示安全的单项 identifier。不得继续生成多个无法区分对象与原因的相同 toast。
- 新增的中英文错误文案必须说明受影响对象、原因类别和可执行的恢复位置。已知 pending recovery 冲突应引导到 Operation Logs，不应建议检查网络。
- Operation Log 详情抽屉继续显示通用 JSON；新增 `failureItems` 必须可见、可复制，不要求本任务重做日志页面信息架构。

### R5. 兼容性与验证

- 新字段采用向后兼容的可选/默认反序列化；不新增 SQLite migration，不清理现有 inventory，不修改 Tauri command 名称。
- 修复必须位于共享 Rust service 边界，覆盖桌面端以及复用 `update_skills_batch` 的 normal、force 和 mirror 路径。
- 变更 command payload 类型后，必须同步 TypeScript 类型、IPC 生成物和架构文档生成物。

### R6. 删除操作按选中技能隔离恢复

- shared `central_skills` Local/SSH/WSL single 和 batch delete 必须在一个 top-level target guard 下，只恢复本次请求技能对应的非终态 journal；无关技能不得被检查、重试、更新时间或阻断删除。
- batch 必须去重后一次读取 pending rows，并保持请求首次出现顺序；under-guard 单项 helper 不得重复获取 guard、重复全 target recovery 或为每项重新建立不必要的 remote transport。
- 选中技能自己的恢复失败只影响该技能，其它技能继续删除；全局 guard/DB inventory/transport 初始化失败可以终止整批，但必须保留 typed failure。
- `FailedCentralSkillDelete` 到 inventory apply 的边界必须保留稳定 `phase / errorCode / errorCategory`，不得再次退化为 `decision_apply / central_updates.delete_missing_failed`。
- startup、Operation Logs 显式 Retry 和 Reconcile 的全 target 语义保持不变；不得自动核销现场 `yao-meta`。

### R7. 仓库快照做一次有界补偿重试

- batch snapshot 首轮继续保持最多 4 个并发；首轮全部 settled 后，只对经过 typed classification 判定为安全可重试的 transport/request/timeout/body/5xx 家族进行最多一次串行补偿。
- 补偿重试不得覆盖或重复下载首轮已成功仓库，不得重试 invalid URL/ref、redirect rejection、access denied、not found、parse/integrity 或 budget 错误。
- 第二次成功必须按正常 snapshot 成功处理并写入 cache；第二次失败保留最终 typed error。进度总数仍以唯一仓库数为准，每个仓库只结算一次，补偿内部尝试不得导致完成数超过 total。
- 不引入无限重试、指数循环、用户可调并发或新生产依赖；不通过简单放宽全局 timeout 掩盖未分类原因。

### R8. 刷新日志保留静态传输诊断

- GitHub archive acquisition 必须在不记录 URL、host、owner/repo/ref、响应正文或 reqwest Display 的前提下，区分至少 timeout、connect/request、response body 和 HTTP status/retry exhausted 子类。
- Failed repository 持久化与 IPC 保持稳定 public message/code，并增加向后兼容的可选静态 diagnostic category；不得持久化原始 endpoint 或响应状态详情。
- refresh/retry Operation Log 增加有界失败仓库项与自动重试统计，Runtime Log 增加排序去重的失败 code/category 和 retry attempted/recovered 数量；仓库标识只使用现有安全 repository ID，最多记录 50 项并标记截断。
- 同一 typed classification 必须同时驱动“是否自动重试”和日志 category，禁止通过 Display 文本或错误码字符串嗅探决定重试。

## Acceptance Criteria

- [ ] Local fixture 中，技能 A 有不可恢复的 pending delete，技能 B 可正常更新；同时更新 A/B 时仅 A 返回 `phase=recovery` 的稳定 coded failure，B 完成更新。
- [ ] 只更新技能 B 时，不重试或更新时间属于 A 的 pending 行；B 完成更新并创建、完成自己的 journal。
- [ ] 选中技能自己的可恢复 pending update/delete 行先完成恢复，再执行新更新；恢复失败时仅影响该技能。
- [ ] Fake SSH 和 Fake WSL 覆盖按技能筛选、无关 pending 非阻断和 target identity 保持不变；不得增加额外远端连接或绕过 target guard。
- [ ] batch 的 lock、recovery、prepare、stage、DB commit 和 copy refresh 失败均产生稳定的 `phase / errorCode / errorCategory`，且结果顺序与请求顺序一致。
- [ ] 最新 apply Operation Log 可将每个失败项关联到 identifier、阶段、错误码和分类；超过 50 项时截断标记正确。
- [ ] Runtime Log partial/failed 事件包含 `failure_codes`、`failure_categories` 和 `phase_counts`，不含任何原始错误文本。
- [ ] Update Center 对 reviewed code 显示中英文可执行提示；toast、Operation Log list/detail/export 和 Runtime Log 的对抗性测试均不含 token、URL、完整路径、repository source path、manifest 或命令输出。
- [ ] 既有全成功/部分失败/全失败状态、startup recovery、显式 Retry/Reconcile、normal/force/mirror、Local/SSH/WSL 批处理测试保持通过。
- [ ] Local fixture 中 `yao-meta` 有不可恢复 pending delete，仅删除 `claude-md-improver` 时后者成功，前者的 phase、updated_at 和 error evidence 不变；batch A/B 时只阻断与 pending row 同技能的 A。
- [ ] 删除自身 recovery collision 在 Apply 结果、Operation Log 和 Runtime Log 中保留 `phase=recovery` 与 `central_operation.delete_restore_collision`；无原始路径、manifest 或错误文本。
- [ ] 离线 downloader fixture 中首轮并发 transport 失败、批后串行重试成功时最终无 failed repository；重试峰值为 1，且进度只结算一次。
- [ ] timeout/connect/request/body/5xx 仅重试一次；invalid ref、redirect rejection、access denied、not found、parse/integrity 和 budget 均不自动重试。
- [ ] refresh/retry Operation Log 可关联安全 repository ID、最终稳定 code/category 和 retry 统计；Runtime Log 仅含聚合，不含 URL、owner/repo/ref、状态正文、token 或 reqwest Display。
- [ ] `pnpm typecheck`、`pnpm lint`、相关 Vitest、`cargo fmt --all -- --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked`、生成物只读检查和最终 `just ci` 全部通过。

## Out Of Scope

- 自动核销、删除或修改现场 `yao-meta` pending 行及其 filesystem evidence。
- 弱化 marker、fingerprint、target identity 或恢复碰撞的 fail-closed 保护。
- 修改 repository ownership、inventory merge 语义或 GitHub redirect authority/policy。
- 对普通 HTTP 4xx、认证拒绝、仓库不存在、解析/完整性或预算错误做自动重试。
- 仅凭当前历史 `transport_failed` 宣称三个现场仓库的精确网络根因已经验证。
- 新增数据库 schema、批量清库命令或历史 Operation Log 回填。
- 重做 Operation Logs 页面布局、筛选器、搜索或导出格式。
- 构建或发布 Windows 安装包、推送远端或创建 PR。
