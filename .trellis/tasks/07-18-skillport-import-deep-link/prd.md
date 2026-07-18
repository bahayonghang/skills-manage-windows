# SkillPort 导入深链

## Goal

增加 `skillport://` 导入深链，让浏览器、文档或外部工具把一个 GitHub 仓库来源交给 SkillPort，并在 Central 的统一导入入口中预填。深链只传递意图，用户仍必须完成 Preview 与 Confirm，不能触发静默安装。

## Dependencies

- 必须等待 `07-18-unified-skill-import` 完成并归档，复用其 `openImportIntent({ kind: "github", source })` 边界。
- 实施前必须单独确认 `tauri-plugin-deep-link` 和 Windows 单实例所需插件的版本、许可与 Tauri 2 兼容性。

## Background

- 当前 `src-tauri/tauri.conf.json:91` 仅配置 sql/updater，没有 scheme 或 deep-link plugin。
- 当前 app router 已有 Central 页面，GitHub wizard 可由 Central/Marketplace 打开，但没有跨进程 import intent channel。
- SkillKit 在 main process 中分派 `skillkit://auth` 与 `skillkit://share`（`ref/skillkit/apps/desktop/electron/main.ts:84`）；SkillPort 不需要账号/share 语义，只借鉴 cold-start queue 与已运行窗口转发。

## Requirements

### R1. URI 契约

- 公开契约固定为 `skillport://import?source=<percent-encoded HTTPS GitHub URL>`。
- parser 将 URI 总长度限制为 4096 bytes，并限制 host/action、单一 `source` 参数和 HTTPS GitHub allowlist；拒绝重复参数、未知 action、userinfo、非 HTTPS、非 GitHub host、控制字符和路径穿越编码。
- source 最终仍通过现有 GitHub URL parser/normalizer，不在 deep-link 层复制 branch/subpath 规则。

### R2. Intent-only 安全边界

- URI 不允许携带 PAT/token、local file path、SSH/WSL target、skill selection、overwrite/rename/skip、目标 agent、自动确认或任意 command 名。
- 有效 URI 只导航到 Central、打开统一入口、选择 GitHub intent 并预填 source；不得在 native handler 中调用 preview/import command。
- 无论 cold/warm 路径，用户必须看到并主动通过现有 Preview/Confirm。

### R3. Windows 生命周期

- `tauri-plugin-single-instance` 必须作为 Tauri builder 的第一个插件注册，再注册 deep-link 与现有插件；该顺序是 Windows warm-instance argv 转发的实现门禁，不得只依赖 frontend-ready 握手补救。
- 覆盖安装包注册 scheme、应用未运行时冷启动、应用已运行时把第二实例参数转发给主实例、窗口未 ready 时排队、ready 后只消费一次。
- 多个 URI 连续到达时进入最多 8 条、按规范化 source 去重的 FIFO；超过上限丢弃最旧项并记录脱敏 warning，不得无界缓存。
- 已运行实例收到 intent 后恢复/聚焦主窗口，再发送 frontend event。

### R4. 前端行为

- frontend event 进入单一 intent store/controller，由 router 导航 Central 并调用统一 launcher。
- 若当前有未完成导入 wizard，不能覆盖用户输入；应显示明确提示并在用户关闭后选择消费或丢弃新 intent。
- 无效/过长 URI 显示本地化、脱敏错误，不回显完整恶意 payload，不导致崩溃或打开外部 URL。

### R5. 平台与打包

- Windows 是验收主平台；其他平台只在插件原生支持且无需扩大范围时配置。
- 修改 Tauri plugin、capability/config、NSIS bundle 和 app initialization 时，必须跑完整 Windows `pnpm tauri build`，安装实际产物后验证 scheme 注册。
- 卸载/回滚不得破坏普通 GitHub UI 导入。

## Acceptance Criteria

- [ ] pure parser tests 覆盖有效 repo/branch/subpath、未知 action、重复/缺少 source、非 HTTPS、非 GitHub、userinfo、控制字符、穿越编码、过长 URI 和敏感参数。
- [ ] 有效深链只打开 Central 的 GitHub import 并预填；未发生 preview/import IPC，直到用户主动操作。
- [ ] Windows 冷启动能在 frontend ready 后消费一次；应用已运行时第二实例把 intent 转发到主窗口并聚焦。
- [ ] 代码审查与 Windows warm-instance 验收确认 single-instance 是 builder 首个注册插件，deep-link/其他插件在其后初始化。
- [ ] wizard 已有未完成状态时不会被覆盖，用户获得可理解选择/提示。
- [ ] 连续 intent 按规范化 source 去重并以最多 8 条 FIFO 顺序消费，溢出时丢弃最旧项；无效 payload 被脱敏记录并安全忽略。
- [ ] 普通 Central/Marketplace GitHub 导入和 SSH/WSL target 行为无回归。
- [ ] 相关 Vitest、Rust tests、`cargo clippy -- -D warnings`、`git diff --check`、`just ci` 和 Windows `pnpm tauri build` 通过。
- [ ] 安装最新 NSIS 后，PowerShell `Start-Process` 的 cold/warm 深链均完成手工验收，并记录安装产物路径与结果。

## Out of Scope

- OAuth、账号、分享短链、云服务或网页 receiver page。
- `skillport://install` 静默安装、自动冲突决策、目标平台选择。
- 通过深链传本地 ZIP/file URI、PAT 或 SSH 凭据。
- Universal Links/App Links、浏览器扩展或公开网页按钮的开发。
