# CI 反馈路径提速实施计划

## Steps

1. 扩展 `sync-version.mjs` 合同测试并实现 `--check`；新增 `version:check`/just recipe，将 `just ci` 从 mutating prerequisite 改为只读检查。
2. 为 `run-ci.mjs` 的 lane 选择、未知 lane、失败传播、计时摘要和 default/all 行为补充脚本测试。
3. 将现有检查重组为 `quick`、`common`、`rust-platform`，保持 default `just ci` 在当前平台覆盖全部检查。
4. 更新 `.github/workflows/ci.yml`：五个 lanes 并行、纯 `just-ci` 汇总、PR base 包含 `dev`/`main`、显式 timeout 和 summary；保留 frozen `workflow_call` 与 manual-only package。
5. 重写 `ciWorkflowContract.test.ts` 断言新 DAG、fail-closed aggregate、stable context、timeout、Action pin、无 push 和 package guard。
6. 更新 CONTRIBUTING、README/README_CN、AGENTS 和 CI quality spec，删除旧串行 DAG/push 描述。
7. 创建真实 task PR 到 `dev` 观察 lane 数据；promotion 时再次记录并比较，不以本地热缓存替代 hosted 结果。

## Focused Validation

```powershell
pnpm version:check
node scripts/run-ci.mjs --lane quick
node scripts/run-ci.mjs --lane common
node scripts/run-ci.mjs --lane rust-platform
pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts
just ci
just audit
git diff --check
```

CI/release workflow 变更按项目要求额外运行 Windows `pnpm tauri build` 并确认 NSIS/MSI 产物路径。真实 PR 必须验证 `just-ci` 的 app/context 未变化，且任一 lane 的失败/取消会传播到汇总。

## Risk And Rollback Points

- 先提交共享 lane 命令与测试，再切换 YAML DAG，避免远端引用不存在的 lane。
- 不在同一变更中重命名 required context 或修改远端 protection。
- 若 hosted runner 超过目标，保留原始计时并定位 setup/compile/test；不通过删除平台覆盖伪造提速。
