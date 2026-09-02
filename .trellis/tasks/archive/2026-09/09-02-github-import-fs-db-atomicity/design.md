# Design: GitHub 导入复用既有 FS+DB Saga

## Change List / Symbols

1. `src-tauri/src/services/github_import/import.rs::import_single_staged_skill`：保留候选/frontmatter/summary 组装，删除 `backup_existing_skill_dir`、`restore_or_cleanup_target_dir`、`drop_existing_backup` 这条生产 apply；构造 `JournaledCentralContentUpsert` 并调用 canonical content-upsert seam。[R1][R6]
2. `src-tauri/src/services/github_import/remote.rs::import_github_repo_skills_remote_from_workspace`：保留 snapshot/workspace/candidate/progress，移除 `remote_import_skill_script` 对 target/backup 的最终切换职责；每个 selected skill 交给同一 seam，remote workspace 仍只属于 preview acquisition。[R1][R6]
3. `src-tauri/src/services/central_updates/core/content_upsert.rs::{JournaledCentralContentUpsert,journaled_central_content_upsert,content_upsert_plan}`：补齐 overwrite/first-upsert 的 existing `Skill`/uid 与 `had_target` 输入验证，但不复制 batch 状态机。[R2][R6]
4. `src-tauri/src/services/central_updates/core/batch.rs::{update_skills_batch,prepare_update,commit_staged_update,persist_updated_skill_in_transaction,rollback_staged_after_db_failure}` 与 `src-tauri/src/services/central_updates/core/batch/recovery.rs::{recover_pending_update_operation,recover_selected_pending_update_operations}`：作为唯一 phase/transaction/recovery owner；只补 GitHub import 所需故障注入 seam，不分叉生产逻辑。[R2-R5][R7]
5. `src-tauri/src/services/central_updates/fs/operation/**` 和 `src-tauri/src/services/central_operation/types.rs`（实际 manifest 定义处）：复用既有 marker/fingerprint/stage/swap/rollback/finalize；仅在缺少可测试 fault point 时增加测试专用注入。[R2][R3][R8]
6. 测试落点：`src-tauri/src/services/central_updates/core/content_upsert.rs`、`src-tauri/src/services/central_updates/core/batch_tests.rs`、`src-tauri/src/services/central_updates/fs/operation/tests.rs`、`src-tauri/src/services/github_import/tests.rs`、`src-tauri/src/services/github_import/remote.rs` 的 FakeRunner tests 与 `src-tauri/src/services/central_operation/recovery.rs`。[R8]

## Contract / State Machine

```text
candidate + snapshot validated (no target lock, no target mutation)
  -> acquire existing target mutation guard
  -> recover matching pending row
  -> insert fs_db_operations(prepared, OperationKind::CentralUpdate, UpdateManifest)
  -> fs stage + durable marker/fingerprint
  -> phase fs_staged
  -> swap canonical data
  -> one SQLite transaction:
       fs_staged -> fs_swapped
       upsert skill + repository membership + commit/digest provenance
       fs_swapped -> db_committed
  -> commit read-back on unknown result
  -> finalize owned backup/marker
  -> db_committed -> completed

prepared | fs_staged | fs_swapped --recover old state--> rolled_back
db_committed --roll forward/finalize--> completed
```

`update_skills_batch` remains the only production orchestrator and `fs_db_operations` remains the only durable journal. Destructive write before `prepared`, direct business upsert outside the phase transaction, or a second GitHub-import journal are contract violations.[R1-R4]

## Manifest / Identity

- First import uses `OperationKind::CentralUpdate` + `UpdateManifest(had_target=false)`; overwrite uses `had_target=true` and preserves old fingerprint/uid.
- Manifest records only operation-owned path identities, marker paths, target ID/kind, old/new fingerprints and bounded projection flags; marker binds artifact to operation ID.
- Restore/finalize verifies marker and expected fingerprint. Missing/occupied/drifted evidence returns typed collision and keeps row/artifacts; no TTL cleanup of pending rows.[R2][R3][R6][R7]

## Failure Matrix

| Injection point | Required result |
| --- | --- |
| candidate/manifest validation | no row, no target mutation |
| after `prepared`, before/during stage | restore/remove only operation-owned stage; `rolled_back` or pending evidence if recovery fails |
| after `fs_staged` or process kill | recover old canonical state; Local startup may run it, SSH/WSL wait for explicit transport |
| swap/DB apply before commit | SQLite rolls back; overwrite restores old target, first upsert removes new target; phase `rolled_back` |
| `commit()` error + row reads `db_committed` | roll forward and finalize; never restore old data |
| `commit()` error + row not `db_committed` | rollback FS and converge `rolled_back` |
| marker/fingerprint/restore collision | fail closed; preserve row and artifacts; stable recovery code |
| finalize/phase write failure after DB commit | keep `db_committed` pending and retry finalize; do not undo canonical/DB state |
| SSH/WSL offline | keep pending row; only that target/skill fails |
| repeated Retry/next mutation | idempotently converge; do not overwrite new user data |
| same-target concurrent mutation | existing guard serializes; different targets do not contend |

## Compatibility

公开 import command/CLI、duplicate selection、progress payload、result summary、snapshot reservation/cleanup 与 remote workspace acquisition 不变。overwrite 复用 persisted uid；first import 的 uid 只创建一次。journal rows 是恢复状态，不进入 operation-log export/telemetry。[R5-R7]

## Verification Boundary

Local/Fake SSH/Fake WSL 与 subprocess kill fixture 可证明 phase、脚本完整性、DB/FS 收敛和 redaction；它们不证明真实网络断连、真实 WSL distro、真实 SSH server 或 SQLite commit-unknown。真实 smoke 未运行时逐项标 `UNVERIFIED`。[R8]

## Rollback

- RP1：先增加/加固 canonical content-upsert tests，不改变调用方。
- RP2：Local import 单独切换到 seam；失败可恢复旧 Local apply，remote 不受影响。
- RP3：SSH/WSL import 切换到同一 seam；失败只回退 remote adapter，不回退已验证 Local 和 canonical journal fixes。
- RP4：删除旧 helper/script 只在所有 call-site/contract 测试通过后进行；若回退必须同时恢复相应调用路径，不能留下双 orchestrator。

## Considered but Not Chosen

- 不把 backup cleanup 简单后移：仍无法覆盖进程崩溃、commit-unknown 与恢复失败。
- 不为 GitHub import 新建 journal/table/kind：会与 Central update 对同一 target/skill 产生两个真相源。
- 不在 startup 自动连接 SSH/WSL：会扩大凭据、时延和外部副作用边界。
