# 设计：Request-scoped TargetContext

## 1. 系统不变量

一个 Tauri command/use case 开始后，只能使用一个 target 身份、该 target 的 cache DB、由该 target 构造的远端连接/FS adapter，以及该 target 的 operation-log identity。切换 active target 只改变后续 resolver 的结果，不改变已经返回的 context。

本任务解决的是跨 target snapshot 一致性，不负责同 target mutation 串行化、远端进程 timeout/cancel 或 FS/DB Saga。

## 2. 数据结构与所有权

在 `targets` 层新增拥有型 context：

```rust
#[derive(Clone)]
pub struct TargetContext {
    target: ActiveTarget,
    db: DbPool,
}
```

公开只读访问器：

- `target() -> &ActiveTarget`
- `db() -> &DbPool`
- `id() -> &str`
- `label() -> &str`
- `kind() -> TargetKind`
- `operation_log_context() -> OperationLogTargetContext` 放在 operation-log 边界的 extension/helper 中，避免 `targets` 反向依赖 operation log。

不在 context 中缓存 `CentralFs` 或 `ConnectedRemoteTarget`：这些对象并非每个 command 都需要，且 `targets` 层不能依赖 service 层。需要远端 IO 的 command/service 从 `context.target()` 按需调用现有连接构造器，所得连接天然属于同一快照。

## 3. Resolver 契约

`TargetRegistry::resolve_active_context(local_db)`：

1. 调用 `active_target_id(local_db)` 一次并保存 ID。
2. 调用 `target_by_id(local_db, &id)` 得到拥有型 `ActiveTarget`。
3. 调用新 `db_for_target(local_db, &target)`：Local 克隆 local pool；SSH/WSL 仅使用该 target 自身 ID/home 获取 cache pool。
4. 返回 `TargetContext::new(target, db)`。

`AppState::resolve_target_context()` 只是 command 边界错误字符串化适配。旧 `active_target()`/`active_db()` 暂时保留，并在 spec 中标记为迁移期兼容 API；`active_db()` 内部改为 resolver 后克隆 context DB，避免旧调用仍有双读竞态。Rust `#[deprecated]` 属性等所有兼容调用迁完后再添加，否则当前 `clippy -D warnings` 会阻断过渡版本。

active target 在第 1 步之后切换不会改变后续步骤使用的 ID。若同 ID 的 target 配置并发更新，当前 context 保留解析时的拥有型配置与 pool；更新路径的 pool invalidation 只影响后续 context。v1 不新增内存 generation，因为它会成为与 SQLite `active_target_id_v1` 并列的第二状态源，且不能提高拥有型快照的跨 target 一致性。

## 4. 迁移边界

首批迁移所有同时需要 target/DB/FS/log identity 的 P1 流程：

- `commands/github_import.rs`，包括 remote preview workspace helper 不再从 `AppState` 重读 target。
- `commands/central_updates.rs` 与 `commands/skill_update_inventory.rs`。
- `commands/portable_state.rs`。
- `commands/scanner.rs`。
- `commands/skills.rs` 中 delete/update/link/unlink 等同时使用 target 与 DB 的路径。
- `commands/agents/mod.rs` 中同时读取 target 与 DB 的 mutation/read paths。
- `commands/settings.rs` 中 target-scoped setting 与 operation log paths。

只需要 target DB、不需要 target 身份的简单 CRUD 可暂时通过迁移期 `active_db()` 兼容；但生产代码中同一 command 不允许再组合调用两个旧 helper。后续子任务触及这些模块时逐步迁移为 context。

## 5. Operation Log

删除 `skill_update_inventory.rs` 中把 SSH/WSL 写成字面 `"ssh"`/`"wsl"` ID 的私有 helper，统一调用 `target_context_from_active_target(context.target())`。`kind` 仍为 `local|ssh|wsl`，`id` 为 `local` 或真实 target UUID，`label` 为配置 label。

业务 operation log 仍写入全局 local DB `state.db`，不改 schema；context 只提供准确身份。日志失败继续 best-effort，不影响业务结果。

## 6. 竞态测试

测试层提供显式阶段，而不是依赖不稳定 sleep：

1. 建立 local、SSH-A、SSH-B、WSL fixtures 和各自 cache DB marker。
2. active=A，调用 resolver 得到 context A。
3. barrier 后切换 active=B。
4. 使用已解析 context 读取 DB marker、构造 target/FS identity、operation-log context 和模拟 event payload，全部必须为 A。
5. 新调用 resolver 必须得到 B。

矩阵覆盖 local→SSH、SSH-A→SSH-B、SSH→WSL、WSL→local。另加纯 resolver 测试证明 active ID 仅读取一次后传入 `target_by_id/db_for_target`；生产 grep/architecture test 禁止同一 command 同时出现旧 helper 组合。

## 7. 兼容性与回滚

不修改数据库 schema、IPC payload 或持久化 target ID。旧 helper 保留一个迁移窗口，因此可按 command 模块回滚。任何回滚不得恢复 `TargetRegistry::active_db()` 内部重读 active target 的实现。

高风险是大范围签名迁移时误把全局 `state.db`（secret/target registry/operation log）替换为 target cache DB；实施时必须逐调用点区分：业务 target 数据用 `context.db()`，全局 settings/secrets/target definitions/log sink 继续用 `state.db`。

## 8. Spec 更新

新增 `.trellis/spec/backend/target-context.md`，并同步：

- `domain-error-enums.md` 中 AppState helper 描述。
- `docs/architecture/backend.md` 与 `docs/architecture/overview.md` 的 active target 说明。
- 必要时 `transport-seam.md` 的显式 target/connection 传参示例。
