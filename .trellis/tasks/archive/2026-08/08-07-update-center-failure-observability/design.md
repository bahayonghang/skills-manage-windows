# Design: recovery-safe Central deletion and truthful apply outcomes

## 1. Problem Boundary

This task repairs one causal chain across four existing boundaries:

```text
installation rows
  -> Central delete manifest
  -> FS/DB recovery journal
  -> Update Center batch result
  -> Operation Logs and recovery UI
```

It does not change the delete commit point, marker/fingerprint fail-closed
rules, Update Center selection/order, or operation-log storage schema.

The task remains one implementation unit because all deliverables protect the
same invariant: a Central mutation must leave unique, isolated recovery
evidence, and every user-facing operational outcome must describe that durable
state truthfully.

## 2. Decisions

| Area | Decision | Reason |
| --- | --- | --- |
| New manifests | Enforce stable path uniqueness in the shared Local/remote manifest builders | All delete callers receive the invariant; DB installation rows remain independent. |
| Legacy manifests | Decode existing version-1 duplicate manifests; collapse consistent duplicates only inside explicit reconciliation | Rejecting them at decode would strand the live row before it can be inspected. |
| Installation isolation | Move target-lock ownership to top-level installation use cases and check requested skill IDs under that guard | Prevents check/mutation races and covers desktop plus CLI without command-only patches. |
| Nested Central lock | Split centralization into guarded top-level entry and under-guard helper | Avoids self-deadlock after the installation boundary acquires the target lock. |
| Batch behavior | Under one target guard, fail only blocked skill IDs and continue unrelated IDs | A pending row for skill A must not make skill B unavailable. |
| Apply logging | Add an apply-specific success builder that classifies item outcomes; retain the generic wrapper for outer `Result` handling | The wrapper is correct for ordinary commands; only result-bearing batch commands need semantic classification. |
| Failure diagnostics | Preserve reviewed code/category/public message through batch DTOs; never reconstruct them from Display strings | Operation logs and UI need stable diagnostics without leaking dynamic error data. |
| Reconciliation | Preview, explicit confirm, then apply with a fresh preflight under the target guard | Preserves operator intent and closes the preview/apply race. |
| Reconcile effect | Only `prepared -> rolled_back`; no filesystem or business-DB writes | Accepts already-converged current ownership without fabricating restored data. |

## 3. New Manifest Invariant

`build_local_delete_manifest` preserves the first occurrence and compares each
candidate with retained paths through `paths::paths_equivalent`. This reuses
canonicalization, Windows case folding, extended-prefix handling, macOS alias
handling, and missing-descendant behavior already owned by `paths.rs`.

Remote delete planning validates/normalizes absolute POSIX paths before the
builder. The builder then performs stable exact deduplication over those
normalized values. It does not connect merely to canonicalize aliases.

Each retained original produces exactly one deterministic backup and marker.
Fingerprinting runs after deduplication, so the same physical path is neither
hashed nor staged twice. Existing DB installation rows are not deduplicated or
deleted early; the normal parent FK cascade remains their owner.

Version-1 decode remains compatible. Ordinary recovery still processes the
stored manifest strictly. The explicit legacy reconciliation path has its own
normalization pass that requires duplicate entries to agree on
`expected_present`, fingerprint, backup, and marker before treating them as one
piece of evidence.

## 4. Installation Mutation Boundary

Introduce top-level Local/SSH/WSL installation use cases that own this sequence:

```text
resolve target + prepare transport
  -> acquire target mutation guard
  -> load nonterminal operations for target
  -> partition requested skill IDs into blocked/unblocked
  -> run existing under-guard install/uninstall logic for unblocked IDs
  -> release guard
```

Single-item operations return a typed
`InstallationError::PendingCentralRecovery` before any FS/DB mutation. Batch
operations return per-item coded failures for blocked skills and continue
unblocked skills under the same guard. Different targets use different lock
identities and do not contend.

All production command and `skillport-cli` call sites use the top-level
wrappers. Existing orchestration becomes private `*_under_guard` logic. Local
`ensure_centralized` no longer acquires the same lock when called inside that
boundary; any standalone caller must use the guarded top-level centralization
entry. A contract test/production grep locks the caller inventory and prevents
future bypass.

The guard is acquired after remote connection/preparation but before target
filesystem or business-DB mutation, matching the existing lock contract.

## 5. Structured Apply Diagnostics

Batch failure payloads gain backward-compatible optional `errorCode` and
`errorCategory` fields plus a reviewed public message. Central operation
failures preserve `central_operation.<code>`; unreviewed failures expose a
fixed category/message and keep dynamic Display text out of IPC and logs.

Promote the existing three-state batch classifier to a neutral operation-log
helper and reuse it from installation and Update Center:

```text
failure_count = 0                         -> succeeded
failure_count > 0 and success+skip = 0   -> failed
failure_count > 0 and success+skip > 0   -> partial
```

`apply_operation_spec` builds the outer-error failure event as before, but its
`Ok(SkillUpdateApplyResult)` builder selects status and summary from item
outcomes. Safe details contain request/result counts and sorted unique reviewed
codes/categories. A partial/failed result also writes one bounded runtime event
with those stable fields. Raw item errors never enter `details_json`,
`error_summary`, export, or tracing.

The frontend renders reviewed item codes through `formatBackendError` and i18n.
Unknown legacy payloads fall back to a fixed generic message, not `String(err)`.

## 6. Explicit Prepared-Delete Reconciliation

Add two typed commands and service operations:

```rust
preview_fs_db_operation_reconciliation(operation_id)
    -> PreparedDeleteReconciliationPreview

reconcile_fs_db_operation(operation_id)
    -> Vec<PendingOperationSummary>
```

The preview contains only operation ID, skill ID, eligibility, duplicate-path
count, missing-unowned-path count, and stable blocker codes. It never exposes a
manifest or path.

Both operations resolve one `TargetContext`. Preview acquires the target guard
while inspecting a consistent snapshot, then releases it. Apply acquires the
guard independently, reloads the row, and runs the same preflight again.

### Preflight

1. Validate target identity, manifest version/operation ID, kind
   `central_delete`, and phase `prepared`.
2. Collapse only duplicate entries whose evidence fields agree; otherwise add
   `recovery.reconcile_inconsistent_duplicate`.
3. Load the current skill and installation rows. Build the current owned-path
   set from the Central canonical directory and installation paths using the
   same Local/remote path identity rules as deletion.
4. For each unique manifest entry:
   - any backup or marker present blocks reconciliation;
   - an existing original must match the stored fingerprint;
   - a missing originally-present path blocks when it is still DB-owned;
   - a missing path that is no longer DB-owned is counted as accepted current
     state;
   - an originally-absent path may remain absent.
5. A missing Central skill, remote inspection failure, invalid path, or any
   blocker makes `eligible=false` and leaves the row unchanged.

Eligible apply performs only the repository transition
`prepared -> rolled_back`, then returns the refreshed pending list. The
repository phase graph already permits this edge. Transactional transition
failure returns an error and leaves the row pending.

### UI Flow

The pending-recovery band retains Retry and adds Reconcile only for
`central_delete/prepared`. Reconcile opens a compact confirmation dialog:

```text
Preview loading
  -> blocked: localized blocker list, no confirm action
  -> eligible: counts + explicit confirmation
  -> apply loading
  -> success: pending list refresh + success toast
  -> failure/stale preview: row stays visible + localized error
```

Retry and reconcile have separate loading IDs. Target changes invalidate old
preview/apply responses through the store's existing latest-wins token.

## 7. Logging And Security

- Successful/failed apply uses `update_center.apply` with truthful status.
- Reconcile apply uses `central.operation_reconcile`; preview writes no log.
- Reconcile log details contain operation ID, skill ID, blocker codes/counts,
  and result only. Failure summaries use reviewed redacted messages.
- No manifest, full path, fingerprint, PAT/API key, SSH credential, repository
  URL, command output, or raw domain Display string crosses IPC or logging.
- Existing operation-log redaction remains defense in depth; callers still
  construct safe structured values before the redaction boundary.

## 8. Compatibility And Generated Surfaces

- No SQLite schema migration and no manifest version bump.
- New optional failure diagnostic fields use Serde defaults so old fixture/
  persisted payloads remain readable.
- New Tauri commands require runtime registry, typed IPC map/fixtures,
  generated command/docs updates, and command-count contract updates.
- English and Chinese i18n cover blocker messages, generic item failure,
  reconcile confirmation, success, and failure.

## 9. Rollout And Rollback

Land in four reversible layers: manifest uniqueness, installation isolation,
diagnostics/logging, then reconcile backend/UI. No layer mutates existing rows
on startup.

Rollback may stop exposing the reconcile command/UI while retaining terminal
rows already reconciled. A row changed to `rolled_back` required an eligible
preflight and represents no filesystem/business-DB mutation, so no reverse data
migration is required.

After all automated validation, inspect the live row through the new preview.
Actually reconciling `yao-meta` is an external destructive-state decision and
requires fresh user authorization; implementation approval alone does not
authorize that click/command.
