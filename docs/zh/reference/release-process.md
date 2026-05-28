# 桌面发布流程

SkillPort 桌面发布通过 `.github/workflows/release-desktop.yml` 中的
`Release Desktop` workflow 从版本 tag 发布。

## 唯一发布入口

- 触发方式：推送匹配 `v*` 的 tag。
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
6. 将发布提交合入 `main`。
7. 推送 `v<version>`。
8. 确认 GitHub Release 包含：
   - `latest.json`
   - `skillport_<version>_windows_x64_nsis.exe`
   - `skillport_<version>_windows_x64_nsis.exe.sig`
   - Windows MSI / ZIP 产物
   - macOS 与 Linux 安装产物（对应 job 成功时）
9. 请求
   `https://github.com/bahayonghang/skills-manage-windows/releases/latest/download/latest.json`，
   确认 version、Windows URL 和 signature 符合预期。

## Updater 不变量

- `src-tauri/tauri.conf.json` 中默认关闭 `bundle.createUpdaterArtifacts`，
  并保留 updater public key placeholder，供本地构建使用。
- 正式 release workflow 必须注入真实 updater public key，并为 Windows 构建
  开启 updater artifacts。
- 在 release metadata 包含 macOS / Linux 平台项之前，应用内更新仅支持
  Windows x64。
- `/releases/latest/download/latest.json` endpoint 假设 GitHub 最新 release
  始终是包含 `latest.json` 的桌面发布。

Last reviewed: 2026-05-27
