# 全链路日志系统设计

状态：**planning ready for final review**。

## 1. Fundamental Boundary

SkillPort 需要解决的不是“记录更多字符串”，而是建立三个可验证事实：

1. 每个 runtime command 都有显式日志策略，新增命令不能静默漏记；
2. 每个需要长期审计的 operation 有唯一 ID、稳定结果和安全失败原因；
3. 每个 fallible IPC rejection 即使 renderer 不可用，也有 backend Runtime evidence。

成功的纯读取不进入 Operation Log。Operation 与 Runtime 两层继续使用不同存储和 retention，但共享
operation/correlation 与 reviewed diagnostic 语义。

## 2. Deep Module and Seam

在 `src-tauri/src/observability/` 建立一个深模块，替代 command 层散点的 timer、自由字符串 event、
raw `Display` 和 tracing 拼装。外部 interface 保持小而稳定：

```rust
pub enum CommandLogPolicy {
    Operation(OperationDefinition),
    RuntimeOnly(RuntimeDefinition),
    Excluded(ExclusionReason),
}

pub struct OperationDefinition {
    pub category: OperationCategory,
    pub action: OperationAction,
    pub default_phase: OperationPhase,
    pub lifecycle: OperationLifecycle,
}

pub struct ReviewedDiagnostic {
    pub code: &'static str,
    pub category: &'static str,
    pub phase: &'static str,
    pub public_message: &'static str,
    pub retryable: bool,
}

pub async fn run_operation<R, F, Fut>(
    state: &AppState,
    definition: OperationDefinition,
    target: OperationTarget,
    build_success: impl FnOnce(&R, Duration) -> SafeOperationResult,
    operation: F,
) -> IpcResult<R>;
```

实际名称可以按 Rust ergonomics 调整，但 interface 不允许 caller 提交 raw error、任意 details JSON、
任意 category/action/status 或 tracing 字符串。模块内部拥有：ID 生成、计时、started/final lifecycle、
reviewed failure、redaction、best-effort DB 写入和 Runtime fallback。

SQLite 是 local-substitutable dependency，已有内存数据库 fixture；无需为了测试暴露一个只有单生产 adapter
的 trait。Runtime tracing 使用现有 subscriber，同样保持模块内部 seam。

## 3. Authoritative Command Policy

扩展 `ipc_registry.rs` 的单点声明，使每个 runtime command 同时携带 policy tag；handler、command names 与
policy inventory 均由同一 macro 输入生成。例如：

```rust
create_collection [operation(Catalog, CollectionCreate, Command)]
    => commands::collections::create_collection,
get_collections [runtime_only(ReadOnly)]
    => commands::collections::get_collections,
record_frontend_runtime_log [excluded(SelfLogging)]
    => commands::logs::record_frontend_runtime_log,
```

policy contract：

- `operation`：写 DB/文件/远端状态、改变设置/凭据、启动/取消业务 job，或产生用户可观察外部副作用；
- `runtime_only`：成功的纯读取、搜索、详情、preview、内部刷新；失败仍由 IPC Runtime boundary 记录；
- `excluded`：仅日志自举/写日志本身或无业务语义的桥接，理由是受控 enum，不能写自由备注。

同一 registry 生成 handler 与 policy，避免“两个数组恰好相等”的浅层治理。测试断言当前 204 个命令全部
可分类；数量随 registry 变化自动更新，不把 204 写成永久常量。

## 4. Operation Identity and Lifecycle

### 4.1 Reuse the row ID

Operation Log 现有 `id` 已是 UUID。将它改为在 operation 执行前由 observability 模块生成，并同时作为
`operationId` / correlation：

- Operation Log row id：长期审计身份；
- Runtime field `operation_id`：跨层检索；
- failed `IpcError.correlationId`：前端错误引用；
- `batch_id`：继续只表示多项/多目标分组，不与 operation ID 混用。

这样无需新增同义 `correlation_id` 列。Operation Log query/search 增加 ID 精确/文本检索；Runtime parser
提取 `operation_id`。旧行 ID 天然可作为 operation ID，保持兼容。

### 4.2 Lifecycle modes

```text
TerminalOnly:
  run -> insert terminal row

StartedThenTerminal:
  generate id -> insert started -> run -> update same row to terminal
                                  \-> process exits: row remains started
next startup -> mark prior-process started rows interrupted
```

- 短小、单 DB transaction 的 operation 可用 `TerminalOnly`；
- 可取消、长运行、跨 FS/DB/remote 或已有 recovery journal 的 operation 必须用 `StartedThenTerminal`；
- 启动恢复只把上一进程遗留的 `started` 改为 `interrupted`，不推断业务 rollback；
- recovery journal 继续是物理恢复权威，Operation Log 只记录审计事实和安全 operation ID/code。

如果 started 写入失败，业务仍继续，Runtime 写安全 warning；final 阶段尝试以同一 ID insert/update terminal
事实。日志失败不得改变业务结果。

## 5. Stable Diagnostic Envelope

所有 operation failure 最终收敛为：

```json
{
  "errorCode": "domain.reason",
  "errorCategory": "domain.family",
  "phase": "controlled_phase",
  "retryable": false,
  "operationId": "uuid"
}
```

- domain mapper 拥有具体 stable code 与固定 public message；observability 模块不解析 `Display`；
- command policy 提供安全 category/default phase fallback；未分类错误使用 `internal.unexpected`，仍带静态
  category/phase/operation ID；
- `OperationLogEvent.error(raw)` 和自动 `error.to_string()` 路径被移除或收口为 private compatibility shim；
- success details 只接受 action-owned typed safe result，再由模块序列化和统一 redaction；
- batch failure item 延续现有最多 50 项的 safe identifier/code/category/phase allowlist。

`IpcError` 增加可选 `correlationId`，仍只暴露 reviewed code/message/retryable。Rust/TypeScript/generated IPC
类型同步；旧 frontend 忽略新增字段，新 frontend 对旧 backend 的缺失字段安全退化。

## 6. Runtime Failure Boundary

`ipc_boundary!` / typed command adapters 在 backend 对每个 fallible rejection 记录一次安全事件：

```text
target = skillport::ipc
source = backend
module = module_path!()
code / category / phase / retryable / duration_ms / operation_id / target_kind
```

不记录 command args、raw source、Display、path、host 或内容。Operation command 使用已有 operation ID；
runtime-only failure 生成临时 correlation ID。Frontend `ipc.failure` 从 rejection 读取同一 ID，并标记
`source=frontend`。若旧 backend 没有 ID，frontend 可生成仅前端的 fallback ID并明确来源。

Runtime UI 不删除前后端两条证据，而是按 operation ID 显示来源并允许聚合/跳转，避免把有价值的双视角
误判为重复垃圾。`record_frontend_runtime_log` 继续走 `invokeRaw` 并标记 `excluded(SelfLogging)`，保持防递归。

## 7. Domain Coverage Strategy

三个 coverage child 只通过 observability interface 接入，不各自发明 recorder：

- Central/targets/settings：复用既有日志，迁移 raw error，自查 update/recovery/secret/log-admin/startup；
- catalog/projects/Obsidian：补 repository/tag/collection/view/group/agent/project/vault operation；
- Marketplace/import/CLI：补 registry/install/import/portable state/Skills CLI operation。

每个 child 都要逐项对照 authoritative policy，明确委托日志、deprecated entry、job cancel、batch/partial、
preview/runtime-only 与 external test/open/reveal 的归属。

## 8. Observability Console

- Operation detail 改为居中约 560px Dialog；视口受限高度与窄窗安全边距；
- 主信息顺序：status/summary -> localized public reason/next action -> code/category/phase/retryable/operation ID
  -> target/duration/batch -> collapsed safe JSON；
- Operation 列表支持 operation ID 精确检索，Runtime 支持 operation ID filter；详情提供“查看关联 Runtime”/“查看关联 Operation”；
- status 用图标、文字和颜色共同表达；`started`/`interrupted` 有明确语义；
- 保留 Close/Escape、focus trap/restore、copy ID/JSON、invalid JSON/legacy/unknown fallback；
- 所有新增文案 en/zh i18n 成对维护，动态 backend Display 不进 DOM。

## 9. Privacy, Retention, and Administration

- Operation Log 持久化前脱敏；Runtime 仍按现有策略读取/导出脱敏，但新增事件在写入前只构造 allowlist envelope；
- Runtime 保留 14 天，Operation Log 手动清理，不增加 telemetry 或 console proxy；
- clear Operation Logs 先清目标范围，再写一条新的 `logs.operation.clear` 事实，记录安全 filter kind 与 count；
- export Operation/Runtime Logs 在成功生成安全 payload 后记录 `logs.*.export`，导出内容不反向包含刚创建的 export event；
- clear Runtime Logs、secret set/clear、database recovery/rebuild 等敏感管理动作只记录动作和结果，不记录值。

## 10. Compatibility and Rollback

- 不增加第三方依赖；不改变 Operation/Runtime storage split；
- 优先复用 Operation Log row ID，避免 correlation schema migration；需要新增的 repo 行为仅是 caller-supplied ID、
  started insert、terminal update、startup interrupted sweep 和 ID filter；
- 旧 Operation rows 继续可读；旧 status/details 有 UI fallback；
- generated IPC/data-model docs 只在真实类型/command metadata变化后重生；
- rollback 顺序为 UI -> domain coverage -> Runtime boundary -> core module；在 child 完成前不得删除旧 recorder，
  core child 提供短期 compatibility adapter，集成 child 统一移除。

## 11. Important Trade-offs

- 不记录成功纯读取，换取更低噪声、隐私面和写放大；失败仍全量进入 Runtime；
- 使用 Operation Log row ID 作为 correlation，减少 schema 与概念数量；代价是 runtime-only failure 的 ID 不对应
  Operation row，但 UI 会明确标记；
- `started` 只用于长/可中断 operation，避免所有短写操作翻倍 DB 写入；
- backend/renderer failure 各保留一条并通过 ID 关联，不盲目丢弃任一视角。
