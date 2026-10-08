# Central Update Batching Contract

## 1. Scope / Trigger

Apply this contract when changing Central skill update writes, copy-install refreshes, or Update Center Platform leftover apply across Local, SSH, and WSL. The goal is to preserve per-item semantics while bounding remote process and round-trip counts.

## 2. Signatures

```rust
CentralFs::write_skill_dirs_atomic_cancellable(
    writes: Vec<CentralSkillWrite>,
    cancel: Option<&AtomicBool>,
) -> Vec<CentralSkillWriteOutcome>

CentralFs::refresh_copy_installs_cancellable(
    copies: Vec<CopyRefreshRequest>,
    cancel: Option<&AtomicBool>,
) -> Vec<CopyRefreshOutcome>
```

`write_skill_dirs_atomic_cancellable` is retained only under `#[cfg(test)]` for the legacy batching contract. Production update writes use operation-scoped `build/stage/swap/rollback/finalize` hooks from `update_skills_batch` and persist one journal row per skill.

Normal update, force update, and force mirror must route through `update_skills_batch`; do not add a second per-skill production loop.

## Scenario: Local stage write progress and owned file contents

### 1. Scope / Trigger

GitHub import reports successful staging writes while the existing Central Saga owns locking, recovery, swap, persistence, copy refresh, and finalize. Large file counts must not cause one Tauri event per file or repeated deep copies of the same owned byte batch.

### 2. Signatures

```rust
journaled_central_content_upsert_with_fs_and_progress(
    pool: &DbPool,
    fs: &CentralFs,
    input: JournaledCentralContentUpsert<'_>,
    progress: Option<StageWriteProgressReporter>,
) -> Result<SkillUpdateState, CentralUpdatesError>

StageWriteProgressReporter::channel()
    -> (StageWriteProgressReporter, watch::Receiver<StageWriteProgress>)
```

The existing no-progress upsert and batch wrappers delegate to the same orchestration body. Public Tauri commands, DTOs, database schemas, and journal manifest versions remain unchanged.

### 3. Contracts

- The blocking writer counts a file and its bytes only after `std::fs::write` succeeds. A preparation/path-validation loop never advances completed writes.
- `StageWriteProgress` contains cumulative successful `completed_files`, `completed_bytes`, and the latest skill-relative `current_path`. It contains no event handle, credential, or full user path.
- Create one reporter for each single-skill journaled import upsert. The importer adds that stage's counters to the completed counts from earlier skills; ordinary multi-skill update batches pass no reporter.
- The writer retains one cumulative watch value. Publish after 256 additional successful files or 100 ms; always flush at success/error. The async importer samples at most once per 100 ms and flushes the latest value when the service settles. Start, phase transition, error, and terminal handling retain existing semantics.
- Closing the data channel cannot leave a polling loop running after the import. A completed staging count does not establish successful swap, DB commit, copy refresh, or finalize.
- Remote progress continues to count completed skills. Increment only after the Remote service succeeds; archive preparation is not a confirmed remote write.
- Move the owned file list through plan, write, stage, and blocking worker. Retained immutable snapshots may require one initial copy. Ordinary update, force update, and force mirror use the same ownership rule.
- Cache successfully created exact parents only inside one operation-owned staging writer. Keep per-file safe-path validation, original file order, and first-error behavior. Never cache filesystem existence across operations.
- Candidate digest optimization may filter selected paths before hashing. Preserve exact manifest validation and each existing digest domain/framing. Immutable memory metadata does not replace fresh staging, target, backup, recovery, or delete fingerprint reads.
- Delete fingerprint sorting may borrow paths for comparison. Preserve the original global `Path` order, tree-collection error order, symlink behavior, and byte framing.

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Preparation rejects a path or target | No completed write count |
| File N fails | Report only the successful preceding writes, retain the original typed error |
| Parent creation fails | Do not cache the parent or advance counters |
| All staging writes succeed but Saga fails later | Keep accurate staging counts; return the existing failure/pending result |
| Data channel closes before service completion | Keep the retained counters, avoid busy polling, and await service settlement |
| Remote service fails | Do not advance completed skill count |
| Target or backup changes externally | Fresh validation rejects the operation under the existing journal contract |

### 5. Good / Base / Bad Cases

- Good: 13,016 successful writes produce bounded cumulative publications and an exact final count, with event emission on the async importer.
- Base: a caller without progress follows the same Saga with `None`.
- Bad: source validation emits a completed write; a blocking worker captures `AppHandle`; a cached memory digest replaces mutable disk validation.

### 6. Tests Required

- Assert zero preparing counts, monotone cumulative counts, exact success/error tail, first-error ordering, and no writes after the failed item.
- Assert publication budget `ceil(files / 256) + ceil(write_ms / 100) + 2`; test the time threshold without waiting on wall-clock sleeps.
- Assert async sampling retains the last value on closed channels and awaits service settlement, including errors; distinguish source-event counts from real Tauri IPC/WebView evidence.
- Assert shared parents, unsafe paths, nested paths, and parent/file collisions preserve contents and errors.
- Compare optimized candidate digests and directory fingerprints against the prior algorithms, including root/nested sources, Unicode, empty/binary files, missing manifest, and sibling directory/path ordering.
- Preserve uid, no-op, selected recovery, stage/swap/DB/copy/finalize faults, repeated cleanup, and directory symlink target-protection regressions.

### 7. Wrong vs Correct

```rust
// Wrong: a source entry does not establish a completed write.
completed_files += 1;
emit_writing(source_entry);

// Correct: the worker publishes data after a successful write.
std::fs::write(&path, &file.bytes)?;
progress.written(&file.relative_path, file.bytes.len() as u64);
// The async importer samples the cumulative value and emits the event.
```

## 3. Contracts

- Remote Central writes group by target parent and use chunks of 16.
- Remote copy refresh uses chunks of 32 and deduplicates installed targets before execution.
- Remote Platform leftover apply validates every path first, groups by unique POSIX path, and deletes with `REMOTE_LEFTOVER_DELETE_SCRIPT` in chunks of 256. Ten Universal Agent leftover groups that share `~/.agents/skills/<skill>` are one remote path.
- Batch archives contain `.skillport-manifest.tsv` plus one generated numeric directory per skill.
- Remote execution remains behind `ConnectedRemoteTarget` and `CommandRunner`.
- Each skill receives a durable operation ID and manifest before staging. Cancellation before staging returns cancelled; after staging starts the Saga settles or leaves a recoverable row.
- Under the target mutation guard, a new update batch reads pending rows once and recovers only rows whose `skill_id` is selected. A row-specific recovery failure occupies only that skill's ordered result slot; unrelated pending rows and their evidence are not touched. Startup and explicit recovery retain their full-target fail-fast behavior.
- Duplicate request skill IDs are classified before scoped recovery. The first request slot owns recovery and mutation; later duplicate slots return a typed `prepare` failure and never duplicate recovery evidence or filesystem writes.
- Shared Local/SSH/WSL delete single and batch entry points use the same selected-row boundary: deduplicate in first-request order, merge requested agent IDs, acquire one target guard, open at most one remote connection, read pending rows once, and continue unrelated skills after a row-specific recovery failure.
- Skill/repository persistence and `db_committed` share one SQLite transaction. Copy plans are persisted before refresh; incomplete copies remain `copies_pending` and are retried without reapplying canonical contents.
- Archive construction and Local recursive IO run through `run_blocking_fs_with`.
- Operation Logs store total action duration and non-sensitive counts only. Phase spans may record target kind, counts, chunk counts, and payload bytes, never host, username, credentials, contents, or full paths.
- Update Center apply status is derived from item outcomes: no failures is `succeeded`, failures with no successful/skipped item is `failed`, and mixed outcomes are `partial`. Apply logs and runtime events contain counts plus reviewed stable codes/categories only; item Display strings never cross the serialization boundary.
- Every failed item carries a controlled `phase`, stable `errorCode`, stable `errorCategory`, safe logical identifier, and fixed public message. Operation Logs retain at most 50 safe item tuples plus a truncation count; Runtime Logs retain only sorted/deduplicated code/category sets and phase counts.

## 4. Validation & Error Matrix

| Condition | Required outcome |
| --- | --- |
| Unsafe skill id, tab/newline, traversal path, or missing target parent | Reject before starting the remote process |
| Archive extraction or manifest failure | Fail the whole chunk; canonical targets remain untouched |
| One staging/backup/swap failure | Return `ERR` for that skill, restore its backup when present, continue unrelated rows |
| Missing or malformed result row | Convert every affected item to a typed `CentralUpdatesError::Batch` failure |
| Cancel flag between chunks | Start no later chunk; return `BatchCancelled` for unstarted rows |
| Cancel or transport failure after durable staging starts | Roll back staged artifacts or retain a pending journal row; never report an unjournaled cancellation |
| Copy refresh fails after DB commit | Preserve canonical new state and exact incomplete projection plan in `copies_pending` |
| Copy target basename differs from skill id | Return `CopyInstallOutsideSkillDir` without executing that target |
| Selected skill has an unrecoverable pending row | Return a `recovery` failure for that skill and continue unrelated plans in request order |
| Unselected skill has a pending row | Do not retry it or change its phase, timestamp, error evidence, or filesystem artifacts |
| A selected delete collides during recovery | Return the typed recovery code/category for that skill; do not degrade it to decision-apply; continue other requested deletes |

## 5. Good / Base / Bad Cases

- Good: 33 writes under one parent produce 3 remote calls and 33 ordered per-skill outcomes.
- Base: one write still uses one atomic archive/swap call and preserves single-skill behavior.
- Bad: `buffer_unordered` over per-skill `ssh.exe` / `wsl.exe` calls; wall time may fall while authentication pressure and process count remain linear.

## 6. Tests Required

- Assert `ceil(N / 16)` Central-write calls and `ceil(C / 32)` copy calls with `FakeRunner`.
- Assert complete argv/stdin through `CommandRunner`; no direct `Command::spawn` in the service.
- Assert mixed `OK` / `ERR` output preserves partial success.
- Assert unsafe ids/paths fail before any runner call.
- Assert cancellation after one chunk prevents the next call.
- Assert Local and Fake SSH/WSL batches skip unrelated pending rows; selected recovery failure affects only the matching ordered result.
- Assert duplicate selected IDs keep first-request ownership even when that skill's pending recovery fails.
- Fake SSH/WSL delete assertions include target ID/kind, one shared connection, and exact command counts so filtering cannot add hidden recovery round trips.
- Keep an ignored Windows WSL `/tmp` benchmark for a fixed 10-skill fixture; never benchmark against `~/.skillsmanage`.
- FakeRunner leftover apply: 10 shared-root groups → 1 call; mixed OK/MISSING/ERR keeps partial success; guards start 0 calls.

## 7. Wrong vs Correct

```rust
// Wrong: one new ssh.exe/wsl.exe process per skill.
for update in updates {
    fs.write_skill_dir_atomic(update).await?;
}

// Correct: one orchestration and compound transport hooks.
let outcomes = update_skills_batch(pool, fs, plans, cancel).await;
```

## Scenario: Remote Platform leftover apply

### 1. Scope / Trigger

Changing `apply_remove_deleted_platform_copies_step` or any SSH/WSL leftover delete path. Leftover inventory groups are `(agent_id, skill_id)`. Universal Agents share `~/.agents/skills/`, so one physical directory can appear many times.

### 2. Signatures

```rust
apply_remove_deleted_platform_copies_step(
    pool, active_target, removals, result, allowed_agent_ids, cancel,
)

delete_leftover_installations_and_observations_for_paths(
    pool, path_aliases, payload_pairs,
)
```

Implementation lives in `services/central_updates/inventory/leftover_cleanup.rs`. Do not grow leftover logic back into `apply_steps.rs`.

### 3. Contracts

- Local leftovers keep `uninstall_skill`. Remote leftovers do not use `InstallTransport`.
- Open one `connect_remote_target` per leftover-only apply. Execute through `run_script_cancellable` + `ProcessPolicy::bulk_transfer()`.
- A path enters the script only after: allowed agent, not `central`, Central still missing, `path == remote_join(agent.global_skills_dir, skill_id)`, and `ensure_remote_child_path`.
- Script rows are `OK\t<index>`, `MISSING\t<index>`, `ERR\t<index>`. `MISSING` is success.
- On OK/MISSING, one transaction deletes leftover `skill_installations` and writable non-plugin `agent_skill_observations` for that path, including shared-root sibling platforms that were not in the payload.
- Cancel before a chunk starts no later chunk.

### 4. Validation & Error Matrix

| Condition | Required outcome |
| --- | --- |
| Path fails a remote guard | Failure for that removal; runner call count 0 if no path remains |
| Script `ERR` for one unique path | Fail only removals that use that path |
| Protocol missing/duplicate/unknown row | Fail the chunk; do not parse stdout with `error.contains` |
| Central skill reappears during plan | Skip remote delete for that skill |

### 5. Good / Base / Bad Cases

- Good: 10 Universal Agent groups, one POSIX path → 1 runner call; scan no longer returns that path.
- Base: one leftover path still uses one script call and keeps per-removal outcomes.
- Bad: `connect_remote_target` + `remove_tree` inside the per-path loop.

### 6. Tests Required

- FakeRunner: shared-root 10-agent payload → 1 call; script stdin is `REMOTE_LEFTOVER_DELETE_SCRIPT`; unique path appears once in argv.
- Mixed OK/MISSING/ERR keeps partial success and leaves the ERR path in DB.
- Guard / `..` / platform-root paths start 0 runner calls.
- Cancel after chunk 1 starts no chunk 2.
- After OK/MISSING, `scan_deleted_platform_copies_with_pool` does not return the path.

### 7. Wrong vs Correct

```rust
// Wrong: one ssh.exe per leftover group, even when they share a directory.
for removal in removals {
    let conn = connect_remote_target(target).await?;
    conn.remove_tree(&removal.paths[0]).await?;
    db::delete_skill_installation(pool, &removal.skill_id, &removal.agent_id).await?;
}

// Correct: validate, unique-path script, then path-scoped DB cleanup.
apply_remove_deleted_platform_copies_step(pool, target, removals, result, allowed, cancel).await;
```
