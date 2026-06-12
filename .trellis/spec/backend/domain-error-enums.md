# 域错误枚举约定（thiserror）

> 来源：06-11-analysis-driven-fixes 父任务 C1/C2 批次落地（installation、scanner、central_skills、github_import、projects、marketplace、local_remote_sync 七域已迁移）。C3 将处理剩余域 + db/repos 透传。

## 1. Scope / Trigger

- 新增或修改 `services/<domain>` 下任何返回错误的函数 → 必须用该域的 thiserror 枚举，禁止新增 `Result<T, String>`。
- 新建服务域 → 先建 `services/<domain>/error.rs`，参考样板：`services/projects/error.rs`、`services/installation/error.rs`。

## 2. Signatures（分层错误契约）

```
db/repos/*        →  Result<T, String>（现状；C3 改 sqlx::Error）
services/<domain> →  Result<T, <Domain>Error>（thiserror 枚举，一域一枚举）
commands/*        →  Result<T, String>（IPC 边界，唯一允许字符串错误的层）
```

错误枚举骨架（`services/<domain>/error.rs`，`mod.rs` 中 `mod error; pub use error::XxxError;`）：

```rust
#[derive(Debug, thiserror::Error)]
pub enum XxxError {
    /// 带操作上下文的 IO 失败；配 io() 构造助手。
    #[error("{context}: {source}")]
    Io { context: String, #[source] source: std::io::Error },

    /// 直接 sqlx 调用（非 repos）透传。
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// targets 远程传输层（返回 String）在调用点 map_err 包装。
    #[error("{0}")]
    Remote(String),

    /// 语义化变体：每个独立失败模式一个变体，调用方 matches! 分支。
    #[error("Skill '{0}' not found in central library")]
    SkillNotFoundInCentral(String),

    /// spawn_blocking join 失败，配 task_join() 助手（见 spawn-blocking-io.md）。
    #[error("Failed to join {label} task: {message}")]
    TaskJoin { label: &'static str, message: String },

    // TODO(C3): repos 改造后删除。
    /// 仅用于 db/repos 字符串错误的临时适配。
    #[error("{0}")]
    Other(String),
}
```

含 reqwest 的域（github_import、marketplace）增补四个 HTTP 变体，`reqwest::Error` 不做 `#[from]`（缺操作上下文），调用点 map：

```rust
#[error("{0}")] Http(String),         // 传输/协议/非 2xx/镜像回退汇总
#[error("{0}")] RateLimited(String),  // 429 / x-ratelimit 分类命中
#[error("{0}")] AccessDenied(String), // 401/403 非限流
#[error("{0}")] Parse(String),        // JSON/UTF-8 解析失败，不复用 Http
```

## 3. Contracts

- **错误文本逐字保留现状英文**：`#[error(...)]` 格式串照抄原 `format!` 字符串——前端 toast 直接展示 Display 输出，文案漂移即 UI 回归。
- IPC 载荷结构体中的 `error: String` 字段（如 `FailedInstall`、`FailedCentralSkillDelete`）保持 String，构造处 `error: e.to_string()`。
- 跨域传播：已迁移域之间用 `#[error(transparent)] Xxx(#[from] XxxError)`（如 `ProjectsError::Installation`、marketplace 对 `GithubImportError`）；被未迁移域调用时调用点 `.map_err(|e| e.to_string())` 过渡。

## 4. Validation & Error Matrix

| 情形                 | 处理                                                                      |
| -------------------- | ------------------------------------------------------------------------- |
| db/repos 返回 String | `.map_err(XxxError::Other)` + 行尾 `// TODO(C3): typed repos passthrough` |
| 直接 sqlx 调用       | `#[from] sqlx::Error` 透传（`?`）                                         |
| reqwest 失败         | 按类别 map 到 `Http`/`RateLimited`/`AccessDenied`/`Parse`，禁止 `#[from]` |
| resource_budget 违规 | `.map_err(XxxError::Budget)`（`#[error("{0}")] Budget(String)`）          |
| spawn_blocking join  | `run_blocking_fs_with(label, task, XxxError::task_join)`                  |
| 调用方需区分错误类别 | 加语义化变体 + `matches!`，禁止 `error.contains("...")` 字符串判断        |

## 5. Good/Base/Bad Cases

- **Good**：`matches!(e, ScannerError::Timeout(_))` 分支处理；新失败路径加专属变体并保留原文案。
- **Base**：repos 调用 `.map_err(XxxError::Other)` 带 TODO(C3) 标记——可接受的过渡态。
- **Bad**：`Other(String)` 用于 repos 适配之外的新分支；新函数返回 `Result<T, String>`；改动 `#[error(...)]` 文案；commands 层之外出现字符串错误。

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
    repo_call(pool).await.map_err(XxxError::Other)?; // TODO(C3): typed repos passthrough
    if bad { return Err(XxxError::Timeout(secs)); }
    Ok(())
}
// 调用方按变体分支
if matches!(err, XxxError::Timeout(_)) { retry(); }
// IPC 边界（commands/*.rs）唯一字符串化点
do_thing_impl(&state.db).await.map_err(|e| e.to_string())
```
