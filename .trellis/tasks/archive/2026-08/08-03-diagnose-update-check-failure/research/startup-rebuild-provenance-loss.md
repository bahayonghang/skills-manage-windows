# Startup Rebuild Provenance Loss Diagnosis

## User-visible symptom

Central Skills still lists 141 skills, but the repository tree contains only `jakubkrehel/skills` with 7 members and groups the other 134 as local/unknown.

## Tight read-only reproduction

The red-capable probe opens both databases in SQLite read-only/query-only mode and compares stable skill IDs:

```text
currentCentralSkills=141
currentMemberships=7
currentGithubRepos=1
recoveryMemberships=111
recoveryGithubRepos=23
recoveryMembershipsMatchingCurrentSkillIds=111
RED: current database lost recoverable repository memberships
```

All three available database files return `PRAGMA quick_check=ok`; this is logical state loss, not SQLite page corruption.

## Root cause timeline

1. On 2026-07-27 the database applied migrations 1-4. Migration 1 stored the then-published Windows checksum `aabde4fd51822355cbe2a7982ac895073f6e49e9f34882a50086d145462a736d`.
2. Commit `a47c7cd9` changed checksum calculation to normalize line endings and replaced the locked migration 1 checksum with `173296a19419edf197e3baa3b22de1f33184a1d8631141549751fbf1cfc24f7f`.
3. On 2026-07-29 the next startup rejected the healthy database during schema preflight. Runtime logging recorded `startup.schema_initialization_failed` with diagnostic `Healthy`.
4. The startup recovery action moved DB/WAL/SHM into `startup-recovery-20260729T035522.330Z-*` and initialized a clean database.
5. Scanner repopulated skill parent rows from disk, but repository memberships, update baselines, projects, tags, reviews, settings and historical operation logs exist only in SQLite and were not imported into the clean database.

## Evidence matrix

| Evidence | Current | Recovery backup |
| --- | ---: | ---: |
| SQLite quick check | ok | ok |
| Skills | 141 | 134 |
| GitHub repositories with members | 1 | 23 |
| Repository memberships | 7 | 111 |
| Update baselines | 0 | 76 |
| Projects | 0 | 3 |
| Tag links | 0 | 49 |
| Operation logs | 40 | 1245 |

The recovery backup's 111 membership skill IDs all exist in the current database. None currently has a membership, so the exact recovery set is 111 addable, 0 conflict and 0 missing parent. The 7 current memberships belong to skills created after the recovery snapshot.

## Recovery boundary

The 111 exact rows are suitable for a previewed merge because the startup recovery directory is the immediate source database that was replaced, not an arbitrary older snapshot. Another 23 preexisting current skills had no membership in that source database. Historical delete logs associate many of them with `mattpocock/skills`, but those logs also record explicit repository deletion; they are evidence candidates, not automatic recovery authority.

No real database, WAL/SHM file, recovery directory or Central skill was modified during this diagnosis. Applying the 111-row merge requires a fresh backup, application shutdown, a drift check against the approved preview, one transaction and explicit user approval.

## Executed preview

`preview_startup_recovery.py` was run against the explicit current and recovery database paths. It reported both `quickCheck=["ok"]`, both foreign-key violation counts as zero, and the expected classification:

```text
addable=111
alreadySame=0
conflict=0
missingParent=0
unresolved=23
readyForApprovedApply=true
```

The preview has no apply mode and opens both databases with SQLite `mode=ro` plus `PRAGMA query_only=ON`.
