# Design

## Boundary

Implement relocation handling in the Rust inventory refresh path, not in the
React dialog. The backend has the authoritative repository membership, pending
addition rows, update-state rows, and snapshot content needed to resolve the
case safely. The frontend should continue to render whatever inventory it
receives.

## Data Flow

Refresh currently follows this shape:

1. Load local Central skills and repository assignments.
2. Compare tracked source paths against repository snapshots.
3. Mark old tracked paths that no longer contain an importable skill as
   `remote_missing`.
4. Inspect repository snapshots for valid candidates whose source paths are not
   already local members, then persist them as `remote_added`.
5. Return the inventory buckets.

Add a reconciliation step between remote-added collection and final inventory
assembly:

1. Build a map of remote-missing states by `(repository_id, skill_id)`.
2. Build a map of non-skipped remote-added candidates by `(repository_id,
   skill_id)`.
3. Only resolve keys with exactly one missing state and exactly one added
   candidate.
4. For a resolved key, update the repository member source path by using the
   same import/update helper path that persists GitHub source metadata, delete
   the pending addition row, then recalculate the update state using the new
   source path.
5. Remove resolved items from `remote_added` and `remote_missing`.

## Contracts

- Detection key: `(repository_id, skill_id)`.
- Safety checks:
  - old and new source paths must both be present and different after
    `normalize_repo_path`.
  - only one missing and one added candidate may match the key.
  - skipped additions are excluded from auto-resolution.
  - if recalculation fails, leave the original manual inventory intact and add
    a failed repository entry rather than partially hiding evidence.
- DB contract:
  - repository membership source path is updated for the existing skill id.
  - `skill_update_states` is replaced with the recalculated state.
  - pending addition for the new source path is removed.

## Compatibility

This is a non-destructive refresh-time metadata repair. Existing apply decisions
remain unchanged. Existing Added/Removed UI continues to work for ambiguous
cases and genuine add/remove cases.

## Trade-Offs

Auto-resolving only same-id moves misses renamed skill ids, but that avoids
guessing. A future feature could present suggested migrations for renamed ids;
that is out of scope here.
