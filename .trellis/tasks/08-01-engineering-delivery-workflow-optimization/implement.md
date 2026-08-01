# 工程交付流程优化实施计划

## Execution Order

1. 实施并归档 `08-01-docs-deployment-generated-integrity`。
2. 实施并归档 `08-01-ci-feedback-acceleration`。
3. 实施并归档 `08-01-developer-pr-experience`。
4. 实施并归档 `08-01-desktop-release-assurance` 的本地与 rehearsal 部分。
5. 汇总四个子任务证据，更新质量 spec，执行父任务集成验收。

## Per-Child Gate

每个子任务必须按以下顺序完成：

1. 从独立 task branch 开始，读取该子任务 manifest 和相关质量 spec。
2. 先运行最小合同测试，再实现该子任务范围。
3. 运行聚焦检查、`just ci`、`just audit`；打包/发布子任务额外运行 Windows `pnpm tauri build` 并核对产物。
4. 检查 tracked/untracked diff，确认没有静默生成物漂移或无关文件。
5. 完成独立检查、spec 更新、提交、归档和 journal 后再进入下一子任务。

## Remote Gates

- Pages、merge settings、分支保护、release environment、Azure 配置和任何 release/tag 写入在执行前再次请求精确授权。
- 仓库设置更新后通过 GitHub API 回读；真实 PR/Pages/rehearsal 证据不能由本地模拟替代。
- 未配置 Azure 或未批准公开发布时，相关验收保持 `deferred`/`not-configured`，不得记为通过。

## Final Validation

```powershell
pnpm docs:gen:check
pnpm docs:build
pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts src/test/contracts/releaseWorkflowContract.test.ts
just ci
just audit
pnpm tauri build
```

最终还需记录真实 PR 的 wall time/runner time/各 lane 时间、Pages HTTP 200 与页面身份、远端设置回读，以及 rehearsal artifact inventory。父任务本身不新增产品代码提交。
