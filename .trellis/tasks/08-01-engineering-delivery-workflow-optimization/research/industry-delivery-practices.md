# 工程交付流程行业实践研究

> 核对日期：2026-08-01
>
> 资料边界：仅采用 GitHub、Microsoft/Azure、Tauri、Node/pnpm、Rust 和 Google Engineering Practices 的官方文档或官方仓库。
>
> 表述约定：`官方事实` 是来源可直接支持的能力或限制；`仓库建议` 是结合本仓库实测与任务约束得出的设计选择。

## 结论摘要

当前任务的四个方向与行业实践一致，推荐按以下优先级落地：

| 优先级 | 方向 | 预期收益 | 必须保护的合同 |
| --- | --- | --- | --- |
| P0 | 将 CI 改为 common / Windows / Linux / macOS / supply-chain 并行，保留唯一 `just-ci` 汇总 | 消除当前 Unix 前置后 Windows 才启动的串行关键路径 | `just-ci` 名称稳定；任一 required lane 失败、取消或未执行都 fail closed |
| P0 | 将版本与文档生成改为真正只读 check | 消除“CI 改完文件后仍然通过”和本地脏树 | 显式生成命令仍可更新文件；check 不写 tracked 文件 |
| P0 | Pages 改为官方 Actions artifact 部署并做线上 smoke | 修复 workflow 成功但公开 URL 404 的验收缺口 | source 切为 GitHub Actions；保留 `gh-pages`；部署后验证 HTTP 与页面身份 |
| P1 | 保护长期 `dev`，任务分支 squash 到 `dev`，`dev -> main` 用 merge commit | 保留日常开发分支，同时避免 squash 长期分支造成祖先关系问题 | 不删除 `dev`；`main` 继续保护；关闭 rebase merge |
| P1 | 固定开发工具链并提供分层入口与 PR template | 缩短新机器排障和日常反馈时间，提高评审输入质量 | 快速门禁不能替代 `just ci` |
| P1 | Release rehearsal、Azure Artifact Signing、最终产物 attestation | 在公开发布前验证同一套产物，并补足 Authenticode 与 provenance | Authenticode、Tauri updater `.sig`、checksum、attestation 分层验证且顺序正确 |

不建议为提速删除 Linux/macOS 真实执行、把完整 installer matrix 放入普通 PR，或用顶层 path filter 跳过 required workflow。当前仓库低并发，也没有证据表明 merge queue 或 self-hosted runner 的维护成本已经合理。

## 1. GitHub Actions CI 反馈路径

### 1.1 DAG 与稳定 required check

**官方事实**

- GitHub Actions job 默认可以并行；`jobs.<job_id>.needs` 才建立依赖。前置 job 失败或跳过时，下游默认跳过；使用 `if: ${{ always() }}` 的汇总 job 可以在依赖失败后仍执行。[Workflow syntax: `needs`](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idneeds)
- required workflow 若因 path、branch 或 commit-message filter 整体未触发，对应检查会保持 `Pending` 并阻塞合并；job 级条件导致的跳过则报告 `Success`。依赖失败后的 required 汇总 job 应使用 `always()`，否则可能被跳过。[Troubleshooting required status checks](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks#handling-skipped-but-required-checks)
- required status check 名称应避免在多个 workflow 中产生歧义；同名检查来源不明确可能阻塞合并。[About protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches#require-status-checks-before-merging)

**仓库建议**

目标 DAG 应为：

```text
common ----------\
windows ----------\
linux -------------+--> just-ci
macos ------------/
supply-chain -----/
```

- 五个 lane 之间不设置 `needs`，进入同一 run 后立即并行排队。
- `just-ci` 只汇总 `needs.<lane>.result`，使用 `if: ${{ always() }}`；除全部 required lane 都为 `success` 外均失败。它不再 checkout、安装依赖或重复跑 Windows 全量检查。
- `just-ci` 继续作为唯一稳定的 branch-protection context。lane 名可以演进，但不得在没有同步迁移保护设置并回读的情况下改名 `just-ci`。
- workflow 对 PR 的 base branch 同时覆盖 `dev` 和 `main`。不要给 required workflow 增加顶层 paths filter；若未来做 affected classification，应在已触发的 workflow 内返回显式、可审计结果。
- `common` 只承担平台无关工作：前端 typecheck/lint/size/test/build、Rust fmt、entrypoint/IPC/版本/生成文档合同、文档 build。Windows/Linux/macOS 只承担真实平台才有意义的 Rust Clippy、tests、进程/路径/换行契约。`supply-chain` 独立运行依赖审计。
- 普通 PR 不运行 Tauri installer matrix；Windows/Linux/macOS package smoke 保持 `workflow_dispatch` 或 release 所有权。

这会直接消除当前 `source-validation` 和 `supply-chain` 完成后才启动 Windows `just-ci` 的串行结构，同时不牺牲平台覆盖。

### 1.2 Concurrency、timeout 与可观测性

**官方事实**

- concurrency group 会限制同组 workflow/job 的并发；`cancel-in-progress: true` 可取消同组旧运行，GitHub 不保证同组运行顺序。[Control workflow concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)
- workflow/job 可配置 `timeout-minutes`；应避免依赖平台默认的长超时才终止挂死任务。[Workflow syntax: `timeout-minutes`](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idtimeout-minutes)

**仓库建议**

- PR CI 使用包含 workflow 名和 PR number/ref 的 concurrency key，并取消旧 SHA 的运行；release 和生产 Pages deploy 不取消运行，也不假定队列按提交顺序执行。
- 每个 lane 依据近期 P95 耗时加合理余量设置 timeout，而不是所有 job 使用同一个值。初始值应宽松，收集 10-20 次真实 run 后再收紧。
- 每个 lane 将开始时间、结束时间、耗时、cache hit 和失败阶段写入 `$GITHUB_STEP_SUMMARY`；`just-ci` 输出各 lane 最终状态。下一次真实 PR 同时记录 wall time、runner time 和排队时间，避免把 runner scarcity 错判成代码执行慢。
- 15 分钟目标应定义为“runner 获得后 required DAG 的活跃关键路径”，排队时间单独报告。

### 1.3 缓存、安全权限与 Action pin

**官方事实**

- GitHub 推荐优先使用 `setup-*` action 的 package-manager cache；cache 先匹配完整 key，再按前缀和 restore keys 回退。cache 内容不应包含敏感信息，因为 fork PR 可能读取 base branch cache。[Dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
- workflow 可以在顶层或 job 级收窄 `GITHUB_TOKEN` 权限；未声明的权限在显式权限块下会变为 `none`。[Automatic token authentication](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication#modifying-the-permissions-for-the-github_token)
- 完整 commit SHA 是引用第三方 Action 的唯一不可变方式；还应确认 SHA 属于官方仓库而不是 fork。[Secure use: third-party actions](https://docs.github.com/en/actions/reference/security/secure-use#using-third-party-actions)

**仓库建议**

- 保留当前 `setup-node` 的 pnpm cache，并继续用 `pnpm-lock.yaml` 作为 dependency key 输入。
- Rust cache 至少隔离 OS、target、Rust toolchain 和 `Cargo.lock`。只缓存可重建依赖/编译中间物，不缓存 release artifact、PAT、OIDC token、Tauri signing private key 或任何 Azure 签名材料。
- PR CI 保持顶层 `contents: read`；Pages、attestation、release publish 和 Azure OIDC 权限只加到真正需要的 job。
- 所有 GitHub、Azure、社区 Action 继续固定完整 SHA，并用同一行注释可读 release tag。现有 Dependabot GitHub Actions weekly 配置应保留。
- 为每个 install/build job使用 lockfile；不要用 cache 掩盖未固定工具版本或允许发布 job消费不可信 PR 产生的发布产物。

### 1.4 生成物与版本检查

**仓库建议**

- `sync-version` 保留为显式写命令；新增 `sync-version --check` 或等价入口，在内存/临时目录计算期望内容并比较，不改写 checkout。
- `docs:gen` 保留为显式写命令；新增 `docs:gen:check`，对 IPC 字典和 schema 表进行 byte-level 对比并报告具体漂移文件。
- `pnpm docs:build` 不再隐式执行写入型 `docs:gen`，而是先执行只读 check，再执行 VitePress build。
- CI 最后可额外运行 `git diff --exit-code` 作为防御性哨兵，但它不能替代真正不写文件的 check 模式。

## 2. GitHub Pages 部署

### 2.1 官方 custom workflow 合同

**官方事实**

- 使用 custom Pages workflow 前，仓库 Pages publishing source 必须切换为 **GitHub Actions**。[Configuring a publishing source](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site#publishing-with-a-custom-github-actions-workflow)
- 官方链路是 `configure-pages`、`upload-pages-artifact`、`deploy-pages`。deploy job 至少需要 `pages: write` 和 `id-token: write`，需要 `needs` 指向 build job，并应使用 `github-pages` environment；部署 URL 可由 deployment step output 暴露。[Using custom workflows with GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
- Pages artifact 是包含单个 tar 的 gzip，不能包含 symbolic/hard links。官方文档示例的 action major 可能落后于实际 release；截至核对日，官方仓库已有 [configure-pages v6.0.0](https://github.com/actions/configure-pages/releases/tag/v6.0.0)、[upload-pages-artifact v5.0.0](https://github.com/actions/upload-pages-artifact/releases/tag/v5.0.0) 和 [deploy-pages v5.0.0](https://github.com/actions/deploy-pages/releases/tag/v5.0.0)。实施时必须审核 release 后固定完整 SHA，而不是照抄 tag。

**仓库建议**

- PR 的 `common` lane 执行 `docs:gen:check` 和 `pnpm docs:build`，但不部署。
- Docs workflow 只 build 一次：checkout/install/check/build -> `upload-pages-artifact`；deploy job下载/部署同一个 Pages artifact，不再 checkout、install、第二次 build。
- 保持当前 `release.published` 作为生产文档触发器，并增加受控 `workflow_dispatch` 用于首次迁移或恢复验证。不要因切换 source 而删除 `gh-pages` 分支。
- deploy job只拥有 `contents: read`、`pages: write`、`id-token: write`，并绑定 `github-pages` environment。build job不获得 Pages 写权限。
- 部署成功后对 `steps.deployment.outputs.page_url` 做有界重试，要求 HTTP 200，并检查稳定的 SkillPort 标识（例如 title 或唯一页面元素）。超时或错误站点均使 workflow 失败。
- 远端变更顺序：合并 workflow -> 将 Pages source 切为 GitHub Actions -> 回读 `build_type`/source -> 手动部署 -> 验证 URL 与页面身份。该顺序避免 workflow 与仓库设置短暂错配被误判为成功。

## 3. 长期 `dev -> main` 与 PR 体验

### 3.1 合并方式的能力与限制

**官方事实**

- GitHub 明确指出：长期 head branch 不适合反复 squash merge，因为后续 PR 可能再次包含已 squash 的旧 commit 并反复产生冲突；长期分支更适合保留 merge ancestry。[About pull request merges: long-running branches](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/about-pull-request-merges#squashing-and-merging-a-long-running-branch)
- 仓库可以同时启用 squash merge 和 merge commit，也可以关闭 rebase merge；启用多种方式时，合并者可在 PR 上选择。[Configure squash merging](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/configuring-commit-squashing-for-pull-requests)；[Configure commit merging](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/configuring-commit-merging-for-pull-requests)
- ruleset 的 `Require linear history` 会禁止 merge commit，只允许 squash 或 rebase。若仓库级关闭 rebase，则可让目标分支实际上只接受 squash。[Available rules for rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets#require-linear-history)
- 自动删除 head branch 是仓库级功能；branch protection/ruleset 可以阻止某个受保护分支被自动删除。[Managing automatic branch deletion](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-the-automatic-deletion-of-branches)

**仓库建议**

- 仓库级允许 squash merge 与 merge commit，关闭 rebase merge，开启 merged head branch 自动删除。
- `dev` ruleset：require PR、require `just-ci`、禁止 force push、限制删除、require linear history。关闭 rebase 后，任务分支进入 `dev` 时只有 squash 可用；`dev` 本身因保护规则不会被自动删除。
- `main` ruleset：require PR、require `just-ci`、禁止 force push、限制删除；不要 require linear history，否则 `dev -> main` 无法 merge commit。
- promotion PR 的 head 固定为 `dev`、base 固定为 `main`，使用 merge commit。promotion 后无需例行 `main -> dev` reverse merge；只在 `main` 存在独立 hotfix 时同步该提交。
- 重要限制：GitHub 普通仓库设置没有“对 `main` 只允许 merge commit、同时对 `dev` 只允许 squash”的完整双向强制开关。`dev` 可通过 linear-history + 关闭 rebase 实现 squash-only；`main` 的 promotion merge-commit 仍需 runbook、PR template/检查提示和精确 `gh pr merge --merge` 流程保证。不能把“仓库已允许 merge commit”误报为“GitHub 已强制 promotion 使用 merge commit”。
- main/dev 两套保护规则和合并设置都属于远端变更；实施时先展示目标 JSON/当前值，修改后从 API 回读。不要删除本地或远端 `dev`。

### 3.2 PR 大小、模板和自动化

**官方事实**

- Google 的官方 code-review 实践建议 CL 小而自包含；小 CL 通常评审更快、更彻底，缺陷更少，也更容易合并与回滚。[Small CLs](https://google.github.io/eng-practices/review/developer/small-cls.html)
- GitHub 支持仓库级 PR template，用于在创建 PR 时预填稳定的评审信息。[Creating a pull request template](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository)

**仓库建议**

- 鉴于最近 15 个 merged PR 中位数达到 137 files / 5716 additions，先把“单一用户问题、可独立验证、可独立回滚”写入 PR template 和贡献说明，不立即用僵硬行数阈值阻断。
- PR template 覆盖：用户问题、范围/明确排除、风险、验证证据、UI evidence（仅适用时）、打包/发布影响、生成物状态、回滚。字段应允许 `N/A + 原因`，不要要求伪造无关证据。
- 可增加非阻断 PR size summary，连续观察 10-15 个 PR 后再决定是否设置软警戒线。
- 允许 auto-merge 可以在 required checks/review 满足后自动合并，但 task PR 必须预先选择 squash；promotion PR 必须选择 merge。低并发场景先不引入 merge queue。

### 3.3 工具链与本地反馈层级

**官方事实**

- `actions/setup-node` 支持从版本文件读取 Node 版本并提供 package-manager cache。[actions/setup-node](https://github.com/actions/setup-node#usage)
- pnpm 支持在 `package.json` 的 `packageManager` 字段声明准确版本。[pnpm package.json](https://pnpm.io/package_json#packagemanager)
- rustup 支持仓库级 `rust-toolchain.toml`，声明 channel、components 和 targets。[The Rust toolchain file](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)

**仓库建议**

- 增加 Node 22 版本文件并让 CI 与本地读取同一来源；`packageManager` 固定 `pnpm@10.12.3`；增加 `rust-toolchain.toml` 固定 Rust 版本、`rustfmt`、`clippy` 和必要 target。
- `just doctor` 只读检查 Node、pnpm、Rust、Cargo、just 及 Windows build prerequisites，输出 expected/actual/fix hint，不自动安装或替换环境。
- 建议三层入口：`just quick`（秒级静态与生成物 check）、聚焦测试入口（按前端/Rust目标选择）、`just ci`（提交前完整门禁）。文档明确前两者只用于迭代，不能替代 `just ci`。
- 本地和 CI 复用同一脚本/just recipe，不复制两套逐渐漂移的命令清单。

## 4. Windows Authenticode 与 Azure Artifact Signing

### 4.1 服务、OIDC 与最小权限

**官方事实**

- Azure Trusted Signing 当前产品名为 **Artifact Signing**。它是 Microsoft 托管的签名服务，证书生命周期由 FIPS 140-3 Level 3 HSM 管理，支持内容保密的 digest signing。[What is Artifact Signing?](https://learn.microsoft.com/en-us/azure/artifact-signing/overview)
- 官方 GitHub Action 运行在 Windows 2022/2025 runner，不支持 Windows ARM，并推荐 OIDC/workload identity。[Azure/artifact-signing-action](https://github.com/Azure/artifact-signing-action)
- GitHub OIDC 允许 workflow 无需保存长期 Azure client secret；workflow 需要 `id-token: write`，通过 `azure/login` 将 GitHub OIDC token 交换为 Azure access token。[GitHub OIDC in Azure](https://docs.github.com/en/actions/how-tos/secure-your-work/security-harden-deployments/oidc-in-azure)；[Microsoft: connect from Azure with OIDC](https://learn.microsoft.com/en-us/azure/developer/github/connect-from-azure-openid-connect)
- 签名身份需要 `Artifact Signing Certificate Profile Signer` 角色。Artifact Signing certificate 有三天有效期，因此 RFC3161 timestamp 对长期验证是关键；Microsoft 推荐 `http://timestamp.acs.microsoft.com/`。[Artifact Signing roles](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-resources-roles)；[Signing integrations](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations)
- GitHub environment 可设置 protection rules，并让 environment secrets 只在规则通过后可用。[Managing environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)

**仓库建议**

- 使用独立、受保护的 `windows-signing` environment；federated credential 约束到本仓库和该 environment，Azure RBAC 尽量收窄到目标 certificate profile。
- `id-token: write` 仅授予 Windows signing job；其他 build、PR CI、aggregate、Pages job 不获得 Azure OIDC 权限。
- GitHub 中只保存 tenant/client/subscription ID、endpoint、account/profile name 等配置，不保存 PFX、PFX password 或 client secret。服务注册、identity validation、付费、RBAC、federated credential 和环境配置继续作为独立外部审批门禁。
- 外部 Actions 固定完整 SHA。不要从 PR 或 `pull_request_target` 执行可访问 signing environment 的不可信代码。

### 4.2 Authenticode 与 Tauri updater 签名是不同边界

**官方事实**

- Tauri `bundle.windows.signCommand` 可在 bundle 流程中调用自定义 Windows signing command。[Tauri Windows code signing](https://v2.tauri.app/distribute/sign/windows/#custom-sign-command)
- Tauri updater 必须用自己的 key 验证更新，不能关闭；Windows updater `.sig` 对 installer/update bundle 生成。[Tauri updater: signing updates](https://v2.tauri.app/plugin/updater/#signing-updates)
- PowerShell `Get-AuthenticodeSignature` 可读取文件的 Authenticode signature 信息。[Get-AuthenticodeSignature](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature)

**仓库建议**

发布字节顺序必须为：

```text
build executable
  -> Azure Artifact Signing Authenticode (通过 Tauri signCommand 进入 bundle 时序)
  -> build/sign NSIS and MSI
  -> generate/verify Tauri updater .sig over final installer bytes
  -> rename assets and generate latest.json
  -> generate/verify SHA256SUMS
  -> generate GitHub artifact attestations
  -> upload/publish the exact same bytes
```

- 不要在 Tauri 已生成 updater `.sig` 后再对 NSIS/MSI 做 post-build Authenticode，因为 Authenticode 会改变 installer bytes，旧 `.sig` 将不再对应最终文件。若实现选择 post-build Action，就必须在 Authenticode 后重新生成并验证 updater `.sig`；优先选择 Tauri `signCommand` 把 Azure SignTool/dlib 调用放入正确 bundling 时序。
- 必须对最终 EXE、NSIS、MSI 执行 `Get-AuthenticodeSignature`/SignTool verify，要求 `Status == Valid`、签名者符合预期且有 timestamp。ZIP 本身不做 Authenticode，但 ZIP 内 EXE 必须是已签名字节。
- 必须另行用 Tauri public key 验证 updater `.sig`，再验证 `latest.json` 指向的 URL/signature 与最终 NSIS asset 一致。Authenticode 通过不能替代 updater 验证，反之亦然。
- 配置状态应显式分为 `disabled` 与 `required`：rehearsal 未接入 Azure 时，要求确认并报告 `NotSigned`；正式签名模式若 OIDC/RBAC/config 缺失则 fail closed，不允许 warning 后继续并声称已签名。

## 5. Release rehearsal 与 artifact attestation

### 5.1 不公开 rehearsal

**仓库建议**

- `workflow_dispatch` 增加显式 `mode: rehearsal|publish`，默认 `rehearsal`。rehearsal 使用与 publish 相同的 frozen tag/SHA、quality gate、全平台 build、inventory、signature/metadata/checksum 和 fresh artifact validation，但不创建或修改 GitHub Release。
- rehearsal 将 validated asset set 作为有期限的 Actions artifact 保存，输出 SHA、版本、各平台产物清单、签名状态和 checksum；这样不需要用 private draft 模拟，也不会触发公开副作用。
- publish 模式才进入受保护 `release` environment、获得 `contents: write`，继续使用当前“同 tag draft -> 上传 -> fresh download -> 唯一一次 `draft=false`”原子流程。
- signing environment 与 publish environment 分离；签名 job只获 Azure OIDC，publish job只获 Release 写权限。

### 5.2 GitHub artifact attestations

**官方事实**

- GitHub 当前推荐使用 `actions/attest` 为 binary 建立 build provenance，workflow 需要 `id-token: write`、`contents: read`、`attestations: write`，并以 `subject-path` 指定最终文件。[Using artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)
- 消费者可以用 `gh attestation verify PATH -R OWNER/REPO` 验证；attestation 提供来源/构建证明，但不证明软件本身安全，只有实际验证才产生供应链价值。[Artifact attestations concepts](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- artifact attestations 对当前 GitHub plans 的公共仓库可用；私有/内部仓库需要 GitHub Enterprise Cloud。当前仓库为 public，因此具备使用条件。[Using artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations#prerequisites)

**仓库建议**

- 在 aggregate job 完成 Authenticode、updater `.sig`、`latest.json` 和 checksum 后，对最终 NSIS/MSI/ZIP 及其他发布资产运行 pin 到完整 SHA 的 `actions/attest`。
- publish 的 fresh-download 验证除 checksum 外，再运行 `gh attestation verify ... -R bahayonghang/skills-manage-windows`，证明公开下载字节与本仓库受控 workflow 生成的 subject 一致。
- attestation 是 provenance 层，不替代 Authenticode、Tauri updater signature、checksum、恶意软件扫描或安装/升级 smoke。
- SBOM 可以后续作为独立 attestation 加入；当前先交付 build provenance，避免在未明确 SBOM 生成器、格式和依赖归属前扩大范围。

## 6. 推荐实施与验收顺序

1. **生成物完整性**：实现版本与 docs 只读 check；确认 `pnpm docs:build` 后 tracked diff 为空。
2. **Pages 本地合同**：改为官方 Pages artifact build/deploy 单构建结构，加入 deployment output URL smoke 和 workflow contract tests。
3. **CI 并行 DAG**：拆 common/platform/supply-chain，增加 timeout/summary，保留 `just-ci` 聚合与 manual-only package。
4. **开发与 PR 入口**：固定工具链、`just doctor`、快速/聚焦/完整门禁和 PR template；同步中英文文档与 quality spec。
5. **远端分支/Pages 设置**：按已授权目标展示精确变更，更新 Pages source、merge methods、auto-delete、`dev`/`main` rulesets并回读；绝不删除 `dev`。
6. **Release rehearsal**：先跑无签名、不公开 rehearsal，验证完整产物与明确 `NotSigned` 状态。
7. **Authenticode 合同**：实现可选但 fail-closed 的 Tauri `signCommand` 时序与 Authenticode/updater 双重验证；真实 Azure 接入仍等待单独授权。
8. **Attestation**：对最终字节生成并在 fresh download 后验证 provenance。
9. **最终证据**：本地 `just ci`、`just audit`、文档检查、workflow contracts、Windows `pnpm tauri build`；随后用真实 PR记录各 lane/关键路径，再决定是否继续做 runner 或更激进缓存优化。

## 7. 不应被误报为已完成的事项

- workflow 文件改好不等于 Pages 已恢复；必须切换仓库 source、回读设置并验证公开 URL。
- 仓库允许 merge commit 不等于 `main` promotion 已强制 merge commit；该限制需要流程/自动化保证。
- 存在 `.sig` 不等于 Windows Authenticode 已签名；Tauri updater `.sig` 与 Authenticode 是不同密码学合同。
- Azure OIDC workflow 已写入不等于真实签名可用；还需要 Azure account、identity validation、certificate profile、RBAC、federated credential 和 environment 配置。
- workflow success 不等于生成物无漂移；必须证明 check 模式不写文件，并在结束时检查工作树。
- attestation 存在不等于产物安全；必须验证 attestation，且仍需签名、checksum 和运行时 smoke。
