# CLI：just 命令

仓库内置 `justfile` 用于固化开发与打包动作。需要先安装 `just`：<https://just.systems>。

## 配方

| 配方 | 作用 |
| --- | --- |
| `just sync-version` | 读取 `package.json`，把版本写入 `src-tauri/tauri.conf.json` / `Cargo.toml` / `Cargo.lock`。幂等。 |
| `just ci` | 跑前端 `typecheck` + `lint` + `test` + `sizecheck`，再跑 Rust `cargo test` 与 `cargo clippy -- -D warnings`。完整本地门禁。 |
| `just dev` | 启动 Tauri 开发模式（`pnpm tauri dev`）。 |
| `just build` | 按当前平台构建，并把产物拷入 `outputs/`。 |
| `just install` | 仅 Windows。构建 NSIS 安装包并以 passive 模式启动安装。 |

## 实现

每个配方都是 `scripts/` 下 Node 脚本的薄包装：

```text
just sync-version  →  node scripts/sync-version.mjs
just build         →  node scripts/build.mjs
just install       →  node scripts/install.mjs
```

要看每条配方实际做什么，最快的方式是直接读对应 Node 脚本。

## 本地门禁

```text
[just ci] ──► sync-version
              │
              ├── pnpm typecheck
              ├── pnpm lint
              ├── pnpm test
              ├── pnpm sizecheck
              ├── cargo test --manifest-path src-tauri/Cargo.toml
              └── cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

`just ci` 是 CI workflow 用来卡 PR 的同一组命令。push 前先跑一次。

## 输出

`outputs/` 已 gitignore，`just build` 按平台填入产物。示例：

```text
outputs/
├── SkillPort_0.10.0_x64-setup.exe       (Windows, NSIS)
├── SkillPort_0.10.0_x64.msi             (Windows, MSI)
├── SkillPort_0.10.0_x64.zip             (Windows portable)
├── SkillPort_0.10.0_universal.dmg       (macOS)
├── skillport_0.10.0_amd64.deb           (Linux Debian)
├── skillport-0.10.0-1.x86_64.rpm        (Linux RPM)
└── skillport_0.10.0_amd64.AppImage      (Linux AppImage)
```

`just build` 只产出当前运行平台的产物。

Last reviewed: 2026-05-04
