# CI 反馈路径提速

## Goal

保持稳定 required check 和跨平台风险覆盖，同时减少平台无关重复计算与串行等待，使 PR 更快得到可合并结论。

## Background

- 当前 `.github/workflows/ci.yml` 只监听指向 `main` 的 PR；`source-validation` 的 Ubuntu/macOS 全量链和 supply-chain 完成后，Windows `just-ci` 才运行另一遍全量链。
- promotion PR #28 的 run `30680246438` wall time 为 32 分 24 秒，Windows `just-ci` 在前置 lanes 完成后单独运行 16 分 34 秒，是本子任务的现场基线。
- 当前 `just ci` 以写入型 `sync-version` 为 prerequisite；`scripts/run-ci.mjs` 只有 default 全量模式且没有脚本级回归测试。
- 首个任务 PR 的 base `dev` 仍使用“仅监听 main”配置，因此该 PR 无法由自身新增的 `dev` filter 触发。首个 exact-head hosted 证据必须使用任务分支 `workflow_dispatch`，真实 PR lane timing 则由随后的 `dev -> main` promotion PR 记录；此后指向 `dev` 的子任务 PR 才直接运行 PR CI。

## Requirements

1. 保持 GitHub check context `just-ci` 稳定；最终汇总 job 必须在所有 required lanes 完成后 fail closed。
2. common、Windows、Linux、macOS、supply-chain lanes 并行调度；common 负责平台无关前端、文档、格式、静态契约与生成物检查，平台 lanes 只运行必须在真实 target 编译/执行的 Rust Clippy 和 tests，不重复前端/文档链。
3. 继续使用锁文件、完整 SHA Action pin、最小权限和当前 PR concurrency cancellation。
4. routine PR 不构建安装包；manual smoke 和 release package ownership 不变。
5. 每个 job 设置与历史耗时相称的 `timeout-minutes`，并在 job summary 输出关键步骤用时与失败 lane。
6. `sync-version` 增加只读 check；`just ci` 不再静默修复版本元数据。
7. workflow contract 测试覆盖新 DAG、stable aggregate、平台矩阵、manual-only package、timeouts 和 action pins。
8. 不使用会让 required workflow 永久 Pending 的顶层 path filter；后续 affected classification 必须在始终触发的 workflow 内 fail safe。
9. 使用独立 `task/ci-feedback-acceleration` 分支，工作提交 squash 合入 `dev`；归档与 journal 完成后，`dev -> main` 通过 exact-head merge-commit promotion PR 晋级，短期分支删除且 `dev` 保留。

## Acceptance Criteria

- [x] contract 测试证明 common、Windows、Linux、macOS、supply-chain 没有串行依赖，`just-ci` 只做稳定汇总。
- [x] 任一 required lane 失败或取消都会使 `just-ci` 失败，其他不必要工作不会因汇总结构被强制串行等待。
- [x] `just ci` 与 GitHub lanes 的命令来源一致，不产生两套语义不同的质量门。
- [x] `sync-version --check` 和 `just ci` 在版本漂移时失败且不修改任何 tracked file；脚本测试覆盖全部漂移路径和只读行为。
- [x] `run-ci` 脚本测试覆盖 lane 选择、未知 lane、default/all、失败传播、兄弟进程终止、计时与 summary 输出。
- [ ] 本地聚焦测试、workflow contract、`just ci`、`just audit`、`git diff --check` 与 Windows `pnpm tauri build` 通过，并核对默认配置实际生成的 NSIS bundle；manual Windows package job 的 MSI 构建合同与 hosted 结果保持通过。
- [ ] 任务分支 exact-head `workflow_dispatch` 全部 required lanes 通过；promotion PR 记录 wall time、runner time、排队时间和各 lane 用时，用于验证 15 分钟活跃关键路径目标。
- [ ] 若目标未达到，保留原始数据、critical lane 和后续结论，不删除跨平台、供应链、打包或 fresh-download 覆盖来换取绿色结果。
- [ ] task PR squash 合入 `dev`，promotion PR 以 merge commit 合入 `main`；两次合并都使用刷新后的 exact head，短期分支已删除，`dev` 与 legacy `gh-pages` 状态未改变，任务已归档且 journal 只记录实际交付提交。

## Local Evidence (2026-08-01)

- 聚焦脚本与 workflow contract：3 files / 20 tests passed；取消进程树用例连续 5 次通过。
- 显式 lanes：quick 9.2s、common 46.0s、Windows rust-platform 96.4s，均通过。
- `just ci`：100.7s；Vitest 143 files / 1590 passed / 1 skipped，Rust 主测试 1031 passed / 6 ignored，其他 Rust integration/doc tests 通过。
- `just audit`：通过；2 个 blocking advisories 均由 2 个当前有效的精确例外覆盖。
- `pnpm docs:gen:check`、`pnpm docs:build`、`pnpm typecheck`、`pnpm lint`、`git diff --check`：通过且未产生 tracked 生成物漂移。
- Windows `pnpm tauri build`：340.4s；本次生成 `src-tauri/target/release/bundle/nsis/SkillPort_0.10.14_x64-setup.exe`，12,284,876 bytes。hosted MSI 与 exact-head DAG 仍等待任务分支 dispatch 证据。

## Out of Scope

- 删除 macOS/Linux 平台覆盖。
- 在 routine PR 中加入全平台安装包构建。
- 当前阶段引入 merge queue、self-hosted runner 或不受信任缓存写入发布 job。
- 修改 GitHub merge settings、ruleset、branch protection、environment、Secrets、tag、release 或 Pages 设置。
