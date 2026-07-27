# FS+DB 双写可恢复协议（operation journal/Saga）

## Goal

为 Central skill 删除与批量更新建立 target-scoped、跨进程互斥、崩溃后可恢复的 FS+DB Saga。任一步失败或进程退出后，系统必须自动收敛到完整 old state 或完整 new state，不再要求用户手工修复 Central 目录、SQLite metadata 与 copied installations 的 split-brain。

对应父任务审计 P1-06 / M-05；前置任务 `07-24-target-context-snapshot` 与 `07-24-db-schema-versioning-fk` 已完成并归档。

## Confirmed Live Evidence

- Local delete 在 `src-tauri/src/services/central_skills/delete.rs:509-579` 先删除 installation paths 与 Central 目录，再调用 `db::delete_skill`；DB 失败时目录不可恢复。
- SSH/WSL delete 在同文件 `:287-359` 使用同样的 FS-first 顺序，但没有 Local mutation guard。
- Central update 在 `src-tauri/src/services/central_updates/core/batch.rs:28-133` 先批量替换 canonical skill directories，再逐 skill 执行多个 DB write，最后刷新 copied installations；失败只返回 per-skill error，没有 durable compensation。
- Local update 的 sibling backup 在 `src-tauri/src/services/central_updates/fs.rs:197-269` 完成单目录 swap 后立即删除；remote script 也在返回 `OK` 前删除 backup，因此进程退出后均没有 durable recovery material。
- `local_archive_import/import.rs:85-201` 已有 staging / sibling backup / restore 先例，但 rollback 只存在于当前进程控制流，没有 durable journal 或启动恢复。
- `TargetContext` 已在 `targets/model.rs:252-281` 固化 target + matching DB，commands 已从同一 snapshot 构造 Local/SSH/WSL transport。
- 现有 `central_mutation` lock 只使用一个 Local lock path；`central-mutation-lock.md` 明确 remote mutations 不取该锁。本任务需要把该契约升级为 per-target mutation lease。
- 数据库已进入 checksum-locked versioned migration 体系，当前版本 2；operation journal 必须作为新的不可变 migration 版本追加，不能修改旧 migration source。
- Operation Logs UI 已能显示 `partial` / `failed` 状态和详情。本任务可复用它呈现非敏感恢复摘要，但 durable compensation payload 不得进入可导出的 operation log。

完整现场记录见 `research/operation-journal-live-2026-07-26.md`。

## Requirements

### R1. Durable Journal Schema

- 新增独立 `fs_db_operations` 表和 repository API；它不是现有可清理、可导出的 `operation_logs` 表。
- 每个 skill 是一个独立恢复单元，批量请求使用可选 `batch_id` 关联，保持现有 per-skill partial-success 语义。
- durable row 至少记录 operation ID、batch ID、target ID/kind、operation kind、skill ID、phase、old/new fingerprints、受控 recovery manifest、错误摘要、created/updated/completed timestamps。
- recovery manifest 只含恢复所需的受控路径/类型/marker，不含文件内容、PAT、API key、SSH 密码、私钥或命令输出，也不进入 export/telemetry。
- schema 通过 versioned migration 3 落地，checksum、旧 fixture 升级和 idempotent reopen 必须测试。

### R2. State Machine And Commit Point

- 稳定 phase 为 `prepared -> fs_staged -> fs_swapped -> db_committed -> copies_pending -> completed`，失败恢复终态为 `rolled_back`；无法立即恢复时保留非终态和 typed error，不伪装 completed。
- `prepared` 必须在任何 destructive FS/DB write 前 durable commit。
- business DB mutations 与 `db_committed` marker 必须在同一 SQLite transaction；不能一个成功、另一个失败。
- FS swap/rename 发生在该 transaction commit 前。若进程在 commit 前退出，SQLite 回滚且 recovery 根据旧 phase 恢复 FS；若 commit 已成功，recovery 根据 `db_committed` roll forward。
- 所有 phase transition、restore、finalize 与 copy refresh 都必须幂等；重复 recovery 不得覆盖用户新数据。

### R3. Delete Saga

- validate/plan 阶段冻结 `TargetContext`、skill identity、Central path、installation rows 与 fingerprints。
- 删除不再直接 `remove_dir_all`。Local 与 remote 都先把应删除路径 rename 到同文件系统的 operation-scoped sibling backup，并持久化 marker。
- backup 全部就绪后，在一个 DB transaction 中删除 `skills` parent（由 FK cascade owned relations）并写 `db_committed` marker，然后 commit。
- commit 前失败或 crash 恢复全部 backup；commit 后只做幂等 cleanup。保留的 copy installation 不得被移动或删除。

### R4. Update Saga

- 保留 `update_skills_batch` 唯一生产编排和 remote chunk 上限，但每个 skill 使用独立 operation ID、staging、backup 与 outcome。
- canonical new contents 先写入 operation-scoped staging；旧目录 rename 到 sibling backup 后 swap new directory。
- skill upsert、repository assignment 与 `db_committed` marker 使用一个 DB transaction。
- copied installations 是 canonical source + DB commit 后的 derived projection。refresh 失败不回滚已提交 canonical update；journal 保持 `copies_pending`，后续 recovery 幂等重试并在现有 Operation Logs UI 记录 partial 摘要。
- cancel 只能阻止尚未进入 destructive phase 的 operation；已进入 `fs_staged` 的 operation 必须同步完成或留下可恢复 journal，不能仅返回 cancelled。

### R5. Per-Target Exclusivity

- Local、SSH、WSL mutation 都通过一个 target-derived cross-process lease boundary；同一 target 的 delete/update/recovery 串行，不同 target 可独立进行。
- lease path 由 target ID 的安全 digest 派生，不能把未经校验的 target ID 拼入路径。
- GUI 与共享 local CLI 必须复用同一 service boundary；内部 helper 不二次加锁。
- 本任务只提供 mutation lease，不实现 `job-concurrency-lease` 子任务拥有的通用 JobRegistry/cancel registry。

### R6. Recovery And Observability

- Local pending operations 在 desktop startup 扫描，并在任何新的 Local mutation 前再次 fail-closed recovery。
- SSH/WSL pending operations 不阻塞应用启动、其他 target 或该 target 的只读缓存。受影响 target 的新 mutation 必须先在同一 lease 内自动恢复；目标离线或恢复失败时 mutation fail closed，pending row 保留。
- Operation Logs 工作面提供当前 active target 的 pending 状态与手动重试。手动重试才建立远端 transport；普通只读命令不得为了 recovery 隐式连接 SSH/WSL。
- 自动或手动恢复都使用 journal row 的 target identity 与本次显式解析的同 ID `TargetContext`；ID 不匹配直接拒绝，禁止用另一个 current active target 代替。
- recovery 必须验证 operation-owned staging/backup marker 和 fingerprints 后再 restore/finalize；marker 冲突或用户路径已被新内容占用时保留 recovery-required 状态并返回 typed error，不能覆盖。
- completed/rolled-back rows 和 backups 按明确 retention 清理；pending/recovery-required backup 永不按 TTL 自动删除。
- Operation Logs 只记录 operation ID、target identity、kind、phase、counts、duration 与脱敏错误；不得记录 recovery manifest、完整路径、内容或凭据。

## Acceptance Criteria

- [ ] Migration 3 从五个冻结 release fixtures 与当前 version-2 DB 升级成功，checksum/preflight/backup/restore 规则保持成立。
- [ ] Local、SSH、WSL delete 在 DB failure、transport failure 与每个 destructive phase crash 后恢复为完整 old 或 new state；source、owned relations 和 retained copies 一致。
- [ ] Local、SSH、WSL update 在 staging、backup rename、final swap、DB update 前/中/后、copy refresh 中、journal completed 前注入 crash，恢复后无 canonical/DB 混合状态。
- [ ] `db_committed` 后 copy refresh failure 保持 canonical + DB new state、journal `copies_pending`，重试成功后只完成缺失 projection。
- [ ] 同 target 的两个独立进程不能并发 mutation；holder crash 后 lease 可重新取得；不同 target 不互相阻塞。
- [ ] Batch duplicate IDs、partial FS failure、cancel boundary、repeated recovery 和 stale marker collision 有定向测试。
- [ ] operation log/export 不含 recovery manifest、完整路径、文件内容或 secret；现有 partial/failed UI 能显示可操作的恢复摘要。
- [ ] Rust fmt、all-targets locked Clippy、locked tests 和 `just ci` 通过。

## Out Of Scope

- 不在本任务接入 local archive import、GitHub import、portable state 或 Central store migration；只提供后续可复用的 coordinator/journal contract。
- 不实现通用 per-job actor/lease/cancel registry；该范围归 `07-24-job-concurrency-lease`。
- 不承诺 FS 与 SQLite 的不可实现分布式原子提交；目标是 durable、幂等、可证明收敛的 Saga。
- 不修改父审计报告原文件，也不顺带重构非 delete/update 的 Central mutation paths。

## Key Product Decision

- 采用 target-scoped availability：SSH/WSL pending operation 不阻塞应用、其他 target 或该 target 的只读缓存；只阻止受影响 target 的新 mutation，联网后由 mutation preflight 自动重试，也允许用户在 Operation Logs 手动重试。
