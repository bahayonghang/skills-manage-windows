# 设计：Skill 删除事务与 orphan 修复

## 1. 数据所有权边界

`skills` 行拥有以下运行态/元数据关系：

```text
skill_update_states
skill_repository_members
collection_skills
skill_tag_links
skill_ai_tag_reviews
skill_explanations
skill_installations
```

这些表的 `skill_id` 在 parent skill 删除后没有独立语义，必须原子删除。集中定义内部 `SkillRelationSpec { table, skill_column }` 静态清单，所有表名来自编译期常量，不接受用户输入。

以下列不进入该清单：

- `project_skill_installations`：项目扫描快照，能独立于 Central/全局 `skills` 行存在，并已有 project FK。
- `agent_skill_observations`：以 agent + `row_id` 记录扫描事实，按 touched-agent keep-set 独立清理；live local DB 中存在 33 条无 `skills` parent 的记录，不能仅凭 parent 缺失判为 orphan，也不能按 parent cascade 删除或加入下一任务 FK。
- `skill_calls` 与 `skill_usage_metadata.resolved_skill_id`：历史事实/可重建分析缓存，不能因当前 catalog 删除而擦除历史。
- `skill_update_inventory_entries`、pending additions、repository skips：一次 inventory/candidate/repository 状态，由其自身 run/repository 生命周期清理。

## 2. Transaction helpers

在 `db/repos/skills_repo.rs` 内提供只接受 transaction 的内部 helper：

- `delete_owned_skill_relations(tx, skill_id)`
- `delete_owned_skill_relations_not_in(tx, keep_ids)`
- `delete_owned_skill_relations_missing_from_keep_table(tx, keep_table)`，keep table 只能取内部固定 `scan_keep_skills`
- `prune_empty_skill_repositories_in(tx)`

`delete_skill` 与 `delete_skills_not_in_scope` 均：begin → relation deletes → skills delete → repository prune → commit。空 keep-set 与非空 keep-set 共享同一 relation spec loop，仅 SQL predicate 不同。

`scanner::delete_scan_stale_rows` 保留 agent-scoped observation 清理；installation 同时受 agent keep-set 与 skill ownership 管理。随后调用共享 keep-table helper 完成 7 张全局 skill ownership cascade，再删除 `skills` 和空 repository。它仍运行在 `persist_scan_batch` 已有 transaction 中。

## 3. Orphan inventory and repair

新增：

```rust
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OrphanRepairReport {
    pub relations: Vec<OrphanRelationReport>,
    pub total_rows: u64,
}

pub async fn repair_orphan_skill_relations(
    pool: &DbPool,
) -> Result<OrphanRepairReport, sqlx::Error>;
```

每张拥有型表执行 `LEFT JOIN skills ... WHERE skills.id IS NULL`，收集稳定排序的 distinct skill IDs 与 row count。报告不含 secret、路径或内容，只含 table、skill IDs、row counts。

按 2026-07-26 用户决策，非空 repair 使用同一 transaction 完成：inventory → JSON serialization → 写入 `operation_logs`（category `database`、action `orphan_repair`）→ 删除 7 张表 orphan → commit。operation log 写入失败即 rollback，不允许 best-effort；这样桌面、无 tracing subscriber 的 `skillport-cli` 和 target cache 都保留相同持久审计。桌面端可在 commit 后额外 tracing 一条摘要，但它不参与正确性。

`init_database_with_agents` 在 schema init 完成、seed 开始前调用 repair。下一 `db-schema-versioning-fk` 子任务负责让最终 release 在 repair/migration 前创建 whole-DB 原子备份；本任务不新增完整行备份、sidecar、UI 或 IPC，且不能把 ID/count 审计表述成恢复介质。

该 repair 幂等：首次清理旧 orphan，后续 startup 返回 total=0。失败时 transaction rollback，数据库初始化失败而不是静默部分修复。

## 4. Error and fault behavior

repos 层继续返回 `sqlx::Error`。任何 inventory、JSON encode、audit insert、relation delete、skills delete、repository prune 或 commit 失败都回滚整个 transaction。

Fault injection 不添加生产 feature flag：测试创建 SQLite trigger，并用 `RAISE(ABORT, 'injected')` 分别让 audit insert 和中间 relation delete 失败；断言 operation log、parent skill、relations 与 repository membership 均按事务整体回滚。移除 trigger 后同一操作成功。

## 5. ID reuse invariant

删除 skill X 并重新 upsert 同 ID 后：

- collection/tag/review/explanation/update/source/install 均为空，直到新流程显式重建。
- observation、project snapshots 与 usage history 保持原有独立生命周期。
- repository row仅在无 member 且非 pinned/system unknown 时按现有 prune 规则清理。

## 6. FK preflight

本任务不重建表或添加 FK。测试使用与下一任务相同的 orphan predicates，证明 7 张拥有型关系无 parent-missing rows。`PRAGMA foreign_key_check` 只能验证当前已存在 FK，因此不能单独作为通过证据；验收同时断言每张目标表的显式 LEFT JOIN count 为 0。`agent_skill_observations` 不属于下一任务的 skill-parent FK 集合。

## 7. 兼容性与回滚

无 schema 或 IPC 变更。删除语义只移除本来无有效 parent 的数据。若启动 repair 暴露未知合法引用，回滚点是停止调用 repair，但保留事务化 runtime delete；operation log 中的 `OrphanRepairReport` 提供定位证据，但不恢复完整行。不得回滚到非事务多语句删除。

## 8. 受影响文件

- `src-tauri/src/db/repos/skills_repo.rs`
- `src-tauri/src/db/repos/repositories_repo.rs`（transaction 内 prune helper）
- `src-tauri/src/db/repos/operation_logs_repo.rs`（transaction-scoped audit insert）
- `src-tauri/src/services/scanner/persistence.rs`
- `src-tauri/src/db/seed.rs`
- `src-tauri/src/db/tests.rs`
- `src-tauri/src/services/scanner/tests.rs`
- 后续新增 `.trellis/spec/backend/skill-deletion-integrity.md`
