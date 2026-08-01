# 工程交付流程优化

## Goal

在不削弱 Windows-first、跨平台源码兼容、供应链审计和桌面发布完整性契约的前提下，缩短开发者从本地修改到可合并结果的反馈时间，并确保文档与桌面产物的部署结果能够被真实验证。

## Background

- 2026-08-01 初始本机热缓存 `just ci` 通过，总耗时 130.3 秒；前端 1565 通过、1 跳过，Rust 主库 1031 通过、6 忽略，其他 CLI/E2E 路径通过。
- 最新 promotion PR #28 的 CI run `30680246438` 总 wall time 为 32 分 24 秒：Ubuntu source 9 分 35 秒、macOS source 7 分 55 秒、supply-chain 40 秒完成后，Windows `just-ci` 才运行 16 分 34 秒，仍存在明确串行关键路径。
- 最近 15 个合并 PR 的中位数为 137 个文件、5716 行新增；9 个超过 100 个文件，4 个超过 400 个文件。
- 2026-08-01 现场刷新后，远端仅有 `dev@79cf85722514d380fbb225877a5590aa924903a6` 与 `main@ecd2b2f18a9ccd922f26e5740861158d1cdbba69`；`dev` 永久保留，legacy `gh-pages` 在 GitHub Branch API、远端 refs 和本地跟踪 refs 中均不存在。
- `main` 受保护并要求 GitHub Actions app `15368` 的 `just-ci`；`dev` 当前未保护、ruleset 列表为空。仓库当前允许 squash、merge commit 和 rebase，且 `delete_branch_on_merge=false`，这些远端设置仍待开发/PR 子任务单独审批。
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
10. 短生命周期任务分支以 squash merge 进入 `dev` 并在合并后自动删除；`dev` 通过 linear-history ruleset 与关闭 rebase 实现 squash-only。`dev -> main` promotion PR 使用精确 head 的 merge commit 保留祖先关系，`dev` 受到保护且不会被自动删除。
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

- [ ] 四个子任务都有独立、可测试的 PRD、design、implement 和验证记录；父任务不承载产品代码实现。
- [ ] 本地 `just ci`、`just audit`、文档生成/构建检查和相关 workflow contract 测试全部通过，最终工作树无意外生成物漂移。
- [ ] GitHub PR workflow 的平台 job 并行启动，稳定 `just-ci` 汇总结果；失败前置检查不再迫使一个完整 Windows job 串行等待后才汇总。
- [ ] 下一次真实 PR 记录各 lane 用时；有 runner 时的目标活跃关键路径不超过 15 分钟，若未达到则保留原始数据并继续定位，不把目标当作已通过。
- [x] Pages source 已切换为 GitHub Actions 并回读验证；项目 Pages URL 返回 HTTP 200 且页面身份正确，部署后 smoke 失败会使部署 workflow 失败（PR #27/#28，run `30682087003`）。
- [ ] 版本、IPC 和文档生成物均有只读漂移门禁；CI 不依赖修改 tracked files 后通过。
- [ ] 开发者使用仓库声明的 Node、pnpm 和 Rust 工具链，快速门禁与完整门禁边界有文档且命令可执行。
- [ ] CI、贡献文档和远端保护设置支持任务分支 squash 到 `dev`、`dev -> main` promotion PR 使用 merge commit 的模型，不存在要求删除或退役 `dev` 的有效说明。
- [ ] 获得外部设置授权后，仓库允许 squash merge 与 merge commit、关闭 rebase merge 并自动删除已合并任务分支；`dev` ruleset 强制 task PR 使用 squash 且禁止 force/delete，实际设置回读与目标合同一致。
- [ ] 当前发布 workflow 至少完成一次不公开 rehearsal；任何公开发布、签名服务接入或 GitHub 设置修改均有单独批准和回读证据。
- [ ] 发布验证能够分别报告 Azure Artifact Signing Authenticode、Tauri updater `.sig` 和未配置签名三种状态；未接入 Azure 时不得把安装包报告为已完成 Authenticode 签名。
- [ ] 正式 publish 对最终签名字节生成 `actions/attest` provenance，并在 fresh-download 后通过 `gh attestation verify` 验证；attestation 不替代 Authenticode、updater `.sig` 或 checksum。

## Out of Scope

- 修改产品功能、数据库、Central 技能语义、IPC 业务契约或 UI。
- 为追求速度删除安全测试、供应链审计、跨平台风险覆盖或发布 fresh-download 校验。
- 未经明确授权创建公开 release、移动 tag、写入签名凭据、购买签名服务或修改生产 Pages/分支设置。
- 重新创建 legacy `gh-pages` 或恢复分支发布模式。
- 删除本地或远端 `dev` 分支。
- 在当前低并发维护模式下引入 merge queue。
