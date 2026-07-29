# Release 流水线重排：先构建验证后原子发布

## Goal

把桌面发布从“先公开 GitHub Release、再构建附件”改为“固定 tag/SHA，质量门禁与全部必需产物通过后，在 draft 中校验附件，最后一次性公开”。失败路径不得产生新的 public release。对应审计 P1-08（橙色客观缺陷）与 QW-05。

## Background

- `.github/workflows/release-desktop.yml:3-7,315-344` 当前由 `release.published` 触发，最终 `publish` job 只是向已经公开的 release 上传附件。
- `.github/workflows/ci.yml:3-12,23-67` 同样由 `release.published` 触发，和 release workflow 并行且没有依赖关系；CI 失败不能阻止附件发布。
- 2026-07-15 的 `v0.10.14` 首次发布真实留下过一个无附件 release：macOS universal CLI 缺失导致 bundle 失败，说明“release 页面存在”不能证明发布完成。
- 当前 Windows job 已生成 NSIS、对应 `.sig`、MSI、ZIP 和 `latest.json`，但 `scripts/release-preflight.mjs` 只验证 pubkey 非 placeholder、签名文件存在及 metadata 文本一致，不做密码学验签、完整产物清单或 checksum 验证。
- 当前 `@tauri-apps/cli 2.11.2` 的 `tauri signer` 只有 `sign` / `generate`，没有 `verify` 子命令；项目锁文件已通过 `tauri-plugin-updater 2.10.1` 锁定 `minisign-verify 0.2.5`。
- 2026-07-27 live 复核确认：`release-desktop.yml:3-5`、`ci.yml:3-10` 仍保留 `release.published`，preflight 仍只做签名文本一致性；锁定 updater 源码实际使用 `PublicKey::decode`、`Signature::decode` 与 `verify(..., true)`，方案可直接复用同一验证语义。

## Requirements

1. **统一 release context**：唯一 workflow 接受 `v*` tag push 或必填 `workflow_dispatch(tag)`。在任何 build 前解析并冻结 `tag + version + peeled commit SHA`，验证 tag 已存在、指向 `main` 历史，并与 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 一致；后续 checkout 和命名只使用该 context，不使用 dispatch 分支名。
2. **CI 硬依赖**：`.github/workflows/ci.yml` 增加 `workflow_call(checkout_ref)`，release workflow 必须等待同一 SHA 的 `just-ci` 成功后才进入 build matrix。移除 post-publication `release.published` 竞跑语义；日常 PR/push 与手动 smoke package 行为保持可测试。
3. **构建后才建 draft**：Windows、macOS、Linux x64 必需 matrix job 全部成功后，聚合 job 才创建或复用同 tag draft。若同 tag release 已公开则失败；若 CI/matrix 失败则不创建新 release，已有 draft 保持 draft 且不公开。
4. **平台产物清单**：Windows 必须包含 NSIS、NSIS `.sig`、MSI、ZIP、`latest.json`；macOS universal 必须包含 DMG、ZIP、TAR.GZ；Linux x64 必须包含 DEB、RPM、AppImage。Linux arm64 保持现有 optional 语义，但出现任一 arm64 文件时必须是完整三件套。清单拒绝缺失、重复和非预期附件。
5. **真实 updater 验签**：使用与运行时 updater 相同的 Tauri base64 包装格式和锁定的 `minisign-verify 0.2.5`，对实际 Windows NSIS 字节和 `.sig` 做密码学验证；损坏 installer、损坏 signature 或错误公钥都必须失败。不得把 `.sig` 存在或与 `latest.json` 文本相等当成验签。
6. **metadata 与 checksum**：`latest.json` 必须是合法 JSON，version、两个 Windows platform key、URL、signature 均与 frozen context 和已验签 NSIS 一致。聚合 job 为最终附件集合生成确定性 `SHA256SUMS`，并在上传前回验。
7. **draft 附件回验后公开**：发布 job 重置同 tag draft 的旧附件，上传完整集合后通过 GitHub API 确认 release 仍为 draft、tag/SHA 正确、附件名和大小完整；随后下载到全新目录并用 `SHA256SUMS` 回验。唯一公开动作是最后一步把该 draft 的 `draft` 改为 `false`；上传或回验失败时 draft 保持不可见。
8. **操作文档同步**：更新英文/中文 release process，明确“先合并到 main、创建并推送 tag（或手动选择已存在 tag）、等待 workflow 原子公开”，替换“先 publish GitHub Release”的旧步骤，并记录失败时 draft 的处理方式。

## Acceptance Criteria

- [ ] workflow contract test 对 required-job DAG 做失败模拟：任一 release-context、reusable CI 或必需 matrix 结论失败时，draft/publish job 不可调度且不存在新的 public release；已有同 tag draft 不会被公开
- [ ] post-upload/pre-publish 任一步失败时，release 对象仍为 draft；只有最后一个状态转换步骤可令其 public
- [ ] wrong tag/version、Cargo lock 不同步、missing/duplicate/unexpected artifact、partial optional arm64 group、malformed `latest.json`、错误 URL/platform key/signature 均有自动化失败用例
- [ ] 有固定 fixture 证明有效 updater signature 通过，篡改 installer、signature 或 public key 均失败
- [ ] 成功路径生成覆盖所有最终附件的 `SHA256SUMS`，fresh download 校验通过后才能 publish
- [ ] `ci.yml` 的 `just-ci` 可由 release workflow 对 frozen SHA 调用，同时保持 `main` PR、`main/dev` push 和手动 smoke package contract
- [ ] 英文/中文发布文档、README 入口描述与 workflow 一致
- [ ] `just ci` 通过；Windows 上 `pnpm tauri build` 成功并确认 NSIS/MSI bundle 实际生成

## Out of Scope

- Actions full-SHA pinning、Dependabot、OSV、`cargo audit/deny` 属于 `07-24-ci-supply-chain`；本任务只保持 action 使用点便于后续加固。
- SBOM、attestation、Windows Authenticode、macOS signing/notarization、Linux 安装 smoke 属于后续供应链/平台签名工作；本任务不把“未签名 macOS”误报为已验证签名。
- 不创建第二套桌面 release workflow，不改变应用内 updater 目前仅支持 Windows x64 的产品范围。
- 不在本任务中执行 push、创建 tag 或真实发布；远端演练需要另行明确授权。

## Dependencies And Ordering

- 无代码前置依赖，可独立实施。
- 与 `07-24-ci-supply-chain` 都修改 `.github/workflows/`；先完成本任务的状态机与测试，再由 supply-chain 子任务在最新 workflow 上做 action pinning，避免相互覆盖。
