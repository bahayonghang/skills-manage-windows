# 开发与 PR 体验治理实施计划

## Steps

1. 获得实施批准后刷新 refs；在 `dev` 尚未保护时验证当前 1/1 divergence 无内容冲突，将 `origin/main` merge 回 `dev` 并推送，再从同步后的 `dev` 创建 `task/developer-pr-experience`，不得直接在 `main` 开发。
2. 添加 `.node-version`、`packageManager`/engine 与 `rust-toolchain.toml` 等工具链声明和合同测试，统一 package、Actions、文档中的 Node 22、pnpm 10.12.3、Rust 1.97.0。
3. 实现跨平台只读 doctor 脚本和 `just doctor`；测试 missing、major mismatch、exact mismatch、成功与 secret-redaction 行为。
4. 将 CI 子任务提供的 quick lane 暴露为 `just check`，在文档中明确它不能替代 `just ci`/`just audit`。
5. 新增 PR template，并补充模板内容合同测试。
6. 更新 README、README_CN、CONTRIBUTING、AGENTS 和质量 spec：task -> `dev` squash、promotion -> `main` merge commit、promotion 后 exact fast-forward、无 push CI、受控 bypass 与远端设置边界。
7. 完成本地验证、spec 更新和原子工作提交；推送任务分支、创建 `task/developer-pr-experience -> dev` PR，并取得 exact-head hosted CI，但暂不合并。
8. 展示 GitHub 当前值、目标值、实际 actor 类型/ID、两个 `dev` ruleset payload、repository PATCH、精确命令/API、副作用与回滚；单独获得远端设置批准后，依次建立/回读 `dev-safety`、建立/回读 `dev-flow`、更新/回读 merge settings 和 auto-delete。`main` protection 不修改。
9. 刷新 refs 并验证 task PR open、non-draft、`MERGEABLE/CLEAN` 与 exact-head checks 后 squash merge；确认 task branch 自动删除、`dev` 保留。归档与 journal 使用 flow bypass 分开提交，journal 只记录 squash delivery SHA。
10. 创建 `dev -> main` promotion PR，记录真实 timing；刷新门禁后使用 merge commit 与 `--match-head-commit` 合并。验证 merge parents 后，通过 flow bypass 将 `dev` 无 force地 fast-forward 到 `main` merge SHA，再写最终证据，确认 `dev` 存在、`gh-pages` 不存在且工作树干净。

## Focused Validation

```powershell
just doctor
just check
pnpm vitest run src/test/scripts/doctor.test.ts src/test/contracts/developerExperienceContract.test.ts src/test/contracts/ciWorkflowContract.test.ts
just ci
just audit
pnpm tauri build
git diff --check
```

远端验证记录 repository merge settings、`main` protection、两个 `dev` ruleset 与 bypass actor、合并后的 task branch、post-promotion `dev == main` SHA 和 promotion merge commit parents。若无远端授权，任务保持 in progress，不归档或误报远端 acceptance。

## Risk And Rollback Points

- 当前开发机 Node 25/pnpm 11 会被 doctor 正确报告 mismatch；doctor 不自动降级环境。
- 在确认不可 bypass 的 `dev-safety` deletion/non-fast-forward 规则生效前不得启用 auto-delete。
- ruleset 创建失败、actor bypass 超出 flow ruleset 或与现有 protection 冲突时停止远端变更；GitHub 无法自动强制 `main` 使用 merge commit，promotion 必须通过 `gh pr merge --merge --match-head-commit` 并显式检查 base/head/SHA。
