# Design: src-tauri 优化任务树

## 1. 设计目标

本任务树不追求新的分层名词，而是让已有的深模块成为唯一 authority：

```text
untrusted input
  -> bounded acquisition / identity validation
  -> service use case (lock + orchestration)
  -> repository transaction
  -> filesystem / target adapter
  -> stable IPC/CLI result
```

任何 child 都必须减少旁路，而不是再新增一套 client、path helper、cache 或 mutation API。

## 2. 核心不变量

### I1 - Central 写入只有一个 use-case authority

- 网络 acquisition 和解析在锁外完成。
- final target recheck、完整目录写入、DB upsert/provenance 和 recovery journal 复用 GitHub import/Central services。
- 远端 display name 永不作为路径组件 authority；stable sanitized skill id/candidate identity 才能决定目标。

### I2 - 外部状态同时受单项和进程总量预算

- 单个 archive/file/tree response 的既有 `ResourceBudget` 保留。
- cache/registry 另有 entry 和 aggregate-byte 上限；TTL 不是容量策略。
- bytes 用 `Arc` 共享，避免命中/插入时深拷贝。

### I3 - ownership 与清理绑定

- registry 删除一个 remote snapshot 前，必须把 storage ownership 交给可识别 owning target 的 cleanup path。
- cleanup failure 可重试；不得为了“内存里看起来干净”先丢掉唯一 workspace 引用。
- active import lease 和 deferred discard 语义保持不变。

### I4 - 拒绝发生在分配或 mutation 之前

- HTTP/file reader 按 chunk 检查 checked cumulative size。
- idle/total deadline、wire byte、decoded text 分别建模。
- 所有 string summary/truncation 以 char boundary 为准。

### I5 - page work 与 page size 成正比

- filter、sort、count、offset/limit 在 SQLite 完成。
- 关联数据只批量加载当前 page IDs；单次 bind 数量被 `limit <= 500` 约束。
- 列表时间字段以 persisted cache/fallback 为 authority，hot path 不同步 stat 全库。

### I6 - 一个业务 mutation 对应一个 transaction

- transaction 由最外层 repository/use case 持有。
- helper 接受 transaction executor，不在循环中隐式 commit。
- 明确 partial-result 的批处理可以逐项提交；本轮 metadata APIs 没有 partial result，必须 all-or-nothing。

## 3. 子任务边界

| Child | Owns | Does not own |
| --- | --- | --- |
| Marketplace Central contract | install acquisition、candidate identity、完整目录 import、installed marker | registry sync cache transaction、通用 HTTP reader |
| Snapshot lifecycle | Central update cache、preview registry、remote workspace ownership | GitHub archive 单项预算、import 业务语义 |
| Bounded ingestion | shared bounded readers、AI deadlines/output、UTF-8 truncation、remaining file/tree reads | 已由 Marketplace task 删除的 raw install downloader |
| SQL pagination | page query、page enrichment、persisted timestamp list semantics、query-plan/perf evidence | unpaged detail APIs、前端 virtualization |
| Transactional metadata | repository/tag/collection/project atomicity、Marketplace sync/remove snapshot cache | Central FS+DB journal、Marketplace install mutation |

## 4. 兼容性

- IPC command 名、请求/响应 DTO 和 stable error envelope 默认不变。
- `marketplace_skills.download_url` 可保留用于展示/旧数据兼容，但不得继续决定 backend request destination。
- schema/index 如需变化，必须走 versioned migration，保留 uid/provenance 语义，并运行 `pnpm docs:gen`。
- CLI 与 GUI 共用 services/repositories；不得让修复只在 Tauri command 路径生效。
- Local/SSH/WSL 的行为差异仅存在 transport adapter，domain decision 不分叉。

## 5. Rollout 与回滚

每个 child 独立分支、独立验证、独立 archive。推荐顺序为 P0 install -> snapshot/input -> transaction -> pagination。每个 child 的实现计划列出更细的回滚点。

若某个 child 需要改变 IPC/schema：先提交兼容读路径，再切写路径，最后删除旧旁路。回滚时旧 persisted data 仍可读；不得以删除用户技能或清空 cache 作为恢复手段。

## 6. 观测与证据

- 资源拒绝使用 typed domain errors 和固定 IPC summary，不把 URL、token、path、body 复制到日志。
- cache 记录 count/bytes/eviction/reclaim 的结构化诊断，但不记录 preview token 或 workspace path。
- benchmark 报告写入 child `research/`，包含 fixture、build profile、warm-up、before/after p50/p95 和结构性计数。
- 最终父任务只在五个 child 各自验收并完成跨域 `just ci` 后归档。
