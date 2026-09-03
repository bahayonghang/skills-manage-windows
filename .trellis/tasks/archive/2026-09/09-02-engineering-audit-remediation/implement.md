# 实施计划：父任务集成复核

> 本父任务不修改产品代码。Goal 授权后已 `task.py start` 以便独立 check / 扫描复跑。父任务不归档，直到 29 个 ID 无未关闭项且发布门禁通过。

1. [x] （R1, R4）独立 review 本父任务与 12 个 child 的最新规划摘要。12 个 child 已归档。
2. [x] （R2, R3）先执行 P0 `github-import-fs-db-atomicity`，验证本地/远程 DB 故障、恢复失败和并发互斥。含后续 QUAL-SIZE-001 拆分。
3. [x] （R2, R3）执行四个 P1：`trellis-path-security`、`windows-release-signing`、`collection-search-correctness`、`usage-refresh-failure-integrity`。签名 child R1 FAIL；父 ledger 将 REL-001/REL-002 记为 contract-evidenced `wontfix`，不宣称 fixed。
4. [x] （R2, R5）在签名边界稳定后执行 `windows-installer-verification`；真实 windows-2022 NSIS/MSI 生命周期仍 `UNVERIFIED`。
5. [x] （R1, R2）执行其余 P2 child；`typed-ipc-remainder`、`backend-boundary-ratchets` 与 types barrel 分别维持自己的 ratchet 基线。
6. [x] （R3, R5）父级 `just ci`、`just audit`、`pnpm docs:gen:check` 已在 12/12 归档后重跑；证据见 `research/integration-2026-09-03.md`。
7. [x] （R1）ledger **29** 个 ID：27 `fixed`，2 `wontfix (contract-evidenced)`（REL-001/REL-002）。**零 open**。未把 REL 标成 fixed。
8. [x] （R4）29 ID 收口后独立 trellis-check **PASS**；随后归档父任务。不 push。
9. [x] 原始扫描信封复跑：`research/scan-rerun-2026-09-03.md`。
