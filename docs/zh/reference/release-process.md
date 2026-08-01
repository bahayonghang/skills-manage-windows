# 桌面发布流程

SkillPort 桌面发布由 `.github/workflows/release-desktop.yml` 中的
`Release Desktop` workflow 完成构建、验证和原子公开。

## 唯一发布入口

- 触发方式：推送已存在的 `v<semver>` tag 执行 publish，或手动以 `rehearsal` 和精确 40 位、位于 `origin/main` 的 `rehearsal_ref` 演练。
- 质量门禁：对 tag peel 后的固定 commit SHA 调用可复用 `just-ci`。
- Release body 来源：`scripts/prepare-release-body.mjs`。
- Updater metadata 来源：`scripts/generate-latest-json.mjs`。
- Windows updater 必需 secrets：
  - `TAURI_UPDATER_PUBLIC_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

不要为同一批桌面产物再新增另一套 release workflow。发布链路需要调整时，
同时更新 `release-desktop.yml` 和这些脚本，保证 Windows 签名与 `latest.json`
保持一致。

## 发布检查清单

1. 修改 `package.json` 版本。
2. 运行 `node scripts/sync-version.mjs`。
3. 核对以下版本字段：
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`
4. 增加 `release-notes/<version>.md`，或提供
   `release-notes/<major>.<minor>.md` 作为版本线 fallback。
5. 运行本地检查：
   - `pnpm typecheck`
   - `pnpm lint`
   - `pnpm test`
   - `pnpm sizecheck`
   - `cargo test --manifest-path src-tauri/Cargo.toml`
   - `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
6. 先使用手动 `rehearsal`。它验证冻结 SHA 并保留 14 天 Actions artifact，绝不创建或修改 GitHub Release。`publish` 仍只能绑定现有 `v<semver>` tag。
7. Authenticode 与 Tauri updater `.sig` 是两项独立检查。Windows 文件先做 Authenticode，之后才对最终 NSIS 字节生成并验证 updater 签名；`.sig` 不证明 Windows Authenticode。
8. 将发布提交合入 `main`。
9. 在该 `main` commit 创建并推送 `v<version>`。重试时可手动触发
   `Release Desktop` 并填写同一个已存在 tag。
10. 等待 frozen context、可复用 CI 和所有必需平台构建完成。workflow 在创建
   或复用 draft 前验证完整产物清单、updater 签名、metadata 和
   `SHA256SUMS`。
11. 确认原子公开后的 GitHub Release 包含：
   - `latest.json`
   - `skillport_<version>_windows_x64_nsis.exe`
   - `skillport_<version>_windows_x64_nsis.exe.sig`
   - Windows MSI / ZIP 产物
   - macOS 与 Linux 安装产物（对应 job 成功时）
12. 请求
   `https://github.com/bahayonghang/skills-manage-windows/releases/latest/download/latest.json`，
   确认 version、Windows URL 和 signature 符合预期。

若上传或上传后回验失败，Release 必须保持 private draft。修复原因后用同一
tag 重跑；workflow 会先清理 draft 中的旧附件。fresh download checksum
验证通过前不得手工公开 draft。若同 tag 已有 public release，workflow 会
fail closed，不覆盖公开版本。

## Updater 不变量

- `src-tauri/tauri.conf.json` 中默认关闭 `bundle.createUpdaterArtifacts`，
  并保留 updater public key placeholder，供本地构建使用。
- 正式 release workflow 会注入真实 updater public key，但保持自动 updater
  artifacts 关闭；它先对 EXE/NSIS/MSI 做 Authenticode，再对最终 NSIS 字节签名并做 updater preflight。
- rehearsal 可报告 `authenticode=not-configured`；publish 必须取得 Azure Artifact Signing 为 EXE、NSIS 和 MSI 生成的含 timestamp 的有效 Authenticode，否则 fail closed。只有 publish 创建 provenance attestation，并在 fresh download 后验证。
- 真实的旧版到候选版 updater smoke 等待 staging feed 获批；执行时遵循 [staging 手册](updater-staging-runbook.md)。
- 所有 build 与可复用 CI 必须使用同一个 tag peel SHA。只有全部必需前置成功
  后才能创建 draft，唯一公开动作是最后的 `draft=false` API 更新。
- 在 release metadata 包含 macOS / Linux 平台项之前，应用内更新仅支持
  Windows x64。
- `/releases/latest/download/latest.json` endpoint 假设 GitHub 最新 release
  始终是包含 `latest.json` 的桌面发布。

Last reviewed: 2026-07-27
