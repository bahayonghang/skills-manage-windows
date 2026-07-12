# 迁移 SSH 到 russh 持久会话（条件子任务）

## Goal

如果父任务完成 lazy open 与批量 remote apply 后仍无法达到 SSH 性能门槛，将系统 `ssh.exe` 的每命令新连接模型迁移为可复用的 `russh` 持久 session，消除单 skill 和小批量操作的固定握手延迟。

本任务是 `.trellis/tasks/07-11-remote-update-performance` 的条件子任务。父任务性能验收前保持 `planning`，不得提前启动。

## Requirements

- 启动条件：父任务固定 10-skill warm fixture 的 LAN SSH apply 仍超过 Local 4 秒以上，或单 skill SSH 固定延迟仍不满足用户体验目标。
- 启动后必须重新研究当前 `russh` 版本、许可证、维护状态、Rust/Tokio 兼容性和 Windows Tauri 打包影响；不得直接采用 2026-05 计划中的旧版本号。
- 保持 password、OpenSSH private key、加密 key/passphrase、connect timeout、keepalive、host-key trust、错误分类和凭据保护能力。
- 明确定义 known-hosts 迁移与首次信任行为，不能静默降低当前 `StrictHostKeyChecking=accept-new` 的安全语义。
- 复用同 target 的 session；不同 target、凭据变更、应用退出和连接失效必须正确关闭或重建 session。
- 保留可注入测试缝，并覆盖并发 channel、取消、超时、断线重连和敏感信息脱敏。
- 不采用 OpenSSH ControlMaster；除非用户另行推翻历史决策，否则迁移完成后不保留双生产实现。

## Acceptance Criteria

- [ ] 真实 `dckj` SSH target 上连续操作只建立一次 SSH transport session，后续命令只创建 channel。
- [ ] 固定 1/10/25-skill fixture 达到父任务中确认的 SSH 性能门槛，并保存前后基线。
- [ ] password 与 key 两种认证均有自动化测试和至少一条真实主机验证路径。
- [ ] host-key 不匹配、认证失败、超时、断线重连和取消返回稳定的 typed error，且日志不泄露凭据。
- [ ] Windows `pnpm tauri build` 通过并生成 NSIS 安装包。
- [ ] 相关 Rust 测试、`cargo clippy -- -D warnings` 与 `just ci` 全部通过。

## Out Of Scope

- 父任务已能通过批量化满足的性能问题。
- SFTP、端口转发、reverse tunnel 等与 skill 管理无关的扩展能力。
- 在父任务性能门槛确认前实现或引入 `russh` 依赖。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
