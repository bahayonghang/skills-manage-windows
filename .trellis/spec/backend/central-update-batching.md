# Central Update Batching Contract

## 1. Scope / Trigger

Apply this contract when changing Central skill update writes or copy-install refreshes across Local, SSH, and WSL. The goal is to preserve per-skill semantics while bounding remote process and round-trip counts.

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

## 3. Contracts

- Remote Central writes group by target parent and use chunks of 16.
- Remote copy refresh uses chunks of 32 and deduplicates installed targets before execution.
- Batch archives contain `.skillport-manifest.tsv` plus one generated numeric directory per skill.
- Remote execution remains behind `ConnectedRemoteTarget` and `CommandRunner`.
- Each skill receives a durable operation ID and manifest before staging. Cancellation before staging returns cancelled; after staging starts the Saga settles or leaves a recoverable row.
- Skill/repository persistence and `db_committed` share one SQLite transaction. Copy plans are persisted before refresh; incomplete copies remain `copies_pending` and are retried without reapplying canonical contents.
- Archive construction and Local recursive IO run through `run_blocking_fs_with`.
- Operation Logs store total action duration and non-sensitive counts only. Phase spans may record target kind, counts, chunk counts, and payload bytes, never host, username, credentials, contents, or full paths.

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
- Keep an ignored Windows WSL `/tmp` benchmark for a fixed 10-skill fixture; never benchmark against `~/.skillsmanage`.

## 7. Wrong vs Correct

```rust
// Wrong: one new ssh.exe/wsl.exe process per skill.
for update in updates {
    fs.write_skill_dir_atomic(update).await?;
}

// Correct: one orchestration and compound transport hooks.
let outcomes = update_skills_batch(pool, fs, plans, cancel).await;
```
