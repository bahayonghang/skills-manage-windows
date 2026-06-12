# Design：深度分析驱动的优化修复（父任务级技术设计）

本文档持有跨子任务共享的技术设计。子任务执行时以此为准；落地中发现偏差，先修订本文档再继续（禁止批内自创第二种模式）。

## 1. 错误处理架构（C1–C3 共享模板，已按 C1 实际落地修订）

### 1.1 目标形态

```
db/repos/*        →  Result<T, sqlx::Error>          （C3 统一改造；之前批次用 map_err 适配）
services/<domain> →  Result<T, <Domain>Error>         （thiserror 枚举，12 个域各一个）
commands/*        →  Result<T, String>                （IPC 边界，唯一允许字符串错误的层）
```

### 1.2 域错误枚举模式（C1 已验证样板：`services/scanner/error.rs`、`services/installation/error.rs`）

```rust
// services/<domain>/error.rs，mod.rs 中 `mod error; pub use error::XxxError;`
#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    /// 直接 sqlx 调用（非 repos）经 #[from] 透传。
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// 远程传输层失败（targets 模块返回 String，调用点 map_err 包装）。
    #[error("{0}")]
    Remote(String),

    /// 语义化变体：调用方 matches!(e, ScannerError::Timeout(_)) 分支处理。
    #[error("Remote skill scan timed out after {0}s.")]
    Timeout(u64),

    /// spawn_blocking join 失败（配合 1.5 节的 run_blocking_fs_with）。
    #[error("Failed to join {label} task: {message}")]
    TaskJoin { label: &'static str, message: String },

    // TODO(C3): repos 改造后删除此变体。
    /// 仅用于 db/repos 字符串错误的临时适配，调用点必须带 // TODO(C3) 标注。
    #[error("{0}")]
    Other(String),
}
```

要点（C1 落地经验，C2/C3 直接复制）：

- **错误文本逐字保留现状英文**（前端 toast 直接展示），不要套用示意性的中文模板。变体的 `#[error(...)]` 格式串照抄原 `format!` 字符串。
- 变体语义化，让调用方可以 match 分支处理（消灭 `error.contains(...)` 判断）。C1 消灭了两处：`commands/scanner.rs` 的 `contains("timed out")` → `matches!(e, ScannerError::Timeout(_))`；`native.rs::should_fallback_to_copy` 的 `contains("Failed to create symlink")` → `matches!(e, InstallationError::SymlinkCreate(_))`。
- 带操作上下文的 IO 错误用统一的 `Io { context: String, source: std::io::Error }` 变体 + `#[error("{context}: {source}")]`，context 为原消息的前缀部分（如 `"Failed to copy 'a' -> 'b'"`），配 `InstallationError::io(context, source)` 构造助手。`#[from]` 仅用于无需附加上下文的透传（如直接 sqlx 调用）。
- db/repos 仍返回 String：调用点统一 `.map_err(XxxError::Other)` 并在行尾标 `// TODO(C3): typed repos passthrough`，C3 扫尾按此标记定位。`Other(String)` 不得用于 repos 适配之外的新分支。
- IPC 载荷结构体里的 `error: String` 字段（如 `FailedInstall`、`CentralBatchInstallFailure`）保持 String，构造处 `error: e.to_string()`。

#### Http 变体约定（C2 增补，适用于含 reqwest 的域：github_import、marketplace 等）

reqwest 失败按类别拆为四个变体，`reqwest::Error` 不做 `#[from]` 透传（缺操作上下文），统一在调用点 map：

```rust
/// HTTP 传输/协议失败（连接、超时、非 2xx 状态、镜像回退汇总）。
/// 消息在调用点预格式化，逐字保留原 format! 文案。
#[error("{0}")]
Http(String),

/// GitHub 限流拒绝（429 / x-ratelimit 分类命中）。消息为 denial 分类器的
/// Display 输出；语义化变体，调用方可 matches! 区分限流走重试/镜像分支。
#[error("{0}")]
RateLimited(String),

/// 认证/权限拒绝（401/403 非限流分类）。消息同样取 denial Display 输出。
#[error("{0}")]
AccessDenied(String),

/// 响应体/归档解析失败（JSON 解码、UTF-8 校验等），不复用 Http。
#[error("{0}")]
Parse(String),
```

资源预算（`resource_budget` 模块仍返回 String）违规用 `#[error("{0}")] Budget(String)` 在调用点 `.map_err(XxxError::Budget)` 包装，C3 改造 resource_budget 时一并收紧。

### 1.3 IPC 边界转换

commands 壳层统一：

```rust
#[tauri::command]
pub async fn scan_all_skills(...) -> Result<ScanResult, String> {
    scan_all_skills_impl(&pool).await.map_err(|e| e.to_string())
}
```

操作日志等需要先检查错误的场景，在 match 后立即转换：`let result = result.map_err(|e| e.to_string());`。

不引入结构化 IPC 错误对象（前端契约零改动）；如未来需要错误码，另行立项。

### 1.4 跨域调用点的过渡适配

已改造域的 pub 函数（如 `copy_dir_all`、`create_symlink`、`project_relative_skills_dir`）被未改造域调用时，调用点加 `.map_err(|e| e.to_string())` 维持原 String 流；该域轮到自己批次时改回 `?` 透传或 map 到自己的域错误。C1 已适配的调用方：`central_migration`（format! 直接 Display 无需改）、`commands/central_store_location`、`commands/central_updates_fs`、`commands/skill_update_inventory/apply_steps`、`commands/collections`、`commands/local_remote_sync`、`services/projects/crud`、`services/obsidian/import`。

### 1.5 spawn_blocking 与域错误的衔接

`crate::fs_util` 提供两个包装：

```rust
run_blocking_fs_with(label, task, join_error)  // 泛型 E，typed 域用：join_error = XxxError::task_join
run_blocking_fs(label, task)                   // Result<T, String>，未改造域沿用
```

typed 域在自己的 fs_util/适当模块定义薄包装固定 E（见 `services/installation/fs_util.rs::run_blocking_fs`），避免调用点类型标注。join 失败映射到域错误的 `TaskJoin { label, message }` 变体，消息格式与原字符串版一致（`"Failed to join {label} task: {message}"`）。

### 1.6 批次划分与依赖

| 批次 | 域 | 理由 |
|------|----|------|
| C1（已完成） | 基建 + installation、scanner | 测试最厚（2086/1452 行），验证模板；顺带消除 scanner.rs:100 字符串判断 |
| C2 | central_skills、github_import、projects、marketplace、local_remote_sync | 业务核心域，按模板机械推广 |
| C3 | usage、obsidian、ai_provider、ai_tagging、portable_state + db/repos 透传 + 散点（central_migration、operation_log、logging、secrets、targets、bootstrap） | 尾批 + 全局扫尾 |

C2、C3 开始前必须确认上一批已归档且 `just ci` 绿。

C1 核对记录：design 预期的 `ScannerError::Io/Parse` 变体未落地——scanner 对单文件解析/IO 失败的策略是静默跳过（`Option`/空列表），不存在对应错误路径；`InstallationError` 实际按真实失败路径展开为 30+ 语义变体（守卫类、占位冲突类、Claude 行级卸载类等），均保留原文案。

## 2. spawn_blocking 架构（A）

- 将 `services/installation/fs_util.rs` 的 `spawn_blocking` 包装提升为跨域共享（建议位置：`src-tauri/src/fs_util.rs`，installation 原路径 re-export 保持兼容）。
- 判定标准：**递归遍历/拷贝/删除、批量落盘、目录搬迁必须包装**；单文件小读写（<1 个目录层级、无循环）可豁免但需记录评估。
- A 在 C 之前执行：包装改造只动函数体，不动签名；C 动签名。顺序错误会导致两批 diff 在同一函数上叠加。

## 3. 兼容性与回滚

- 全程 IPC 契约不变（命令名、参数、返回结构、错误为字符串），前端零改动（除 D 中的 Sidebar selector 与遮罩 token）。
- 每个子任务独立 commit/归档，单批出问题 revert 该批 commit 即可，不连坐。
- 测试基线：709 Rust + 1214 前端用例是回归安全网；错误断言文本如需调整，逐条在 PR 中列明。

## 4. 验证命令（所有子任务通用）

```bash
just ci                                  # 完整门禁
cd src-tauri && cargo test               # Rust 测试
cd src-tauri && cargo clippy -- -D warnings
pnpm typecheck && pnpm lint && pnpm test # 前端链
```

扫尾专用（C3）：

```bash
# 排除 tests 后应仅剩 commands/ 边界签名
grep -rn "Result<.*, String>" src-tauri/src --include="*.rs" | grep -v tests
```
