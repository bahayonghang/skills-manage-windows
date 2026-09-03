# Design

## Change List

- `scripts/check/check-dependency-audit.mjs::normalizeNpmBlockers` 与 `normalizeCargoBlockers`：行为保持不变，仅继续负责 policy blockers。[R1]
- `scripts/check/check-dependency-audit.mjs` 新增同文件私有函数 `normalizeNpmEvidence`、`normalizeCargoEvidence`、`summarizeEvidence`：从原始报告提取非阻断 finding，去重、排序和固定上限截断；不建立新模块。[R2][R3]
- `scripts/check/check-dependency-audit.mjs::evaluateDependencyAudit`：在现有返回值上增加只读 `evidence`/`provenance`，不得让 evidence 参与 `errors` 或 exception matching。[R1][R2][R5]
- `scripts/check/check-dependency-audit.mjs::runDependencyAudit` 与 CLI main：阻断判定完成后渲染控制台；存在 `GITHUB_STEP_SUMMARY` 时用同一 summary model 追加 Markdown，最后再决定 Passed/非零。[R3][R4][R6]
- `src/test/contracts/dependencyAuditContract.test.ts`：扩展类型与 fixture，覆盖 policy 不变、两生态 warning、排序/去重/上限、currentness、控制台/Step Summary 和 I/O 失败。[R1-R6]

## Contract

1. Policy plane：`blockers`、`approved`、`errors` 继续由现有 blocker normalizer 与 exception manifest 唯一决定。[R1]
2. Evidence plane：warning normalizer 只消费 npm 非阻断 severity 与 Cargo `warnings`；固定 key 为 `ecosystem:category:advisory:package:version`，同 key 只增加 occurrence count。[R2]
3. Rendering：`summarizeEvidence` 先按 ecosystem/category/advisory/package/version 排序，再取代码内固定 `MAX_EVIDENCE_ROWS`；控制台和 Step Summary 消费同一结果，不各自重算。[R3][R4]
4. Provenance：输出实际命令标签和报告提供的时间字段；没有可校验时间就固定为 `currentness: unverified`，不额外发起网络探测。[R5]
5. Failure direction：warning 本身不改变 exit code；采集命令/JSON/policy 格式错误沿用非零，已声明 Step Summary 写失败作为输出基础设施错误非零，且 “Passed” 只在所有这些错误为空后写出。[R1][R6]

## Compatibility

- `evaluateDependencyAudit` 保留现有 `errors`、`blockers`、`approved` 字段及含义，只追加字段；既有调用方不需要迁移。
- `security/dependency-audit-exceptions.json` 不修改，exception key 与 expiry 规则不变。
- 本地没有 `GITHUB_STEP_SUMMARY` 时不写文件；GitHub job 不需要新权限或 workflow 变更。
- 固定上限是输出保护常量，不形成用户配置面。

## Verification Boundary

- 自动验证：归一化字段、policy 结果不变、稳定排序/去重/截断、两个输出渠道、currentness 标签、命令/JSON/I/O 失败方向。[AC1-AC19]
- 人工检查：CI Step Summary Markdown 可读性，以及控制台在上限内仍能定位 advisory。
- 外部证据：registry/RustSec 数据是否为实时最新、upstream advisory 正确性和依赖可升级性均为 `UNVERIFIED`，除非单独授权在线审计。[AC20]

## Rollback

- Rollback A：evidence normalizer 与纯函数 fixture 可成组回退，不触碰 policy plane。
- Rollback B：控制台/Step Summary renderer 与 I/O tests 可独立回退，`evaluateDependencyAudit` 仍保持原 policy 返回。
- Rollback C：若任何既有 blocker fixture 漂移，停止并整体回退本 task；不得用调整阈值或例外清单“修复”测试。

## Considered but Not Chosen

- 不把 moderate/low、unmaintained/unsound 升级为失败：这会改变已确认的治理阈值。
- 不新增 SARIF、数据库或 artifact：控制台与现有 Step Summary 已满足当前可见性目标。
- 不增加在线 freshness probe 或 CLI mode：它会扩大网络行为和配置面，且不能保证第三方数据库真实最新。
