# CI 反馈路径提速设计

## 1. Shared Command Plan

`scripts/run-ci.mjs` 继续是本地与远端检查的唯一命令计划，新增显式 lane：

| Lane | Ownership |
| --- | --- |
| `quick` | version/generated checks、typecheck、lint、capability、size、entrypoint、IPC、Rust fmt；用于开发中快速反馈 |
| `common` | `quick` 加 Vitest、前端生产 build、文档 build；仅在一个 Ubuntu job 运行 |
| `rust-platform` | 当前 runner 的 all-target Clippy 和 locked Rust tests；Windows/Linux/macOS 各运行一次 |
| default/all | 本机并行运行 `common` 与 `rust-platform`，保持 `just ci` 完整门禁 |

平台 Rust lane 仍会编译部分相同代码，但其职责是验证真实 target、`cfg`、进程、路径和系统依赖，不把平台覆盖替换为字符串筛选或手工测试名单。

`scripts/sync-version.mjs --check` 只比较四处版本并失败，不写文件。`just ci` 和所有 hosted lanes 使用 check；只有 `just sync-version`、build/release 准备等显式入口可以写入版本元数据。

## 2. GitHub DAG

```text
common ─────────┐
windows-rust ───┤
linux-rust ─────┼──> ci (name: just-ci, always, fail closed)
macos-rust ─────┤
supply-chain ───┘
```

- 五个 required lanes 无 `needs`，在 runner 可用时同时排队。
- job id `ci`、job name/check context `just-ci` 保持稳定；汇总 job 不 checkout、不安装依赖、不重复测试，只检查所有 `needs.*.result == success`。
- `pull_request.branches` 覆盖 `dev` 和 `main`，支持任务 PR 与 promotion PR；不增加 push trigger。
- `workflow_call(checkout_ref)` 在冻结 SHA 上运行同一 DAG；manual package jobs 仍只在直接 `workflow_dispatch` 运行。
- concurrency 继续按 workflow/ref 取消旧 PR 运行；release reusable call 不允许被无关 PR 取消。

## 3. Timeouts, Cache, Observability

- common 15 分钟、supply-chain 10 分钟、Windows/macOS Rust 25 分钟、Linux Rust 20 分钟、汇总 5 分钟；package jobs 保留更长的显式 timeout。
- 继续使用 setup-node/pnpm cache 和按 runner/lockfile 隔离的 Rust cache；不让不可信 PR 写入发布缓存或读取发布凭据。
- runner 命令输出步骤耗时，并在 `GITHUB_STEP_SUMMARY` 写 lane、步骤、状态和秒数。汇总 job 输出失败/取消 lane 清单。
- 下一次真实任务 PR 和 promotion PR 分别记录 wall time、runner time、排队时间与 critical lane；15 分钟目标按 runner 开始后的活跃关键路径评估。

## 4. Failure Semantics

- 任一 lane failure/cancel/skipped 都使 `just-ci` 失败；只有明确不属于 required DAG 的 package job 可以 skip。
- 顶层 path filter 禁止使用，避免 required workflow 永久 Pending。
- Action pin、锁文件、最小权限、manual-only package 和 frozen checkout 均由 YAML 合同测试保护。

## 5. Compatibility And Rollback

远端 required check 无需迁移。若新 DAG 回归，可将 lane 内容回收到原 runner，同时保持 `ci`/`just-ci` 名称不变；不需要先修改 branch protection。`.trellis/spec/quality/ci-quality-gate.md` 必须与实际无 push、并行 lane 合同同步。
