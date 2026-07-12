# Stage 2 Performance Results

Date: 2026-07-12

## Environment

- Host: Windows, Rust debug test build
- WSL distribution: `Ubuntu-24.04`
- Remote fixture root: a unique directory under WSL `/tmp`; the benchmark never writes to `~/.skillsmanage`
- Fixture: 10 skills, one `SKILL.md` per skill, and 10 copy-install targets
- Timed phases: one hash pass, Central archive/write, and copy refresh
- Samples: 5 warm runs per shape; p50 is the middle sorted sample

## Results

| Target / shape | Samples (ms) | p50 (ms) |
| --- | --- | ---: |
| Local batch | 28, 29, 30, 31, 35 | 30 |
| WSL legacy process shape | 3065, 3102, 3127, 3226, 3227 | 3127 |
| WSL batch process shape | 425, 425, 453, 473, 511 | 453 |

The WSL 10-skill p50 improved by 85.5%: `(3127 - 453) / 3127`.
The WSL batch fixture added 423 ms over Local, below the accepted 1.5-second gate.

The legacy comparison deliberately invokes the same current atomic protocol one skill/copy at a time. This isolates the process and round-trip shape from unrelated protocol differences: one hash process plus 10 write processes plus 10 copy processes versus one hash process plus one write process plus one copy process.

## Process And Cancellation Proof

Deterministic `FakeRunner` tests prove:

- 33 Central writes use 3 remote processes: `ceil(33 / 16)`.
- 65 copy refreshes use 3 remote processes: `ceil(65 / 32)`.
- Cancellation raised after the first 16-skill chunk prevents the remaining 17 writes from starting another process.
- Partial `OK` / `ERR` rows remain per skill.

## SSH Gate

A live LAN SSH fixture with test credentials was not available in this session, so no SSH timing is presented as measured. The pre-change operation-log evidence fitted 423-626 ms of SSH added time per skill for process-heavy operations. For the same 10-skill plus 10-copy fixture, Stage 2 reduces the process shape from 21 to 3. Using the higher observed 626 ms slope as a conservative per-process bound gives approximately 1.88 seconds of SSH added time after batching, below the accepted 4-second gate.

This is sufficient evidence not to start `07-11-ssh-persistent-session`: Stage 2 passed the measured WSL gate and the existing SSH evidence does not show a remaining gate miss. The child task stays in `planning` until a real LAN SSH benchmark demonstrates otherwise.

## Commands

```powershell
$env:SKILLPORT_TEST_WSL_DISTRO='Ubuntu-24.04'
cargo test live_wsl_ten_skill_batch_benchmark -- --ignored --nocapture
cargo test local_ten_skill_batch_benchmark -- --ignored --nocapture
cargo test remote_batch_writes_use_one_process_per_sixteen_skills
cargo test remote_copy_refresh_uses_one_process_per_thirty_two_targets
cargo test remote_batch_write_checks_cancellation_between_chunks
```
