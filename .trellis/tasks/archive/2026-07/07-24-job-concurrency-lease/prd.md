# 并发作业模型：exclusive job lease 取代共享 cancel flag

## Goal

消除 Central update 与 SkillPort portability 的进程级共享取消标志竞态：同一作业族同时最多运行一个 job，取消请求只影响调用方持有的 jobId，进度事件不能污染后来启动的 job。与此同时，将 one-shot legacy Central migration 纳入现有 Local Central mutation lock，并把递归文件系统工作移出 async runtime worker。

对应审计 P2-01、P2-10 与 QW-04。用户价值是：重复点击、跨页面入口或陈旧取消请求不会重置/取消另一个作业，启动迁移也不会与 Central 写入竞争。

## Confirmed Current Behavior

- `src-tauri/src/lib.rs:57` 的 `central_update_cancel` 与 `src-tauri/src/lib.rs:64` 的 `portable_state_cancel` 分别是一个进程级 `Arc<AtomicBool>`。作业入口会把共享 flag 重置为 `false`，因此第二个作业可清除第一个作业已经收到的取消请求。
- Central update 共享 flag 的生产入口是 `commands/central_updates.rs` 中的 check skill、check repository 与 update skill，以及 `commands/skill_update_inventory.rs:193` 的 `apply_skill_update_decisions`。force update/mirror 使用无取消批次，不属于本竞态范围。
- Portability 的 export、JSON preview、file preview 与 import 共用同一个 flag；`preview_skillport_state_import_file` 当前嵌套调用 JSON preview command，迁移后必须只取得一次 top-level lease。
- `central://skill-update-progress` 与 `central://state-portability-progress` payload 没有 jobId；Zustand merge 无法区分陈旧事件。Update Center inventory refresh 已有前端生成 `operationId`、listen-before-invoke、按 ID 过滤的可复用先例。
- `AiTagJobRegistry` 允许多个 ID 同时注册，且 poisoned mutex 时会返回未登记的 cancel flag；它不满足 exclusive/fail-closed 语义。GitHub preview registry 的 lease 是 snapshot 专用生命周期，也不应被扩成通用 job registry。
- `src-tauri/src/central_migration.rs:87-131` 在 async function 内直接执行递归 `std::fs` 与 `copy_dir_all`，且 startup spawn 未取得 Central mutation lock。刚完成的 FS+DB task 已把 Local lock 保留为 `central-mutation.lock`，并增加 target-scoped lease；本任务应复用该边界，而不是创建第二种锁。
- Legacy migration 是 Local、copy-only、保留旧目录、目标存在即跳过的 one-shot 操作。它不需要写入 FS+DB Saga journal，但 marker 必须在持锁后重查，避免两个进程同时依据过期 marker 执行 copy。

## Requirements

### R1. Fail-closed exclusive job registry

- 新增可复用的 exclusive registry，分别在 `AppState` 中持有 Central update 与 portability 两个实例；两个作业族可以彼此并行，但同族最多一个 active job。
- jobId 由 renderer 在 invoke 前生成并显式传入。Registry 校验非空且有界的 ID，注册成功返回拥有 cancel flag 的 RAII lease；所有成功、失败与 unwind 路径都只释放同一 jobId。
- 同族已有 active job 时返回稳定 coded busy error，绝不 reset 或复用 active cancel flag。Mutex poison 或内部状态异常必须 fail closed。
- cancel 只在 jobId 与当前 active job 匹配时设置该 job 的 flag；没有 active job 时幂等成功并保留一个有界 pending-cancel ID，使先于 start 注册到达的同 ID 取消不会丢失。存在另一个 active job 时返回稳定 mismatch error且不得影响它。

### R2. Migrate every shared-flag entrypoint

- Central update family 覆盖 `check_central_skill_updates`、`check_central_repository_sync`、`update_central_skills` 与 `apply_skill_update_decisions`；`cancel_central_skill_updates` 改为接收 jobId。
- Portability family 覆盖 `export_skillport_state`、`preview_skillport_state_import`、`preview_skillport_state_import_file` 与 `import_skillport_state`；`cancel_skillport_state_portability` 改为接收 jobId。
- 每个 command 在第一次 `.await` 前取得 lease，并把 lease 的 cancel flag 传给既有 service。不得在 service 内重新注册，也不得保留共享 flag fallback。
- `save_skillport_state_export`、inventory refresh、force update/mirror 与 AI tagging 保持现有独立生命周期，不借本任务扩大范围。

### R3. Correlate events and renderer state

- 两种 progress payload 都新增 camelCase `jobId`。后端的每个 started/running/terminal event 都携带同一 ID；不得从 ambient state 推断。
- `CentralSkillUpdateJob` 与 `SkillportStatePortabilityJob` 保存 `jobId: string | null`。Store action 在 invoke 前生成 ID、写入 running state并传入 command；cancel 从当前 state 读取并发送该 ID。
- Merge helper 只接受与当前 jobId 相同的 payload。旧 job 的进度、失败或 terminal event 不得覆盖新 job。
- 同一 store 内重复启动不得覆盖现有 active state；跨 store/入口竞态仍由后端 registry 权威拒绝。

### R4. Stable bilingual busy feedback

- Registry domain error 通过现有 `code:summary` 字符串边界输出稳定的 busy/mismatch code，不引入结构化 `IpcError`，保持 `domain-error-enums.md` 的 command `Result<T, String>` 契约。
- 中英文 `backendErrors` 同步增加文案。Central update workflow、Update Center 与 portability dialog 的可见错误使用 `formatBackendError`，不能向用户展示原始 code envelope。

### R5. Serialize and unblock legacy migration

- Migration 的 fast-path marker 检查后取得现有 Local `central-mutation.lock`，在锁内重查 marker，再执行 copy 与 marker write；lock acquisition 失败时不允许无锁 fallback。
- 递归 create/read/copy/cleanup 作为一个 blocking unit 经 `fs_util::run_blocking_fs_with` 执行；async 侧保留 DB marker 和 progress event，不把 `AppHandle` 移入 blocking closure。
- Migration 持续保留 source，只复制目标不存在的 skill；目标冲突、失败摘要与下次启动重试语义保持不变。

## Acceptance Criteria

- [ ] 并发注册两个同族 job：第二个稳定返回 coded busy error，第一个 cancel flag 与 lease 保持不变；不同作业族可同时注册。
- [ ] cancel 先于同 ID start 注册到达时，新 lease 初始即为 cancelled；cancel job A 后 A 可观察取消；A 完成、B 启动后，陈旧 cancel A 返回 mismatch 且 B 的 flag 保持 false；job 任意返回后可再次启动。
- [ ] 上述 8 个 command 均取得 exclusive lease并接收 jobId；生产代码不再存在 `central_update_cancel` / `portable_state_cancel` 字段或无 jobId cancel command。
- [ ] update 与 portability 的全部 progress event 带 jobId；前端测试证明 A 的陈旧事件不能修改 B，取消 invoke 携带当前 ID，busy 文案按中英文 locale 显示且不泄露 envelope。
- [ ] Legacy migration 与同一 Local mutation lock 的 contender 发生真实 contention；锁释放后可重试。Marker 在锁内二次检查，递归 FS 只在 `run_blocking_fs_with` closure 内执行。
- [ ] Migration copy/skip/source-preservation/partial-failure 行为回归测试通过；startup 仍是 best effort，失败时 marker 不写入并在下次启动可重试。
- [ ] Focused Rust/Vitest、`pnpm typecheck`、`pnpm lint`、locked fmt/Clippy/tests 与默认并发 `just ci` 全部通过。

## Out Of Scope

- 不把长命令改成 spawn-and-return 的后台 IPC，也不新增持久化 job queue、跨进程 job registry 或 per-target actor。
- 不迁移 AI tagging registry、Update Center inventory `operationId`、force update/mirror 或 GitHub preview snapshot registry。
- 不改变 Central update、portability 的业务结果 shape、快照算法、Saga journal 或取消检查点。
- 不把 legacy copy migration 写入 `fs_db_operations`；它是 source-preserving one-shot copy，不是跨 FS/DB destructive commit。
- 不处理 startup fatal/recovery UX；该范围属于 `07-24-startup-resilience`。

## Risks And Deferred Items

- Cancel 在 command 完成后的竞态按幂等 no-active success 处理，并只保留最后一个 pending ID；下一次不同 ID 的 start 会丢弃该陈旧 pending，只有“另一个 job 正在运行”的 stale ID 才返回 mismatch。
- Registry 是进程内 UI job ownership；跨进程 Central 写互斥仍由现有 file lock负责，两者不能互相替代。
- 迁移持锁期间可能让新 Central mutation 等待至现有 10 秒 timeout；这是阻止 copy/write 竞争所需的有界阻塞，下次启动仍可重试。
