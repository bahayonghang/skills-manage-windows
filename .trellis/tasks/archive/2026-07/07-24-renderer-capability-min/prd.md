# Renderer 权限最小化与 capability drift check

## Goal

把主 WebView 的文件系统与敏感命令权限收敛到"完成当前 UI 动作所需的最小集合"，并让 capability 文档不再漂移。对应审计 P1-02（🟠）、P2-02（🟡）、P2-09（🟡）与 QW-02/QW-06。

## 核对证据（2026-07-26 dev 分支，live 复核）

- `src-tauri/capabilities/default.json:10-27`：fs scope 含 `$HOME/**`、`$DOCUMENT/**`、`$DOWNLOAD/**`、`$DESKTOP/**`；含 `fs:allow-mkdir`、`fs:allow-read-text-file`、`fs:allow-write-text-file`、`shell:default`。
- `src/components/central/CentralStatePortabilityDialog.tsx:189-211`：renderer 直接 `writeTextFile`/`readTextFile`，读取后才交 backend，前端无大小 cap。
- `src/pages/MarketplaceView.tsx:212-215`：renderer 直接 `writeTextFile` 到 `BaseDirectory.Home`（审计未单列，属同类问题，纳入本任务）。
- `src/lib/externalUrl.ts:1,14-18`：renderer 仍使用 `@tauri-apps/plugin-shell` 打开经 URL parser 限制的外链；因此需把 `shell:default` 收缩为 `shell:allow-open`，不能移除 shell plugin 本身。
- `docs/reference/ipc-capability-inventory.md:19,61-65`：文档声称无 shell frontend import 且 `shell:default` removed，两项均与 live 状态漂移。
- `src-tauri/src/lib.rs:232`、`src-tauri/Cargo.toml:20`、`package.json:37`：`plugin-fs` 同时注册在 Rust runtime 并作为前端依赖；当前 `src/` 业务调用只存在于上述两处。
- `src-tauri/src/lib.rs:474,478` 注册 `reveal_ai_api_key`/`reveal_github_pat`；`src/stores/settingsStore.ts:267-271` 与 `src/stores/settingsStore.aiSlice.ts:359-363` 可直接 invoke 并取得明文。
- `src-tauri/src/commands/portable_state.rs:35-132,135-254` 已有导出和预览 command/service 边界，缺口是安全的本地文件路径 adapter；`src-tauri/src/commands/marketplace.rs:109-120` 已有 backend 安装 command，但 GitHub repo preview 的安装路径仍在页面内旁路。

## Requirements

1. **capability 收紧**：移除 `$HOME/**`、`$DOCUMENT/**`、`$DOWNLOAD/**`、`$DESKTOP/**`、app 专用目录 scope、全部 fs command permission 与 `shell:default`，使 renderer 侧文件系统读写权限归零；外链行为仅保留 `shell:allow-open` 并继续经过 `externalUrl.ts` 的 HTTP(S) 校验。
2. **plugin-fs 后移**：portability 导入/导出与 Marketplace preview 安装写盘改为 backend command——dialog 只返回路径，backend 做 open + metadata + extension + size cap + 解析（重 IO 走 `run_blocking_fs_with`）；export 使用同目录临时文件 + rename 原子替换。Marketplace preview 安装复用现有 Marketplace/GitHub import/Central mutation 服务边界，不直接从 command 或 renderer 拼接 Central 路径。
3. **删除明文 reveal**：按 2026-07-26 用户决策，删除 `reveal_github_pat` / `reveal_ai_api_key` command、service helper、store action 与 Eye 明文查看 UI。已保存 secret 只显示固定掩码；新输入保持 password 类型且不提供 Eye；仍允许覆盖、清除和连接测试。
4. **drift check**：required CI gate 校验 capability 文件、前端 plugin import、plugin 依赖/注册和 `ipc-capability-inventory.md` 一致；文档中的 marker JSON 是唯一机器契约，人类可读清单必须由同一渲染函数确定性生成并通过逐字校验（QW-06）。

## Acceptance Criteria

- [x] `capabilities/default.json` 不含 `$HOME/**` 及三个用户目录通配 scope、不含 `shell:default`
- [x] `src/` 中无 `@tauri-apps/plugin-fs` import，主窗口 capability 无 fs permission/scope；不再需要时移除前端依赖、Rust plugin 注册和 Cargo 依赖
- [x] `shell:default` 被 `shell:allow-open` 取代，HTTP(S) 外链仍可打开，非 HTTP(S) scheme 仍被拒绝
- [x] portability 导入超过 size cap / 非 .json 扩展名时由 backend 拒绝并给出语义化错误
- [x] Marketplace repo preview 安装不再由 renderer `fetch`/写盘，而是复用 backend GitHub import/Central mutation 链路
- [x] 前后端不存在 `reveal_github_pat` / `reveal_ai_api_key` IPC 入口或调用；已保存 secret 永不返回 renderer
- [x] `ipc-capability-inventory.md` 的 marker JSON、人类可读清单与实际 capability 三方一致，drift check 脚本进 CI
- [x] 前端相关测试（CentralStatePortabilityDialog.test.tsx 等）随实现同步更新，`pnpm test`、`just ci` 通过
- [x] Windows `pnpm tauri build` 通过，并确认 `src-tauri/target/release/bundle/nsis/` 下 NSIS 安装包真实生成

## 非目标 / 依赖

- 不在本任务内为全部 custom commands 建 per-command permission（审计修复顺序第 4 步，工作量大，另行拆分或在 design.md 中界定首批 destructive/secret 命令范围）。
- 无前置依赖。涉及用户可见文案走 i18n 双语。

## Key Decision

- 2026-07-26 用户批准删除明文 secret reveal。不会引入 Windows Hello/Credential UI、短时授权 token 或跨平台 OS authentication fallback。
