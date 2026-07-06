# SkillPort 领域上下文

本文件记录架构讨论必须沿用的项目语言。后续计划、代码审查和重构建议优先使用这里的名称。

## 项目定位

SkillPort 是一个 Tauri 桌面应用，用来在一个界面里管理多平台 AI coding agent skills。

本仓库是 `iamzhihuix/skills-manage` 的 fork。当前 fork 保持 Windows-first build 约束：涉及 Tauri 打包、资源文件、发布说明、构建脚本和依赖升级时，必须先保证 Windows 安装包链路可用。

## 核心领域词汇

### SkillPort

桌面应用本体。前端使用 React、TypeScript、Tailwind CSS 4 和 Zustand；后端使用 Rust 与 Tauri v2。

### Skill

AI coding agent 可读取的技能目录。通常包含 `SKILL.md`，可能带有额外资源文件。

### Central Skills

SkillPort 的私有中央技能库。README 中的权威目录是：

```text
~/.skillsmanage/skills/
```

Central Skills 不等同于 Universal Agents。只有显式安装到目标平台的 Skill 才会暴露给对应工具。

### Universal Agents

多个 AI coding agent 共享的技能目标位置。README 中的权威目录是：

```text
~/.agents/skills/
```

Codex CLI、Cursor、OpenCode、Amp、Copilot 等 universal agents 读取这个全局位置。Google 平台需要区分：Antigravity 全局目录是 `~/.gemini/antigravity/skills/`，Antigravity CLI 全局目录是 `~/.gemini/antigravity-cli/skills/`，两者的项目级技能仍共享 `.agents/skills/`；Gemini CLI 作为 legacy/shared 目标承载 `~/.gemini/skills/`。

### Platform

SkillPort 支持的目标工具或平台，例如 Claude Code、Codex CLI、Cursor、Antigravity、Antigravity CLI、Gemini CLI (legacy)、OpenCode、OpenClaw。Platform 有自己的 skills 目录，也可能共享 Universal Agents 目录。

### Platform install

把 Central Skills 中的 Skill 安装到一个或多个 Platform 的流程。安装方式包括 copy 和 symlink。涉及安装、卸载、中央化链路时，优先复用现有 linker 逻辑，尤其是 `ensure_centralized` 约束。

### Discover

扫描本地磁盘中的项目级 skill 库。Discover 识别项目目录下的 platform skill patterns，把找到的 Skill 汇总成可导入项。

当前版本的 SSH remote target 不启用远程 Discover 项目扫描。

### Marketplace import

从 marketplace 或 GitHub 仓库导入公开 Skill 元数据和内容。GitHub 导入支持鉴权请求和重试回退。

### SSH remote target

SkillPort 通过 SSH 管理远程 Linux 或 macOS 用户的全局 skills。桌面界面仍在本机运行，后端连接当前选中的 remote target。

约束：

- SSH remote target 支持 key 和 password 两种 OpenSSH 登录方式。
- 私钥内容不保存。
- password 登录把密码存入系统凭据库，不写入 SQLite。
- 每个 SSH remote target 有本地缓存数据库：`~/.skillsmanage/targets/<target_id>/db.sqlite`。
- remote install 默认使用 copy。
- remote path 不能用本机文件管理器打开，只能复制路径。

### Operation Log

SkillPort 的操作日志。记录安装、卸载、扫描、设置、target 切换、导入导出等用户可见操作，是面向用户审计和历史回看的一层。

Operation Log 必须保护敏感信息。password、token、PAT、API key、secret、private key、credential 等字段需要 redaction。

Operation Log 不用于承载前后端异常栈、IPC 失败、tracing 诊断或开发期调试噪声；这类内容属于 Runtime Log。

### Runtime Log

SkillPort 的运行时诊断日志。记录 Rust tracing、前端 `error` / `unhandledrejection`、显式 `frontend.runtime` 事件和 IPC 失败等可诊断事件。

Runtime Log 是有界本地文件日志，不是 `operation_logs` 表。文件名固定为 `skillport-YYYY-MM-DD.log`，默认保留 14 天；读取、导出和前端写入都必须做敏感字段 redaction。

### Observability Console

`/logs` 页面中的双层日志控制台。Operation layer 展示 Operation Log；Runtime layer 展示 Runtime Log。两个 layer 共享诊断视觉语言，但数据源、生命周期和清理语义必须保持分离。

### Local-first storage

SkillPort 默认本地优先。元数据、集合、扫描结果、设置和 AI explanation 缓存在本地数据库或用户管理的本地 skill 目录中。

权威数据库路径：

```text
~/.skillsmanage/db.sqlite
```

## 架构走查约束

### 路径语义

路径词汇以 README 为准：

- Central Skills：`~/.skillsmanage/skills/`
- Universal Agents：`~/.agents/skills/`
- 本地数据库：`~/.skillsmanage/db.sqlite`
- SSH remote target 缓存数据库：`~/.skillsmanage/targets/<target_id>/db.sqlite`

如果其它文档出现不同说法，先按 README 解释，再决定是否修正文档。

### 前端状态访问

`src/stores/` 是 Zustand 状态层。用户界面里的 Module 不应直接调用 Tauri `invoke()`；应通过 store 或更窄的 Adapter 访问后端能力。

### 技能卡片

`src/components/skill/UnifiedSkillCard.tsx` 是技能卡片的唯一实现。新增展示场景优先复用它。

### Windows-first build

涉及打包或发布链路时，不能只验证前端构建。Tauri Windows bundle 是验收范围的一部分。

## 不要重复建议的方向

- 不要把 SSH remote target lifecycle 作为独立 deepening target。当前更真实的摩擦来自 Operation Log policy 泄露。
- 不要因为 Central Skills 文件大就机械拆分。只有 deletion test 证明 workflow 独立时才拆。
- 不要创建泛用 Operation Log DSL。目标是提高 Locality，不是扩大 Interface。
- 不要把 Settings store 只按文件数量拆分。只有调用依赖或测试 setup 明显变小时才拆。

## 当前优先 deepening opportunities

2026-07 架构深化专项（9 个子任务）已全部落地：Path policy（含 remote 半边）、Platform management Module、Redaction policy 统一、typed IPC adapter、Rust test-support harness、UnifiedSkillCard 显式场景、frontmatter 解析统一、Update Center service 域归位、Local/SSH/WSL transport seam 试点（install/uninstall）。旧清单中「收窄 Central Skills workflow Modules」评审确认已拆分、「Settings store behavior slices」证据不支持、「Discover scan core」随 Discover 页面废弃（重定向 `/projects`）而失效，均不再保留。

当前登记（依据 transport seam 试点结论，见 `.trellis/spec/backend/transport-seam.md`）：

1. Transport seam 扩展到 central_skills：delete×3 族收进 `InstallTransport` 编排（顺删该域 3 个死 `_ssh_impl`）；preview×2 族先统一 `_ssh_impl` 命名再评估收敛。
2. exec.rs 远程执行的 spawn_blocking 债：同步 `std::process` 在 async 上下文直跑；若补应做在 `CommandRunner` runner 边界单点，不是 10 个调用点。
3. `InstallationError::Remote(String)` 类型化：拍平边界已收敛到 `transport_error` 单点，出现按错误类别分支的需求时再类型化。

低 fork 密度域（scanner / agents / github_import / usage）观望：等该域出现新操作需求时顺势收进 seam，不单独立项；local_remote_sync（remote-only 语义）与 obsidian（守卫非分发）不收。
