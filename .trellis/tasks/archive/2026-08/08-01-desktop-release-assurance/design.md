# 桌面发布可信度提升设计

## 1. Workflow Modes

保留一个 canonical `.github/workflows/release-desktop.yml`：

- `push v*`：正式 publish 模式，沿用冻结 tag 和原子公开合同。
- `workflow_dispatch(mode=rehearsal|publish)`：默认 `rehearsal`，且必须提供位于 `origin/main` 的精确 40 位 `rehearsal_ref` SHA；`publish` 必须提供并校验现有 `v<semver>` tag。只有显式 `publish` 才进入 draft/upload/fresh-download/公开路径。
- rehearsal 使用同一 frozen tag SHA、reusable CI、全 required builds、aggregate、安装 smoke 和 artifact inventory，但停止在 Actions artifact，不创建 GitHub Release 或公开 deployment。

release-context 输出规范化 `mode`，所有 build checkout 仍绑定 peeled tag SHA。publish job 同时要求 `mode == publish`、受保护 environment 和前置成功。

## 2. Windows Signing Order

Authenticode 会修改文件字节，因此顺序固定为：

```text
Tauri build (createUpdaterArtifacts=false)
  -> Azure Artifact Signing: skillport.exe + NSIS.exe + MSI
  -> verify Authenticode status/timestamp on signed files
  -> pnpm tauri signer sign <final signed NSIS>
  -> verify final NSIS against generated updater .sig/public key
  -> ZIP the signed skillport.exe
  -> latest.json + SHA256SUMS + artifact inventory
```

不得先生成 `.sig` 再做 Authenticode。release preflight 必须验证 `.sig` 对应最终发布 NSIS 字节，并分别记录：

- `authenticode=valid|not-configured|invalid`
- `updater-signature=valid|invalid`

rehearsal 在 Azure 尚未配置时允许明确的 `not-configured` 结果，用于验证其余合同；publish 模式必须要求 Azure 配置完整且 Authenticode 为 `valid`，否则在 draft 创建前失败。

## 3. Azure Boundary

- 目标 provider 是 Azure Artifact Signing（原 Trusted Signing），GitHub Actions 通过 OIDC 获取短期凭据。
- 只有 Windows signing job 获得 `id-token: write`；不保存可导出 PFX 或证书密码。
- endpoint、account、profile、tenant/client/subscription identity 使用受控 environment variables；Tauri updater 私钥只属于 signing environment secret。
- Azure login/Artifact Signing Actions 固定完整 SHA。服务注册、identity validation、付费、environment/variables/secrets 写入均在本地代码之外单独审批。

## 4. Environments And Permissions

- workflow 默认 `contents: read`。
- `build-windows` 使用受保护 `desktop-signing` environment；仅该 job 能读取 updater 私钥和请求 OIDC。
- `publish` 使用受保护 `desktop-release` environment；仅该 job 获得 `contents: write`，正式 attestation 时额外获得 `id-token: write`/`attestations: write`。
- rehearsal 不调度 publish job。validated artifact 保留有限天数并记录 frozen SHA/mode。

## 5. Smoke And Attestation

- `windows-install-smoke` 下载 build-windows artifact，验证 Authenticode/updater 状态，passive 安装、确认安装目录和可执行文件、短暂启动并检查进程、终止后运行卸载；它只在 manual/release workflow 运行。
- 从上一稳定版到候选版的真实 updater smoke 需要 staging feed。当前阶段交付可执行 runbook、输入/签名/回滚合同和 fail-closed job guard；在 staging URL/凭据另行批准前保持 deferred，不借用公开 latest channel 测试未发布候选版。
- publish 模式使用统一的 `actions/attest` 对 aggregate 中的最终文件生成 GitHub artifact attestation；fresh-download 后使用 `gh attestation verify -R bahayonghang/skills-manage-windows` 验证 NSIS/MSI/ZIP provenance。生成或验证失败均阻止公开。是否启用远端 attestation 权限在执行前回读确认。

## 6. Failure And Rollback

- build/sign/smoke/aggregate 任一失败时 publish 不调度，不产生新 release。
- publish 中 draft 上传或 fresh-download 失败时保留私有 draft供审计；唯一 `draft=false` 仍是最终转换。
- tag 在 context、draft 前或 publish 前移动都 fail closed。
- Azure 未配置不会被 updater `.sig` 掩盖；正式发布不可降级为 unsigned。
- 回滚 workflow 不移动 tag、不删除公开 release、不轮换 secret；外部环境设置按保存的原值恢复。
