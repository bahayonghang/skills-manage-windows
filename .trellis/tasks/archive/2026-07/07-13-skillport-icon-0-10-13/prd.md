# 替换 SkillPort 图标并升级 0.10.13

## Goal

使用 `ref/SkillPort-icon-assets/` 中的新 SkillPort 图标替换仓库现有应用图标，并将桌面应用版本元数据同步升级到 `0.10.13`，确保 Windows 安装包使用新的统一品牌图标和正确版本号。

## Background

- 新资产包含 4096x4096、1024x1024 透明 PNG 源图，以及一组 Tauri 核心图标产物。
- `src-tauri/tauri.conf.json` 同时引用 PNG、ICNS 和 ICO 图标；`src-tauri/icons/` 还包含 Windows Store、Android 和 iOS 尺寸资源。只替换核心文件会导致仓库混用新旧图标。
- 当前发布版本 `0.10.12` 分别记录在 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 和根 crate 对应的 `src-tauri/Cargo.lock` 条目中。

## Requirements

- 将 `SkillPort-icon-transparent-1024.png` 保存为仓库内的规范图标源，并通过仓库现有 Tauri CLI 重建 `src-tauri/icons/` 的完整平台图标集；不提交 5.21 MiB 的 4096x4096 源图，避免无必要地增大 Git 历史。
- Tauri CLI 的二进制编码输出与用户提供的五个核心 PNG/ICO 资产不同，因此这些核心资源必须精确采用 `ref/SkillPort-icon-assets/src-tauri/icons/` 中的文件；其余平台资源由 1024x1024 规范源生成。
- 将 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 和 `src-tauri/Cargo.lock` 中的应用版本同步为 `0.10.13`。
- 保留现有应用标识符、安装器配置和用户可见文案，不改动功能行为。
- 保留 `release-notes/0.10.12.md` 作为历史发布记录；本任务不创建 `0.10.13` tag、GitHub Release 或发布说明。

## Acceptance Criteria

- [x] `src-tauri/icons/icon-source.png` 使用新 SkillPort 透明源图，所有生成型平台图标均已切换到新设计且尺寸/格式有效。
- [x] `src-tauri/tauri.conf.json` 引用的每个图标文件都存在，Windows 使用的 `icon.ico` 和 Store logo 资源不再包含旧设计。
- [x] 四处版本元数据均为 `0.10.13`，仓库中除历史记录和归档任务外不存在仍应升级的 `0.10.12` 当前版本声明。
- [x] `just ci` 通过。
- [x] 在 Windows 上执行 `pnpm tauri build` 成功，并实际生成 `0.10.13` NSIS 安装包。

## Out Of Scope

- 发布 GitHub tag、Release 或上传安装产物。
- 修改前端界面、应用名称、bundle identifier 或更新器配置。
