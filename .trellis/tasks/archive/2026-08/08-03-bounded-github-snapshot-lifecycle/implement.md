# Implementation Plan: Bounded GitHub snapshot lifecycle

## Step 1 - Baseline and red tests

- [x] 记录典型/上限 snapshot 的 retained bytes 与当前 clone 点。
- [x] 为 Central cache 写 entry/byte/TTL/LRU/Arc identity failing tests。
- [x] 为 preview registry 写 per-target/global cap 与跨-target prune ownership failing tests。
- [x] 为 remote cleanup failure/retry 写 FakeRunner 状态测试。

## Step 2 - Central cache

- [x] 将 snapshot map/result 改为 `Arc<GitHubRepoSnapshot>`。
- [x] 实现 checked retained-byte accounting 和 injectable limits/clock。
- [x] 实现 expired prune、deterministic LRU、oversized current-use-only outcome。
- [x] 更新 Central update call sites，避免 `(*arc).clone()` 回退。

Gate: `cargo test central_updates::snapshots --locked` 与 inventory/core focused tests。

## Step 3 - Preview registry limits

- [x] 扩展 entry state/metadata，加入 per-target ready/byte 与 global entry accounting。
- [x] 保持 import lease/deferred discard transitions 并加入 deterministic victims。
- [x] 让 expired lookup 继续 fail closed，但由显式 owning-target sweep 完成回收。
- [x] 新增 typed capacity errors 与固定 IPC mapping。

Gate: registry lifecycle tests在并行运行下稳定，无全局测试互相 prune。

## Step 4 - Ownership-aware remote cleanup

- [x] 用 target-scoped sweep 替换 global prune/current-connection cleanup。
- [x] 实现 CleanupTicket generation、CleanupPending 与 ack/retry。
- [x] 覆盖 pre-acquisition reservation 与 new-workspace cleanup failure ownership。
- [x] 接入 preview creation、explicit discard、import success/failure 和 owning-connection retry call sites。

Gate: A/B target、remove failure、retry、active lease matrix 全过。

## Step 5 - Docs and observability

- [x] 更新 preview snapshot spec 的容量与 cleanup state。
- [x] 增加不含 token/path/repo/digest 的 count/bytes/reason tracing。
- [x] 搜索旧 `prune_expired_preview_snapshots` 和 deep clone call sites，确认无生产旁路。

## Step 6 - Validation

- [x] focused Central snapshot and GitHub snapshot tests。
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [x] IPC 若变更：`pnpm docs:gen` + `pnpm docs:gen:check` + frontend contract tests。
- [x] `just ci`
- [x] 记录 before/after retained-byte/clone evidence 到 task research。

## Rollback points

- Arc conversion独立可回滚，不改变行为。
- Central cache limits独立于 preview registry。
- Preview state machine 与所有 remote call sites作为一个原子阶段；不要恢复会丢 foreign-target ownership 的旧 cleanup。
