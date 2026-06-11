# Design：深度分析驱动的优化修复（父任务级技术设计）

本文档持有跨子任务共享的技术设计。子任务执行时以此为准；落地中发现偏差，先修订本文档再继续（禁止批内自创第二种模式）。

## 1. 错误处理架构（C1–C3 共享模板）

### 1.1 目标形态

```
db/repos/*        →  Result<T, sqlx::Error>          （C3 统一改造；之前批次用 map_err 适配）
services/<domain> →  Result<T, <Domain>Error>         （thiserror 枚举，12 个域各一个）
commands/*        →  Result<T, String>                （IPC 边界，唯一允许字符串错误的层）
```

### 1.2 域错误枚举模式

```rust
// services/<domain>/error.rs（或 mod.rs 内）
#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("IO 失败 ({path}): {source}")]
    Io { path: String, #[source] source: std::io::Error },

    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error("远程扫描超时（{0}s）")]
    Timeout(u64),

    #[error("SKILL.md 解析失败 ({path}): {reason}")]
    Parse { path: String, reason: String },
    // 变体按域内真实失败类别定义，禁止 Catch-all 的 Other(String) 兜底成为主通道
}
```

要点：

- 变体语义化，让调用方可以 match 分支处理（消灭 `error.contains(...)` 判断）。
- `#[from]` 仅用于无需附加上下文的透传（如 sqlx::Error）；需要路径等上下文的 IO 错误用显式构造。
- 错误消息文本保持与现状语义等价——前端 toast 直接展示该文本，不允许用户可见信息回退。
- 允许保留一个 `Other(String)` 变体用于个别难以归类的遗留分支，但不得作为主要通道（review 时检查占比）。

### 1.3 IPC 边界转换

commands 壳层统一：

```rust
#[tauri::command]
pub async fn scan_all_skills(...) -> Result<ScanResult, String> {
    scan_all_skills_impl(&pool).await.map_err(|e| e.to_string())
}
```

不引入结构化 IPC 错误对象（前端契约零改动）；如未来需要错误码，另行立项。

### 1.4 批次划分与依赖

| 批次 | 域 | 理由 |
|------|----|------|
| C1 | 基建 + installation、scanner | 测试最厚（2086/1452 行），验证模板；顺带消除 scanner.rs:100 字符串判断 |
| C2 | central_skills、github_import、projects、marketplace、local_remote_sync | 业务核心域，按模板机械推广 |
| C3 | usage、obsidian、ai_provider、ai_tagging、portable_state + db/repos 透传 + 散点（central_migration、operation_log、logging、secrets、targets、bootstrap） | 尾批 + 全局扫尾 |

C2、C3 开始前必须确认上一批已归档且 `just ci` 绿。

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
