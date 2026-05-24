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

SkillPort 的操作日志。记录安装、卸载、扫描、设置、target 切换、导入导出等用户可见操作。

Operation Log 必须保护敏感信息。password、token、PAT、API key、secret、private key、credential 等字段需要 redaction。

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

1. 统一 Path policy Module。
2. 提取 Platform management Module。
3. 加深 Operation Log Module。
4. 拆出 Discover scan core。
5. 收窄 Central Skills workflow Modules。
6. 条件拆分 Settings store behavior slices。
