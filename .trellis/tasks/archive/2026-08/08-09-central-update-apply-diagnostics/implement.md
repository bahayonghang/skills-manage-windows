# 实施计划：修复 Central apply 阻断并补齐诊断

## Stage 0. Red Regressions

- [x] 在 Central update batch 测试中构造技能 A 的不可恢复 pending delete 和技能 B 的有效更新。证明当前代码将 A 的 recovery error 复制给 A/B，且 B 未创建 journal。
- [x] 增加「只选择 B」回归，断言当前代码错误地重试并更新 A 的 pending 行。
- [x] 扩展 apply log 单元测试，要求 `failureItems`、截断计数、runtime stable fields 和阶段信息；当前实现必须先失败。
- [x] 扩展 Update Center 测试，要求 toast 能区分 identifier 与 reviewed error；当前通用提示必须先失败。

Focused commands:

```powershell
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked central_updates::core -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked commands::skill_update_inventory::apply_log::tests -- --nocapture
rtk pnpm vitest run src/test/components/central src/test/stores/updateCenterStore.test.ts
```

## Stage 1. Scoped Recovery

- [x] 从 Local/remote delete recovery 中提取单行 under-guard helper；保留全 target fail-fast wrapper。
- [x] 从 pending update recovery 中提取单行 helper，并增加 selected skill 过滤入口。
- [x] 在 `update_skills_batch` 中一次读取 pending rows，只恢复 selected skills，并将 row-specific failure 写入对应结果槽位。
- [x] 保持 target guard 覆盖 recovery、journal insert、filesystem mutation 和 DB commit；不得新增嵌套锁或 unlocked fallback。
- [x] 覆盖 Local、Fake SSH、Fake WSL、同技能恢复成功/失败、无关技能不触碰、global preflight failure 和结果顺序。

Review gate：pending skill A 的失败不改变 B 的执行结果；只选择 B 时 A 的 `updated_at` 和 error evidence 不变化。

## Stage 2. Structured Failure Propagation

- [x] 增加受控 `CentralUpdateFailurePhase` 和内部 item error，移除 batch/core/inventory 边界上的提前 `.to_string()`。
- [x] 为 `CentralUpdatesError` 建立总是返回稳定 category、phase 和安全 code 的映射；Central operation code 使用命名空间。
- [x] 扩展 `CentralSkillUpdateFailure` / `SkillUpdateApplyFailure`，并确保 `error` 只序列化固定 public message。
- [x] 将所有 failure constructor 的 `identifier` 收敛为安全逻辑标识；移除完整 path、source path 和逗号拼接 ID 列表。
- [x] 更新 update state/progress 的失败消息，禁止原始路径、URL、数据库文本或远端输出进入 IPC。
- [x] 覆盖 mutation lock、recovery、prepare、stage、DB commit、copy refresh、decision apply 和 result finalization 的字段映射。

Review gate：所有 item failure 均有非空 `phase / errorCode / errorCategory`，且对抗性原始错误只存在于内部 error source，不出现在序列化结果。

## Stage 3. Operation And Runtime Diagnostics

- [x] 为 `apply_result_details` 增加最多 50 条 `failureItems` 和 `failureItemsTruncated`。
- [x] partial/failed tracing 事件增加排序去重的 `failure_codes`、`failure_categories` 和 `phase_counts`。
- [x] 测试全成功、部分失败、全失败、50/51 项边界、稳定排序和历史可选字段。
- [x] 对 Operation Log list/detail/export 与 Runtime Log read/export 增加 token、URL、完整路径、manifest 和命令输出缺失断言。

## Stage 4. Frontend And Generated Contracts

- [x] 更新 `SkillUpdateApplyFailure` TypeScript 类型、fixtures 和 generated command map。
- [x] Update Center toast 显示 identifier 与 localized reviewed error；未知码保持固定 fallback。
- [x] 添加中英文 Central operation/update failure 文案，说明到 Operation Logs 处理 pending recovery。
- [x] 覆盖 Apply selected 和 deleted-copy cleanup 两条失败反馈路径，消除字符串拼接的旧错误解析。
- [x] 运行 `pnpm ipc:codegen` 和 `pnpm docs:gen`，提交所需生成物；随后运行两类只读检查。

Focused frontend commands:

```powershell
rtk pnpm vitest run src/test/components/central src/test/stores/updateCenterStore.test.ts src/test/pages/OperationLogsView.test.tsx
rtk pnpm typecheck
rtk pnpm lint
rtk pnpm ipc:codegen:check
rtk pnpm docs:gen:check
```

## Stage 5. Specs And Full Validation

- [x] 更新 `central-update-batching.md`：batch recovery 按 selected skill 隔离，并定义 item diagnostics。
- [x] 更新 `fs-db-operation-journal.md` 与 `central-mutation-lock.md`：新 mutation 在同一 target guard 下恢复受影响技能，startup/显式 recovery 仍为全 target。
- [x] 运行任务内 live probe，记录历史现场仍为 RED；不得把未再次执行现场 apply 写成修复通过证据。
- [x] 运行完整验证：

```powershell
rtk pnpm typecheck
rtk pnpm lint
rtk pnpm test
rtk pnpm ipc:codegen:check
rtk pnpm docs:gen:check
rtk pnpm docs:build
Push-Location src-tauri
rtk cargo fmt --all -- --check
rtk cargo clippy --all-targets --locked -- -D warnings
rtk cargo test --locked
Pop-Location
rtk just ci
```

- [x] 运行 `task.py validate`、`git diff --check`，并检查最终 diff 只包含本任务代码、测试、i18n、spec、生成物与 Trellis 工件。

## Rollback Points

- Scoped recovery helper 与 batch 调用必须一起回滚。
- Typed Rust failure、TypeScript 类型、i18n 和生成物必须一起回滚。
- Operation Log item payload 与 Runtime Log aggregates 可独立回滚，但不得恢复原始错误字符串日志。

## Live Data Boundary

自动验证不得修改 `~/.skillsmanage/db.sqlite`、Central skills 或 `yao-meta` recovery evidence。修复完成后，如需对现场数据再次 Apply 或 Reconcile，必须取得单独明确授权。

## Stage 6. Delete-Missing Regression And Shared Fix

- [x] 增加 Local 红灯：无关 `yao-meta` pending collision 时，仅删除 `claude-md-improver` 必须成功且无关 row evidence 字节级不变。
- [x] 增加同技能 collision 与 A/B batch 红灯，锁定 typed recovery failure、其它技能继续和首次请求顺序。
- [x] 将 Local/SSH/WSL delete single/batch 收敛为一个 top-level guard、一次 selected pending inventory 和 under-guard 单项执行；保留 full-target startup/Retry/Reconcile。
- [x] 扩展 `FailedCentralSkillDelete` 与 inventory delete adapter，保留稳定 phase/code/category 和固定 public message；补 IPC/codegen/i18n 对抗性测试。
- [x] Fake SSH/WSL 覆盖 target identity、无关 pending 非阻断、同技能阻断和无多余连接/guard acquisition。

Focused command:

```powershell
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_skills::tests -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates::inventory::tests::apply_delete_missing -- --nocapture
```

## Stage 7. Snapshot Retry And Typed Diagnostics

- [x] 增加离线红灯 downloader seam：首轮并发失败、批后串行一次成功；断言调用次数、峰值并发、最终集合与一次性 progress settlement。
- [x] 增加 non-retry matrix：invalid ref、redirect rejection、access denied、not found、parse/integrity、budget 不重试；transport timeout/connect/request/body/5xx exhaustion 最多重试一次。
- [x] 将 GitHub snapshot 获取错误分类为静态 typed family；同一 mapper 提供 retryability、public code 和 diagnostic category，不解析 Display。
- [x] 在生产 snapshot wrapper 中保留首轮并发 4，并在全部首轮 settled 后对 typed retryable failures执行稳定顺序、并发 1、最多一次补偿。
- [x] recovered snapshot 正常写 cache；最终 failure 保留最终 typed error，且 `completed <= total`。

Focused command:

```powershell
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates::snapshots::tests -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked services::github_import::tests -- --nocapture
```

## Stage 8. Refresh Logging And Compatibility

- [x] 为 `FailedRepository` 增加 optional/default static diagnostic category；更新 Rust/TypeScript/generated contracts 与历史 fixture。
- [x] refresh/retry Operation Log 增加最多 50 个安全 repository failure items、截断数和 retry attempted/recovered；Runtime Log 增加排序去重 code/category 与 retry counts。
- [x] 对 URL、owner/repo/ref、token、response body、status detail 和 reqwest Display 做 IPC/Operation/Runtime 对抗性缺失断言。
- [x] 更新 GitHub archive、update inventory retry、journal/mutation lock、redaction 与 async error specs。
- [x] 重新运行所有定向测试、生成物只读检查、`task.py validate`、`git diff --check` 和最终 `just ci`；保留现场行为为 `UNVERIFIED`。
