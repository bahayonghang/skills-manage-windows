# 替换 SkillPort 应用图标为 B2 路由枢纽

## Goal

把 SkillPort 桌面应用图标换成已锁定的 B2「三仓枢纽」稿，并让 Windows / macOS / iOS / Android 全套 Tauri 图标与参考资产一致。

## Background

- 当前仓库图标是 0.10.13 的等距门户（`src-tauri/icons/icon-source.png`，提交 `c02ef2f8`）。
- 用户从四套方向里选定方案 B，再从 B1 / B2 / B3 里锁定 **B2**：深色三仓坞 + 三块立着的技能卡（左薰衣草、中 Mauve 带桃色方块、右青绿），Catppuccin Mocha 圆角底。
- 会话里已合成过 1024 RGBA 主图，路径为会话 `images/b2-master.png`。该主图曾写入 `src-tauri/icons/`，但工作区已回到门户稿，需要重新落地。
- `scripts/generate_icon.py` 仍按旧「L 形色块 + sparkle」画图，并覆盖 `icon-source.png`。保留该行为会冲掉 B2。
- `src-tauri/tauri.conf.json` 引用 PNG、ICNS、ICO。`src-tauri/icons/` 还包含 Windows Store、Android、iOS 尺寸。只换核心文件会混用新旧图标。
- 上一轮同类任务 `07-13-skillport-icon-0-10-13` 的做法：1024 透明源进仓库，不提交 4096 源图，用 `pnpm tauri icon` 重建全套。

## Requirements

- 以 B2 主图作为仓库规范源：`src-tauri/icons/icon-source.png` 为 1024×1024 RGBA，Mocha 圆角主体，圆角外透明。`ref/` 被 `.gitignore` 忽略，不作为 Git 真源。
- 用仓库现有 `pnpm tauri icon src-tauri/icons/icon-source.png --ios-color "#1e1e2e"` 重建 `src-tauri/icons/` 全套平台图标；Android 自适应背景为 `#1e1e2e`。
- 修改 `scripts/generate_icon.py`：停止绘制 L 形色块；不得覆盖 `icon-source.png`。脚本只核验主图存在，并打印 `pnpm tauri icon` 用法。
- 不改应用名、bundle identifier、安装器配置、更新器配置、版本号和用户可见文案。

## Acceptance Criteria

- [x] `src-tauri/icons/icon-source.png` 是 B2 三仓枢纽，1024×1024 RGBA，圆角外透明。
- [x] `src-tauri/tauri.conf.json` 引用的每个图标文件都存在，且画面为 B2，不再是等距门户。
- [x] Windows Store、iOS、Android 生成型图标均为 B2；`android/values/ic_launcher_background.xml` 为 `#1e1e2e`。
- [x] 运行 `python scripts/generate_icon.py` 不会覆盖或重画 `icon-source.png`。
- [x] `just ci` 通过。
- [x] 不改 `package.json` / `tauri.conf.json` / `Cargo.toml` 中的当前版本号。

## Out Of Scope

- 升版本、写 release notes、打 tag、发 GitHub Release 或打 Windows 安装包。
- 改 `index.html` 启动闪屏（当前是三根 CSS 条，不是应用图标）。
- 重画 README / 文档截图。
- 用程序重绘 B2 的三维坞体。
- 把 4096 源图纳入 Git。
