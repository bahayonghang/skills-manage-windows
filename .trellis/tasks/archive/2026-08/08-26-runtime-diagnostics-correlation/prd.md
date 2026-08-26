# Runtime 诊断、IPC 失败保真与去重

状态：**in_progress**。依赖：`08-26-observability-core-contracts` 完成并冻结 interface。

## Goal

让每个 fallible IPC rejection 在 backend 留下安全、结构化、可关联的 Runtime evidence；让 frontend
recorder保留同一 correlation并清除 raw message/stack/path风险，同时让双视角事件可辨认而非混成重复噪声。

## Requirements

- R1：通用 backend IPC failure boundary 对全部 fallible commands写一次 code/category/phase/retryable/duration/
  target kind/operation ID事件；不依赖 renderer recorder。
- R2：operation failure复用 Operation ID；runtime-only failure生成临时同格式ID；`IpcError.correlationId`
  返回给 frontend，旧 backend 缺失时安全fallback。
- R3：frontend `ipc.failure`只记录 command名、reviewed error envelope、source与correlation；args内所有字符串
  继续替换，不能记录raw rejection object/source chain。
- R4：window error/unhandled rejection不记录 filename、stack、动态code或任意message/reason；只记录受控source、
  allowlisted Error name和安全数字，未知字符串固定化。
- R5：backend source标记`backend`、frontend source标记`frontend`；Runtime DTO/parser提取operation ID，
  UI可按ID聚合/筛选但不删除任一视角。
- R6：`record_frontend_runtime_log`绕过failure recorder，self-logging失败不递归；14-day retention、whitelist、
  read/export redaction与pagination不退化。
- R7：业务command/service文件的dynamic tracing由各coverage child迁移；本child提供shared安全interface并禁止
  新的raw tracing入口，最终governance child做全仓验收。

## Acceptance Criteria

- [x] 每个 backend fallible IPC fixture产生一条安全 backend failure event并返回同一correlation。
- [x] frontend recorder存在/缺失/失败时，backend evidence均不受影响；self-logging无递归。
- [x] 同一 rejection 的 backend/frontend Runtime lines有同一ID、不同source、相同code且可筛选。
- [x] filename/stack/path/URL/token/args/raw reason对抗种子不进入Runtime磁盘新事件、read/export或DOM fixture。
- [x] legacy rejection/no-correlation、unknown error、infallible command、browser fixture路径安全退化。
- [x] runtime logging、IPC wrapper、store/parser targeted tests与retention/export regressions通过。

## Out of Scope

- 业务domain Operation Log wiring；由coverage children负责。
- Runtime/Operation关联UI和详情Dialog；由console child负责。
- 全量console代理、remote telemetry或改变Runtime retention。
