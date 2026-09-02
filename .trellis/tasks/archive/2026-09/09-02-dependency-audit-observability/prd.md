# 依赖审计非阻断告警可见性

## Goal

保留现有阻断策略，同时让 npm moderate/low 与 Cargo unmaintained/unsound 等非阻断发现以有界、可复现的证据持续可见。

## Findings

- `QUAL-003`（Medium / S）：`scripts/check/check-dependency-audit.mjs:91-140,204-205` 只归一化 npm high/critical 与 Cargo vulnerabilities，成功输出丢弃 npm moderate/low 和 Cargo `warnings`。
- 审计时离线 `cargo audit --no-fetch` 可见 18 个 unmaintained、3 个 unsound warning；两个既有 vulnerability 例外仍由 `security/dependency-audit-exceptions.json` 的精确规则管理。

## Requirements

- R1： [QUAL-003] `evaluateDependencyAudit` 的既有 blocker、精确例外、例外过期/未使用和 malformed-report 判定及退出语义保持不变；本任务不得把 warning 升级为 blocker。
- R2： [QUAL-003] npm moderate/low 与 Cargo informational warnings 归一为只读 evidence finding，至少包含 ecosystem、advisory id（缺失时明确 `unknown`）、package、version、category/severity 和 occurrence count。
- R3： [QUAL-003] evidence finding 按稳定 key 去重、确定性排序，并由代码内固定上限截断；控制台和 GitHub Step Summary 都必须显示总数、展示数和截断数。
- R4： [QUAL-003] 本地运行始终输出简短控制台摘要；仅当 `GITHUB_STEP_SUMMARY` 存在时追加同一证据的 Markdown，不新增 workflow 权限、输入或配置面。
- R5： [QUAL-003] 输出明确标记数据来源和 currentness；未获得可验证在线更新时间时使用 `currentness: unverified`，不得把缓存/离线结果描述为 registry 最新状态。
- R6： [QUAL-003] 审计命令启动失败、空/非法 JSON、malformed blocker 数据或已声明 Step Summary 文件写入失败均是基础设施错误并保持非零；这些错误不得输出 “Passed” 或伪装成零发现。

## Acceptance Criteria

- [x] AC1（R1）：npm high/critical fixture 的 `errors` 与 `blockers` 保持原行为。
- [x] AC2（R1）：Cargo vulnerability fixture 的 `errors` 与 `blockers` 保持原行为。
- [x] AC3（R1）：当前精确例外 fixture 的 `approved` 与退出结果保持原行为。
- [x] AC4（R1）：过期例外 fixture 继续失败。
- [x] AC5（R1）：未使用例外 fixture 继续失败。
- [x] AC6（R1）：只含 npm moderate/low 或 Cargo unmaintained/unsound 的 fixture 返回零 policy errors，并且进程成功退出。
- [x] AC7（R2）：npm moderate/low fixture 的结构化 evidence 包含 advisory、package、version、severity 和 count。
- [x] AC8（R2）：Cargo unmaintained/unsound fixture 的结构化 evidence 包含 advisory（若源数据缺失则为 `unknown`）、package、version、category 和 count。
- [x] AC9（R3）：乱序重复 fixture 得到字节稳定的去重排序结果。
- [x] AC10（R3）：超量 fixture 显示 total/shown/truncated，并且输出行数不超过固定上限。
- [x] AC11（R4）：无 `GITHUB_STEP_SUMMARY` 时只写控制台且不创建 summary 文件。
- [x] AC12（R4）：存在有效 `GITHUB_STEP_SUMMARY` 路径时追加 Markdown，且计数与控制台一致。
- [x] AC13（R5）：fixture 未提供可验证更新时间时，控制台与 Step Summary 均出现 `currentness: unverified`。
- [x] AC14（R5）：currentness 未验证时，输出不包含 `latest` 或等价声明。
- [x] AC15（R6）：命令启动失败、空 JSON 和非法 JSON 参数化 fixture 均非零。
- [x] AC16（R6）：malformed blocker report fixture 非零。
- [x] AC17（R6）：已声明 Step Summary 文件写入失败 fixture 非零。
- [x] AC18（R6）：AC15-AC17 的输出均不包含 `[audit] Passed`。
- [x] AC19（R1, R6）：`src/test/contracts/dependencyAuditContract.test.ts` 与 `just audit` 通过，且没有 exception manifest/lockfile diff。
- [x] AC20（R5）：实时 npm registry/RustSec 最新状态仅在获准在线复测后记录，否则明确标为 `UNVERIFIED`。

## Out of Scope

- 升级/降级依赖，修改 lockfile，或新增 audit 工具/依赖。
- 修改风险阈值、例外清单、许可证策略或 CI 阻断规则。
- 增加新的 CLI flag、环境配置、遥测或持久化 advisory 数据库。
