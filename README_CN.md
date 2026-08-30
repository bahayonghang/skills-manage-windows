# SkillPort

`SkillPort` 是一个基于 Tauri 的桌面应用，用来在一个界面里统一管理多平台 AI coding agent skills。

[English](README.md)

> **免责声明**
>
> `SkillPort` 是一个独立的非官方桌面应用，用于管理本地 skill 目录并导入公开 skill 元数据。它与 Anthropic、OpenAI、GitHub、MiniMax 或其他受支持平台、发布方、商标所有者均无隶属、背书或赞助关系。

## 项目简介

`SkillPort` 遵循 [Agent Skills](https://github.com/anthropics/agent-skills) 的开放模式，但中央技能库默认使用私有目录 `~/.skillsmanage/skills/`。在本机 Local 目标下，可以在中央技能库页面修改这个位置：切换前会先预览，迁移时当前中央库覆盖目标目录同名技能，目标目录独有技能会保留并扫描导入，旧目录不会删除。共享的 Universal Agents 目标仍是 `~/.agents/skills/`，只有显式安装到这里的技能才会暴露给 Codex CLI、Cursor、OpenCode、Amp、Copilot 等读取该目录的工具。Grok 按上游兼容的独立目标管理，全局目录为 `~/.grok/skills/`，项目安装目录为 `.grok/skills/`。SkillPort 明确区分 Google 的 Antigravity 应用目标与 Antigravity CLI：Antigravity 全局技能保留在 `~/.gemini/antigravity/skills/`，Antigravity CLI 全局技能使用 `~/.gemini/antigravity-cli/skills/`，两者的 workspace / project 安装都使用 `.agents/skills/`。Gemini CLI 仍作为 legacy/shared Google 目标保留在 `~/.gemini/skills/`。

## 与上游关系

`SkillPort` 源自上游 [`iamzhihuix/skills-manage`](https://github.com/iamzhihuix/skills-manage)。本 fork 独立维护和分发。上游项目仍作为原始基础保留致谢；当前 fork 保持 Windows-first 构建和发布契约，用于安装包打包与 release 工作流。

## 核心能力

- 中央技能库与按平台安装、卸载工作流。
- 在本机 Local 目标上管理 Skills CLI 全局技能（`npx skills -g`）。SkillPort 并不把 `~/.agents/skills/` 整棵树视为 Skills CLI 所有；所有权以 Skills CLI lock 文件为准。
- 完整技能详情视图，支持 Markdown 预览、原始源码查看和 AI 解释生成。
- 通过技能集合整理和批量安装 skills。
- 支持扫描本地项目级 skill 库的 Discover 能力。
- 支持 marketplace 浏览，以及带鉴权请求和重试回退的 GitHub 仓库导入。
- 通过延迟查询、懒加载索引和虚拟列表提升大规模 skill 库搜索体验。
- 提供中英文界面、Catppuccin 主题、强调色、首次引导和响应式导航。
- **中央技能库 V2（默认开启）**：支持结构化查询语法（`tag:`、`repo:`、`owner:`、`has:source` 等）、URL-as-state、保存视图、命令面板（`Ctrl+K`）、标签分组、列表分组视图（不分组 / 按仓库 / 按 owner / 按标签 / 按状态）。通过 Beta 徽章旁的"切回经典布局"链接，或在 DevTools localStorage 中设 `featureFlag.central.newLayout=off`，可退回 V1 布局。

## SSH 远程模式

SkillPort 可以通过 SSH 管理远程 Linux 或 macOS 用户目录里的全局 skills。桌面界面仍在本机运行，后端会连接当前选中的远程目标，并扫描远程用户的 Central 与各平台 skills 目录。

- 在 Settings 中新增、测试、删除和切换 SSH 目标。
- SSH 目标支持 key 和账号密码两种 OpenSSH 登录方式。SkillPort 不保存私钥内容；密码登录会把密码存入系统凭据库，不写入 SQLite。
- 连接成功后探测远程 HOME；远程 Central Skills 使用该主机上的 `~/.skillsmanage/skills/`，Universal Agents 使用 `~/.agents/skills/`，Grok 使用 `~/.grok/skills/`。
- 每个 SSH 目标都有独立的本机缓存数据库：`~/.skillsmanage/targets/<target_id>/db.sqlite`。
- 远程安装默认使用 copy。首版不启用 symlink 安装，也不启用远程 Discover 项目扫描。
- 文件管理器打开动作会改为复制远程路径，因为该路径存在于远程主机，不存在于本机。

远程模式只管理当前远程用户目录。切回 Local 之前，不会修改本机 skills。

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

## 本机 CLI

`skillport-cli` 与桌面端共用同一个本机 SQLite 数据库、稳定技能 `uid`、GitHub/skills.sh 导入服务、安装服务和跨进程 Central mutation lock。

```powershell
npm run cli -- skills list
npm run cli -- skills show <uid、slug 或唯一名称>
npm run cli -- skills search "react" --limit 10
npm run cli -- skills install vercel-labs/agent-skills@react-best-practices --sync
npm run cli -- skills sync <uid 或 slug> --agent codex --method copy --dry-run
```

重复安装默认停止，不会静默覆盖；需要覆盖时必须显式传入 `--replace`，从一个 GitHub URL 批量覆盖多个技能时还需 `--yes`。首版 CLI 只管理 Local 目标。CLI 修改不会向已经运行的桌面窗口推送事件，请在对应页面手动刷新。GitHub 凭据继续读取 SkillPort 现有的受保护 secret store。

使用以下命令把 binary 安装到 `PATH`：

```powershell
cargo install --path src-tauri --bin skillport-cli --locked --force
```

完整的命令参数、JSON 输出、退出码、重复项安全规则和同步工作流见
[SkillPort CLI 使用参考](docs/zh/reference/skillport-cli.md)。

## 下载

- 最新发布：<https://github.com/bahayonghang/skills-manage-windows/releases/latest>
- 当前桌面发布目标：Windows x64（`.exe`、`.msi`、`.zip`）、macOS Universal（`.dmg`、`.zip`、`.tar.gz`），以及 Linux x86_64 / arm64（`.deb`、`.rpm`、`.AppImage`）
- Windows 自动更新使用 Tauri 对最终 NSIS 产物生成的签名和 `latest.json`；updater `.sig` 与 Windows Authenticode 是两项独立合同。macOS 仍未签名 / notarize，Linux arm64 产物是否可用取决于 GitHub Actions runner 矩阵。
- 维护者应先手动对精确的 `origin/main` commit SHA 做 rehearsal。rehearsal 只保留已验证产物，不创建 GitHub Release；只有 `v<semver>` tag 能进入受保护 publish 路径，且必须完成 Azure Authenticode、updater 签名、checksum、provenance attestation 和 fresh-download 验证。

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
| Coding | Grok | `~/.grok/skills/` |
| Coding | Cursor | `~/.agents/skills/` |
| Coding | Antigravity | `~/.gemini/antigravity/skills/` |
| Coding | Antigravity CLI | `~/.gemini/antigravity-cli/skills/` |
| Coding | Zed（社区兼容） | `~/.config/zed/skills/` |
| Coding | Gemini CLI（legacy） | `~/.gemini/skills/` |
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

> 说明：Claude Code 还会把 `~/.claude/plugins/marketplaces/*` 下的 marketplace plugin 目录显示成只读行。这些条目只做展示，不按 `~/.claude/skills/` 里的原生技能那套方式管理。Antigravity plugin bundle 属于独立 CLI 插件机制；SkillPort 当前只管理 Google 平台的 `SKILL.md` 技能目录，不导入或导出 plugin bundle。 Zed 以社区兼容 skills 路径列出；SkillPort 不宣称该目录是 Zed 官方原生 skills 规范。

也可以在 Settings 中添加自定义平台。

## 隐私与安全

- **本地优先** — 元数据、集合、扫描结果、设置和 AI explanation 缓存都保存在 `~/.skillsmanage/db.sqlite` 或你自己管理的本地 skill 目录中。`.skillsmanage` 路径会继续保留，用来兼容已有安装。
- **无遥测** — 应用不包含分析、崩溃上报或使用追踪。
- **网络访问由功能触发** — 只有在你显式使用 marketplace 同步/下载、GitHub 导入或 AI explanation 时才会发起外部请求。
- **SSH 只作用于当前目标** - 只有当前远程目标会建立 SSH 连接；远程文件改动只发生在该远程用户的 skills 目录内。
- **凭据只保存在本机** — GitHub PAT、AI API key 和 SSH 密码会优先写入操作系统凭据库。Windows 上如果系统凭据库不可用，SkillPort 会退回到 `~/.skillsmanage/protected-secrets/` 下由 DPAPI 保护的应用本地 secret 文件。
- **旧版密钥迁移** — 如果 SQLite settings 中仍有旧版 GitHub PAT 或 AI API key，应用会把它们迁移到 secret store，并从 settings 中移除。若无法使用持久化受保护存储，该值只会保留在当前应用会话中。
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

- [Node.js](https://nodejs.org/) 26（见 `.node-version`）
- [pnpm](https://pnpm.io/) 10.34.5
- [Rust toolchain](https://rustup.rs/) 1.98.0（见 `rust-toolchain.toml`）
- Tauri v2 系统依赖：<https://v2.tauri.app/start/prerequisites/>

仓库工具链固定为 Node 26、pnpm 10.34.5 和 Rust 1.98.0。

### 安装依赖

```bash
pnpm install
```

### 常用 just 命令

```bash
just doctor
just check
just ci
just audit
just version-check
just dev
just build
just install
```

- `just doctor` 是只读的工具链与 Tauri 前置依赖诊断；它只报告缺失或版本漂移，不会安装依赖、修改 PATH 或切换 toolchain。
- `just check` 在开发过程中运行快速静态/生成物检查；它不能替代提交或合并 PR 前必须运行的完整 `just ci` 与 `just audit` 门禁。
- `just ci` 会并行运行平台无关的 `common` lane（只读版本/生成物检查、前端验证与构建、文档、Rust entrypoint/格式/IPC 合同）和当前平台的全 targets Clippy、锁文件 Rust 测试。
- `just version-check` 会只读检查 Tauri/Cargo 元数据是否与 `package.json` 一致；需要显式更新时使用 `just sync-version`。
- `just dev` 会直接启动 Tauri 开发应用。
- `just build` 会按当前平台构建桌面应用，并把最新打包产物复制到 `outputs/`（Windows 为 `.exe`，macOS 为 `.app` + `.dmg`，Linux 为 `.AppImage`/`.deb`）。
- `just install` 会构建 Windows NSIS 安装包、复制到 `outputs/`，并以 passive 模式运行安装器；在 macOS 上会显示提醒并改为运行 `just build`。

### 启动开发环境

```bash
pnpm tauri dev
```

Vite 开发服务器默认使用 `24200` 端口。

### 验证命令

```bash
just ci
```

GitHub 会为指向 `dev` 或 `main` 的 PR 并行运行 `common`、Windows Rust、Linux Rust、macOS Rust 和供应链 lane；稳定的 `just-ci` 检查只有在全部 required lane 成功时才通过。直接手动触发还会运行跨平台 smoke package；桌面发布 workflow 则在冻结 SHA 上复用同一质量门禁，并独立负责正式发布打包。

### 分支与 PR 流程

`dev` 是长期保留的日常开发分支，不会退役。短生命周期 task 分支以 `dev` 为目标并使用 squash merge，合并后自动删除 task 分支。`dev` -> `main` 的 promotion PR 使用 merge commit 保留祖先关系。每次 promotion 后，先刷新并确认精确的 promotion merge SHA，再把 `dev` fast-forward 到该 SHA，之后才能写 Trellis 证据或开始下一个 task。CI 只响应目标为 `dev` 或 `main` 的 PR，普通 push 不触发 CI。

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
├── docs/                       # VitePress 文档、产品说明和设计资源
├── public/                     # 静态资源
├── scripts/                    # 构建和维护脚本
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

## 文档站点

`docs/` 下提供基于 VitePress 的中英双语文档站点。本地预览：

```bash
pnpm docs:gen
pnpm docs:gen:check
pnpm docs:dev
pnpm docs:build
pnpm docs:preview
```

修改 Tauri command 或数据库 schema 源码后，运行 `pnpm docs:gen`，并共同提交 `docs/architecture/_generated/` 下刷新的两个文件。`pnpm docs:gen:check` 和 `pnpm docs:build` 都是只读入口：生成物漂移时直接失败，不会改写工作树。

英文入口为 `/`，中文镜像为 `/zh/`。构建产物输出到仓库根的 `dist-docs/`。公开 release 会把该产物部署到 GitHub Pages；维护者也可以从 canonical `main` 手动触发 Docs workflow 进行迁移或恢复。workflow 只部署这一份构建产物，随后验证公开页面确实属于 SkillPort。

## 许可证

本项目使用 Apache License 2.0，详见 [LICENSE](LICENSE)。
