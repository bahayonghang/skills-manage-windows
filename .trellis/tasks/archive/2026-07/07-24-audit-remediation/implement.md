# 父任务集成复核

> 复核日期：2026-07-28。父任务不承载产品代码；本文件记录全部子任务归档后的本地集成证据。

## 1. 子任务闭环

- [x] `07-24-net-boundary-ssrf`
- [x] `07-24-renderer-capability-min`
- [x] `07-24-target-context-snapshot`
- [x] `07-24-remote-process-supervisor`
- [x] `07-24-remote-path-canonical`
- [x] `07-24-db-stale-cleanup-fix`
- [x] `07-24-db-schema-versioning-fk`
- [x] `07-24-fs-db-operation-journal`
- [x] `07-24-job-concurrency-lease`
- [x] `07-24-release-pipeline-gate`
- [x] `07-24-ci-supply-chain`
- [x] `07-24-settings-domainization`
- [x] `07-24-github-preview-snapshot`
- [x] `07-24-startup-resilience`
- [x] `07-24-typed-ipc-migration`
- [x] `07-24-size-budget-debt`

所有 16 个子任务均位于 `.trellis/tasks/archive/2026-07/`，其 `task.json`
均为 `status: completed`；`task.py list --mine` 不再报告活动子任务。

## 2. 跨任务验收

- [x] 最终本地 `just ci` 通过：web 链于 57.8 秒完成，Rust 链于 138.8 秒完成，输出
  `[ci] All checks passed.`。
- [x] 父任务 `task.py validate` 通过；每个子任务归档前均保留自己的定向验证与交付记录。
- [x] P3-01 直接计数：`central_updates/core.rs` 470、`commands/collections.rs` 332、
  `db/seed.rs` 202、`CentralSkillsView.tsx` 599、`UnifiedSkillCard.tsx` 595，全部低于
  本次清债目标 600。
- [x] `pnpm sizecheck` 对 611 个生产源文件执行统一 800 行限制；提交后的 policy 中没有
  `BASELINE_ALLOWLIST`。
- [x] 对最后子任务运行了任务范围 `git diff --check`、独立审阅、类型检查、lint、定向 Rust/
  前端测试和完整 CI；抽取后的 Rust 重导出、页面组合和唯一 `UnifiedSkillCard` 入口保持不变。

## 3. 收尾边界

- 不创建远程 PR，不推送，不修改未纳入该父子任务的 Trellis 工具、`.gitattributes` 或审计报告。
- 子任务日志只引用其实际交付提交 `a9a337f3`；父任务归档仅记录集成复核，不把 archive/journal
  bookkeeping SHA 伪装为产品交付。
