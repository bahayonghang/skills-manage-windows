# Skill Deletion Integrity Contract

## 1. Scope / Trigger

Use this contract whenever a code path deletes or reconciles `skills` rows,
cleans scan-stale skill state, repairs pre-existing orphan rows, or prepares a
skill-parent FK migration.

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

pub(crate) async fn delete_owned_skill_relations_missing_from_scan_keep(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error>;

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

- The compile-time relation spec in `skill_relations_repo.rs` is the only
  source of owned table and column identifiers. Never accept those identifiers
  from input or repeat a partial table list at a call site.
- Single deletion and keep-set reconciliation run one transaction: delete all
  seven owned relations, delete parent skills, prune repositories, then commit.
- Scanner reconciliation first performs agent-scoped installation and
  observation cleanup in its scan transaction, then invokes the shared
  seven-relation keep-table helper before deleting parents and pruning repos.
- Database initialization order is `schema init -> orphan repair -> seed`.
- Repair inventory uses explicit `LEFT JOIN skills` predicates. Its stable JSON
  contains only table names, sorted skill IDs, per-table counts, and total count;
  it contains no paths, content, credentials, or full row backups.
- A non-empty repair writes one `operation_logs` row with
  `category=database`, `action=orphan_repair`, and `status=succeeded`. JSON
  encoding, audit insert, seven-table deletion, and commit share one transaction.
- A zero-row report commits without an operation log. Repeated repair is
  idempotent.
- Whole-database backup, FK constraints, and schema versioning are separate
  release work. The orphan audit is diagnostic evidence, not recovery data.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Any owned-relation delete fails | Roll back relation rows, parent rows, repository pruning, and any repair audit |
| Operation-log insert fails | Roll back repair; do not delete orphan rows |
| Report JSON encoding fails | Return `sqlx::Error::Encode`; do not insert audit or delete rows |
| Inventory count is negative or overflows `u64` | Return `sqlx::Error::InvalidArgument`; do not mutate data |
| Keep set is empty | Delete all owned rows and all parent skills in one transaction |
| Keep set is non-empty | Delete only owned rows and parents absent from the keep set |
| Repair finds zero rows | Return an empty report and write no audit log |
| Startup repair fails | Fail database initialization before seed; never continue with partial cleanup |

All repository-layer errors remain `sqlx::Error`; do not replace them with
string errors or best-effort logging.

## 5. Good / Base / Bad Cases

- Good: deleting skill `X` removes all seven owned relations and the parent in
  one commit; reinserting `X` starts with no inherited collection, tag, review,
  explanation, update, source, or installation metadata.
- Base: startup finds no orphan rows, returns `totalRows=0`, writes no operation
  log, and proceeds to seed.
- Good: startup finds orphan explanations, persists the stable report, deletes
  them, commits, and a second repair returns zero.
- Bad: delete four familiar tables at one call site while omitting collection,
  review, or explanation relations.
- Bad: delete orphan rows first and write the audit afterward as best effort.
- Bad: treat parentless observations or project snapshots as corrupt owned rows.

## 6. Tests Required

- Single delete and empty/non-empty keep sets remove all seven owned relations.
- Scanner stale cleanup removes all seven while preserving observations outside
  the touched-agent keep set.
- Trigger-injected intermediate relation and audit failures prove full rollback,
  including the audit row after a mid-repair delete failure.
- Startup repair asserts exact stable JSON, persisted audit fields, cleanup, and
  idempotent second execution.
- Reusing a deleted skill ID does not restore owned metadata, while observation,
  project, and usage history remains.
- FK preflight executes an explicit parent-missing `LEFT JOIN` predicate for
  every owned relation. `PRAGMA foreign_key_check` alone is insufficient before
  those FKs exist.
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

This duplicates an incomplete ownership list and can commit partial deletion.

### Correct

```rust
let mut transaction = pool.begin().await?;
delete_owned_skill_relations(&mut transaction, skill_id).await?;
sqlx::query("DELETE FROM skills WHERE id = ?")
    .bind(skill_id)
    .execute(&mut *transaction)
    .await?;
prune_empty_skill_repositories_in_transaction(&mut transaction).await?;
transaction.commit().await?;
```

All delete paths consume the shared compile-time ownership definition and keep
their parent, relation, audit, and repository mutations atomic.
