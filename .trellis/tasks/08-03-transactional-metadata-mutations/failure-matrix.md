# Transaction Failure Matrix

| API | Validation before mutation | Atomic statements | Injected failure and invariant |
| --- | --- | --- | --- |
| `detach_skill_remote_source` | Missing skill remains a no-op | update-state delete, repository-member delete, empty-repository prune | member-delete trigger restores update state, member, and repository; retry removes all |
| `assign_skills_to_repository` | Repository first, then every skill ID in caller order | bounded membership upsert chunks | mixed valid/missing skill writes zero rows; row 181 trigger rolls back the first 180-row chunk |
| `assign_skill_tags` | Every tag, then every skill, before writes | bounded skill-by-tag upsert chunks | mixed valid/missing skill and second-tag trigger write zero links |
| `replace_skill_ai_tags` | Every tag and the skill before deleting old AI links | AI-only delete plus bounded inserts | invalid tag or second insert preserves old AI and manual links; retry replaces only AI links |
| `replace_pending_ai_tag_reviews` | Existing tag references and skill before deleting pending rows | pending delete plus bounded review upserts | invalid tag or second insert restores the complete old pending set; retry replaces it |
| `delete_collection` | No new validation; missing parent remains a no-op | child delete plus parent delete | parent-delete trigger restores both parent and child |
| `delete_project` | Pool connection contract verifies `foreign_keys=1` | one parent delete with FK cascade | parent trigger preserves parent/child; retry cascades on a production-opened multi-connection pool |
| `remove_registry_impl` | Built-in flag read inside the transaction | cache delete plus registry delete | parent trigger restores cache and registry; missing remains a no-op and built-in fails closed |
| successful `sync_registry_impl` DB phase | Fetch/parse and installed-identity derivation finish before begin | cache delete, bounded fresh inserts, success metadata update | A,B -> B,C and empty snapshots commit; second insert, success-status, deferred-FK commit, and row 113 failures preserve the complete prior snapshot and record error status |

SQLite statements use the shared 900-bind budget in `db::sqlite_batch`. Tag
cross-products use checked multiplication; every chunk is executed through the
same transaction connection. Trigger fixtures use `RAISE(ABORT, ...)` on a
later statement or later batch, never an input that fails before mutation.
