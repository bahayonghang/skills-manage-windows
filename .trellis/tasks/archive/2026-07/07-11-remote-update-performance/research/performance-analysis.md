# SSH/WSL Update Performance Analysis

## Conclusion

The remaining Local versus SSH/WSL gap is dominated by remote process and session granularity, not by GitHub snapshot download or per-file upload.

The May optimization already removed the original worst cases:

- repository snapshots are deduplicated and downloaded with bounded concurrency;
- remote hashing handles up to 32 Central skill roots per script;
- one skill is uploaded as one tar.gz archive instead of one command per file;
- normal apply reuses a fresh update state and snapshot after refresh.

However, apply still performs one remote process per updated skill and one more process per copy installation. `ConnectedRemoteTarget` reuses configuration and credentials inside one action, but every `run_*` call starts a new `ssh.exe` or `wsl.exe` process. SSH therefore repeats transport handshakes; WSL repeats the Windows-to-VM process boundary.

## Reproducible Evidence

### Process startup benchmark

Command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\.trellis\tasks\07-11-remote-update-performance\research\benchmark_process_startup.ps1
```

Environment: Windows host, warm `Ubuntu-24.04`, 25 iterations per command.

| Command | p50 | p95 | Meaning |
| --- | ---: | ---: | --- |
| `cmd.exe /c exit 0` | 32 ms | 34 ms | Windows child-process reference only; Local update does not need this process |
| `wsl.exe ... -- true` | 147 ms | 173 ms | Current WSL command boundary |
| `wsl.exe ... -- sh -lc true` | 147 ms | 165 ms | Current Central remote command shape |
| `wsl.exe ... --exec true` | 144 ms | 173 ms | Correct argv-preserving mode |
| `wsl.exe ... --exec sh -lc true` | 146 ms | 165 ms | Correct mode with shell |

`--exec` fixes argument correctness but does not materially reduce startup time. Each avoidable WSL primitive still costs roughly 145-150 ms before useful work.

### Application operation-log evidence

Command:

```powershell
python .\.trellis\tasks\07-11-remote-update-performance\research\analyze_operation_logs.py
```

The script reads only aggregated `duration_ms` and redacted operation details from the local SkillPort database.

| Existing action | Local fitted slope | WSL fitted slope | SSH fitted slope |
| --- | ---: | ---: | ---: |
| `central.delete_repository` | 12 ms/skill | 476 ms/skill | 626 ms/skill |
| `skill.batch_uninstall` | 2 ms/skill | no samples | 423 ms/skill |

The two unrelated remote workflows show the same linear cost class. This makes a Central-update-only CPU or database explanation unlikely. SSH intercepts are noisy because samples differ in installed slots; they are evidence of fixed preparation cost, not a stable constant.

### WSL argv probe

The Rust probe in `wsl_argv_probe.rs` uses the same `std::process::Command::args` boundary as production:

- `-d Ubuntu-24.04 -- sh -c <script> -- <paths>` produced empty `$1/$2`;
- replacing `--` with `--exec` preserved both paths.

This proves the update failure is caused by the WSL launch contract, not by `remote_parent()` or the selected skill.

## Current Call Counts

Let:

- `N` be selected Central skills;
- `H` be roots that require a fresh local/remote hash;
- `R` be distinct GitHub repositories;
- `C` be copy-install targets refreshed after update.

### Refresh inventory

- target connection probe: 1 remote process;
- remote hash: `ceil(H / 32)` remote processes;
- GitHub snapshots: at most `R` HTTP downloads, deduplicated and concurrency-limited to 4;
- inventory persistence: one SQLite upsert per resulting row.

Manual refresh intentionally bypasses snapshot cache, so network time is expected and common to Local, SSH, and WSL. The remote-only delta is primarily the connection probe and batched hash scripts.

### Apply checked updates

Typical fresh-state path:

- target connection probe: 1 remote process;
- local hash: reused, so normally 0 hash processes;
- GitHub snapshots: reused, so normally 0 downloads;
- Central writes: `N` archive processes, sequential;
- copy refresh: `C` remote scripts, concurrency 4 within each skill;
- database persistence and progress: sequential per skill.

Current minimum is therefore `1 + N + C` remote processes. SSH repeats authentication/session establishment for each process. WSL pays approximately 150 ms per process even before archive extraction or copying.

## Ranked Bottlenecks

1. Per-skill SSH/WSL process creation during apply.
2. Eager connection probe before an action that will immediately execute another remote command.
3. Per-copy-install remote scripts; nested concurrency shortens wall time but does not reduce process count.
4. Sequential per-skill apply orchestration and database persistence.
5. Missing total-duration and phase-level instrumentation for Update Center refresh/apply; direct update samples are absent from operation logs.
6. Repeated gzip initialization and synchronous archive construction per remote skill. This is secondary for small skills but matters when batching large selections.
7. GitHub snapshot download and remote hash are not the leading remaining remote delta because they are already repo-deduplicated/chunked.

## Hypotheses And Predictions

1. If process/session granularity is dominant, removing the eager probe and batching writes will improve 10+ skill apply by at least 60% while leaving GitHub timing unchanged.
2. If SSH handshake cost remains dominant after batching, one-skill and one-chunk SSH actions will still be hundreds of milliseconds slower than WSL; a persistent SSH session is then justified.
3. If database writes are secondary, Local timing will change little after remote batching; wrapping persistence in a transaction should only be considered after phase timings prove it material.
4. If `--exec` is correctness-only, its startup distribution will match `--`; the measured medians confirm this.

## Option Assessment

| Option | Benefit | Risk | Decision |
| --- | --- | --- | --- |
| WSL `--exec` | Fixes all WSL argv expansion at the transport boundary | Low, but updates exact argv tests | Required |
| Lazy operation connection | Removes one process/handshake from every remote action | Low; explicit target test must retain probe | Required |
| Spawn per-skill commands concurrently | Reduces wall time but keeps process count and can create auth/process storms | Medium | Reject as primary design |
| Batch Central writes/copy refresh | Changes process growth from per item to per chunk while preserving system SSH compatibility | Medium; needs a framed result protocol and rollback tests | Recommended MVP |
| OpenSSH ControlMaster | Reuses TCP session but has Windows socket/lifecycle issues | Medium/high | Keep rejected per 2026-05 decision |
| `russh` persistent session | Removes repeated SSH process and handshake cost, improves single-item actions | High; changes auth, known-hosts, cancellation, packaging, and test seams | Conditional phase requiring explicit scope confirmation |

## Proposed Performance Gates

The implementation must capture an update-specific before baseline before changing behavior. Proposed gates for a warm snapshot and a fixed 10-skill fixture are:

- remote process count without copies: current `1 + N` to at most `ceil(N / 16)`;
- copy refresh process count: current `C` to at most `ceil(C / 32)`;
- WSL and SSH apply p50 improves by at least 60% versus the captured baseline;
- single-skill apply does not regress and should improve by removing the eager probe;
- WSL added transport time for the 10-skill fixture is at most 1.5 seconds above Local;
- LAN SSH added transport time is at most 4 seconds above Local while still using `ssh.exe`.

If the SSH gate cannot be met after batching, persistent-session work should become a separate child task rather than weakening the gate silently.
