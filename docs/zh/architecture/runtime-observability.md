# 运行时可观测性

运行时可观测性是 `/logs` 控制台背后的诊断层。它把用户操作历史和开发/诊断日志分开，让两层分别使用匹配自身目的的存储、保留周期与隐私策略。

## ADR：双层日志

**状态：** 已接受

**决策：** SkillPort 使用双层日志模型：

- **Operation Log** 保持在 SQLite（`operation_logs`）中，记录安装、卸载、扫描、设置变更、target 切换、导入、导出等用户可见操作。
- **Runtime Log** 使用有界日文件，文件名为 `skillport-YYYY-MM-DD.log`，记录后端 tracing 与前端诊断事件，例如 `error`、`unhandledrejection`、显式 `frontend.runtime` 事件和 IPC 失败。

两层在 Observability Console UI 中汇合，但不共享存储或生命周期规则。

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

| 模式 | 来源 | 主要用途 | 清理语义 |
| --- | --- | --- | --- |
| Operation | SQLite `operation_logs` | 用户操作历史与审计轨迹 | 现有手动 Operation Log 清理流程 |
| Runtime | 日文件 `skillport-YYYY-MM-DD.log` | 前后端诊断 | 删除选中的匹配 runtime 文件或有界 runtime 文件集合 |

Runtime 模式支持文件选择、query / level / source 过滤、tail 读取、raw line detail、复制、导出和清理确认。Runtime 导出与读取输出使用同一套敏感字段 redaction 策略。

## 隐私与保留周期

两层都会 redact password、token、PAT、API key、secret、private key、credential 等敏感字段。Runtime Log 的生命周期更严格：

- 启动清理会删除 14 天前的 runtime 文件；
- 手动清理只触达白名单 runtime log 文件名；
- Runtime Log v1 有意不代理全部 `console.*` 输出，避免噪声和隐私风险。

Last reviewed: 2026-06-03
