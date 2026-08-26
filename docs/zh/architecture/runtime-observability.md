# 运行时可观测性

运行时可观测性是 `/logs` 控制台背后的诊断层。它把用户操作历史和开发/诊断日志分开，让两层分别使用匹配自身目的的存储、保留周期与隐私策略。

## ADR：双层日志

**状态：** 已接受

**决策：** SkillPort 使用双层日志模型：

- **Operation Log** 保持在 SQLite（`operation_logs`）中，记录安装、卸载、扫描、设置变更、target 切换、导入、导出等用户可见操作。
- **Runtime Log** 使用有界日文件，文件名为 `skillport-YYYY-MM-DD.log`，记录后端 tracing 与前端诊断事件，例如 `error`、`unhandledrejection`、显式 `frontend.runtime` 事件和 IPC 失败。

两层在 Observability Console UI 中汇合，但不共享存储或生命周期规则。

## 命令策略与关联追踪

`src-tauri/src/ipc_registry.rs` 是全部 runtime command 及其唯一日志策略的权威来源：`operation`、
`runtime-only` 或 `excluded`。生成的 [IPC 命令字典](./ipc-commands.md) 同时是当前命令审计矩阵；它直接从
registry 生成，不依赖手工维护的命令总数。

```text
前端 IPC 调用
  └─ 合法 correlation UUID
      ├─ Operation Log 行（用户可见副作用）
      ├─ backend Runtime rejection（安全且已审查的诊断）
      └─ frontend Runtime rejection（renderer 视角）
```

Operation command 使用 Operation 行 UUID 作为 correlation ID。Runtime-only 失败使用同格式 UUID，但不制造
业务审计历史。backend 与 frontend Runtime 事件是两个独立视角，并显式标注 event source。

## 为什么 Operation Log 使用 SQLite

Operation Log 是长期产品数据，需要结构化筛选、稳定分页、target / category / action 字段，以及面向用户的导出语义。SQLite 也让它与其它 local-first 元数据一起保存在 `~/.skillsmanage/db.sqlite`。

保持 Operation Log 的 SQLite 形态，可以避免把诊断噪声混入审计历史，也保留现有 `operation_logs` 表和 list / detail / clear / export 命令契约。

## 为什么 Runtime Log 使用文件

Runtime Log 是短生命周期诊断轨迹。文件日志更适合这一层，因为：

- Rust 后端可以在数据库或 UI 就绪前写入 tracing 输出；
- 前端诊断事件可以追加写入，不需要数据库迁移；
- 日文件便于检查、复制、redact、导出和删除；
- 保留策略可以通过删除 14 天前的匹配文件实现。

只有匹配 `skillport-YYYY-MM-DD.log` 的文件能被列出、读取、导出或删除。后端会拒绝任意文件名，避免 IPC 表面穿越日志目录。

## Observability Console 契约

`/logs` 是双模式控制台：

| 模式      | 来源                              | 主要用途               | 清理语义                                           |
| --------- | --------------------------------- | ---------------------- | -------------------------------------------------- |
| Operation | SQLite `operation_logs`           | 用户操作历史与审计轨迹 | 现有手动 Operation Log 清理流程                    |
| Runtime   | 日文件 `skillport-YYYY-MM-DD.log` | 前后端诊断             | 删除选中的匹配 runtime 文件或有界 runtime 文件集合 |

Runtime 模式支持文件选择、query / level / source / operation ID / event source 过滤、tail 读取、安全行详情、
复制、导出和清理确认。Runtime 行在写盘前以及读取/导出时都会经过同一套 fail-closed redaction；即使一行由
多次 writer 分片写入，也不能绕过脱敏。writer 在半行处 flush 时，sink 只写入脱敏占位并丢弃该逻辑行在
下一换行符之前的续写，避免多个 flush 在磁盘上重新拼成敏感值。

Operation 详情使用居中、紧凑、不会溢出视口的小窗。主视图展示本地化状态、已审查原因、下一步、安全诊断键
和有界失败项；安全结构化详情默认折叠。合法 correlation UUID 可跳转到匹配的 Runtime evidence，Runtime
记录也能回到精确 Operation 行。

## 隐私与保留周期

新事件必须在构造时即安全：code、category、action、phase、status、source 来自受控集合；动态值只允许通过
校验的 UUID/逻辑 ID、数字和布尔值。password、token、PAT、API key、SSH 凭据、host/username、路径、
URL、ref/SHA、命令/环境、输出、stack 和 raw source error 都不是日志输入。写盘前 redaction 是最后一道
sink 防线；读取/导出 redaction 用于兼容历史文件，不能作为构造 raw 事件的理由。

Runtime Log 的生命周期更严格：

- 启动清理会删除 14 天前的 runtime 文件；
- 手动清理只触达白名单 runtime log 文件名；
- Runtime Log 有意不代理全部 `console.*` 输出，避免噪声和隐私风险。

## 排障流程

1. 从失败的 Operation 行复制 correlation UUID。
2. 打开匹配 Runtime evidence，对照 backend 与 frontend 两个来源。
3. 依据已审查 code、phase 和 retryable 决定下一步，不从 raw JSON 猜测原因。
4. 只读/preview 本就不应产生 Operation 行时，直接用返回的 correlation UUID 筛选 Runtime。
5. 异常终止后残留的 `started` 行会在下次一次性启动审计中转为 `interrupted`。

Last reviewed: 2026-08-27
