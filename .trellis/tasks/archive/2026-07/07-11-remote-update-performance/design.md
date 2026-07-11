# Design: SSH/WSL Central Update Reliability And Performance

## Scope And Boundaries

This design changes the transport and Central update execution shape while preserving Update Center product semantics. It does not change update detection, inventory scope, force-mirror meaning, GitHub cache policy, or platform installation rules.

The design has two required stages and one conditional stage:

1. Correct WSL argv handling, remove eager action probes, and add measurements.
2. Batch remote Central writes and copy refresh while preserving per-skill outcomes.
3. Conditionally migrate SSH to a persistent `russh` session only if the user includes that high-risk scope and batching does not meet the accepted SSH gate.

## Current Data Flow

```text
Update Center apply
  -> create CentralFs
     -> connect_remote_target
        -> ssh.exe/wsl.exe probe
  -> prepare skills + reuse hashes/snapshots
  -> for each skill, sequentially
     -> build one tar.gz
     -> ssh.exe/wsl.exe atomic update script
     -> DB upserts
     -> for each copy target
        -> ssh.exe/wsl.exe refresh script
  -> reload inventory / Central rows
```

The `CentralFs::Remote` object is reused, but its `ProcessRunner` creates a new process for every call.

## Stage 1: Transport Correctness And Measurement

### WSL launch contract

Change `ConnectedWslTarget::base_command()` from:

```text
wsl.exe -d <distribution> -- ...
```

to:

```text
wsl.exe -d <distribution> --exec ...
```

`--exec` makes WSL preserve the argv built by Rust. All existing higher-level shapes remain valid:

- `sh -s -- <args>` with script on stdin;
- `sh -lc <command>`;
- direct commands used by tests and discovery-adjacent operations.

Update tests to assert `--exec`, script arguments, stdin bytes, and failure detail. Keep a Windows-only live probe for the exact `$1/$2` regression.

### Separate open from probe

The current `connect_ssh_target` / `connect_wsl_target` constructs a connection and immediately runs a no-op command. Split the concepts:

- `open_*_target`: validate local configuration, load credentials/askpass state, construct the connection, but do not start a remote process;
- `probe_*_target`: call `open_*_target`, then run the explicit connectivity/probe script.

Normal business operations and `CentralFs::from_active_target` use `open_*`. Settings create/edit/test flows retain `probe_*`. The first real operation therefore remains the authority for connection failure and preserves typed error mapping.

### Observability

Wrap user-visible refresh/apply/force commands with total-duration operation logs. Add narrow runtime spans for:

- `target_open`;
- `local_or_remote_hash`;
- `snapshot_download`;
- `archive_build`;
- `central_write`;
- `copy_refresh`;
- `db_persist`.

Record target kind and counts only. Do not log host, username, token, password, repository credentials, file contents, or full paths. Phase timings belong to Runtime Log/tracing; Operation Log stores the user action total and summary.

## Stage 2: Batched Composite Operations

### New CentralFs contract

Introduce domain-level composite inputs and outcomes rather than exposing more generic filesystem primitives:

```rust
pub(crate) struct CentralSkillWrite {
    pub skill_id: String,
    pub target_dir: PathBuf,
    pub files: Vec<RemoteSkillFile>,
}

pub(crate) struct CentralSkillWriteOutcome {
    pub skill_id: String,
    pub result: Result<(), CentralUpdatesError>,
}

impl CentralFs {
    pub async fn write_skill_dirs_atomic(
        &self,
        writes: Vec<CentralSkillWrite>,
    ) -> Vec<CentralSkillWriteOutcome>;

    pub async fn refresh_copy_installs(
        &self,
        copies: Vec<CopyRefreshRequest>,
    ) -> Vec<CopyRefreshOutcome>;
}
```

This follows the existing transport-seam rule: the service owns one orchestration, while Local and Remote implement compound hooks with intentionally different mechanics.

### Local implementation

Run the existing atomic replacement logic inside one `run_blocking_fs_with` closure. Process writes sequentially and collect per-skill outcomes. Do not change Local semantics merely to match remote batching.

### Remote archive format

Group writes by target parent and chunks of 16. Build one tar.gz per chunk in `run_blocking_fs_with` or an equivalent blocking helper.

Archive layout:

```text
.skillport-manifest.tsv
0000/<skill files>
0001/<skill files>
...
```

Manifest rows contain a generated archive key, skill id, and target directory. Reject tabs, newlines, unsafe ids, unsafe relative paths, and targets without a parent before starting the remote process.

The remote script:

1. creates one sibling batch staging root;
2. extracts the archive before touching any canonical target;
3. validates every manifest entry and extracted directory;
4. updates each skill independently with sibling staging/backup paths;
5. restores that skill's backup if its swap fails;
6. emits one parseable `OK` or `ERR` row per skill;
7. cleans all batch, staging, and backup paths on success or handled failure.

An archive/extraction/manifest failure returns a chunk-level error and leaves every target untouched. A per-skill swap failure does not prevent unrelated valid rows from completing, matching current partial-success behavior.

### Batch copy refresh

Collect copy targets for all successful Central writes, deduplicate by installed path, and execute chunks of 32 triplets `(skill_id, source_dir, target)`. The remote script validates target basename, refreshes each copy independently, and returns per-target outcomes.

This replaces nested per-skill `buffer_unordered(4)`. Bounded process concurrency is not needed once calls are chunked, preventing an SSH authentication storm.

### Orchestration split

Refactor the current `update_one_skill` flow into explicit phases:

1. plan remote content and validate every file/path;
2. execute batched Central writes;
3. persist successful Central rows and repository assignments;
4. execute batched copy refresh for successful writes;
5. persist update states and emit per-skill progress/outcomes.

Normal update, force update, and force mirror must call the same batch executor. Do not maintain a second force-only write loop.

Cancellation remains checked between chunks and before persistence. Current subprocess execution is not cancellable mid-command, so chunking does not remove a supported capability. Use chunk size 16 to bound the longest uninterruptible update unit and command payload.

## Stage 3: Conditional Persistent SSH Session

Historical ADR `plans/ssh-perf/decisions.md` selected `russh` and rejected ControlMaster, but that phase was never implemented. It changes security and compatibility surfaces:

- password and private-key authentication;
- encrypted key/passphrase behavior;
- known-hosts storage and accept-new semantics;
- server algorithm compatibility;
- cancellation and timeout behavior;
- Windows packaging and dependency supply chain;
- the current `CommandRunner` test seam.

Therefore Stage 3 is not silently bundled into Stage 1/2. If included, create a child task with its own design, dependency/version research, compatibility matrix, rollback plan, and real-host test gate. Do not keep dual ssh.exe/russh production implementations after migration unless the user explicitly reverses the historical no-fallback decision.

## Compatibility

- Local behavior and paths remain unchanged.
- SSH continues using system `ssh.exe` through required Stages 1/2.
- Password/key auth, askpass, connect timeout, server-alive settings, and `accept-new` behavior remain unchanged in Stages 1/2.
- WSL continues using the configured distribution, default user HOME, and no sshd.
- Existing update/force payloads and frontend decision shapes remain unchanged.
- Existing progress events remain per skill; within a remote chunk they may arrive after the chunk process returns.

## Rollback

- Stage 1 can restore `--` and eager probes independently, though restoring `--` reintroduces the confirmed WSL bug.
- Stage 2 keeps the existing single-skill helpers until batch tests and live baselines pass. Roll back the orchestrator to them before removing dead helpers.
- Batch staging paths are siblings under the target parent and are uniquely named. No migration or persistent format change is introduced.
- Runtime/operation timing fields are additive and can be removed without data migration.

## Rejected Alternatives

- Increasing per-skill concurrency as the main optimization: lowers wall time at the cost of unchanged process count, higher authentication pressure, and nested concurrency.
- Re-downloading repositories less often during manual refresh: violates the established cache-bypass product contract and does not explain the remote-only delta.
- Per-file SFTP before persistent SSH exists: adds a new protocol path without removing repeated session establishment.
- ControlMaster: retained as rejected due to Windows lifecycle/socket behavior and the existing user decision.
