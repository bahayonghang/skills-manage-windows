# Transactional Metadata and Cache Mutations

## Scope

Applies to repository membership, tags and AI reviews, collection/project
deletion, and Marketplace registry cache mutation.

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
- Marketplace fetch and parse complete before opening the cache transaction. A
  successful fetch replaces, rather than upserts into, the registry snapshot:
  delete old rows, insert all fresh rows, and publish success metadata in one
  transaction. An empty fresh snapshot clears the cache.
- Marketplace snapshot failure preserves the complete old cache. Only after
  rollback may a named best-effort helper publish the derived error marker; it
  must warn if that marker cannot be written.

## Required Tests

- Mixed valid/missing IDs prove zero partial writes and stable validation text.
- A trigger on a later statement proves the previous statement rolls back.
- A batch larger than the bind budget fails in a later chunk and restores the
  pre-call state.
- AI tests cover manual-link preservation, old pending-set restoration, and
  successful retry after removing the trigger.
- Project tests acquire multiple production pool connections, read
  `foreign_keys=1` from each, and prove cascade.
- Marketplace tests cover A,B -> B,C, empty, second-insert failure,
  success-status failure, deferred commit failure, later-chunk failure, and
  remove rollback.
