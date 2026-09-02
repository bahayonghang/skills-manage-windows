# Design: Usage target commit gate

## Change List / Symbols

1. `src-tauri/src/services/usage/fs_backend.rs::FsBackend`：把 `exists` 改为 `Result<bool, UsageError>`；`LocalFsBackend` 将 `NotFound` 映射 `Ok(false)`，其他 IO error 保留；`RemoteFsBackend::{exists,walk_jsonl,list_entries,read_to_string,read_many_to_strings,fetch_to_local}` 使用统一 typed constructor，不再 `unwrap_or(false/default)`。[R1][R3]
2. `src-tauri/src/services/usage/mod.rs::UsageProvider::available` 及 `providers/{claude_code,codex,droid,grok,opencode,antigravity,kiro,zed}.rs`：availability 改为 `Result<bool, UsageError>`，真实 missing 为 `Ok(false)`，远程 probe error 上抛。[R1]
3. `src-tauri/src/services/usage/error.rs::UsageError`：为 target-fatal remote transport 保留内部 source，但公开 `Display`/IPC 使用固定脱敏 message；新增 `stable_code()`、`retryable()`、`is_target_fatal()`，禁止 message parsing。[R3][R5]
4. `src-tauri/src/services/usage/mod.rs::refresh_with_providers`：收集 provider outcome 后先检查 target-fatal error；存在时直接返回，绝不执行 enrichment、file-cache mutation 或 `db::replace_calls_for_target`。本地可容错 provider error 仍生成 unavailable outcome。[R2][R3][R6]
5. `src-tauri/src/services/usage/tests.rs`、`fs_backend.rs` tests、provider tests 与 `src-tauri/src/db/repos/usage_repo.rs` tests：增加三态、逐表保全、target isolation、empty success 与 redaction fixtures。[R1-R5]

## Contract

```text
available/collect per provider
  -> Success(calls, including [])
  -> SourceUnavailable (confirmed absent; non-fatal)
  -> ProviderFailure (existing local-tolerable class)
  -> TargetFatalRemote(code, retryable, redacted) -- abort target refresh

all providers complete without TargetFatalRemote
  -> enrich
  -> replace_calls_for_target(one SQLite transaction)
  -> RefreshSummary

any TargetFatalRemote
  -> Err before enrichment/replace
  -> old target rows and scan timestamp remain authoritative
```

`Ok([])` is data; `Err(TargetFatalRemote)` is lack of evidence. Only the former authorizes cache replacement.[R1-R3]

## Error / Logging Contract

底层 `TargetsError` 仅作为 internal source 保留，不用其 Display 形成 IPC/log。对外字段固定为 usage domain code（例如 transport unavailable/protocol/permission 的稳定枚举映射）、retryable 和通用 message；tracing 可记录 provider/target ID/code，不记录 path/command/raw stream/host detail。[R3][R5]

## Compatibility

`RefreshSummary`、overview、recent usage、provider health DTO、SQLite schema 与 target-scoped transaction 不变。Local missing semantics 不变；仅把以前伪装为空/不可用的 remote IO failure 恢复为 error。Stub provider 继续 `Ok(false)`。[R6]

## Verification Boundary

Fake SSH/WSL 可证明 typed propagation、commit gate、row preservation 和 redaction；真实远端 permission、network drop、shell/protocol 差异不能由 FakeRunner 证明。未执行的真实 SSH/WSL smoke 必须标 `UNVERIFIED`。[R5][R6]

## Rollback

- RP1：先落 `UsageError`/trait signature 与 compile-only provider migration，不改 commit gate；整批可回退。
- RP2：commit gate + DB preservation tests 是独立单元；失败只回退 aggregator，不改 backend propagation。
- RP3：redaction/log tests 独立；不得通过恢复 raw error 输出修复诊断。

## Considered but Not Chosen

- 不在 `RemoteFsBackend` 内保存隐藏的“曾失败”全局 flag：会制造跨 provider 可变状态和竞态。
- 不把所有 provider error 都升级为 target-fatal：本任务只阻止远程 transport/protocol/permission 缺证据覆盖缓存，保留既有本地容错语义。
- 不新增 stale marker/schema/retry queue：旧 rows 未变已经表达最后一次成功状态。
