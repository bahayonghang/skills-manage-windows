# 域错误枚举约定（thiserror）

> 来源：06-11-analysis-driven-fixes 父任务 C1/C2/C3 批次落地；07-04-central-updates-service-domain 补齐 Update Center 两域。全部服务域已迁移（installation、scanner、central_skills、github_import、projects、marketplace、local_remote_sync、usage、obsidian、ai_provider、ai_tagging、portable_state、central_store_location、central_updates），db/repos 透传统一，散点模块（targets、logging、central_migration、operation_log、resource_budget、paths、fs_util 等）同步收口。

## 1. Scope / Trigger

- 新增或修改 `services/<domain>` 下任何返回错误的函数 → 必须用该域的 thiserror 枚举，禁止新增 `Result<T, String>`。
- 新建服务域 → 先建 `services/<domain>/error.rs`，参考样板：`services/projects/error.rs`、`services/installation/error.rs`。

## 2. Signatures（分层错误契约）

```
db/repos/*        →  Result<T, sqlx::Error>（直接透传；repos 内非 sqlx 的业务校验/防御错误用 sqlx::Error::InvalidArgument(消息) 承载，Display 为 "{0}" 保文案逐字不变）
services/<domain> →  Result<T, <Domain>Error>（thiserror 枚举，一域一枚举）
commands/*        →  IpcResult<T> = Result<T, IpcError>（Tauri IPC 边界；内部 impl/helper 可暂用 String，但不得直接成为 command 返回类型）
```

错误枚举骨架（`services/<domain>/error.rs`，`mod.rs` 中 `mod error; pub use error::XxxError;`）：

```rust
#[derive(Debug, thiserror::Error)]
pub enum XxxError {
    /// 带操作上下文的 IO 失败；配 io() 构造助手。
    #[error("{context}: {source}")]
    Io { context: String, #[source] source: std::io::Error },

    /// db/repos 调用 + 直接 sqlx 调用透传。
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// targets 远程传输层（TargetsError）在调用点 to_string 后包装。
    #[error("{0}")]
    Remote(String),

    /// 语义化变体：每个独立失败模式一个变体，调用方 matches! 分支。
    #[error("Skill '{0}' not found in central library")]
    SkillNotFoundInCentral(String),

    /// spawn_blocking join 失败，配 task_join() 助手（见 spawn-blocking-io.md）。
    #[error("Failed to join {label} task: {message}")]
    TaskJoin { label: &'static str, message: String },
}
```

含 reqwest 的域（github_import、marketplace）增补四个 HTTP 变体，`reqwest::Error` 不做 `#[from]`（缺操作上下文），调用点 map：

```rust
#[error("{0}")] Http(String),         // 传输/协议/非 2xx/镜像回退汇总
#[error("{0}")] RateLimited(String),  // 429 / x-ratelimit 分类命中
#[error("{0}")] AccessDenied(String), // 401/403 非限流，未使用认证
#[error("{0}")] ConfiguredTokenAccessDenied(String), // 401/403 且请求已使用认证
#[error("{0}")] Parse(String),        // JSON/UTF-8 解析失败，不复用 Http
```

## 3. Contracts

- **错误文本逐字保留现状英文**：`#[error(...)]` 格式串照抄原 `format!` 字符串——前端 toast 直接展示 Display 输出，文案漂移即 UI 回归。
- targets process supervision 的 timeout/cancel/output-limit/termination-failure 使用独立 `TargetsError` typed variants；Start/WriteStdin/Wait IO 继续保留旧 Display 前缀。监督错误字段只含 transport、policy/stream/limit/trigger，不得包含 command、env 或捕获输出。
- IPC 载荷结构体中的 `error: String` 字段（如 `FailedInstall`、`FailedCentralSkillDelete`）保持 String，构造处 `error: e.to_string()`。
- 跨域传播：服务域之间用 `#[error(transparent)] Xxx(#[from] XxxError)`（如 `ProjectsError::Installation`、marketplace 对 `GithubImportError`）；targets 传输层错误（`TargetsError`）在调用点 `.to_string()` 包入本域 `Remote(String)` 变体。

## 4. Validation & Error Matrix

| 情形                 | 处理                                                                                                |
| -------------------- | --------------------------------------------------------------------------------------------------- |
| db/repos 调用        | `#[from] sqlx::Error` 的 `Db` 变体透传（`?`）；commands 边界映射为稳定 `IpcError`  |
| 直接 sqlx 调用       | `#[from] sqlx::Error` 透传（`?`）                                                                   |
| repos 内业务校验失败 | `sqlx::Error::InvalidArgument(原消息)`（Display "{0}"，文案逐字保留）                               |
| reqwest 失败         | 按类别 map 到 `Http`/`RateLimited`/`AccessDenied`/`Parse`，禁止 `#[from]`                           |
| GitHub 401/403       | 保留 typed `used_auth`；匿名与 configured-token failure 使用不同稳定 code，禁止解析 Display       |
| resource_budget 违规 | `.map_err(XxxError::Budget)`（`#[error("{0}")] Budget(BudgetExceeded)`，typed struct 文案逐字保留） |
| spawn_blocking join  | `run_blocking_fs_with(label, task, XxxError::task_join)`                                            |
| 调用方需区分错误类别 | 加语义化变体 + `matches!`，禁止 `error.contains("...")` 字符串判断                                  |

## 5. Good/Base/Bad Cases

- **Good**：`matches!(e, ScannerError::Timeout(_))` 分支处理；新失败路径加专属变体并保留原文案。
- **Base**：command 内部 impl/helper 可保留 `Result<T, String>` 作为迁移期实现细节，但 `#[tauri::command]` 必须返回 `IpcResult<T>`；targets 错误经 `Remote(String)` 携带 Display 文案跨入服务域。
- **Bad**：新增 `Other(String)` 兜底变体；新增服务函数返回 `Result<T, String>`；任何 fallible `#[tauri::command]` 返回字符串错误；改动 `#[error(...)]` 文案。

## 6. Tests Required

- 错误断言用 `err.to_string().contains(...)` 或 `matches!`，不得依赖错误类型为 String。
- 迁移/改动错误类型时不得删除或弱化既有用例；断言调整逐条在 PR 列明。
- 验收 grep（排除 tests 与 commands/）：`grep -rn "Result<.*, String>" src-tauri/src/services/<domain> | grep -v tests` 应 0 命中。

## 7. Wrong vs Correct

### Wrong

```rust
// services 层新函数返回 String
pub async fn do_thing(pool: &DbPool) -> Result<(), String> {
    repo_call(pool).await.map_err(|e| format!("Failed: {e}"))?;
    if bad { return Err("Operation timed out".to_string()); }
    Ok(())
}
// 调用方靠字符串嗅探分支
if err.contains("timed out") { retry(); }
```

### Correct

```rust
pub async fn do_thing(pool: &DbPool) -> Result<(), XxxError> {
    repo_call(pool).await?; // repos 返回 sqlx::Error，经 Db(#[from]) 透传
    if bad { return Err(XxxError::Timeout(secs)); }
    Ok(())
}
// 调用方按变体分支
if matches!(err, XxxError::Timeout(_)) { retry(); }
// IPC 边界只暴露稳定对象；原始 Display 不得直接跨边界。
do_thing_impl(&state.db)
    .await
    .map_err(|_| IpcError::new("storage.unavailable", "Storage is unavailable.", false))
```

## Scenario: Structured Tauri IPC Error Boundary

### 1. Scope / Trigger

- 新增或修改 `#[tauri::command]`、IPC 错误映射、command registry 或 Specta 类型时适用。

### 2. Signatures

```rust
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub correlation_id: Option<String>,
}
pub type IpcResult<T> = Result<T, IpcError>;
```

runtime command、fallible boundary 和例外项的当前数量由
`src/test/contracts/ipcCommandCoverage.test.ts` 从 registry 权威源计算，不在本规范复制快照。

### 3. Contracts

- `code` 是 locale-neutral 的小写点分标识；`retryable` 默认 `false`。
- `correlation_id` 是可选 UUID；命名 IPC boundary 在 Runtime/Operation evidence 已建立时返回同一 ID。
- 仅当 mapper 能证明 mutation 前失败且重试安全时设 `retryable=true`。
- 已审查的域错误映射为固定 code/message；未知 Display 只得到
  `internal.unexpected` 固定摘要，不能把原文复制进 payload。
- `GithubImportError::ArchiveRedirectRejected` 是零动态字段的已审查变体；
  `CentralUpdatesError::GithubImport` 透明保留其 envelope，并固定映射为
  `github_import.archive_redirect_rejected`、`retryable=false`。禁止通过错误
  文本或 code 字符串嗅探判断该变体。
- GitHub 网络族（传输、限流、拒绝访问、仓库不存在、archive 不可用、响应不可解析、
  地址不合法、预算超限、凭据不可读）同样是已审查变体，各自映射固定
  `github_import.*` code。`ipc_error_code` 返回完全限定 `&'static str`，`ipc_code`
  由它裁剪而来，二者不可能不一致。新增变体时只改这一张表。
- payload 内部业务字段（例如 `FailedInstall.error`、`FailedRepository.error`）不属于
  command rejection，保持原契约；但当该字段来自已分类的域错误时，必须写稳定
  `error_code` 与经审阅文案，而不是域错误的 Display。

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| validation/conflict/credential missing | stable code, `retryable=false` |
| cancellation | `operation.cancelled`, `retryable=false` |
| proven pre-mutation rate limit | stable rate-limit code, `retryable=true` |
| path/credential/command/output in unknown error | `internal.unexpected`; raw text absent |
| trusted archive redirect validation fails | `github_import.archive_redirect_rejected`; fixed public message; `retryable=false` |
| fallible command returns `Result<_, String>` | contract test fails |

### 5. Good / Base / Bad Cases

- Good: command explicitly maps a typed domain variant to a stable public `IpcError`.
- Base: legacy internal helper reaches `ipc_boundary!` and unknown text fails closed.
- Bad: `.map_err(|e| e.to_string())` is returned directly from a Tauri command.

### 6. Tests Required

- `ipc_error` serialization asserts `code/message/retryable` and optional camel-case `correlationId`; absent remains
  backward-compatible and invalid dynamic values are rejected.
- Seed PAT, AI key, SSH password, absolute/relative paths, command/output and file content; none may survive serialization.
- Archive redirect tests seed PAT, URL, repository/file path, and response text;
  the serialized payload retains only its fixed code/message/retryable fields.
- IPC coverage derives and asserts runtime/fallible/infallible membership and zero raw string command boundaries.

### 7. Wrong vs Correct

```rust
// Wrong
#[tauri::command]
async fn save() -> Result<(), String> { service().await.map_err(|e| e.to_string()) }

// Correct
#[tauri::command]
async fn save() -> IpcResult<()> {
    service().await.map_err(|_| IpcError::new("storage.unavailable", "Storage is unavailable.", false))
}
```
