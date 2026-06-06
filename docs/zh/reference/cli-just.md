# CLI：just 命令

仓库内置 `justfile` 用于固化开发与打包动作。需要先安装 `just`：<https://just.systems>。

## 配方

| 配方 | 作用 |
| --- | --- |
| `just sync-version` | 读取 `package.json`，把版本写入 `src-tauri/tauri.conf.json` / `Cargo.toml` / `Cargo.lock`。幂等。 |
| `just ci` | 并行运行前端 `typecheck` → `lint` → `sizecheck` → `test`，以及 Rust `cargo clippy -- -D warnings` → `cargo test`。完整本地门禁。 |
| `just dev` | 启动 Tauri 开发模式（`pnpm tauri dev`）。 |
| `just build` | 按当前平台构建，并把产物拷入 `outputs/`。 |
| `just install` | Windows 上构建 NSIS 安装包并以 passive 模式启动安装；macOS 上显示提醒并改为运行 `just build`。 |

## 实现

大多数配方都是 `scripts/` 下 Node 脚本的薄包装；`just install` 会先做平台分流，再进入构建或安装路径：

```text
just sync-version  →  node scripts/sync-version.mjs
just ci            →  node scripts/run-ci.mjs
just build         →  node scripts/build.mjs
just install       →  macOS：just build；Windows：先 just build，再 node scripts/install.mjs
```

要看每条配方实际做什么，最快的方式是同时读 `justfile` 和对应 Node 脚本。

## 本地门禁

```text
[just ci] ──► sync-version ──► scripts/run-ci.mjs
                                 │
                                 ├─ web:  pnpm typecheck
                                 │        pnpm lint
                                 │        pnpm sizecheck
                                 │        pnpm test
                                 │
                                 └─ rust: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
                                          cargo test --manifest-path src-tauri/Cargo.toml
```

两条链并发执行；任一链失败会停止另一条链并让门禁失败。`just ci` 是 CI workflow 用来卡 PR 的同一组命令。push 前先跑一次。

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

Last reviewed: 2026-05-31
