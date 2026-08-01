# CI 反馈路径提速

## Goal

保持稳定 required check 和跨平台风险覆盖，同时减少平台无关重复计算与串行等待，使 PR 更快得到可合并结论。

## Requirements

1. 保持 GitHub check context `just-ci` 稳定；最终汇总 job 必须在所有 required lanes 完成后 fail closed。
2. common、Windows、Linux、macOS、supply-chain lanes 并行调度；common 负责平台无关前端、文档、格式、静态契约与生成物检查，平台 lanes 只运行必须在真实 target 编译/执行的 Rust Clippy 和 tests，不重复前端/文档链。
3. 继续使用锁文件、完整 SHA Action pin、最小权限和当前 PR concurrency cancellation。
4. routine PR 不构建安装包；manual smoke 和 release package ownership 不变。
5. 每个 job 设置与历史耗时相称的 `timeout-minutes`，并在 job summary 输出关键步骤用时与失败 lane。
6. `sync-version` 增加只读 check；`just ci` 不再静默修复版本元数据。
7. workflow contract 测试覆盖新 DAG、stable aggregate、平台矩阵、manual-only package、timeouts 和 action pins。
8. 不使用会让 required workflow 永久 Pending 的顶层 path filter；后续 affected classification 必须在始终触发的 workflow 内 fail safe。

## Acceptance Criteria

- [ ] contract 测试证明 common、Windows、Linux、macOS、supply-chain 没有串行依赖，`just-ci` 只做稳定汇总。
- [ ] 任一 required lane 失败或取消都会使 `just-ci` 失败，其他不必要工作不会因汇总结构被强制串行等待。
- [ ] `just ci` 与 GitHub lanes 的命令来源一致，不产生两套语义不同的质量门。
- [ ] 本地聚焦测试、`just ci`、`just audit` 通过。
- [ ] 下一次真实 PR 记录 wall time、runner time 和各 lane 用时，用于验证 15 分钟活跃关键路径目标。

## Out of Scope

- 删除 macOS/Linux 平台覆盖。
- 在 routine PR 中加入全平台安装包构建。
- 当前阶段引入 merge queue、self-hosted runner 或不受信任缓存写入发布 job。
