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
- 中央技能目录：`~/.agents/skills/`
- 本地数据库：`~/.skillsmanage/db.sqlite`

## 常用命令

```powershell
pnpm install
just ci
just dev
just build
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

- `just ci`：跑前端 `typecheck`、`lint`，以及 Rust `cargo clippy`。
- `just dev`：直接启动 Tauri 开发模式。
- `just build`：跑 `pnpm tauri build`，然后把 `src-tauri/target/release/bundle/nsis/` 里最新的 Windows 安装包复制到根目录 `outputs/`。
- 改 CI 或发布流程时，优先保持 `just ci` 和 GitHub Actions 的检查项一致，避免本地和远端两套标准。

## 修改约束

- 所有用户可见文本都走 i18n。改文案时同步检查 `README.md` / `README_CN.md` 和前端中英文资源。
- 涉及技能安装、卸载、中央化链路时，优先复用现有 `linker.rs` 逻辑，尤其是 `ensure_centralized` 约束。
- Windows 相关命令默认按 PowerShell 场景写，避免只适用于 bash 的写法。
- 改打包链路时，不要只验证前端构建。要把 Tauri Windows bundle 一起看成验收范围。

## 验证要求

- 前端改动：默认跑 `pnpm typecheck && pnpm lint`
- 交互或状态相关改动：按改动范围补跑对应的 Vitest 用例
- Rust 改动：默认跑 `cargo clippy -- -D warnings`
- 涉及平台文件系统差异的 Rust 变更：按需要补跑对应平台的定向测试
- 打包或发布改动：至少在 Windows 上跑通 `pnpm tauri build`，并确认安装产物实际生成
