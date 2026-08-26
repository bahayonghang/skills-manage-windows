# SkillPort 日志系统覆盖盘点

## Scope

本盘点只读取仓库、现有 Trellis 历史和本机日志数据库的聚合信息，不修改产品代码，不连接远端，
不读取或记录目标地址、用户名、凭据、路径或日志正文。

## Existing Architecture

- `docs/architecture/runtime-observability.md` 已接受双层模型：
  - Operation Log：SQLite `operation_logs`，长期用户操作历史，手动清理；
  - Runtime Log：`skillport-YYYY-MM-DD.log`，前后端诊断，默认保留 14 天。
- `src-tauri/src/operation_log.rs` 已有 `OperationLogEvent`、`OperationSpec`、`with_operation_log` 和
  best-effort recorder。
- `src-tauri/src/logging.rs` 已有 daily writer、文件名白名单、读取/过滤/导出/清理、14 天 retention 和
  frontend runtime payload sanitizer。
- `src/lib/ipc/invoke.ts` 会统一捕获 frontend IPC rejection；`runtimeLogger.ts` 捕获 window error、
  unhandled rejection、显式 runtime event 与 `ipc.failure`。
- `/logs` 已提供 Operation/Runtime 双模式控制台。

因此问题不是“没有日志系统”，而是**覆盖、结构化错误、关联与治理不完整**。

## Registry and Coverage Evidence

### Runtime command inventory

- `src-tauri/src/ipc_registry.rs`：204 个 runtime commands。
- `src/test/contracts/ipcCommandCoverage.test.ts`：200 个 fallible，4 个 infallible。
- 当前 registry 只登记名称与 handler，不声明 read/write、外部副作用或日志策略。

### Static candidate scan

按命令名前缀识别可能写状态或产生外部副作用的命令，再检查命令体内是否直接出现
`record_operation_log_best_effort` / `OperationLogEvent::new`：

| Candidate result | Count |
| --- | ---: |
| likely write/side-effect commands | 101 |
| direct Operation Log reference visible in command body | 33 |
| no direct Operation Log reference visible | 68 |

这是**复核队列**，不是最终缺陷数。以下情况会产生静态误差：委托给其它 command/service 的记录器、
分拆在同模块其它文件的记录器、preview/cancel/internal bridge、deprecated commands。

高价值候选域包括：

- Central metadata：repository/tag assignment、AI review；
- Collections、saved views、tag groups；
- Projects 与 project install/uninstall；
- Marketplace registries、marketplace/skills.sh install、AI connection/explanation refresh；
- GitHub PAT 与 AI API key 的 set/clear/test；
- Obsidian imports；
- Central store relocation、legacy/deprecated update commands；
- Operation/Runtime log clear/export；
- startup retry/rebuild/exit 和后台 job cancel；
- agents enable/detect 与部分 Skills CLI actions。

### Existing local Operation Log aggregate

只读取聚合值：当前 323 条记录包含 21 种 action、9 种 category。记录主要集中在扫描、安装/卸载、
Central 删除、target、settings、portable state、recovery 与 Update Center。该快照只说明当前实际使用路径，
不能证明某个未出现 action 的代码路径一定没有 logger，但能证明控制台目前远未呈现完整产品操作面。

## Structural Gaps

### 1. No authoritative audit policy

新增 runtime command 只需进入 registry 和 IPC coverage；没有编译期/契约要求声明其 Operation/Runtime
策略。因此“记得补日志”仍是人工约定。

### 2. Free-form Operation Log vocabulary

`OperationLogEvent` 的 category/action/status/summary/error 都是 String。没有稳定 action registry、字段
allowlist 或每 action schema，导致命名漂移、过滤不完整、详情 JSON 形状各异。

### 3. Raw Display can reach operation history

`with_operation_log` 在失败时执行 `error.to_string()` 后写入 `error_summary`；多个 commands 也直接
`.error(error)`。details JSON 会经统一 redaction，但 error summary 只做空白折叠/截断。它既可能暴露动态
信息，也会让 stable code/category/phase 丢失。

### 4. Backend runtime evidence is not universal

frontend invoke wrapper 会记录 IPC failure，但 backend `ipc_boundary!` 只映射 `IpcError`。如果 renderer
未启动、崩溃或 recorder 自身失败，backend 可能没有含 command/code/phase/duration 的统一失败事件。

### 5. No first-class cross-layer correlation

Operation Log 只有 row id 和 batch id。row id 在操作结束持久化时才生成，batch id 又不能代表单次调用。
Runtime 与 IPC 没有统一可筛选 correlation，因此“看见一个 toast 后去哪一条日志”依赖时间猜测。

### 6. Outcome-only records miss interrupted attempts

多数 Operation Log 在业务操作结束后写一条结果。进程在长任务/跨层 mutation 中间退出时，可能没有
Operation Log；recovery journal 能保护部分 Central FS/DB 操作，但不是全产品审计事实。

### 7. Console detail prioritizes raw shape

Operation Log 详情仍是右侧全高 drawer，通用错误和 JSON 比 code/phase/next action 更突出；Runtime 行与
Operation 行也不能通过 correlation 互跳。

## Planning Implication

本任务应先建立 command audit policy、稳定 Operation schema、backend failure boundary 与 correlation，
再按领域批次补覆盖，最后完善控制台和防漂移契约。直接在每个 command 手写更多日志会扩大重复、遗漏和
隐私风险。
