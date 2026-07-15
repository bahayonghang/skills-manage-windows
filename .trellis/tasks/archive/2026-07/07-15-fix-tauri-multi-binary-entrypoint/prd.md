# 修复 Tauri 多 binary 桌面入口

## Goal

修复 CLI 引入后 just dev 无法选择 binary、NSIS 将 CLI 安装为桌面主程序的问题，并补齐入口契约验证。

## Background

- `src-tauri/Cargo.toml` 同时存在隐式 `skillport` GUI binary 和 `src/bin/skillport-cli.rs`，但 `[package]` 没有 `default-run`。
- `just dev` 当前执行无 `--bin` 的 `cargo run`，Cargo 因两个 binary 无法选择而返回 101。
- 已安装的 `skillport.exe` 实测在 0.72 秒后以 exit 2 退出，并打印 CLI usage；显式 `cargo build --release --bin skillport` 生成的 GUI 可持续运行。
- Tauri 2.11.2 的 `mainBinaryName` 只重命名已选中的 app binary，不负责选择 Cargo target；现有 backend spec 将其写成 target pin，结论错误。

## Requirements

- 在 Cargo package 层显式把 `skillport` 设置为默认运行 binary，同时修复 `cargo run` 与 Tauri app binary 选择。
- 保持 `skillport-cli` 为独立 binary，现有 CLI 命令、JSON/human 输出与 `cargo install --bin skillport-cli` 契约不变。
- 修正 `.trellis/spec/backend/shared-local-cli.md` 的多 binary 打包约定，明确 `default-run` 与 `mainBinaryName` 的不同职责。
- 在 `just ci` 覆盖的检查链中验证 Cargo metadata 的 package、`default_run` 和两个 binary target，避免以后只检查产物存在却再次打包错误入口。
- 改动保持最小，不重构 Tauri 初始化、CLI 业务逻辑或通用构建编排。

## Acceptance Criteria

- [x] `cargo metadata` 返回 `default_run = "skillport"`，并同时包含 `skillport` 与 `skillport-cli` bin targets。
- [x] `just dev` 不再出现 `cargo run could not determine which binary to run`，桌面进程成功启动。
- [x] manifest 入口契约检查纳入 `just ci` 并能在缺失或错误 `default-run` 时返回非零退出码。
- [x] `just ci` 全量通过。
- [x] Windows `pnpm tauri build` 通过，`target/release/skillport.exe` 与 `skillport-cli.exe` 分别运行 GUI 和 CLI 入口。
- [x] `just install` 后快捷方式启动的已安装 `skillport.exe` 持续运行，不输出 CLI usage 或 exit 2。
- [x] backend spec 的 Wrong/Correct 示例以 Cargo `default-run` 作为主入口选择机制。

## Out Of Scope

- 改变 CLI 命令、输出 schema 或业务行为。
- 将 CLI 自动写入 Windows PATH。
- 重构 `scripts/build.mjs`、NSIS 模板或 Tauri 应用初始化。
