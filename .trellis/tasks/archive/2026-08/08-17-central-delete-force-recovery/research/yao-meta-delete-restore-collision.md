# yao-meta delete_restore_collision 现场记录

Date: 2026-08-17

## User-visible failure

- Central Skills delete dialog for `yao-meta-skill` reported
  `The operation failed. See runtime logs for details.`
- Operation Log `CENTRAL.DELETE` failed in 45 ms with
  `Central operation recovery collision (delete_restore_collision)`.
- Observability Console showed `Pending Central recovery`.
- Details JSON had `removedAgentIds: null` and `retainedAgentIds: null`.
  The new delete never started.

## Live journal row

`fs_db_operations.id = 1198b10f-ecf0-4d4a-9ae2-23f0513314ab`

| Field | Value |
| --- | --- |
| skill_id | `yao-meta` |
| operation_kind | `central_delete` |
| phase | `prepared` |
| created_at | 2026-08-05T01:54:13Z |
| updated_at | 2026-08-17T09:39:36Z |
| last_error_code | `delete_restore_collision` |
| current installations | only `central` / `native` |
| current Central path | `C:\Users\lyh\.skillsmanage\skills\yao-meta` (present) |

A completed `central_update` for the same skill exists
(`b5bf0e43-09fc-43ce-9a6b-10fb21f79453`, 2026-08-04) and is terminal.

## Manifest evidence

The prepared delete lists 14 paths, 5 unique originals, 9 duplicate extras
of `C:\Users\lyh\.agents\skills\yao-meta`.

| Original | original exists | backup exists | marker exists | expectedPresent |
| --- | --- | --- | --- | --- |
| `~\.agents\skills\yao-meta` | no | no | no | true |
| `~\.gemini\antigravity-cli\skills\yao-meta` | no | no | no | true |
| `~\.claude\skills\yao-meta` | no | no | no | true |
| `~\.config\zed\skills\yao-meta` | no | no | no | true |
| `~\.skillsmanage\skills\yao-meta` | yes | no | no | true |

No `.skillport-delete-backup-*` or `.skillport-operation-*.marker` siblings
remain in those roots.

A Python replica of the local directory fingerprint for the current Central
copy matched the journal fingerprint
`ff8a79b229cfcb07a4520b97ebcba55e93bf7a5aa03f49d1a15253ca586e307c`.

## Why delete failed

`delete_central_skills_under_guard` recovers selected pending rows before
inserting a new delete. The 12-day-old row is still `prepared`, so recovery
calls `restore_delete_local`.

`restore_delete_local_blocking` only accepts:

- `(original=false, backup=true)` restore backup
- `(original=true, backup=false)` already restored

Any other pair, including `(false, false)` on `expected_present` paths,
returns `RecoveryCollision { code: "delete_restore_collision" }`.

The four platform copies disappeared after the journal was prepared. No
backup was ever created because the operation never reached `fs_staged`.
Restore therefore fail-closes. The unique pending row then blocks every later
delete of `yao-meta`.

## Existing escapes

| Path | Result on this incident |
| --- | --- |
| Observability Console Retry | Calls the same restore. Fails again. |
| Observability Console Reconcile | Journal-only `prepared -> rolled_back`. Eligible when no artifacts remain, owned Central path exists, and missing platform paths are unowned. This incident looks eligible. |
| Delete dialog | No pending-recovery card. No force-delete. IPC maps the collision to `internal.unexpected`. |

`installation.pending_central_recovery` already blocks install/uninstall of
the same skill.

## Product gap

The user who wants to delete the live Central copy is sent to Runtime Logs.
The working escape (reconcile) is a shield button on `/logs`, not in the
delete dialog. There is no reviewed force-delete that abandons a stale
prepared journal and then deletes the current owned Central copy.
