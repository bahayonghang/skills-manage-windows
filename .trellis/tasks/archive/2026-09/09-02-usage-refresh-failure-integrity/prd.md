# Usage 远程刷新失败数据保全

## Goal

让 SSH/WSL usage transport、protocol 或 permission failure 保持为 target-scoped 错误，在采集完整性未成立时禁止替换该 target 的缓存；真实空目录/零条目仍是可提交的成功结果。

## Confirmed Evidence

- `BE-CORR-004`（High / S）：`src-tauri/src/services/usage/fs_backend.rs:240-255` 将远程 `walk_jsonl`/`list_entries` 错误转为空结果；同文件 `RemoteFsBackend::exists` 也把 transport error 转为 `false`。
- `src-tauri/src/services/usage/mod.rs:301-323` 将 provider collect 错误转为 unavailable + empty calls；`src-tauri/src/services/usage/mod.rs:374-382` 仍调用 `src-tauri/src/db/repos/usage_repo.rs:121-143::replace_calls_for_target`，事务性清空并替换该 target 的旧 usage rows。

## Requirements

- R1：**远程 IO 三态。** `FsBackend::exists` 必须返回 `Result<bool, UsageError>`；Remote `walk_jsonl`、`list_entries`、`exists` 以及 read/fetch 的 transport/protocol/permission failure 必须为 `Err`。只有经 transport 成功确认的 missing directory 或零条目可返回 `Ok(false)`/`Ok([])`。
- R2：**Target commit gate。** `refresh_with_providers` 在任一 target-fatal remote error 时必须在 enrichment 和 `replace_calls_for_target` 前返回 `Err`；该 target 的 calls、provider health、usage metadata、scan state 与 file cache 均保持刷新前状态。合法 `Collected([])` 可提交清空。
- R3：**Provider 与 target 分类。** 本地单 provider 的可容错 parse/source failure 可继续沿既有 provider-unavailable 语义；`UsageError` 必须提供稳定 `code`、`retryable` 与 bounded/redacted public message，明确区分 target-fatal remote failure，不能靠 message 字符串判断。
- R4：**Target 隔离。** 一个 target 的失败不得读取、删除、替换、更新时间戳或隐藏另一个 target 的 rows/result；成功 target 仍可独立提交。当前 refresh API 仍是单 target，不新增 multi-target coordinator。
- R5：**日志与 IPC。** warning/IPC 只包含 target-safe identifier、provider ID、stable code 和 retryable；不得包含 remote path、command、raw stderr/stdout、credential、host diagnostic 或底层 `TargetsError::to_string()`。
- R6：**简单性与兼容。** 不新增 usage schema、重试队列、离线编辑、stale-data flag 或自动连接；保持公开 `RefreshSummary`/overview/provider-health DTO 形状，失败继续经现有 command error boundary 返回。

## Acceptance Criteria

- [x] AC1（R1）：Fake SSH/Fake WSL 的 `exists` 可区分 `Ok(true)`、确认 missing 的 `Ok(false)` 与 transport/protocol/permission `Err`。
- [x] AC2（R1）：Fake SSH/Fake WSL 的 `walk_jsonl`/`list_entries` 可区分合法 `Ok([])` 与 transport/protocol/permission `Err`。
- [x] AC3（R2）：预置 calls/provider health/metadata/scan state/file cache 后注入 remote fatal error，上述 rows 逐表完全不变，且 `replace_calls_for_target` 未调用。
- [x] AC4（R2）：合法空目录/零条目刷新成功，允许事务清空 calls 并更新 provider health/scan state。
- [x] AC5（R3）：本地可容错 provider parse/source failure 保持既有 unavailable 行为，不被误分类为 target-fatal。
- [x] AC6（R3）：Remote fatal error 的 stable code、retryable 与 public message 不依赖原始 message 文本且跨 fixture 稳定。
- [x] AC7（R5）：捕获的 tracing、IPC error 与 operation log 不含 path、command、stderr/stdout、host/credential diagnostic。
- [x] AC8（R4）：Local/SSH/WSL 多 target rows 预置后，一个 target 失败时其他 target byte-for-byte 不变。
- [x] AC9（R4）：另一个 target 的独立成功刷新仍返回其真实 summary，不被先前失败覆盖。
- [x] AC10（R6）：无 migration/schema/新后台队列/DTO 变化；现有 overview、recent、provider health、incremental local cache tests 保持通过。
- [x] AC11（R1, R2, R3, R4, R5, R6）：Usage 定向 Rust tests、fmt、locked all-target Clippy/tests、默认并发 `just ci` 与独立 review 通过。
- [x] AC12（R5）：未运行真实 SSH/WSL permission/断连 smoke 时逐项标记 `UNVERIFIED`。

## Out of Scope

- 新增 usage schema、自动重试、跨 target 批量刷新 coordinator 或把 stale cache 标为“已成功刷新”。
- 改变 usage parsing/aggregation 算法、TTL、UI 数据模型或本地增量缓存策略。
