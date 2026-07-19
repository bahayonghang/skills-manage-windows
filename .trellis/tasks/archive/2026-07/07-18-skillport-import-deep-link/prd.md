# SkillPort 导入深链

## Goal

增加 `skillport://` 导入深链，让浏览器、文档或外部工具把一个 GitHub 仓库来源交给 SkillPort，并在 Central 的统一导入入口中预填。深链只传递意图，用户仍必须完成 Preview 与 Confirm，不能触发静默安装。

## Dependencies

- `07-18-unified-skill-import` 已完成并归档（后端 `dca1afa9`、前端 `2bb8c774`、归档 `14fbf5f2`）。它交付了 `SkillImportLauncher`，但当前实现只有 `onOpenIntent("github" | "local_zip")` 组件回调，并不存在规划中假定的 `openImportIntent({ kind: "github", source })` controller。本子任务必须在不修改归档任务工件的前提下补齐这个 typed controller，再让 Central、Marketplace 和 deep-link 复用。
- 实施需要用户单独批准两个 Rust 生产依赖：`tauri-plugin-deep-link = "2.4.9"` 与 `tauri-plugin-single-instance = "2.4.3"`。两者均为 `Apache-2.0 OR MIT`，要求 Rust 1.77.2、依赖 Tauri 2.10；仓库当前锁定 Tauri 2.11.0、使用 Rust 1.97.0，版本范围兼容。
- 本设计不使用 deep-link JavaScript guest API，也不存在 single-instance JavaScript 包，因此不新增 npm/pnpm 生产依赖；frontend 继续使用已有 `@tauri-apps/api` 的 event API 接收 native 自定义 typed event。

## Background

- 当前 `src-tauri/tauri.conf.json` 仅配置 sql/updater，没有 scheme 或 deep-link plugin；`src-tauri/src/lib.rs:135-141` 的首个插件目前是 sql。
- 当前 app router 已有 `/central`，GitHub wizard 可由 Central/Marketplace 打开，但两处各自维护 `isGitHubImportOpen` / `githubRepoUrl` 局部状态。wizard 的 step/selection 也在组件内，当前没有跨进程 channel、typed intent controller 或可共享 dirty/pending 状态。
- 当前 GitHub parser 位于 `src-tauri/src/services/github_import/source.rs:67`，会规范化 owner/repo/branch/subpath，但目前为 `pub(super)`，且 GitHub UI 允许 shorthand；deep-link 层必须先强制 HTTPS GitHub URL、安全参数与凭据边界，再复用该 parser，而不是复制 branch/subpath 规则。
- SkillKit 在 main process 中分派 `skillkit://auth` 与 `skillkit://share`（`ref/skillkit/apps/desktop/electron/main.ts:84`）；SkillPort 不需要账号/share 语义，只借鉴 cold-start queue 与已运行窗口转发。

## Requirements

### R1. URI 契约

- 公开契约固定为 `skillport://import?source=<percent-encoded HTTPS GitHub URL>`。
- parser 将 URI 总长度限制为 4096 bytes，并限制 scheme/host/action、空 path/fragment、单一 `source` 参数和 HTTPS GitHub allowlist；拒绝重复/未知参数、缺失 source、userinfo、端口、source query/fragment、非 HTTPS、非 GitHub host、控制字符、反斜线和路径穿越编码。
- source 最终仍通过现有 GitHub URL parser/normalizer，不在 deep-link 层复制 branch/subpath 规则。

### R2. Intent-only 安全边界

- URI 不允许携带 PAT/token/credential/auth、local file/UNC path、SSH/WSL target、skill selection、overwrite/rename/skip、目标 agent、自动确认或任意 command 名；所有 outer unknown 参数和 source 自身 query/fragment 均 fail closed。
- 有效 URI 只导航到 Central，并调用本任务补齐的 `openImportIntent({ kind: "github", source })` 统一入口预填 source；不得在 native handler 中调用 preview/import command。
- 无论 cold/warm 路径，用户必须看到并主动通过现有 Preview/Confirm。

### R3. Windows 生命周期

- `tauri-plugin-single-instance` 必须作为 Tauri builder 的第一个插件注册；第二个注册 `tauri-plugin-deep-link`，现有 sql/fs/dialog/shell/process/updater 全部在其后。官方说明插件按 builder 添加顺序运行，single-instance 必须首位。
- 不启用 single-instance 的可选 `deep-link` feature：该 feature 会在用户 callback 前额外触发 deep-link plugin event。本任务改由 cold `get_current()` 与 warm callback 的完整 argv 分别提取 URI，再统一进入自有 parser/queue，避免双入口和初始化竞态。
- 覆盖安装包注册 scheme、应用未运行时通过 `get_current()` 接收 cold URI、应用已运行时通过第二实例 argv 转发到主实例、窗口未 ready 时排队、ready command 幂等 flush 且每项只发送一次。
- 多个 URI 连续到达时进入最多 8 条、按规范化 source 去重的 FIFO；超过上限丢弃最旧项并记录脱敏 warning，不得无界缓存。
- 已运行实例收到 intent 后恢复/聚焦主窗口，再发送 frontend event。

### R4. 前端行为

- frontend event 进入单一 import-intent store/controller；payload 形状必须防御性校验，通过后由 router 导航 `/central` 并调用统一 launcher。
- Central 与 Marketplace 必须把 GitHub wizard 的 open/source/preview/import 状态接到同一 controller。若当前 session dirty，不能覆盖用户输入；新 intent 进入最多 8 条、规范化 source 去重的 pending FIFO，并显示明确提示与数量。
- 用户关闭 dirty wizard 后可以消费 FIFO 首项或丢弃；关闭提示也不能自动 Preview/Confirm。重复当前/queued source 和无效 frontend event 安全忽略或显示脱敏、本地化错误。
- 无效/过长 URI 显示本地化、脱敏错误，不回显完整恶意 payload，不导致崩溃或打开外部 URL。

### R5. 平台与打包

- Windows 是验收主平台；其他平台只在插件原生支持且无需扩大范围时配置。
- `tauri.conf.json > plugins > deep-link > desktop > schemes` 只配置 `skillport`。Tauri CLI 2.11 会把该配置映射到 bundler `deep_link_protocols`，NSIS 在 `SHCTX\\Software\\Classes\\skillport` 写入 `URL Protocol`、icon 和 `"<installed exe>" "%1"` command，并在卸载时仅删除仍指向本安装目录的键。
- native Rust 使用 deep-link plugin 的 cold `get_current` 与 single-instance 的 warm callback argv，frontend 只监听自定义 event，所以不调用 deep-link guest command；当前 capability 的 `core:default` 已包含 core event 权限，不新增 `deep-link:default`。single-instance 无 JavaScript API，也不需要 capability。
- 修改 Tauri plugin/config、NSIS bundle 和 app initialization 时，必须跑完整 Windows `pnpm tauri build`，安装实际产物后验证 `HKCU:\Software\Classes\skillport`（当前 NSIS `currentUser`）及 open command。
- 卸载/回滚不得破坏普通 GitHub UI 导入。

## Acceptance Criteria

- [x] pure parser tests 覆盖有效 repo/branch/subpath、未知 action、重复/缺少 source、非 HTTPS、非 GitHub、userinfo、控制字符、穿越编码、过长 URI 和敏感参数。
- [x] 有效深链只打开 Central 的 GitHub import 并预填；未发生 preview/import IPC，直到用户主动操作。
- [x] Windows 冷启动能在 frontend ready 后消费一次；应用已运行时第二实例把 intent 转发到主窗口并聚焦。
- [x] 代码审查与 Windows warm-instance 验收确认 single-instance 是 builder 首个注册插件，deep-link/其他插件在其后初始化。
- [x] Central 或 Marketplace wizard 已有未完成状态时不会被覆盖；用户看到 pending 数量，并可在关闭后消费 FIFO 首项或丢弃。
- [x] native 与 frontend 连续 intent 均按规范化 source 去重并以最多 8 条 FIFO 顺序消费，溢出时丢弃最旧项；ready command 重复调用不重放，无效 payload 被脱敏记录并安全忽略。
- [x] 普通 Central/Marketplace GitHub 导入和 SSH/WSL target 行为无回归。
- [x] 相关 Vitest、Rust tests、`cargo clippy -- -D warnings`、`git diff --check`、`just ci` 和 Windows `pnpm tauri build` 通过。
- [x] 安装最新 NSIS 后，读取 `HKCU:\Software\Classes\skillport` 证明 scheme/open command 指向最新安装路径；PowerShell `Start-Process` 的 cold/warm 深链均完成手工验收，并记录命令、进程/窗口、脱敏日志和截图。

## Out of Scope

- OAuth、账号、分享短链、云服务或网页 receiver page。
- `skillport://install` 静默安装、自动冲突决策、目标平台选择。
- 通过深链传本地 ZIP/file URI、PAT 或 SSH 凭据。
- Universal Links/App Links、浏览器扩展或公开网页按钮的开发。
