# Implementation Plan: src-tauri 优化任务树

## 0. Planning gate

- [x] 在 `dev@b242ed92` 完成代码、spec、测试和旧任务增量审计。
- [x] 保存 `research/src-tauri-deep-audit.md`。
- [x] 创建 5 个可独立验收的 child，并写明依赖。
- [x] 六个任务均通过 `task.py validate`，规划基线 `just ci` 通过。
- [ ] 用户评审父任务与 child 规划。
- [ ] 只在用户明确批准后，对选定 child 执行 `task.py start`；不要 start 父任务。

## 1. Child execution order

1. `08-03-marketplace-install-central-contract`
   - release blocker；先删除危险旁路并建立 registry-backed import authority。
2. `08-03-bounded-github-snapshot-lifecycle`
   - 可在 P0 后独立实施；避免与 input helper 同时修改同一 snapshot type 时并行落码。
3. `08-03-bounded-external-text-ingestion`
   - 以 P0 后剩余 HTTP surfaces 为准；先 shared helper，再 AI/tree/file call sites。
4. `08-03-transactional-metadata-mutations`
   - repositories/tags 与 Marketplace sync 分阶段提交；不要接管 Marketplace install。
5. `08-03-sql-central-pagination`
   - 先 reference/perf fixture，后 query/index，最后删除 page path 的全量 enrichment。

Snapshot、transaction 和 pagination 在文件所有权不重叠时可独立排期；树结构不代表依赖，以上顺序以各 child 文档为准。

## 2. Per-child review gate

- [ ] 重新读取 child PRD/design/implement 与 context manifests。
- [ ] 记录实现前代码基线；确认没有被其他 task 改写关键假设。
- [ ] 先补失败回归或 benchmark/reference oracle。
- [ ] 按 child rollback point 分小步实现，保持 IPC/CLI/schema 兼容。
- [ ] 跑定向测试，再跑 Rust fmt、all-targets locked Clippy、locked tests。
- [ ] 涉及 schema/IPC 时运行 `pnpm docs:gen` 并检查两份 generated docs。
- [ ] 运行 `just ci`，检查 final diff，提交并按 Trellis 生命周期 archive child。

## 3. Parent integration review

- [ ] 五个 child 均已完成或有用户明确接受的风险记录。
- [ ] 搜索并确认旧 Marketplace direct writer、无界 response readers、unbounded cache path、page full-enrichment path 和非事务 mutation 已消失。
- [ ] 审核 Central lock / DB transaction / remote cleanup 的获取顺序，无嵌套自锁或连接泄漏。
- [ ] 审核资源常量无重复定义，typed error 与 redaction 仍符合 spec。
- [ ] 对生成 docs、architecture docs 和活动 backend specs 做 drift 检查。
- [ ] 最终 `just ci` 通过；若任何平台/GUI/field evidence 未运行，明确保留为 open gate。
- [ ] 写 final cross-child acceptance，再 archive 父任务并记录 session。

## 4. Rollback rule

每个 child 独立回滚，不回滚其他 child 或用户 WIP。任何涉及 persisted schema/data 的回滚必须保持新旧数据均可读；remote workspace cleanup 与 Central filesystem mutation 不允许通过删除未知目录“恢复”。
