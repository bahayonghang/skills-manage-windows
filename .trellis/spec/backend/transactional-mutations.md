# Transactional Metadata and Cache Mutations

## Scope

Applies to repository membership, tags and AI reviews, collection/project
deletion, Marketplace registry cache mutation, and Update Center inventory
replacement.

## Contracts

- A public mutation owns one top-level SQLite transaction. Transaction-scoped
  helpers receive the same `Transaction`/`SqliteConnection`; they never begin a
  nested transaction or return to the pool.
- Validate every caller-supplied repository, skill, tag, or review reference in
  caller order before the first authoritative write. Validation queries and
  writes use bounded chunks under the shared SQLite bind budget.
- `detach_skill_remote_source` deletes update state and membership and prunes
  empty repositories in one transaction.
- Manual tag links survive AI replacement. Pending-review replacement restores
  the complete previous pending set after any validation, SQL, trigger, or
  commit failure.
- Collection child and parent deletes share one transaction. Project deletion
  is one parent delete and relies on the pool's per-connection, fail-closed
  foreign-key contract for cascade.
- Project-skill copy/symlink installation compensates a metadata-write failure by
  removing the newly materialized target; a replaced symlink is restored before
  returning the database error. Uninstall deletes metadata before filesystem
  removal and restores the complete installation row if removal fails.
- Marketplace fetch and parse complete before opening the cache transaction. A
  successful fetch replaces, rather than upserts into, the registry snapshot:
  delete old rows, insert all fresh rows, and publish success metadata in one
  transaction. An empty fresh snapshot clears the cache.
- Marketplace snapshot failure preserves the complete old cache. Only after
  rollback may a named best-effort helper publish the derived error marker; it
  must warn if that marker cannot be written.
- `replace_skill_update_inventory` replaces the scoped inventory run and all of
  its entries in one top-level transaction. Entry serialization completes
  before the write transaction; deleting the old run, inserting the new run,
  and inserting every bucket entry either all commit or all roll back.
- Inventory refresh is isolated from the installed update baseline. It must not
  call `upsert_skill_update_state` or otherwise mutate `skill_update_states`;
  non-actionable results such as `unsupported` belong only to inventory entries.
- Trigger-injected failure on any later inventory entry must preserve the
  previous run and entries exactly. Rollback evidence must also compare the
  complete `skill_update_states` rows before and after, not only row counts.

## Required Tests

- Mixed valid/missing IDs prove zero partial writes and stable validation text.
- A trigger on a later statement proves the previous statement rolls back.
- A batch larger than the bind budget fails in a later chunk and restores the
  pre-call state.
- AI tests cover manual-link preservation, old pending-set restoration, and
  successful retry after removing the trigger.
- Project tests acquire multiple production pool connections, read
  `foreign_keys=1` from each, and prove cascade.
- Project-skill tests inject install/update/delete failures and assert the
  canonical source, project target, and complete installation row converge to
  the pre-call state; symlink variants may skip only when the platform cannot
  create them.
- Marketplace tests cover A,B -> B,C, empty, second-insert failure,
  success-status failure, deferred commit failure, later-chunk failure, and
  remove rollback.
- Update inventory tests cover mixed actionable/unsupported buckets, reload,
  legacy payloads without `unsupported`, a later-entry trigger failure, and
  byte-for-byte/field-for-field baseline preservation.
