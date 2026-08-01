# CI 反馈路径提速实施计划

## Steps

1. 用户批准最终规划后，刷新 refs，从最新 `origin/dev` 创建 `task/ci-feedback-acceleration`，设置 Trellis branch/base metadata，再运行 `task.py start`；记录起始 SHA 和干净树，不带入无关改动。
2. 新增 `syncVersion.test.ts`，先锁定全部目标漂移、`--check` 不写文件、写模式保持兼容和错误信息；实现 `--check`、`pnpm version:check`、`just version-check`，移除 `just ci` 的 mutating prerequisite。
3. 新增 `runCi.test.ts`，用注入执行器覆盖 lane 选择、未知 lane、default/all、失败传播、兄弟进程终止、计时与 summary；再把命令计划和 CLI 分离，将检查重组为 `quick`、`common`、`rust-platform`。
4. 先改写 `ciWorkflowContract.test.ts` 锁定五个无依赖 required lanes、fail-closed aggregate、stable context、timeouts、Action pin、无 push、frozen `workflow_call` 和 manual-only package，再更新 `.github/workflows/ci.yml` 使测试通过。
5. 更新 CONTRIBUTING、README/README_CN、AGENTS 和 CI quality spec，删除旧串行 DAG/push 描述，明确 `dev`/`main` PR triggers、lane ownership、只读版本检查和本地完整门禁。
6. 由 Trellis implement agent 完成实现后，使用 Trellis check agent 做全范围 spec/PRD/design/implement 检查并直接修复 findings；主会话复核 diff 和全部验证证据。
7. 运行聚焦测试、文档检查、`just ci`、`just audit`、`git diff --check` 与 Windows `pnpm tauri build`，核对默认配置真实生成的 NSIS bundle；由 workflow contract 和 bootstrap hosted run 继续证明 manual Windows package job 的 MSI 构建。验证命令后工作树不得出现静默生成物漂移。
8. 原子提交任务改动，推送任务分支并创建到 `dev` 的非 draft PR；在精确 head SHA 上 `workflow_dispatch` CI，等待并记录所有 required lanes、manual package jobs、wall/runner/queue 时间与结论。
9. 每次远端合并前刷新 refs，验证 PR open、non-draft、`MERGEABLE`、`CLEAN` 和 exact-head hosted checks；任务 PR 用 `--squash --match-head-commit` 合入 `dev`，随后删除短期分支。
10. 在 `dev` 上归档该子任务并让 journal 只引用实际交付 squash commit，分别提交 bookkeeping；创建 `dev -> main` promotion PR，记录新 DAG 的真实 PR lane timing，再用 `--merge --match-head-commit` 合入 `main`。最终确认 `dev` 存在、`gh-pages` 不存在、远端 task branch 已删除且本地工作树干净。

## Focused Validation

```powershell
pnpm version:check
pnpm vitest run src/test/scripts/syncVersion.test.ts src/test/scripts/runCi.test.ts
node scripts/run-ci.mjs --lane quick
node scripts/run-ci.mjs --lane common
node scripts/run-ci.mjs --lane rust-platform
pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts
pnpm docs:gen:check
pnpm docs:build
just ci
just audit
pnpm tauri build
Get-ChildItem src-tauri/target/release/bundle/nsis
git diff --check
git status --short
```

CI workflow 变更按项目要求运行 Windows `pnpm tauri build` 并确认默认 NSIS 产物路径；MSI 由保留的 manual package job 在 bootstrap hosted run 中真实构建。任务分支 bootstrap dispatch 必须绑定 exact head；promotion PR 必须验证 `just-ci` 的 app/context 未变化，且任一 required lane 的失败、取消或 skipped 会传播到汇总。

## Risk And Rollback Points

- 先提交共享 lane 命令与测试，再切换 YAML DAG，避免远端引用不存在的 lane。
- 不在同一变更中重命名 required context 或修改远端 protection。
- 首个 task PR 不会被 base `dev` 的旧 trigger 自动运行；不得把“无 PR checks”误报为通过，必须先取得 exact-head manual hosted run，再由 promotion PR 提供真实 PR timing。
- 若 hosted runner 超过目标，保留原始计时并定位 setup/compile/test；不通过删除平台覆盖伪造提速。
