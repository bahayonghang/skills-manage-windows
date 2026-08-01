# 桌面发布可信度提升实施计划

## Steps

1. 扩展 release workflow/context 合同测试，先覆盖 `rehearsal` 默认不公开、publish 条件、environment/permission 边界和失败传播。
2. 为 Authenticode/updater 状态、签名顺序和最终字节验证补充脚本测试；测试必须证明“Updater 签名后再改字节”会失败。
3. 将 Windows build 改为先构建未生成 updater artifact 的 EXE/MSI/NSIS；加入 Azure 可配置探测、Authenticode 签名/验证，再对最终 NSIS 调用 Tauri signer 生成 `.sig`，最后生成 ZIP/metadata/checksum。
4. rehearsal 允许 `authenticode=not-configured` 并在 summary/artifact manifest 中显式记录；publish 模式缺少 Azure 或 Authenticode 无效时 fail closed。
5. 新增 Windows 安装/启动/卸载 smoke job，并使 aggregate 依赖它；routine PR CI 不引用该 job。
6. 增加 publish-only `actions/attest` 和 fresh-download `gh attestation verify` 合同与受保护 environments；保持唯一公开转换、checksum 和 tag 重检顺序。
7. 编写上一稳定版到候选版 updater staging runbook与 guarded job contract；未批准 staging feed 时不运行真实升级。
8. 更新 README、README_CN、CONTRIBUTING、AGENTS 和质量 spec，明确 updater `.sig` 不等于 Authenticode。
9. 本地代码合并后，分别请求 `desktop-signing`/`desktop-release` environments、Azure OIDC/variables/secrets、attestation 权限和实际 rehearsal 的外部授权；不请求 tag/public release。

## Focused Validation

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/release*.test.ts
cargo test --manifest-path src-tauri/Cargo.toml --bin release-signature-verifier --locked
node scripts/release-preflight.mjs --help
just ci
just audit
pnpm tauri build
```

Windows bundle 后核对 NSIS、MSI、ZIP 和 `.sig` inventory；在 Azure 未配置的本地环境中，验证状态必须是 `not-configured` 而非 `valid`。真实 rehearsal 记录 frozen SHA、mode、所有 job conclusions、安装 smoke、validated artifact、retention 和“无 GitHub Release”查询结果。

## Risk And Rollback Points

- 在签名顺序测试通过前不替换现有 updater 构建步骤。
- 先创建并回读受保护 environment，再提交依赖其名称的远端运行；不在日志输出 OIDC token、private key 或 Azure 响应中的敏感字段。
- 正式 publish 仍需要独立 tag/release 授权；rehearsal 成功不能自动升级为公开发布。
