# 开发与 PR 体验治理实施计划

## Steps

1. 添加工具链声明与合同测试，统一 package/Actions/文档中的 Node 22、pnpm 10.12.3、Rust 1.97.0。
2. 实现跨平台只读 doctor 脚本和 `just doctor`；测试 missing、major mismatch、exact mismatch、成功与 secret-redaction 行为。
3. 将 CI 子任务提供的 quick lane 暴露为 `just check`，在文档中明确它不能替代 `just ci`/`just audit`。
4. 新增 PR template，并补充模板内容合同测试。
5. 更新 README、README_CN、CONTRIBUTING、AGENTS 和质量 spec：task -> `dev` squash、promotion -> `main` merge commit、无 push CI、远端设置边界。
6. 在代码与文档合并后，展示 GitHub 当前/目标 merge settings、`dev`/`main` protection 和 branch rulesets；取得执行确认后先建立 `dev` 的 PR/check/linear-history/force/delete rules，再启用 repository auto-delete 与 squash/merge、关闭 rebase 并回读。
7. 用一个真实 task PR 和后续 promotion PR 验证 base、check、merge method、task branch 删除和 `dev` 保留。

## Focused Validation

```powershell
just doctor
just check
pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts
just ci
just audit
git diff --check
```

远端验证记录 repository merge settings、`main` protection、`dev` protection、合并后的 task branch、`dev` SHA 和 promotion merge commit parents。若无远端授权，本地实现可以完成，但远端 acceptance 保持未验证。

## Risk And Rollback Points

- 当前开发机 Node 25/pnpm 11 会被 doctor 正确报告 mismatch；doctor 不自动降级环境。
- 在确认 `dev` protection 生效前不得启用 auto-delete。
- ruleset 创建失败或与现有 protection 冲突时停止远端变更；GitHub 无法自动强制 `main` 使用 merge commit，promotion 必须通过 `gh pr merge --merge --match-head-commit` 并显式检查 base/head/SHA。
