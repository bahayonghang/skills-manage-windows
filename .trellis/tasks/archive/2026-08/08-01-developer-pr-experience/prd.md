# 开发与 PR 体验治理

## Goal

让新机器、日常修改、提交前验证和 PR 评审使用可复现、分层且语义清晰的入口，避免长期 promotion PR 承担首次集成验证。

## Background

- 2026-08-01 现场回读：仓库允许 squash、merge commit 和 rebase，`delete_branch_on_merge=false`；`main` 保护要求 strict、GitHub Actions app `15368` 的 `just-ci`、PR、resolved conversations、管理员受约束并禁止 force/delete；`dev` 未保护且 ruleset 列表为空。
- CI 子任务已归档并通过 PR #29/#30 晋级。当前 `dev@34021e8579eb2a2d2913933f66ffab250776d761` 与 `main@df7dfcd8711155e17fec0c001f206277a9cd79e4` 各有一个独立提交，实施前需先在未启用 linear-history ruleset 时把 `main` merge 回 `dev`，再从同步后的 `dev` 创建短期任务分支。
- 当前主机为 Node 25.9.0、pnpm 11.19.0、Rust/Cargo 1.97.0；doctor 应只报告 Node/pnpm mismatch，不自动修改环境。CI 使用 Node 22、pnpm 10.12.3 和未固定版本号的 stable Rust。

## Requirements

1. 在仓库中固定 Node 22 LTS、pnpm 10.12.3 和明确 Rust toolchain，并提供跨平台 `just doctor` 输出缺失/版本不匹配。
2. 保留 `just ci` 完整门禁，新增可快速运行的静态/聚焦入口；文档不得暗示快速门禁可以替代提交前完整门禁。
3. 增加 PR template，覆盖用户问题、实现边界、风险、验证、UI 证据、打包/发布影响和回滚。
4. README、CONTRIBUTING、AGENTS 和 quality spec 对 CI 触发器、分支模型与合并要求保持一致。
5. 保留长期 `dev` 作为用户的日常开发分支，不删除本地或远端 `dev`；远端保护设置修改前后都必须读取并验证实际规则。
6. 短生命周期任务分支以 squash merge 进入 `dev`；`dev -> main` promotion PR 使用 merge commit 保留祖先关系。每次 promotion 后，在任何新 task 或 Trellis 证据提交前，受控 fast-forward `dev` 到刷新后的 `main` merge SHA，保持下一次 strict/CLEAN promotion 可执行。
7. 合并后自动删除短生命周期任务分支。`dev` 使用两个 ruleset：不可 bypass 的 safety ruleset 禁止 force/delete；flow ruleset 要求 PR、app-bound `just-ci` 和 linear history，并仅给受控维护者路径提供 always bypass，用于 Trellis bookkeeping 与 promotion 后精确 fast-forward。仓库允许 squash merge 和 merge commit、关闭 rebase；promotion 使用受控精确-head merge 命令，`dev` 不会被自动删除。

## Acceptance Criteria

- [x] 声明工具链与 CI 完全一致，`just doctor` 在当前主机给出明确结果且不修改环境。
- [x] 快速与完整门禁都有合同测试/文档，`just ci` 继续通过。
- [x] PR template 可直接支持功能、修复、UI、打包和发布变更，不要求无关字段伪造证据。
- [x] workflow/docs/spec 明确任务分支 squash 到 `dev`、`dev -> main` promotion PR 使用 merge commit、promotion 后先 fast-forward `dev` 的模型，不存在要求删除或退役 `dev` 的有效说明。
- [x] 获得外部设置授权后，仓库允许 squash merge 和 merge commit、关闭 rebase merge 并自动删除已合并任务分支；`dev` safety ruleset 不可 bypass 地禁止 force/delete，flow ruleset 对常规 task PR 强制 PR/`just-ci`/linear history，受控 bypass 仅用于 Trellis bookkeeping 与 exact fast-forward；实际设置回读与目标合同一致。

## Delivery Evidence

- PR #31 (`task/developer-pr-experience` -> `dev`) used squash merge: work commit `4625e9f600eff7b551b7993d155c00c05dc9ae01`, delivery SHA `cc8a12bde9394142d5ac6cb100d2f28e596e1451`.
- Hosted CI run `30690364410` ran at the exact head: common 5m47s, Windows Rust 9m30s, Linux Rust 6m39s, macOS Rust 6m37s, supply-chain 54s, and `just-ci` 3s; all required checks passed.
- Local evidence: focused contracts 3 files/17 tests, `just check`, `just ci`, `just audit`, `git diff --check`, and Windows `pnpm tauri build` all passed. NSIS `src-tauri/target/release/bundle/nsis/SkillPort_0.10.14_x64-setup.exe` was generated at 15:35 on 2026-08-01.
- Remote settings readback: `dev-safety` ruleset `20175337` and `dev-flow` ruleset `20175349` are active; repository squash/merge are enabled, rebase is disabled, and merged task branches auto-delete. `main` protection was unchanged.

## Out of Scope

- 强制引入本地 Git hooks、merge queue 或多审批人流程。
- 自动安装/替换开发者系统 Node、pnpm、Rust 或 just。
- 删除本地或远端 `dev` 分支。
