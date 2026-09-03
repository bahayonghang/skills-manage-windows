# 实施计划：父任务集成复核

> 本文件仅描述顺序；本父任务不执行产品修改，也不运行 `task.py start`。父任务保持 `planning`，不归档。

1. [x] （R1, R4）独立 review 本父任务与 12 个 child 的最新规划摘要。12 个 child 已归档；父任务保持 `planning`。
2. [x] （R2, R3）先执行 P0 `github-import-fs-db-atomicity`，验证本地/远程 DB 故障、恢复失败和并发互斥。
3. [x] （R2, R3）执行四个 P1：`trellis-path-security`、`windows-release-signing`、`collection-search-correctness`、`usage-refresh-failure-integrity`。签名 child 按用户范围 1 fail-closed，REL-001/REL-002 保持 open。
4. [x] （R2, R5）在签名边界稳定后执行 `windows-installer-verification`；真实 windows-2022 NSIS/MSI 生命周期仍 `UNVERIFIED`。
5. [x] （R1, R2）执行其余 P2 child；`typed-ipc-remainder`、`backend-boundary-ratchets` 与 types barrel 分别维持自己的 ratchet 基线。
6. [x] （R3, R5）父级 `just ci`、`just audit`、`pnpm docs:gen:check` 已在 12/12 归档后重跑；证据与 UNVERIFIED 边界见 `research/integration-2026-09-03.md`。
7. [x] （R1）ledger 28 个 ID：26 `fixed`（含 QUAL-002 contract/fixture 与 ARCH-003/004 渐进），2 `open (fail-closed)`（REL-001/REL-002）。未标 `wontfix`。
8. [x] （R4）父任务 planning artifacts 入库；**不**归档父任务、**不** push。REL-001/REL-002 与原始扫描重跑仍阻塞目标完成。
