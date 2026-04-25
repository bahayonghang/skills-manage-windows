# SkillPort

`SkillPort` 是一个基于 Tauri 的桌面应用，用来在一个界面里统一管理多平台 AI coding agent skills。

[English](README.md)

> **免责声明**
>
> `SkillPort` 是一个独立的非官方桌面应用，用于管理本地 skill 目录并导入公开 skill 元数据。它与 Anthropic、OpenAI、GitHub、MiniMax 或其他受支持平台、发布方、商标所有者均无隶属、背书或赞助关系。

## 项目简介

`SkillPort` 遵循 [Agent Skills](https://github.com/anthropics/agent-skills) 的开放模式，但中央技能库使用私有目录 `~/.skillsmanage/skills/`。共享的 Universal Agents 目标仍是 `~/.agents/skills/`，只有显式安装到这里的技能才会暴露给 Codex CLI、Cursor、Gemini CLI 等读取该目录的工具。

## 与上游关系

`SkillPort` 源自上游 [`iamzhihuix/skills-manage`](https://github.com/iamzhihuix/skills-manage)。本 fork 独立维护和分发。上游项目仍作为原始基础保留致谢；当前 fork 保持 Windows-first 构建和发布契约，用于安装包打包与 release 工作流。

## 核心能力

- 中央技能库与按平台安装、卸载工作流。
- 完整技能详情视图，支持 Markdown 预览、原始源码查看和 AI 解释生成。
- 通过技能集合整理和批量安装 skills。
- 支持扫描本地项目级 skill 库的 Discover 能力。
- 支持 marketplace 浏览，以及带鉴权请求和重试回退的 GitHub 仓库导入。
- 通过延迟查询、懒加载索引和虚拟列表提升大规模 skill 库搜索体验。
- 提供中英文界面、Catppuccin 主题、强调色、首次引导和响应式导航。

## 项目截图

### 中央技能库与平台安装

![中央技能库视图](images/01.png)

### 查看特定平台的已安装技能

![平台技能视图](images/06.png)

### 扫描本地项目技能库

![项目技能库发现页](images/03.png)

### 浏览 marketplace 发布者与技能

![技能市场视图](images/04.png)

### 从 GitHub 仓库导入技能

![GitHub 仓库导入向导](images/02.png)

### 管理可复用技能集合

![技能集合视图](images/05.png)

## 下载

- 最新发布：<https://github.com/bahayonghang/skills-manage-windows/releases/latest>
- 当前已提供的预编译安装包：Windows x64（`.exe`、`.msi`、`.zip`）和 macOS Universal（`.dmg`、`.zip`、`.tar.gz`）
- 其他平台：当前请从源码运行

### macOS 未签名构建说明

当前公开发布的 macOS 安装包还没有 notarization。如果 macOS 提示：

![macOS 应用损坏警告](images/app-damaged.png)

- `"SkillPort" is damaged and can't be opened`
- `"SkillPort" cannot be opened because Apple could not verify it`

这通常不代表安装包真的损坏，而是未签名应用被 Gatekeeper 的 quarantine 机制拦截。

把应用移动到 `/Applications` 后，执行：

```bash
xattr -dr com.apple.quarantine "/Applications/SkillPort.app"
```

然后回到 Finder 再次打开应用。如果你的应用不在 `/Applications`，把命令中的路径替换成实际 `.app` 路径即可。

## 支持的平台

| 类别 | 平台 | Skills 目录 |
|------|------|------------|
| Coding | Claude Code | `~/.claude/skills/` |
| Coding | Codex CLI | `~/.agents/skills/` |
| Coding | Cursor | `~/.agents/skills/` |
| Coding | Gemini CLI | `~/.agents/skills/` |
| Coding | Trae | `~/.trae/skills/` |
| Coding | Factory Droid | `~/.factory/skills/` |
| Coding | Junie | `~/.junie/skills/` |
| Coding | Qwen | `~/.qwen/skills/` |
| Coding | Trae CN | `~/.trae-cn/skills/` |
| Coding | Windsurf | `~/.windsurf/skills/` |
| Coding | Qoder | `~/.qoder/skills/` |
| Coding | Augment | `~/.augment/skills/` |
| Coding | OpenCode | `~/.agents/skills/` |
| Coding | KiloCode | `~/.kilocode/skills/` |
| Coding | OB1 | `~/.ob1/skills/` |
| Coding | Amp | `~/.agents/skills/` |
| Coding | Kiro | `~/.kiro/skills/` |
| Coding | CodeBuddy | `~/.codebuddy/skills/` |
| Coding | Hermes | `~/.hermes/skills/` |
| Coding | Copilot | `~/.agents/skills/` |
| Coding | Aider | `~/.aider/skills/` |
| Lobster | OpenClaw（开爪） | `~/.openclaw/skills/` |
| Lobster | QClaw（千爪） | `~/.qclaw/skills/` |
| Lobster | EasyClaw（简爪） | `~/.easyclaw/skills/` |
| Lobster | EasyClaw V2 | `~/.easyclaw-20260322-01/skills/` |
| Lobster | AutoClaw | `~/.openclaw-autoclaw/skills/` |
| Lobster | WorkBuddy（打工搭子） | `~/.workbuddy/skills-marketplace/skills/` |
| Central | 中央技能库 | `~/.skillsmanage/skills/` |

> 说明：Claude Code 还会把 `~/.claude/plugins/marketplaces/*` 下的 marketplace plugin 目录显示成只读行。这些条目只做展示，不按 `~/.claude/skills/` 里的原生技能那套方式管理。

也可以在 Settings 中添加自定义平台。

## 隐私与安全

- **本地优先** — 元数据、集合、扫描结果、设置和 AI explanation 缓存都保存在 `~/.skillsmanage/db.sqlite` 或你自己管理的本地 skill 目录中。`.skillsmanage` 路径会继续保留，用来兼容已有安装。
- **无遥测** — 应用不包含分析、崩溃上报或使用追踪。
- **网络访问由功能触发** — 只有在你显式使用 marketplace 同步/下载、GitHub 导入或 AI explanation 时才会发起外部请求。
- **凭据仅本地存储** — GitHub PAT 和 AI API key 会保存在本地 SQLite settings 表中，应用本身不提供静态加密。
- 不要在 issue、PR、截图或日志里公开真实密钥。

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri v2 |
| 前端 | React 19、TypeScript、Tailwind CSS 4 |
| UI 组件 | shadcn/ui、Lucide icons |
| 状态管理 | Zustand |
| Markdown | react-markdown |
| 国际化 | react-i18next、i18next-browser-languagedetector |
| 主题 | Catppuccin 4 种风格 |
| 后端 | Rust（serde、sqlx、chrono、uuid） |
| 数据库 | SQLite via sqlx（WAL 模式） |
| 路由 | react-router-dom v7 |

## 开发

### 前置依赖

- [Node.js](https://nodejs.org/)（LTS）
- [pnpm](https://pnpm.io/)
- [Rust toolchain](https://rustup.rs/)（stable）
- Tauri v2 系统依赖：<https://v2.tauri.app/start/prerequisites/>

### 安装依赖

```bash
pnpm install
```

### 常用 just 命令

```bash
just ci
just dev
just build
```

- `just ci` 会运行前端 `typecheck`、`lint`，以及 Rust 的 `cargo test` 和 `cargo clippy`。
- `just dev` 会直接启动 Tauri 开发应用。
- `just build` 会构建桌面应用，并把 `src-tauri/target/release/bundle/nsis/` 里最新的 NSIS 安装包复制到 `outputs/`。

### 启动开发环境

```bash
pnpm tauri dev
```

Vite 开发服务器默认使用 `24200` 端口。

### 验证命令

```bash
pnpm test
pnpm typecheck
pnpm lint
cd src-tauri && cargo test
cd src-tauri && cargo clippy -- -D warnings
```

## 项目结构

```text
skillport/
├── src/                        # React 前端
│   ├── components/             # UI 组件
│   ├── i18n/                   # 语言文件和 i18n 配置
│   ├── lib/                    # 前端工具函数
│   ├── pages/                  # 路由页面
│   ├── stores/                 # Zustand stores
│   ├── test/                   # Vitest + RTL 测试
│   └── types/                  # 共享 TypeScript 类型
├── src-tauri/                  # Rust 后端
│   └── src/
│       ├── commands/           # Tauri IPC 处理器
│       ├── db.rs               # SQLite schema、迁移、查询
│       ├── lib.rs              # Tauri 应用初始化
│       └── main.rs             # 桌面入口
├── public/                     # 静态资源
├── CHANGELOG.md                # 英文更新日志
├── CHANGELOG.zh.md             # 中文更新日志
└── release-notes/              # GitHub release notes
```

## 数据库

SQLite 数据库位于 `~/.skillsmanage/db.sqlite`，首次启动时会自动初始化。这个旧目录名会继续保留，避免已有安装丢失当前数据。

## 更新日志

- 英文：[CHANGELOG.md](CHANGELOG.md)
- 中文：[CHANGELOG.zh.md](CHANGELOG.zh.md)

## 参与贡献

开发环境、验证命令和 PR 约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 安全报告

漏洞反馈和数据处理说明见 [SECURITY.md](SECURITY.md)。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=bahayonghang/skills-manage-windows&type=Date)](https://www.star-history.com/#bahayonghang/skills-manage-windows&Date)

## 许可证

本项目使用 Apache License 2.0，详见 [LICENSE](LICENSE)。
