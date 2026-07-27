# Request-scoped TargetContext Contract

## 1. Scope / Trigger

- Command 或 use case 同时需要 target identity、target cache DB、远端连接、Central FS、operation log 或 event payload 时，入口必须先解析一个 `TargetContext`。
- 只需要 target-scoped DB 的既有调用可暂时保留 `active_db()`；只需要 target identity 的既有调用可暂时保留 `active_target()`。两者都是迁移期兼容 API，禁止在同一 command 模块组合使用，也禁止新增调用。
- 此契约解决跨 target snapshot 一致性，不替代同 target mutation lock、preview snapshot token、远端进程监督或 FS/DB Saga。

## 2. Signatures

```rust
#[derive(Debug, Clone)]
pub struct TargetContext {
    target: ActiveTarget,
    db: DbPool,
}

impl TargetContext {
    pub fn target(&self) -> &ActiveTarget;
    pub fn db(&self) -> &DbPool;
    pub fn id(&self) -> &str;
    pub fn label(&self) -> &str;
    pub fn kind(&self) -> TargetKind;
}

impl TargetRegistry {
    pub async fn resolve_active_context(&self, local_db: &DbPool)
        -> Result<TargetContext, TargetsError>;
    pub async fn db_for_target(&self, local_db: &DbPool, target: &ActiveTarget)
        -> Result<DbPool, TargetsError>;
}

impl AppState {
    pub async fn resolve_target_context(&self) -> Result<TargetContext, String>;
}
```

## 3. Contracts

- `resolve_active_context()` 只读取一次 active target ID；之后用该显式 ID 解析拥有型 `ActiveTarget`，再由同一 target 选择 DB pool。
- `TargetContext` 拥有 target config 和可克隆的 pool。active target 切换或同 ID 配置更新只影响后续 resolver，不改变已返回 context。
- Command 在任何 `.await`、远端连接、FS 构造、operation log 或 event payload 之前冻结 context；service/helper 接受显式 `&ActiveTarget`、`&DbPool` 或领域参数，不重新读取 `AppState`。
- 业务 target 数据使用 `context.db()`；target registry、secrets、全局 settings 和 operation log sink 仍使用 always-local `state.db`。
- Operation log identity 统一由 `target_context_from_active_target(context.target())` 生成：`id`/`label` 为真实 target 身份，`kind` 保持 `local|ssh|wsl` 兼容值。
- Pending-operation list/retry commands resolve one context and require the durable row's target ID/kind to match it. They never recover with a newly selected target's DB or transport.
- Remote pending inventory is DB-only. Listing does not connect SSH/WSL; explicit retry or a mutation for the same frozen target may construct transport under that target's lease.
- 不给迁移期 helper 添加 Rust `#[deprecated]` 属性，直到所有调用完成迁移；否则 `clippy -D warnings` 会把兼容调用变成构建失败。迁移状态由本规范和 architecture test 强制。

## 4. Validation / Error Matrix

| 情形 | 结果 |
| --- | --- |
| Local active target | context target 为 Local，DB 为 always-local pool |
| SSH/WSL active target | context target 保留真实 ID/label，DB 按该 ID 使用对应 cache pool |
| 解析后切换 active target | 旧 context 的 target、DB、log/event identity 不变；新 resolver 返回新 target |
| active ID 指向缺失 target | 返回 `TargetsError::ActiveTargetMissing(id)`，不得静默回退 Local |
| target cache pool 初始化失败 | 返回原 `TargetsError`；不得改用另一 target 或 local DB |
| 同一 command 同时出现两个 ambient helper | architecture test 失败 |
| service/helper 需要远端资源 | 从传入 context target 构造；禁止读取 ambient `AppState` |
| recovery row target 与 context 不一致 | 拒绝恢复；不得连接或改写另一 target |

## 5. Good / Base / Bad Cases

- **Good**：command 入口解析一次 context，把 `context.db().clone()` 与 `context.target().clone()` 移入异步 operation；日志从同一 target 派生。
- **Base**：只读取一个 target DB 的未迁移 CRUD 暂时调用 `active_db()`；该 helper 自身委托 `resolve_target_context()`，不再二次解析 target。
- **Bad**：先调用 `active_target()`，经过 `.await` 后再调用 `active_db()`；helper 接受 `AppState` 后自行重读 target；把 SSH/WSL 日志 ID 写成字面 `"ssh"`/`"wsl"`。

## 6. Tests Required

- Local、SSH、WSL context 的 target kind、ID、label 与 DB pool identity。
- 显式阶段的 local -> SSH、SSH-A -> SSH-B、SSH -> WSL、WSL -> local 切换矩阵；旧 context 的 DB marker、operation-log identity 与 event identity 必须保持不变。
- 两个 SSH target 的 operation log ID/label 必须可区分，`kind` 仍为 `ssh`。
- 递归扫描 `src/commands`，拒绝任何模块同时包含 `state.active_target()` 与 `state.active_db()`。
- 至少运行 targets、operation_log 和受迁移 command 的定向测试，再运行 locked Rust 全量检查与 `just ci`。

## 7. Wrong vs Correct

```rust
// Wrong: 两次 ambient 解析之间可发生 target 切换。
let target = state.active_target().await?;
let pool = state.active_db().await?;
run(pool, target).await

// Correct: target 与 DB 来自同一个 request-scoped snapshot。
let context = state.resolve_target_context().await?;
let target = context.target().clone();
let pool = context.db().clone();
run(pool, target).await
```

> 来源任务：07-24-target-context-snapshot（2026-07-26）
