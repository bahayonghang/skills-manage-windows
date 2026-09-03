# Implementation Plan

本文件只定义后续实施步骤；当前任务保持 `planning`，不运行在线审计、不升级依赖。

## Steps

1. Freeze the policy plane [R1]
   - Files/symbols：`src/test/contracts/dependencyAuditContract.test.ts::{AuditResult,evaluate,dependency audit policy}`；`check-dependency-audit.mjs::evaluateDependencyAudit`。
   - 先扩展现有 fixture，明确记录 npm high/critical、Cargo vulnerability、精确/过期/未使用例外和 malformed report 的当前结果。
   - 定向验证：`pnpm exec vitest run src/test/contracts/dependencyAuditContract.test.ts`。
   - Rollback point：只回退新增 baseline assertions；禁止修改 exception manifest 或 blocker severities 以通过测试。

2. Add bounded evidence normalization [R2][R3][R5]
   - Files/symbols：`check-dependency-audit.mjs::{normalizeNpmEvidence,normalizeCargoEvidence,summarizeEvidence,evaluateDependencyAudit}`；同一 contract test 的 warning/duplicate/overflow/provenance fixtures。
   - 保持 normalizer 私有、固定上限、稳定 key；缺失 advisory 明确渲染 `unknown`，不建立 closed warning validator。
   - 定向验证：`node --check scripts/check/check-dependency-audit.mjs`；`pnpm exec vitest run src/test/contracts/dependencyAuditContract.test.ts`。
   - Rollback point：成组回退新增 evidence 字段与纯函数；policy plane 不动。

3. Wire console and Step Summary [R3][R4][R6]
   - Files/symbols：`check-dependency-audit.mjs::{runDependencyAudit,CLI main}`；contract test 的临时 summary path 与 write-failure fixture。
   - 两个渠道消费同一 summary model；仅检测既有 `GITHUB_STEP_SUMMARY`，写失败阻止 Passed；warning finding 自身不改变退出码。
   - 定向验证：`pnpm exec vitest run src/test/contracts/dependencyAuditContract.test.ts`；在测试临时目录内验证有/无环境变量两条路径。
   - Rollback point：单独回退 renderer/I/O 接线；不保留只写控制台却声称 Step Summary 成功的中间状态。

4. Close deterministic and external evidence [R1-R6]
   - 运行总验证块，确认 `security/dependency-audit-exceptions.json` 与所有 lockfile 无 diff。
   - `just audit` 可能访问外部 registry；仅在实施授权允许该既有入口时运行，并记录在线/缓存状态。不可用时报告 skipped/`UNVERIFIED`，不得用安装新工具补齐。

## Total Verification

```powershell
node --check scripts/check/check-dependency-audit.mjs
pnpm exec vitest run src/test/contracts/dependencyAuditContract.test.ts
pnpm typecheck
just audit
just ci
git diff --check -- scripts/check/check-dependency-audit.mjs src/test/contracts/dependencyAuditContract.test.ts
```

## Human and External Evidence

- 人工：在一份受控 fixture 输出中检查控制台与 Step Summary 的 total/shown/truncated、排序和 Markdown 可读性。
- 外部：`just audit` 的联网数据新鲜度、upstream advisory 当前状态和可升级版本均不由 fixture 证明；未获准或网络不可用时标记 `UNVERIFIED`。

## Final Rollback Point

最终 task commit 仅修改 audit wrapper 与其 contract test。任何 blocker/exception/exit-code 回归都整体 revert；不得附带 lockfile、依赖版本、例外清单或 workflow 权限变更。
