# 安装

有两条路径：直接装预构建发布包，或从源码编译。

## 预构建下载

- 最新发布：<https://github.com/bahayonghang/skills-manage-windows/releases/latest>
- 当前桌面发布目标：Windows x64（`.exe`、`.msi`、`.zip`）、macOS Universal（`.dmg`、`.zip`、`.tar.gz`），以及 Linux x86_64 / arm64（`.deb`、`.rpm`、`.AppImage`）。
- 当前桌面构建仍未签名；Linux arm64 产物是否可用取决于 GitHub Actions runner 矩阵。

### macOS 未签名构建

当前公开发布的 macOS 安装包还没有 notarization。如果 macOS 提示：

- `"SkillPort" is damaged and can't be opened`
- `"SkillPort" cannot be opened because Apple could not verify it`

这通常不代表安装包真的损坏，而是未签名应用被 Gatekeeper 的 quarantine 机制拦截。把应用移动到 `/Applications` 后执行：

```bash
xattr -dr com.apple.quarantine "/Applications/SkillPort.app"
```

然后回到 Finder 再次打开应用。如果你的应用不在 `/Applications`，把命令中的路径替换成实际 `.app` 路径即可。

## 从源码构建

### 前置依赖

- [Node.js](https://nodejs.org/)（LTS）
- [pnpm](https://pnpm.io/)
- [Rust toolchain](https://rustup.rs/)（stable）
- Tauri v2 系统依赖：<https://v2.tauri.app/start/prerequisites/>

### 安装依赖

```bash
pnpm install
```

### 启动开发环境

```bash
pnpm tauri dev
```

Vite 开发服务器默认使用 `24200` 端口。

### 验证命令

```bash
pnpm test
pnpm sizecheck
pnpm typecheck
pnpm lint
cd src-tauri && cargo test
cd src-tauri && cargo clippy -- -D warnings
```

### Just 快捷命令

```bash
just ci
just dev
just build
just install
```

- `just ci` 运行前端 `typecheck` + `lint` + `test` + `sizecheck`，以及 Rust `cargo test` 和 `cargo clippy`。
- `just dev` 启动 Tauri 开发应用。
- `just build` 按当前平台构建桌面应用，并把最新打包产物复制到 `outputs/`。
- `just install` 构建 Windows NSIS 安装包，复制到 `outputs/`，并以 passive 模式运行安装器。该命令仅支持 Windows。

## 文档站点

本地预览本文档站：

```bash
pnpm docs:dev
pnpm docs:build
pnpm docs:preview
```

站点源码位于 `docs/`。构建产物输出到仓库根的 `dist-docs/`，与桌面应用构建产物 `dist/` 互不冲突。

---

Last reviewed: 2026-05-04
