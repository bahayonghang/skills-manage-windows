# Request-scoped TargetContext 快照

## Goal

让"一个 operation 只有一个不可变 TargetContext"成为 backend invariant，消除跨 target split-brain 竞态；顺带修复 operation log 的 target 身份丢失。对应审计 P1-01（🟠）、P2-08（🟡）、M-01、QW-07。

## 核对证据（2026-07-24 dev 分支）

- `src-tauri/src/lib.rs:73-85`：`AppState::active_db()` 与 `active_target()` 是两个独立 async 方法，各自解析。
- `src-tauri/src/targets/registry.rs:359,405`：`active_db()` 内部再次调用 `active_target()`。
- `src-tauri/src/commands/github_import.rs` 等多个 command 先 `active_target()` 再 `active_db()`，两次读取之间 `set_active_target` 可并发执行——T0 读到 SSH-A 连接、T2 拿到 SSH-B 的 cache DB。
- `src-tauri/src/commands/skill_update_inventory.rs:341-342`：operation log 把所有 SSH target 记为字面 `"ssh"`、WSL 记为 `"wsl"`，多远端环境无法归因。

## Requirements

1. 引入 request-scoped `TargetContext { target, db }`，ID/label/kind 从同一 `ActiveTarget` 派生，远端连接/FS adapter 按需从该 target 构造。`AppState::resolve_target_context()` 在 command 入口只读取一次 active target ID，之后按该显式 ID 解析 target 与 DB；service 层不再读取 ambient state。
2. 切换 active target 只影响后续 command，不改变 in-flight operation 的 context。
3. 迁移策略：新 API 与旧 `active_target()`/`active_db()` 并存，旧方法在 spec 中标记为迁移期兼容 API，按模块迁移（优先 P1 流量路径：import/update/delete/portable state/scanner）。Rust `#[deprecated]` 属性等所有兼容调用迁完后再添加，避免当前 `clippy -D warnings` 把过渡调用变成构建失败。
4. operation log 统一走 target context helper（`target_context_from_active_target` 已存在），记录真实 target ID/label；保留 `kind` 字段兼容旧聚合。
5. v1 不引入第二套内存 generation 作为一致性来源。快照身份是已解析的 target ID 与拥有的 config/DbPool；target 切换或同 ID 配置更新只影响后续 resolver。需要拒绝过期 UI preview 的域继续使用自身的 target ID/workspace/snapshot token 校验。

## Acceptance Criteria

- [ ] 审计 §7.1 race 测试落地：command 于 target A 开始 → barrier → 切换 B → 断言 FS/DB/log/event payload 全部仍为 A；覆盖 local↔SSH、SSH-A↔SSH-B、WSL 组合
- [ ] 生产代码中除 context resolver 外不再出现 `state.active_target()` + `state.active_db()` 的组合调用（grep 断言）
- [ ] 两个不同 SSH target 的 operation log 可明确区分（`target_id` 不再是字面 "ssh"）
- [ ] `TargetRegistry::active_db()` 不再二次解析 active target；所有新路径通过 `resolve_target_context()` 或显式 `db_for_target()` 获取匹配 pool
- [ ] `cd src-tauri && cargo test` 全绿，`just ci` 通过

## 非目标 / 依赖

- 不在本任务内实现 per-target mutation 串行化（属 job-concurrency-lease 与长期 L-01）。
- 无前置依赖；本任务是 remote-process-supervisor、fs-db-operation-journal 的建议前置。
- 属复杂任务：`task.py start` 前需补 design.md（TargetContext 结构与 generation 语义）+ implement.md（模块迁移顺序）。
