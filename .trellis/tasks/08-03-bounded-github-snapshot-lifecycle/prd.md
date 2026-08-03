# 约束 GitHub 快照缓存与远端工作区生命周期

## Goal

让 Central update snapshot cache 与 renderer-driven GitHub preview registry 同时具备硬容量边界、零深拷贝共享和正确的 storage ownership。过期、淘汰、discard、import success/failure 与 cleanup failure 都必须有可验证的终态，不能因为清理当前 target 而遗失其他 target 的远端 workspace 引用。

## Evidence

- `central_updates/snapshots.rs:59-107` 是无 entry/byte 上限的 `HashMap`，expired entry 只 miss 不删除。
- `snapshots.rs:71-84,281-284` 深 clone `GitHubRepoSnapshot`；其 `files` 是 `HashMap<String, Vec<u8>>`。
- `github_import/snapshot_registry.rs:20-61` 是 process-global、无容量上限的 registry；lookup expired 仍保留 entry。
- `github_import/remote.rs:4-16` 全局 prune 后只删除当前 target workspace，其他 target 的 storage ownership 被丢弃。
- 活动 preview spec 要求 active import lease 不被 prune，失败释放 lease 供 retry，discard during lease 延迟到 release；这些语义必须保持。

## Requirements

1. Central update cache 用 `Arc<GitHubRepoSnapshot>` 或等价共享 ownership；cache hit、insert 和本次 request result 不复制 `Vec<u8>` payload。
2. Central cache 同时限制 ready entries 与 aggregate retained bytes。初始生产 policy：最多 8 个 repository snapshot、aggregate bytes 最多 256 MiB；单项仍受现有 `ResourceBudget`。limits 以 injectable/testable policy 表达，不散落 magic numbers。
3. Central cache 在 read/insert 时回收 expired entries，并按 deterministic LRU/oldest-ready 策略淘汰；大于 aggregate limit 的单项不缓存但可供当前 request 使用，不能破坏更新功能。
4. Preview registry 按 target 限制 ready entries/storage：每 target 最多 4 个 ready preview；Local retained bytes 每 target最多 256 MiB。active import lease 不计入可淘汰集合但仍计入 observability。
5. 设全局 registry entry safety cap（初始 64）。达到 cap 且没有可安全回收的 owning-target entry 时，拒绝新 preview并返回 typed capacity error；不得丢弃已有 ownership 来“腾空间”。
6. prune/eviction 只能把 snapshot 转入 owning target 的 cleanup 流程。当前 connection 不得删除其他 target 的 registry entry；remote `remove_tree` 成功后才彻底释放 ownership。
7. Remote cleanup 失败进入 `cleanup_pending` 可重试状态，lookup/import 对该 token fail closed；下一次连接 owning target、explicit discard 或 target lifecycle cleanup 时重试。新 workspace 注册失败时立即清理它，清理失败也必须保留可重试 ownership。
8. 保持 snapshot binding、digest、TTL、single import lease、retry-after-import-failure、consume-on-success 和 deferred discard 契约。
9. 结构化诊断可记录 count/bytes/reason/target kind，禁止记录 preview id、workspace path、repo URL、digest 或凭据。
10. 更新 `github-import-preview-contract.md` 中关于容量、过期清理和 cleanup pending 的终态说明。

## Acceptance Criteria

- [x] pointer/Arc identity 测试证明 cache hit 与 request result共享同一 snapshot payload，未深 clone file bytes。
- [x] Central cache 的 entry、byte、TTL、LRU、oversized-not-cached 和 checked-size-overflow 测试全部通过。
- [x] Preview registry 的 per-target ready cap、Local byte cap、global cap、active lease、deferred discard 和 capacity rejection 测试全部通过。
- [x] 两个 remote target A/B：在 A connection 上清理时，B 的 expired workspace 不从 registry 消失；连接 B 后才由 B adapter 删除。
- [x] `remove_tree` 故障注入后 snapshot 进入 cleanup-pending 且不可 lookup/import；重试成功后 entry 与 workspace ownership 一起释放。
- [x] import failure 仍释放 lease 供 retry；import success consume；cleanup/eviction 不改变 digest/binding/provenance。
- [x] 所有容量路径有 deterministic test policy，测试不依赖 process-global wall clock 或其他并行测试的 registry entry。
- [x] focused snapshot tests、Rust fmt、all-targets locked Clippy、locked Rust tests和 `just ci` 通过。

## Non-Goals

- 不改变 GitHub archive/tree 单项文件预算或 acquisition fallback；归 bounded ingestion/既有 GitHub import contract。
- 不持久化可用于重新 import 的 preview token；registry 仍为 session-scoped。
- 不引入第三方 cache/LRU crate，除非实现证明标准库结构无法满足并取得单独依赖批准。

## Dependency

可独立于 Marketplace P0 规划。若与 bounded ingestion 同期实施，snapshot type/`ResourceBudget` 的修改必须串行集成以避免同文件冲突。
