# GitHub 导入 FS-DB 原子性与恢复

## Goal

把本地、SSH 与 WSL 的 GitHub 内容导入收敛到仓库已有 Central FS+DB journal，使文件切换、SQLite 写入、崩溃恢复和并发互斥具有同一可观察语义；任何 DB/恢复失败都不得永久丢失旧内容或留下无 durable evidence 的新内容。

## Confirmed Evidence

- `BE-CORR-001`（Critical / M）：`src-tauri/src/services/github_import/remote.rs:426,544-564` 的远程覆盖脚本在发布新目录后删除 backup；`src-tauri/src/services/github_import/remote.rs:484-495` 随后才写 DB，DB 失败时旧内容已不可恢复。
- `BE-CORR-002`（High / M）：`src-tauri/src/services/github_import/import.rs:604-618,666-684` 在 DB upsert 失败后 best-effort 恢复，忽略 remove、rename 和 blocking-task 错误。
- `BE-CONC-001`（High / M）：`src-tauri/src/services/github_import/remote.rs:321-396` 的最终写入路径未取得与 delete/update/install 相同的 target mutation guard。
- 仓库已有可复用机制：`src-tauri/src/services/central_updates/core/content_upsert.rs::journaled_central_content_upsert` 已把 GitHub-backed content 转成 `SkillUpdatePlan`，并由 `src-tauri/src/services/central_updates/core/batch.rs::update_skills_batch` 持锁、恢复、journal、交换、同事务提交和 finalize；canonical contract 为 `.trellis/spec/backend/fs-db-operation-journal.md`。

## Requirements

- R1：**唯一 orchestrator。** 本地 `src-tauri/src/services/github_import/import.rs::import_single_staged_skill` 与远程 `src-tauri/src/services/github_import/remote.rs::import_github_repo_skills_remote_from_workspace` 的最终 apply 必须复用 `journaled_central_content_upsert → update_skills_batch`；不得保留另一条生产 FS→DB 直写路径，也不得新增平行 journal/schema/operation kind。
- R2：**Durable phase。** 使用现有 `fs_db_operations`、`OperationKind::CentralUpdate` 和 `UpdateManifest(had_target=false|true)`；phase 只能沿 `prepared → fs_staged → fs_swapped → db_committed → completed` 或失败收敛到 `rolled_back`。任何破坏性 FS/业务 DB 写入前必须先有 durable `prepared` row。
- R3：**Manifest identity。** versioned manifest 必须持有 operation-owned stage/backup/marker、target identity、old/new fingerprint 与 provenance 所需标识；swap、restore、finalize 前验证 marker/operation ID/fingerprint，碰撞时 fail closed 并保留 row/artifacts。manifest/log/IPC 不得含文件内容、凭据、命令、stdout/stderr 或可外泄的诊断。
- R4：**原子 commit 与 commit-unknown。** skill row、repository membership、commit/digest provenance 与 `fs_swapped → db_committed` 必须在同一 SQLite transaction；`commit()` 返回错误时读取 operation row，看到 `db_committed` 则 roll-forward，否则 rollback，绝不凭返回值盲目恢复。
- R5：**Target guard 与恢复入口。** candidate validation/snapshot acquisition 在锁前完成；final apply 取得现有 target-derived mutation guard，先只恢复同 target/selected skill 的 pending row，再执行新 mutation。桌面启动仅自动恢复 Local；SSH/WSL 保留 pending row，由 `retry_fs_db_operation` 或同 target 下一次 mutation 显式建立 transport 后重试。
- R6：**UID 与结果兼容。** overwrite 保留 persisted `uid`；first upsert 创建一次 uid 并使用 `had_target=false`；导入 summary、skip/rename/overwrite、progress、preview snapshot 生命周期和多文件 payload 保持现有公开行为。
- R7：**故障可观察性。** primary failure、rollback completed、rollback incomplete、recovery collision 与 commit-unknown 以现有 Central update phase/stable code 表示；journal/error/rollback/finalize 失败不得吞掉。日志/IPC 只暴露 bounded/redacted operation ID、phase、stable code。
- R8：**可判定故障矩阵。** Local、Fake SSH、Fake WSL 覆盖 R2-R7 的每个 durable 边界；真实 SSH/WSL 的断连、进程 kill 与 commit-unknown 只在真实执行后才可记 verified。

## Acceptance Criteria

- [ ] AC1（R1）：两条旧 apply 路径不再在 `upsert_skill_with_github_repository` 前后自行 rename/remove；生产最终 apply 只有 `journaled_central_content_upsert → update_skills_batch`。
- [ ] AC2（R2）：Local/Fake SSH/Fake WSL 的每次 destructive write 前均可查询到 `central_update/prepared` row，且无新增 journal table/kind。
- [ ] AC3（R2, R3）：First import 与 overwrite 分别以 `had_target=false/true` 进入 exact phase graph，manifest 可判定 operation ID、target ID/kind、marker 和 old/new fingerprint。
- [ ] AC4（R3）：重复 restore/finalize/Retry 幂等收敛；marker/fingerprint/occupied-target 碰撞 fail closed 并保留 row/artifacts。
- [ ] AC5（R4）：skill row、repository membership、commit/digest provenance 与 `fs_swapped → db_committed` 在同一 SQLite transaction。
- [ ] AC6（R4, R6）：Overwrite 的 DB apply 失败恢复旧 FS/旧 metadata，persisted uid 不变。
- [ ] AC7（R4, R6）：First upsert 的 DB apply 失败删除 operation-owned 新 target，且 DB 不出现新 skill/repository assignment。
- [ ] AC8（R4）：`commit()` error 且 read-back 可见 `db_committed` 时 roll-forward，不恢复旧内容。
- [ ] AC9（R4）：`commit()` error 且 read-back 不可见 `db_committed` 时 rollback 并收敛到 `rolled_back`。
- [ ] AC10（R5）：同 target 的 import 与 delete/update/install 互斥，不同 target 可并行。
- [ ] AC11（R5）：Local startup 自动收敛 Local pending row；SSH/WSL startup 不连接，显式 Retry/下一 mutation 只处理匹配 target/skill。
- [ ] AC12（R7, R8）：stage/swap/DB/commit/rollback/finalize/marker/fingerprint/offline 故障逐项返回稳定 phase/code，无 best-effort terminal state；IPC/log redaction assertions 通过。
- [ ] AC13（R6）：Multi-file `SKILL.md`/references/scripts/assets 在 Local/Fake SSH/Fake WSL 字节一致，skip/rename/overwrite summary 与 progress 顺序保持既有 contract。
- [ ] AC14（R1, R2, R3, R4, R5, R6, R7, R8）：相关 Rust 定向 tests、subprocess kill tests、fmt、locked all-target Clippy/tests、默认并发 `just ci` 和独立 review 通过。
- [ ] AC15（R8）：未执行的真实 SSH、WSL、process kill、commit-unknown 分别标记 `UNVERIFIED`，不由 FakeRunner 结果代替。

## Out of Scope

- 新增 journal schema/operation kind、通用 Saga framework、自动连接所有远端或后台重试队列。
- 改变 GitHub parsing、PAT/SecretStore、archive redirect、preview 获取、duplicate 选择或用户可见导入功能。
