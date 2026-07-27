# Heavy Filesystem Work On Blocking Threads

## 1. Scope / Trigger

Apply this contract inside async commands and services when work recursively traverses, copies, deletes, or moves directories, writes files in bulk, or performs synchronous lock polling. Single-file I/O with no loop and at most one directory level may remain synchronous only when its bounded cost is clear.

## 2. Signatures

```rust
pub async fn run_blocking_fs<T, F>(label: &'static str, task: F) -> Result<T, FsUtilError>;

pub async fn run_blocking_fs_with<T, E, F, M>(
    label: &'static str,
    task: F,
    map_join_error: M,
) -> Result<T, E>;
```

`src-tauri/src/fs_util.rs` is the single implementation; `services/installation/fs_util.rs` is only a compatibility re-export.

## 3. Contracts

- Put one coherent recursive filesystem unit in one blocking closure. Preserve error order, early returns, loop control, cleanup, and user-visible error text.
- Clone owned paths and small metadata into the closure. Do not pre-clone whole batches of file bytes into a second in-memory copy.
- Map join failure into the caller's typed domain error with `run_blocking_fs_with`; do not return `String` below the command boundary.
- Keep `AppHandle`, `Option<AppHandle>`, progress emission, database work, and other async handles outside the blocking closure. Return a summary and emit progress on the async side.
- If the filesystem unit belongs to a mutation lease, acquire the top-level lease first and keep its guard alive across the blocking await and required DB marker/write.
- Legacy Central migration runs source create/read/copy/failed-partial cleanup as one blocking unit while the Local mutation guard remains alive; its marker read/write remains async.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Closure returns success | Resume async side with returned summary |
| Closure returns domain error | Preserve the typed error and original behavior |
| Tokio blocking task cannot join | Map to the domain's `TaskJoin` variant |
| Recursive copy partially creates a new target then fails | Apply the domain's existing partial-target cleanup; do not broaden deletion |
| Mutation guard cannot be acquired | Do not run the blocking unit without a lock |
| Progress is required | Emit before/after await from async code, never from a captured `AppHandle` |

## 5. Good / Base / Bad Cases

- Good: migration owns source/target `PathBuf`s in one closure, returns `CentralStoreMigrationSummary`, then writes the marker asynchronously while still locked.
- Base: a bounded single-file read remains async-side synchronous under the documented exemption.
- Bad: each directory entry gets its own `spawn_blocking`, or an `AppHandle` is moved into the closure.

## 6. Tests Required

- Preserve copy/skip/source-retention/partial-failure assertions when moving code to blocking execution.
- Assert join errors map to a typed `TaskJoin` variant where injection is available.
- On Windows, run the affected Rust test binary to prove it loads; compile-only evidence cannot catch `TaskDialogIndirect` import failure caused by `AppHandle` drop glue.
- Search changed async functions for recursive `std::fs` calls outside the blocking closure, then run locked Clippy/tests and `just ci`.

## 7. Wrong vs Correct

```rust
// Wrong: AppHandle drop glue enters the Windows test binary through the closure.
run_blocking_fs("copy", move || {
    copy_tree(&source, &target)?;
    app.emit("copy-progress", payload)?;
    Ok(())
}).await?;

// Correct: closure performs pure filesystem work; async side owns events.
emit_started(&app);
let summary = run_blocking_fs_with(
    "copy",
    move || copy_tree(&source, &target),
    DomainError::task_join,
).await?;
emit_completed(&app, &summary);
```

This Windows constraint was first established by task `06-11-spawn-blocking-io`: capturing `AppHandle` can link `comctl32.dll!TaskDialogIndirect` into a test binary without the v6 manifest and fail process startup with `STATUS_ENTRYPOINT_NOT_FOUND` (`0xc0000139`).
