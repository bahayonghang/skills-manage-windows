# 工程交付流程优化

## Goal

在不削弱 Windows-first、跨平台源码兼容、供应链审计和桌面发布完整性契约的前提下，缩短开发者从本地修改到可合并结果的反馈时间，并确保文档与桌面产物的部署结果能够被真实验证。

## Background

- 2026-08-01 初始本机热缓存 `just ci` 通过，总耗时 130.3 秒；前端 1565 通过、1 跳过，Rust 主库 1031 通过、6 忽略，其他 CLI/E2E 路径通过。
- 最新 promotion PR #28 的 CI run `30680246438` 总 wall time 为 32 分 24 秒：Ubuntu source 9 分 35 秒、macOS source 7 分 55 秒、supply-chain 40 秒完成后，Windows `just-ci` 才运行 16 分 34 秒，仍存在明确串行关键路径。
- 最近 15 个合并 PR 的中位数为 137 个文件、5716 行新增；9 个超过 100 个文件，4 个超过 400 个文件。
- CI 子任务已归档：PR #29 squash 交付 `4119855d516bd2e91f3a68fa381a7c912d909d9e`，PR #30 merge commit `df7dfcd8711155e17fec0c001f206277a9cd79e4`；最终 promotion run `30687477887` wall 9m19s、queue 32s、critical lane 8m34s，达到 15 分钟目标且跨平台与供应链覆盖保留。
- 2026-08-01 developer/PR 最终规划审查时，远端仅有 `dev@34021e8579eb2a2d2913933f66ffab250776d761` 与 `main@df7dfcd8711155e17fec0c001f206277a9cd79e4`，两者 1/1 divergence 且 merge-tree 无内容冲突；`dev` 永久保留，legacy `gh-pages` 在远端 refs 和本地跟踪 refs 中均不存在。
- developer/PR 子任务已通过 PR #31 squash 合入 `dev@cc8a12bde9394142d5ac6cb100d2f28e596e1451`；批准后的远端回读为 `dev-safety` ruleset `20175337`、`dev-flow` ruleset `20175349`，repository 已启用 squash/merge、关闭 rebase 并自动删除合并 task 分支。`main` protection 未修改。
- developer/PR bookkeeping 已通过 promotion PR #32 以 merge commit `92940b04788fdaca8a78d8afa4da7a2f4ccd87b1` 合入 `main`，parents 为 `df7dfcd8711155e17fec0c001f206277a9cd79e4` 与 `24705e3bfb9edbca4d38d821fb3ea9ab8b6b70f9`；exact-head hosted CI run `30691230221` 首次 macOS 进程终止测试失败，失败 job 重跑后全量 success（common 6m44s、Windows 9m01s、Linux 5m55s、macOS 5m21s、supply-chain 55s、just-ci 4s），随后 `dev` 无 force fast-forward 到同一 promotion SHA。
- 文档子任务已归档：PR #27 squash 合入 `dev`，PR #28 以 merge commit 合入 `main`；Pages `build_type=workflow`，run `30682087003` 成功，公开 URL 返回 HTTP 200、标题为 `SkillPort`。`pnpm docs:gen:check` 与 `pnpm docs:build` 已改为只读门禁。
- 当前原子发布 workflow 与 checksum 合同晚于最后一次 `v0.10.14` 发布，尚未完成真实发布演练；公开 NSIS 的 Authenticode 状态为 `NotSigned`。

## Requirements

1. 使用四个独立子任务交付文档完整性、CI 提速、开发/PR 体验和桌面发布可信度，父任务只拥有跨子任务目标、顺序和最终集成验收。
2. 保留 `just ci` 作为本地完整门禁，并保持 GitHub required check context `just-ci` 稳定，除非在同一受控变更中完成远端保护规则迁移和回读验证。
3. 平台无关检查只执行一次；Windows、Linux、macOS 继续覆盖真实平台敏感的 Rust、进程、路径和换行契约，不以提速为由删除已有风险覆盖。
4. CI、文档和发布验证不得靠静默改写 checkout 后继续通过；生成物和版本元数据漂移必须由只读 check 模式阻断。
5. 文档部署必须验证公开 URL，而不只验证分支写入或 workflow success。
6. Windows updater `.sig`、NSIS、MSI、ZIP 和 `latest.json` 的现有原子发布契约必须保持；Windows Authenticode 与 updater 签名必须作为两个独立边界处理。
7. 用户可见开发/发布说明必须同步英文、中文和项目质量 spec，不能留下 workflow 与文档互相矛盾的触发器或命令。
8. 已完成的 GitHub Actions Pages 设置保持不变；后续若需再次修改 Pages、分支保护/合并策略、release environment、签名服务或任何公开发布，执行前必须单独展示并确认精确副作用。
9. 保留长期 `dev` 作为用户的日常开发分支，不删除本地或远端 `dev`；workflow、文档和保护设置不得把它视为待退役分支。
10. 短生命周期任务分支以 squash merge 进入 `dev` 并在合并后自动删除；`dev` 以不可 bypass 的 safety ruleset 禁止 force/delete，以带受控维护者 bypass 的 flow ruleset 要求常规 task PR、`just-ci` 与 linear history。`dev -> main` promotion PR 使用精确 head 的 merge commit，随后无 force fast-forward `dev` 到 merge SHA；`dev` 受到保护且不会被自动删除。
11. 文档部署已使用 GitHub Actions source，并完成线上 HTTP 200、SkillPort 页面身份 smoke 和设置回读；legacy `gh-pages` 已删除且禁止重新创建或恢复分支发布模式。
12. Windows Authenticode 以 Azure Artifact Signing（原 Trusted Signing）为目标方案，通过 GitHub Actions OIDC 获取短期凭据，不在 GitHub Secrets 保存可导出的 PFX 或证书密码；本阶段实现可测试的可选签名合同，Azure 注册、付费、身份验证和凭据配置另行审批。

## Child Deliverables

| Child | Ownership | Order |
| --- | --- | --- |
| `08-01-docs-deployment-generated-integrity` | 生成文档 check、PR 文档构建、Pages artifact/deploy、线上 smoke | 第一批 |
| `08-01-ci-feedback-acceleration` | CI DAG、公共/平台检查拆分、超时、耗时摘要、版本 check | 第一批 |
| `08-01-developer-pr-experience` | 工具链固定、分层本地命令、PR 模板、任务分支 squash 到 `dev` 与 `dev -> main` promotion 的分支/合并策略 | 第二批 |
| `08-01-desktop-release-assurance` | rehearsal、发布环境、Authenticode、安装/升级 smoke | 第二批，外部凭据与发布审批独立 |

## Acceptance Criteria

- [x] 四个子任务都有独立、可测试的 PRD、design、implement 和验证记录；父任务不承载产品代码实现。
- [x] 本地 `just ci`、`just audit`、文档生成/构建检查和相关 workflow contract 测试全部通过，最终工作树无意外生成物漂移。
- [x] GitHub PR workflow 的平台 job 并行启动，稳定 `just-ci` 汇总结果；失败前置检查不再迫使一个完整 Windows job 串行等待后才汇总（PR #29/#30）。
- [x] 真实 promotion PR #30 run `30687477887` 已记录各 lane 用时；wall 9m19s、queue 32s、活跃关键路径 8m34s，达到 15 分钟目标，原始失败 run 仍保留。
- [x] Pages source 已切换为 GitHub Actions 并回读验证；项目 Pages URL 返回 HTTP 200 且页面身份正确，部署后 smoke 失败会使部署 workflow 失败（PR #27/#28，run `30682087003`）。
- [x] 版本、IPC 和文档生成物均有只读漂移门禁；CI 不依赖修改 tracked files 后通过。
- [x] 开发者使用仓库声明的 Node、pnpm 和 Rust 工具链，快速门禁与完整门禁边界有文档且命令可执行（PR #31；当前主机 doctor 明确报告 Node 25/22 mismatch，其余检查通过）。
- [x] CI、贡献文档和远端保护设置支持任务分支 squash 到 `dev`、`dev -> main` promotion PR 使用 merge commit 并随后 fast-forward `dev` 的模型，不存在要求删除或退役 `dev` 的有效说明（PR #31/#32，promotion `92940b04788fdaca8a78d8afa4da7a2f4ccd87b1`）。
- [x] 获得外部设置授权后，仓库允许 squash merge 与 merge commit、关闭 rebase merge 并自动删除已合并任务分支；`dev-safety`（`20175337`）不可 bypass 地禁止 force/delete，`dev-flow`（`20175349`）对常规 task PR 强制 PR/`just-ci`/linear history 且仅为 bookkeeping/exact fast-forward 提供受控 bypass，实际设置回读与目标合同一致。
- [x] 当前发布 workflow 已完成最终 main SHA 的不公开 rehearsal：PR #38 squash merge `82c6c550e96d7d2d83b27f8ce97d8180072c9f94`，PR #39 promotion merge `d68387bffdd2f9e0b9f05d978ed925976913ef42`，rehearsal run `30700955460` 成功；首次失败 run `30698509277`（旧 SHA `fe8d869`）及其 macOS Bash 3.2 `mapfile` 根因和修复仍保留。该 rehearsal 未创建 Release、未移动 tag、未运行 updater staging，Azure Authenticode 保持 `not-configured`；公开发布、签名服务接入和 GitHub 设置变更仍需单独批准。
- [x] 发布验证能够分别报告 Azure Artifact Signing Authenticode、Tauri updater `.sig` 和未配置签名三种状态；未接入 Azure 时不得把安装包报告为已完成 Authenticode 签名。
- [x] 正式 publish 对最终签名字节生成 `actions/attest` provenance，并在 fresh-download 后通过 `gh attestation verify` 验证；attestation 不替代 Authenticode、updater `.sig` 或 checksum。

## Out of Scope

- 修改产品功能、数据库、Central 技能语义、IPC 业务契约或 UI。
- 为追求速度删除安全测试、供应链审计、跨平台风险覆盖或发布 fresh-download 校验。
- 未经明确授权创建公开 release、移动 tag、写入签名凭据、购买签名服务或修改生产 Pages/分支设置。
- 重新创建 legacy `gh-pages` 或恢复分支发布模式。
- 删除本地或远端 `dev` 分支。
- 在当前低并发维护模式下引入 merge queue。

## Integration Evidence

- Desktop child PR #33 was squash-merged into `dev` as `f4dadb798acf0bdd22f82818379144de9eefe7eb` from exact task head `b8850a6fa6982033756b9a529b8b60690090d9fd`; hosted run `30693986360` passed common, Windows/Linux/macOS Rust, supply-chain, and `just-ci`.
- Desktop local gates passed: release/workflow contracts, verifier tests, `just ci`, `just audit`, docs checks, script syntax checks, and Windows NSIS plus explicit NSIS/MSI bundle builds. The local unsigned Authenticode state is retained as rehearsal `not-configured` evidence.
- PR #38 fixed hosted rehearsal run `30698509277`'s macOS Bash 3.2 `mapfile: command not found` failure; its squash merge SHA is `82c6c550e96d7d2d83b27f8ce97d8180072c9f94`, and exact-head PR checks are recorded in run `30700333767`.
- PR #39 updated the promotion head by a merge of `main` into `dev` (`5242d97d58007e8fb1010e263f543fd409f56f65`), then promoted with merge commit `d68387bffdd2f9e0b9f05d978ed925976913ef42`; exact-head hosted checks are recorded in run `30700638444`. `dev` was fast-forwarded without force to the same `d68387b` promotion SHA and remains present locally and remotely.
- Final non-public rehearsal run `30700955460` used exact `d68387b` and passed quality/common/Rust/supply-chain/just-ci, Windows MSI smoke, Linux x86_64 and arm64 smoke, macOS universal smoke, all four release bundles, Windows install/launch/uninstall, and `Verify release artifacts`. Its artifacts included `validated-desktop-release`, `desktop-release-validation`, `desktop-release-windows`, `windows-signing-evidence`, `desktop-release-linux-x86_64`, `desktop-release-linux-arm64`, and `desktop-release-macos`; validation recorded 15 assets, `authenticode=not-configured`, and `updaterSignature=valid`. Publish and updater staging jobs were skipped by design.
- `dev` and `main` both resolve to `d68387b`; `gh-pages` and all short-lived task branches are absent. No public Release, tag movement, Azure OIDC/variables/secrets, or new GitHub setting was created or changed. Azure Artifact Signing remains deferred and publish remains fail-closed until separately authorized configuration exists.
