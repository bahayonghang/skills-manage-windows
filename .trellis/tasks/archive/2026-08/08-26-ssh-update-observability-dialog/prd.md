# 全链路日志系统、操作审计与可观测性控制台完善

状态：**planning ready for final review**。本任务尚未授权 `task.py start` 或产品代码修改。

## Goal

在不泄露凭据、路径、远端地址、命令或内容的前提下，让所有需要审计的用户操作都有稳定记录，
让所有失败都能从可见反馈追到 Operation Log 与 Runtime Log，并让新增命令无法绕过日志覆盖规则。
同时保留现有双层存储模型，优化 Observability Console 与日志详情居中小窗。

## User Value

- 用户可以回答“在什么目标上执行了什么、是否成功、失败在哪一步、能否重试、下一步做什么”。
- 开发者可以用同一个 correlation ID 对照 IPC rejection、Operation Log 与 Runtime Log，避免只看到
  `The operation failed` 或孤立的原始错误行。
- 新增功能必须显式声明日志策略，日志覆盖不再依赖开发者记得手写 `OperationLogEvent`。
- 日志可读、可筛选、可导出、可清理，但不会为了“更多细节”保存秘密或用户内容。

## Confirmed Facts

1. 本次 SSH 现象已由用户确认是目标地址输入错误；不再调查 SSH transport、服务端或特定更新失败。
2. `docs/architecture/runtime-observability.md:3-14` 已接受双层模型：Operation Log 是 SQLite 中的
   用户操作历史，Runtime Log 是保留 14 天的日文件诊断；本任务延续而非推翻该模型。
3. `src-tauri/src/ipc_registry.rs` 当前登记 204 个 runtime commands；契约测试把其中 200 个视为
   fallible、4 个视为 infallible。
4. 基于命令名与命令体的静态候选扫描识别 101 个可能写入状态或产生外部副作用的命令：33 个命令体内
   可见直接 Operation Log 记录，68 个没有直接记录。该数字是待逐项复核的审计候选，不是最终缺陷数，
   因为部分命令会委托给已有记录器、属于 preview/cancel，或是 deprecated 入口。
5. 当前本机数据库有 323 条 Operation Log，仅出现 21 种 action、9 种 category；已有覆盖集中在扫描、
   安装/删除、target、settings、portable state 与 Update Center。collections、projects、saved views、
   tag groups、Central metadata、Obsidian、Marketplace/registry、secret 管理和日志管理等域缺少或覆盖不一致。
6. `src-tauri/src/operation_log.rs:69-134` 的事件字段仍是自由字符串；`:151-180` 的通用 wrapper 会把
   `Display` 文本写进 `error_summary`，而不是强制使用 reviewed stable diagnostic。这既会丢失结构化原因，
   也可能绕过“只记录安全公开摘要”的项目约定。
7. `src/lib/ipc/invoke.ts:53-72` 会在前端捕获每个 IPC rejection，`src/lib/runtimeLogger.ts:150-164`
   会写 `ipc.failure`；但它依赖 renderer 正常运行，且没有统一的跨层 correlation ID。
8. `src-tauri/src/ipc_error.rs:458-471` 的通用 IPC boundary 只做错误映射，没有统一记录 command、phase、
   duration 或 correlation；各 command 的 backend Runtime tracing因而不一致。
9. `src/components/logs/OperationLogDetailDrawer.tsx:113-124` 仍是右侧全高抽屉；本任务继续把它改为
   居中、紧凑、响应式且可键盘操作的小窗。

## Requirements

### R1. 全命令日志策略清单

- 以 `ipc_registry.rs` 的 204 个 runtime commands 为权威全集，为每个命令声明且只声明一种策略：
  `operation`、`runtime_only` 或 `excluded`。
- `operation`：用户触发的状态变更、文件/数据库变更、远端变更、凭据变更或可观察外部副作用。
- `runtime_only`：成功的纯读取、搜索、详情读取、preview 和内部刷新等不形成长期业务审计事实的动作；
  失败仍进入 Runtime Log。用户显式触发的连接测试、文件打开、导出、清理等可观察外部副作用仍属于
  `operation`。
- `excluded`：日志系统自举、日志写入本身或明确无业务语义的内部桥接；必须有稳定理由，不能静默遗漏。
- 契约测试比较策略清单与 runtime command registry；新增、删除或重命名命令时未更新策略必须失败。

### R2. Operation Log 覆盖与生命周期

- 每个 `operation` 命令必须通过统一 recorder 记录目标、稳定 category/action、status、duration、
  safe subject/counts 和 correlation ID；禁止散点复制 run/timer/record 模板。
- 成功、失败、partial、cancelled 均可检索；批量操作记录一个 batch 事实并保留有界安全失败项，
  不为每个成功项制造噪声。
- 长运行、可取消、跨文件系统/数据库或可能被进程中断的动作记录 `started` 与最终状态；崩溃留下的
  started 记录必须可识别为 interrupted/stale，而不能伪装成成功。
- 日志清理、导出、数据库恢复、凭据写入/清除等“管理日志或安全状态”的操作本身也必须留下安全审计事实。

### R3. 稳定诊断与错误保真

- Operation Log 不再接受任意 `Display` 作为 `error_summary`；失败必须经过 domain-owned reviewed
  diagnostic，至少包含 stable code、static category、phase、retryable 与固定 public message。
- 未分类错误固定为 `internal.unexpected`，但保留静态 domain/category/phase 和 correlation，
  不能只留下通用摘要，也不能复制 source chain。
- IPC、Operation Log、Runtime Log 与前端 `formatBackendError` 对同一次失败使用同一 stable code；
  任何一层退化都由 contract test 捕获。

### R4. 跨层 Correlation

- 在 IPC 调用起点生成受限 correlation ID，并贯穿 frontend recorder、Tauri command、domain operation、
  Operation Log 与 backend Runtime tracing。
- correlation 必须是一等可筛选语义；不能复用已有 `batch_id`，因为 batch grouping 与单次调用追踪语义
  不同。实现优先把预先生成的 Operation Log row ID 作为 operation correlation，避免增加同义数据库列；
  runtime-only failure 使用同格式的临时 correlation。查询、类型、导出和 generated docs 必须一致更新。
- 旧行没有 correlation 时继续可读；新前端连接旧 backend、旧前端连接新 backend 均安全退化。

### R5. Runtime Log 完整性

- Backend IPC boundary 对所有 fallible commands 记录一次安全失败事件，包含 command、code、category、
  phase、retryable、duration、target kind 与 correlation；renderer 的 `ipc.failure` 作为前端视角补充，
  不作为唯一证据。
- window error、unhandled rejection、显式 frontend runtime 事件、启动/恢复/后台 job 与受监督进程失败
  继续进入 Runtime Log；禁止全量代理 `console.*`。
- 同一失败不得因前后端都记录而在 UI 中表现为无法区分的重复项；来源与 correlation 必须清晰。
- 保持 14 天自动保留、文件名白名单、读取/导出 redaction 和 self-logging recursion guard。

### R6. 隐私与日志可靠性

- code/category/action/phase/status 使用受控字面量或 enum；动态值只允许经过字段级 allowlist 的逻辑 ID、
  数量、耗时和布尔值。
- PAT、API key、SSH 密码/密钥、host、username、绝对/相对路径、URL/ref/SHA、命令/env、stdout/stderr、
  文件内容、manifest/fingerprint、AI prompt/response 不得进入 IPC、Operation Log、Runtime Log 读取/导出。
- Operation Log 持久化前脱敏；Runtime Log 保持现有生命周期，但新增事件必须在写入前只构造安全 envelope，
  不能依赖读取时 redaction 才变安全。
- Operation Log 写入保持 best-effort，不得覆盖业务结果；写入失败必须在 Runtime Log 留下安全告警。

### R7. Observability Console 与详情小窗

- Operation Log 详情从右侧全高 drawer 改为约 560px 的居中 Dialog；小视口安全边距、视口内滚动，
  保留 Close/Escape、focus trap/restore、copy ID/JSON。
- 详情优先显示状态、公开原因、建议动作、code/category/phase/retryable/correlation，再显示紧凑元数据；
  安全 Details JSON 作为默认折叠的次级信息。
- Operation 与 Runtime 两层都支持按 correlation 搜索/跳转；同一 correlation 的重复前后端事件应易于辨认。
- 日志列表补充稳定 action/category 的本地化标签与失败诊断；旧行、invalid JSON 和 unknown code 有诚实 fallback。

### R8. 覆盖矩阵、测试与文档

- 建立 source-controlled command audit matrix，覆盖每个 `operation` command 的 success/failure/cancel/partial
  适用状态、target、稳定 code 和隐私字段。
- 后端测试覆盖 recorder lifecycle、correlation migration/filter/export、backend boundary、日志写入失败 fallback、
  stale started、redaction 对抗矩阵和所有 operation policy 命令的覆盖。
- 前端测试覆盖 correlation 注入/保真、双层关联检索、coded feedback、居中 Dialog、键盘/焦点/窄窗、
  长 JSON、复制、旧行 fallback 与中英文 parity。
- 更新 runtime observability 架构文档、backend/frontend/quality specs、IPC/data-model generated docs；最终运行
  targeted tests、typecheck、lint、format、Clippy、locked Rust tests、docs checks 与 `just ci`。
- Windows Tauri/WebView2 视觉、真实日志文件轮转/清理、异常退出后的 started 识别必须人工验证；未执行标为
  `UNVERIFIED`。

## Acceptance Criteria

- [x] AC1（R1）：全部 runtime commands（规划快照为 204 个）出现在唯一日志策略清单中，无未分类、
      重复或无理由排除项。
- [x] AC2（R2）：所有被分类为 `operation` 的命令均由统一 recorder 覆盖成功与失败；适用时覆盖
      partial/cancelled/started/interrupted，契约测试可阻止新增命令绕过。
- [x] AC3（R3）：代表性本地、SSH/WSL、DB、文件、HTTP、凭据和后台 job fixture 在 IPC、Operation、
      Runtime、DOM 中保持同一 stable code/category/phase/correlation。
- [x] AC4（R3/R6）：任何 raw `Display`、source chain 或敏感对抗种子均不进入用户日志或导出。
- [x] AC5（R4）：新 Operation Log 行有可索引 correlation；两层可按同一值筛选/跳转，旧行仍可查看。
- [x] AC6（R5）：每个 fallible backend IPC rejection 都有 backend Runtime evidence；前端 recorder 缺失或
      renderer 崩溃时仍可追踪，前后端重复事件来源明确。
- [x] AC7（R6）：日志记录失败不改变业务返回值，并产生安全 Runtime warning；清理/导出日志本身可审计。
- [x] AC8（R7）：详情在桌面居中紧凑、小视口不溢出；原因与下一步无需阅读 JSON，键盘/焦点/复制不退化。
- [x] AC9（R8）：覆盖矩阵与 registry parity、错误保真、redaction、migration、UI 回归测试通过，0 tests 不算通过。
- [ ] AC10（R8）：`just ci` 通过；原生 Windows 行为和异常退出证据分别报告 PASS/FAIL/`UNVERIFIED`。

## Out of Scope

- 继续调查或修复本次地址输入错误、SSH transport、服务端配置或特定 Update Center 业务逻辑。
- 远程 telemetry、云端日志上传、集中式日志服务或新的第三方日志依赖。
- 保存 raw command/stdout/stderr、目标地址、路径、凭据或用户内容以换取更多诊断。
- 回填或重写历史 Operation Log；旧行只要求兼容读取。
- 与日志覆盖无关的业务重构，或把 Runtime Log 与 Operation Log 合并为同一存储。

## Confirmed Product Decisions

- 用户已确认采用推荐边界：写操作、状态变更和可观察外部副作用进入长期 Operation Log；成功的纯读取、
  搜索、详情与 preview 不写 Operation Log，但其失败必须进入 Runtime Log 并带 correlation。
- 保留现有双层日志存储与 retention，不新增远程 telemetry 或全量 `console.*` 捕获。
- 日志详情继续从右侧全高 drawer 改为居中紧凑 Dialog。

## Task Map

| Child task | Deliverable | Dependency |
| --- | --- | --- |
| `08-26-observability-core-contracts` | command policy、稳定诊断、operation lifecycle、correlation interface | none |
| `08-26-audit-central-target-settings` | Central/targets/settings/security/log-admin coverage | core contracts |
| `08-26-audit-catalog-project-obsidian` | metadata/collections/projects/Obsidian/agents coverage | core contracts |
| `08-26-audit-marketplace-import-cli` | Marketplace/import/portable-state/Skills CLI coverage | core contracts |
| `08-26-runtime-diagnostics-correlation` | backend IPC Runtime evidence、frontend correlation 与去重 | core contracts |
| `08-26-observability-console-dialog` | correlation navigation、coded detail、居中 Dialog | core contracts + runtime diagnostics |
| `08-26-observability-governance-integration` | 全 registry 覆盖、文档、generated artifacts、CI/原生验收 | all preceding children |

## Risks and Deferred Items

- 204 命令的语义分类必须逐项人工复核；命令名前缀扫描只用于发现候选，不能替代领域判断。
- operation ID/correlation 与 started/final lifecycle 是跨层变更，应先由 core child 落地并保持旧行/旧前端兼容。
- 原生 UI、异常退出和真实文件轮转需要 Windows Tauri 证据，自动测试不能冒充现场验证。
