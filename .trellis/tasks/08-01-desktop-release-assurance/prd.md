# 桌面发布可信度提升

## Goal

在保留冻结 tag、同字节晋级和原子公开合同的基础上，让当前发布 workflow 能够在不公开版本的情况下演练，并提升 Windows 安装和升级可信度。

## Requirements

1. 保留 tag 必须位于 `origin/main`、四处版本一致、全 required build、完整 artifact inventory、updater signature、`latest.json`、`SHA256SUMS`、fresh-download 和最终唯一 `draft=false` 转换。
2. 增加显式 rehearsal/dry-run 模式：构建并验证同一套产物，但不创建公开 release；是否创建私有 draft 和保留 artifact 由 design 明确。
3. 发布 job 使用受保护 environment；私钥和发布写权限只在必要 job 可见。
4. 将 Windows Authenticode 与 Tauri updater `.sig` 分开验证；未配置真实签名时不得把 updater signature 报告为 Windows 已签名。
5. 增加 Windows 安装/启动/卸载 smoke，以及从上一稳定版到候选版的 updater smoke 设计；不可在 routine PR 执行真实升级。
6. 评估并优先加入 GitHub artifact attestation；SBOM 和签名服务成本作为显式 deferred item。
7. 任何公开 release、tag 写入、签名服务注册或 secret 更新均需要独立授权。
8. Windows Authenticode 以 Azure Artifact Signing（原 Trusted Signing）为目标方案，通过 GitHub Actions OIDC 获取短期凭据，不保存可导出的 PFX 或证书密码；本阶段实现可测试的可选签名合同，Azure 注册、付费、身份验证和凭据配置另行审批。
9. 正式 publish 使用固定 SHA 的 `actions/attest` 为最终签名字节生成 provenance，并在 fresh-download 后用 `gh attestation verify` 验证；attestation 不替代 Authenticode、updater `.sig` 或 checksum。

## Acceptance Criteria

- [ ] release workflow contract 覆盖 rehearsal 不公开、正式模式原子发布和失败 draft 保留行为。
- [ ] rehearsal 对冻结 SHA 运行 CI、required builds、artifact/signature/metadata/checksum 验证并留下可审计 artifact。
- [ ] Windows smoke 能区分 updater 签名、Authenticode、安装成功、启动成功和升级成功。
- [ ] 发布 workflow 能分别验证 Azure Artifact Signing Authenticode、Tauri updater `.sig` 和未配置签名状态，且未配置 Azure 时不会误报 Authenticode 已完成。
- [ ] publish 模式在公开前生成并验证最终 NSIS/MSI/ZIP 的 artifact attestation；rehearsal 不获取 attestation/release 写权限。
- [ ] `pnpm tauri build`、release 聚焦测试、`just ci` 和 `just audit` 通过。
- [ ] 未获得公开发布授权时，不创建或公开 GitHub release，不移动 tag。

## Out of Scope

- 未经授权购买签名服务、上传证书/私钥、修改 GitHub secrets 或发布新版本。
- macOS notarization 和 Linux 内置 updater 支持。

## Deferred Items

- 上一稳定版到候选版的真实 updater 升级执行，等待独立 staging feed、URL 与回滚方案获批；本阶段交付可执行 runbook 和 fail-closed workflow contract，不借用公开 latest channel 测试未发布候选版。
- Azure 服务注册、identity validation、付费、OIDC variables/secrets 和 GitHub environment 实际配置均等待单独外部授权。
