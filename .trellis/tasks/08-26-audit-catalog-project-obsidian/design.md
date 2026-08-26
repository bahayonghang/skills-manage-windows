# Catalog/Project/Obsidian 覆盖设计

## Ownership

Own `central_metadata.rs`, `collections.rs`, `saved_views.rs`, `tag_groups.rs`, `agents/mod.rs`, `projects.rs`,
`obsidian.rs`. Reuse the core interface; do not add domain-local logging helpers.

## Safe Subjects

- IDs may be retained only when existing domain validators prove them to be bounded logical identifiers;
- user labels/names are optional display facts but are not copied into errors/details; prefer type + ID + counts;
- paths, saved queries, repository locations, vault names derived from paths, AI content and collection import payload are banned;
- reorder records item count, not the ordered ID vector.

## Lifecycle

Terminal-only: small metadata CRUD/reorder/pin/assignment. Started-then-terminal: AI bulk suggestion, project rescan,
batch install/import and Obsidian import when they touch multiple files/DB rows. Cancel updates the same operation row.

## Delegation

Collection/project install may call existing installation modules. The user-facing outer command owns the audit row; nested
install helpers return typed safe counts/failures and do not create duplicate rows. Existing lower-level business transaction
semantics remain unchanged.
