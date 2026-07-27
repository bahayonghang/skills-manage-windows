# Skill Deletion Integrity Contract

## 1. Scope / Trigger

Use this contract whenever a code path deletes or reconciles `skills` rows,
cleans scan-stale skill state, repairs a pre-version-2 database, or changes the
skill-parent FK migration.

Central filesystem deletion additionally follows `fs-db-operation-journal.md`:
backup renames occur before the business transaction, and parent deletion plus
the `db_committed` marker are atomic.

The `skills` row owns exactly these seven relations, in this stable order:

1. `skill_update_states`
2. `skill_repository_members`
3. `collection_skills`
4. `skill_tag_links`
5. `skill_ai_tag_reviews`
6. `skill_explanations`
7. `skill_installations`

`agent_skill_observations`, `project_skill_installations`, `skill_calls`, and
`skill_usage_metadata.resolved_skill_id` have independent history or snapshot
lifecycles. Update inventory, pending additions, and repository skip rows are
owned by their run or repository lifecycle. Skill deletion must not cascade
into these records.

## 2. Signatures

```rust
pub async fn delete_skill(
    pool: &DbPool,
    skill_id: &str,
) -> Result<(), sqlx::Error>;

pub async fn delete_skills_not_in_scope(
    pool: &DbPool,
    found_skill_ids: &[String],
) -> Result<(), sqlx::Error>;

pub async fn repair_orphan_skill_relations(
    pool: &DbPool,
) -> Result<OrphanRepairReport, sqlx::Error>;

pub(crate) async fn prune_empty_skill_repositories_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<u64, sqlx::Error>;
```

The repair report is serializable with camel-case field names:

```rust
pub struct OrphanRelationReport {
    pub table: String,
    pub skill_ids: Vec<String>,
    pub row_count: u64,
}

pub struct OrphanRepairReport {
    pub relations: Vec<OrphanRelationReport>,
    pub total_rows: u64,
}
```

## 3. Contracts

- The immutable compile-time relation spec in `skill_relations_spec.rs` is the only
  source of owned table and column identifiers. Never accept those identifiers
  from input or repeat a partial table list at a call site.
- Migration 2 consumes the relation spec and adds `FOREIGN KEY(skill_id)
  REFERENCES skills(id) ON DELETE CASCADE` to all seven tables.
- Single deletion and keep-set reconciliation run one transaction: delete only
  parent skills, let SQLite cascade all seven owned relations, prune
  repositories, then commit.
- Central delete uses `commit_delete_fs_db_operation`: delete the parent, prune
  repositories, and transition `fs_staged -> db_committed` in the same
  transaction. FS backup cleanup happens only after that commit; a pre-commit
  failure restores all operation-owned backups.
- Scanner reconciliation first performs agent-scoped installation and
  observation cleanup in its scan transaction, then deletes stale parents and
  prunes repos. It never repeats the seven-table list.
- Database initialization order is `backup -> migration 1 -> orphan repair ->
  migration 2 FK rebuild -> foreign_key_check -> seed`.
- Repair inventory uses explicit `LEFT JOIN skills` predicates. Its stable JSON
  contains only table names, sorted skill IDs, per-table counts, and total count;
  it contains no paths, content, credentials, or full row backups.
- A non-empty repair writes one `operation_logs` row with
  `category=database`, `action=orphan_repair`, and `status=succeeded`. JSON
  encoding, audit insert, seven-table deletion, and commit share one transaction.
- A zero-row report commits without an operation log. Repeated repair is
  idempotent.
- Repair is a pre-migration-2 compatibility step. Current FK-enforced databases
  reject new owned-relation orphans; the audit remains diagnostic evidence, not
  recovery data.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Parent delete or DB cascade fails | Roll back parent rows, cascades, and repository pruning |
| Operation-log insert fails | Roll back repair; do not delete orphan rows |
| Report JSON encoding fails | Return `sqlx::Error::Encode`; do not insert audit or delete rows |
| Inventory count is negative or overflows `u64` | Return `sqlx::Error::InvalidArgument`; do not mutate data |
| Keep set is empty | Delete all owned rows and all parent skills in one transaction |
| Keep set is non-empty | Delete only owned rows and parents absent from the keep set |
| Repair finds zero rows | Return an empty report and write no audit log |
| Startup repair fails | Fail database initialization before seed; never continue with partial cleanup |
| Central FS staging succeeds but DB delete/marker fails | Roll back DB, restore every staged path, retain a recoverable row if restore cannot finish |

All repository-layer errors remain `sqlx::Error`; do not replace them with
string errors or best-effort logging.

## 5. Good / Base / Bad Cases

- Good: deleting skill `X` removes all seven owned relations through FK cascade
  and the parent in one commit; reinserting `X` starts with no inherited
  collection, tag, review, explanation, update, source, or installation data.
- Base: startup finds no orphan rows, returns `totalRows=0`, writes no operation
  log, and proceeds to seed.
- Good: startup finds orphan explanations, persists the stable report, deletes
  them, commits, and a second repair returns zero.
- Bad: retain manual relation-delete loops beside the FK cascade; the two
  ownership mechanisms will drift.
- Bad: delete orphan rows first and write the audit afterward as best effort.
- Bad: treat parentless observations or project snapshots as corrupt owned rows.

## 6. Tests Required

- Single delete and empty/non-empty keep sets cascade all seven owned relations.
- Scanner stale cleanup removes all seven while preserving observations outside
  the touched-agent keep set.
- Trigger-injected intermediate relation and audit failures prove full rollback,
  including the audit row after a mid-repair delete failure.
- Startup repair asserts exact stable JSON, persisted audit fields, cleanup, and
  idempotent second execution.
- Reusing a deleted skill ID does not restore owned metadata, while observation,
  project, and usage history remains.
- Legacy repair inventory executes an explicit parent-missing `LEFT JOIN`
  predicate for every owned relation before the FKs exist. Migration 2 and
  every completed startup additionally run `PRAGMA foreign_key_check`.
- Minimum closeout gate is `just ci` after focused `cargo test db:: --locked`
  and `cargo test scanner --locked` checks.

## 7. Wrong vs Correct

### Wrong

```rust
sqlx::query("DELETE FROM skill_installations WHERE skill_id = ?")
    .bind(skill_id)
    .execute(pool)
    .await?;
sqlx::query("DELETE FROM skills WHERE id = ?")
    .bind(skill_id)
    .execute(pool)
    .await?;
```

This duplicates an incomplete cascade that the database already owns and can
commit parent/relation changes separately.

### Correct

```rust
let mut transaction = pool.begin().await?;
sqlx::query("DELETE FROM skills WHERE id = ?")
    .bind(skill_id)
    .execute(&mut *transaction)
    .await?;
prune_empty_skill_repositories_in_transaction(&mut transaction).await?;
transaction.commit().await?;
```

Runtime delete paths do not consume or duplicate the relation list. The
compile-time definition remains the source for migration, repair, and tests;
SQLite owns runtime cascade atomicity.
