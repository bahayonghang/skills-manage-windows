# 工程交付流程优化设计

## 1. Architecture

父任务只负责跨子任务合同、顺序和最终集成验收，不直接拥有产品或 workflow 实现。交付路径保持为：

```text
task branch --squash PR--> dev --merge-commit promotion PR--> protected main
     |                         |                                  |
     +-- focused/local CI      +-- full hosted CI                 +-- docs/release automation
```

四个子任务按以下依赖执行：

1. `docs-deployment-generated-integrity` 先提供只读文档生成检查和单次 Pages artifact。
2. `ci-feedback-acceleration` 将文档检查纳入 common lane，并把平台 lane 并行化。
3. `developer-pr-experience` 固定工具链、命令和 `dev -> main` 协作合同。
4. `desktop-release-assurance` 复用稳定 CI 与版本/生成物检查，增加不公开 rehearsal 和 Windows 发布证明。

## 2. Stable Contracts

- 本地完整门禁仍是 `just ci`，依赖审计仍是 `just audit`。
- GitHub required check context 保持 `just-ci`；远端保护规则不在代码合并前迁移到新名称。
- routine PR 不构建安装包；安装包、安装 smoke 和升级演练只属于 manual/release 路径。
- Windows NSIS、MSI、ZIP、Tauri updater `.sig`、`latest.json` 和 checksum 的原子发布合同保持不变。
- 所有外部 Actions 固定完整 commit SHA，默认权限为 `contents: read`。
- CI、生成物 check 和 rehearsal 不允许通过静默修改 tracked files 获得绿色结果。

## 3. Branch And Merge Model

- `dev` 是长期日常开发分支，不删除。
- 短生命周期任务分支 PR 到 `dev`，使用 squash merge，合并后自动删除。
- `dev -> main` promotion PR 使用 merge commit；合并后先验证 parents，再无 force fast-forward `dev` 到新的 `main` merge SHA，保持下一轮 strict/CLEAN promotion 可执行。
- 仓库允许 squash merge 与 merge commit，关闭 rebase merge；`dev` 在启用自动删分支前由不可 bypass 的 safety ruleset 禁止 force/delete，并由带最小维护者 bypass 的 flow ruleset 约束常规 PR/check/linear history。bypass 只供 Trellis bookkeeping 和 exact fast-forward。
- GitHub 不能把 `main` 自动限制为 merge-commit-only；promotion 使用 `gh pr merge --merge --match-head-commit` 并在执行前验证 base/head/SHA。
- promotion 后不制造反向 merge commit；受控 fast-forward `dev` 到 promotion merge SHA 后才能写最终证据或开始新 task。只有 `main` 存在独立 hotfix 时才进入额外同步审查。

## 4. External Change Gates

本地实现批准不自动授权以下外部写操作。执行前必须展示当前值、目标值、精确 API/命令和回滚方式：

- 已上线的 GitHub Actions Pages source、部署后公开 URL smoke 或 `gh-pages` 不存在性发生任何变更。
- 仓库 merge settings 和 `dev`/`main` 保护设置。
- Azure Artifact Signing（原 Trusted Signing）注册、付费、身份验证、OIDC/变量/secret 配置。
- release environment、artifact attestation 权限、tag 或 GitHub Release 写入。

## 5. Rollout And Rollback

- 每个子任务独立实现、验证、提交和归档；下一子任务只依赖已验证的前序合同。
- workflow 改动先由 YAML 合同测试和本地命令验证，再通过真实 PR 观察 runner 时间。
- 远端设置改动后立即回读；不一致则停止，不继续叠加下一项设置。
- Pages 回滚只能在 GitHub Actions source 内恢复已验证 workflow，不得重建 legacy `gh-pages` 或恢复分支发布；CI 可回退到原 DAG 而不改变 required context；签名合同未配置时明确报告 `not-configured`，不得伪装成功。

## 6. Integration Evidence

父任务最终只在四个子任务均完成后验收：干净树上的 `just ci`、`just audit`、文档生成/构建检查、Windows bundle、workflow 合同测试、真实 PR lane 时间、Pages HTTP/身份 smoke，以及未公开 rehearsal 的 artifact inventory。
