# AGENTS.md

## 项目定位

- 本仓库是 `iamzhihuix/skills-manage` 的 fork。
- 上游 `README.md` 当前只写明提供 Apple Silicon macOS 预编译包，其他平台先从源码运行。
- 这个 fork 的明确目标，是在本地稳定构建 **Windows 安装包**。后续涉及 Tauri 打包、资源文件、发布说明、构建脚本、依赖升级时，默认先保证 Windows 构建链路可用，不把 Windows 当次要平台。

## 技术栈与目录

- `src/`：React + TypeScript + Tailwind CSS 4 前端。
- `src/stores/`：Zustand 状态层。组件里不要直接调 Tauri `invoke()`。
- `src/components/skill/UnifiedSkillCard.tsx`：技能卡片唯一实现，新增场景优先复用。
- `src-tauri/src/`：Rust 后端，含 commands、数据库、linker、marketplace。
- 私有 Central 技能目录：`~/.skillsmanage/skills/`；Universal Agents 共享目标：`~/.agents/skills/`
- 本地数据库：`~/.skillsmanage/db.sqlite`

## 常用命令

```powershell
pnpm install
just ci
just dev
just build
just install
pnpm tauri dev
pnpm build
pnpm test
pnpm typecheck
pnpm lint
cd src-tauri; cargo test
cd src-tauri; cargo clippy -- -D warnings
pnpm tauri build
```

## justfile 约定

- `just ci`：并行跑前端 `typecheck`、`lint`、`sizecheck`、Vitest、生产构建，以及 Rust entrypoint 契约、`fmt --check`、全 targets Clippy、测试；Cargo 检查使用锁文件。
- `just dev`：直接启动 Tauri 开发模式。
- `just build`：跑 `pnpm tauri build`，然后把 `src-tauri/target/release/bundle/nsis/` 里最新的 Windows 安装包复制到根目录 `outputs/`。
- `just install`：跑 `just build`，然后以 passive 模式运行根目录 `outputs/` 里的最新 NSIS 安装包。
- 改 CI 或发布流程时，优先保持 `just ci` 和 GitHub Actions 的检查项一致，避免本地和远端两套标准。
- GitHub Actions 的 `just-ci` 在指向 `main` 的 PR、手动触发和 release 上运行；跨平台 smoke package 只在手动触发或 release 上运行。

## 修改约束

- 所有用户可见文本都走 i18n。改文案时同步检查 `README.md` / `README_CN.md` 和前端中英文资源。
- 涉及技能安装、卸载、中央化链路时，优先复用现有 `linker.rs` 逻辑，尤其是 `ensure_centralized` 约束。
- Windows 相关命令默认按 PowerShell 场景写，避免只适用于 bash 的写法。
- 改打包链路时，不要只验证前端构建。要把 Tauri Windows bundle 一起看成验收范围。

## Code Review Rules

### 共享状态与 Central 变更

- 把数据库 schema、技能 `uid` / 引用解析、导入安装服务和 Central 文件变更视为桌面端与 `skillport-cli` 的共享兼容性契约。只在 Tauri command 或只在 CLI 路径生效的修复、重新生成已持久化的 `uid`、绕过跨进程 Central mutation lock，都会造成另一入口状态错乱或用户技能丢失。安全路径：将共享行为放在 Rust service / repository 层，使用向后兼容的迁移并保留现有 `uid` 语义；Central 写操作复用现有中央化、安装和变更锁链路，中央库迁移时保留目标仅有技能且不删除旧目录。

### 凭据与可移植数据边界

- GitHub PAT、AI API key、SSH 密码及私钥内容不得进入 SQLite 明文、日志、错误文本、遥测或状态导出文件。安全路径：复用 `SecretStore` 边界，优先写入操作系统凭据库，Windows 仅回退到 DPAPI 保护的本地文件，持久化不可用时只留在当前会话；旧明文设置只能在受保护存储写入并回读成功后删除，所有导出和 operation log 保持脱敏。

### Windows 发布与更新契约

- 审查 Tauri 配置、发布 workflow、版本/依赖和产物命名变更时，保护 Windows x64 发布面：已签名 NSIS、对应 `.sig`、指向该产物的 `latest.json`，以及 MSI / ZIP 产物必须保持一致。本地 bundle 成功但签名或更新元数据缺失，仍然是发布回归。安全路径：同步更新唯一桌面发布 workflow、`latest.json` 生成器和 release preflight；本地配置继续使用占位公钥并关闭 updater artifacts，只在发布 workflow 中注入真实公钥并启用签名产物。

## 验证要求

- 任何任务收尾前，至少跑一遍 `just ci` 并确认通过；如果失败，先修到通过再宣布完成。
- 前端改动：默认跑 `pnpm typecheck && pnpm lint`
- 交互或状态相关改动：按改动范围补跑对应的 Vitest 用例
- Rust 改动：默认跑 `cargo fmt --all -- --check`、`cargo clippy --all-targets --locked -- -D warnings` 和 `cargo test --locked`
- 涉及平台文件系统差异的 Rust 变更：按需要补跑对应平台的定向测试
- 打包或发布改动：至少在 Windows 上跑通 `pnpm tauri build`，并确认安装产物实际生成

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
