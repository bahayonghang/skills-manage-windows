# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 开发命令

### 前端（React + TypeScript）

```bash
pnpm install              # 安装依赖
pnpm dev                  # 启动 Vite 开发服务器（端口 24200，单独前端调试用）
pnpm build                # TypeScript 编译 + Vite 构建
pnpm test                 # Vitest 原生单次运行全部测试
pnpm test:serial          # 逐文件串行 Vitest 回退/隔离排障
pnpm test -- src/test/skillStore.test.ts  # 运行单个测试文件
pnpm test:watch           # Vitest 监听模式
pnpm typecheck            # tsc --noEmit 类型检查
pnpm lint                 # ESLint 检查
```

### Rust 后端（Tauri v2）

```bash
cd src-tauri && cargo test           # 运行全部 Rust 测试（700+）
cd src-tauri && cargo test db::      # 运行指定模块测试
cd src-tauri && cargo clippy -- -D warnings  # Lint 检查
```

### 完整应用

```bash
just ci                   # 完整门禁：先 sync-version，再并行跑 Web 与 Rust 检查链
pnpm tauri dev             # 启动 Tauri 开发模式（含前端热重载）
pnpm tauri build           # 构建可分发的桌面应用
```

## 架构概述

跨平台 AI 技能管理桌面应用：

```
React 前端 (src/)  ──Tauri IPC──▶  Rust 后端 (src-tauri/src/)  ──SQLx──▶  SQLite
```

后端内部为三层结构：

```
commands/   —— IPC 壳层：171 个 #[tauri::command]（24 个文件），负责参数翻译、
  │            操作日志记录、错误字符串化；业务逻辑不写在这一层
  └─ services/   —— 业务逻辑：12 个域（ai_provider / ai_tagging / central_skills /
       │           github_import / installation / local_remote_sync / marketplace /
       │           obsidian / portable_state / projects / scanner / usage），
       │           每域一个 error.rs 域错误枚举
       └─ db/    —— 数据访问：repos/（17 个 repo 模块）+ schema/（9 个建表模块）
                    + migrations.rs + pool.rs，多步写操作走事务
```

- **前端**：React 18 + TypeScript + Tailwind CSS 4 + shadcn/ui，Zustand 状态管理（大 store 切片化，如 `centralSkillsStore` 拆 install/list/metadata/update 四个 slice），React Router v7 路由
- **后端**：Rust（Tauri v2），前端用 `invoke()` 调用 IPC 命令；跨域基础设施在 services 之外：`targets/`（SSH/WSL 远程目标传输层）、`logging/`、`secrets/`、`operation_log.rs`、`fs_util.rs`（spawn_blocking 包装）、`paths.rs`（路径解析）
- **数据库**：SQLite（WAL 模式），位于 `~/.skillsmanage/db.sqlite`，SQLx 异步驱动，schema 在 `db/schema/` 各模块中定义并自动迁移
- **HTTP**：`reqwest` 用于 GitHub API 调用（Marketplace 源同步、更新中心）和 AI API 调用（技能解释、AI 打标）

### 核心业务模型

- **技能（Skill）**：包含 YAML 前缀的 Markdown 文件（SKILL.md），是核心管理单元
- **中央目录**：`~/.skillsmanage/skills/` 是技能的唯一真实来源（canonical source，SkillPort 私有中央仓库），可通过 `central_store_location` 命令 preview/apply 迁移位置；注意区分 `~/.agents/skills/`——那是 Universal Agents 的安装目标目录，不是中央仓库
- **平台安装**：通过符号链接（symlink）将中央技能安装到各平台目录（如 `~/.claude/skills/`）
- **自动中央化（Auto-centralize）**：安装仅存在于某平台的技能到其他平台时，`services/installation/centralize.rs` 的 `ensure_centralized` 会自动将其拷贝到中央目录并更新 DB 的 `canonical_path`/`is_central`，再走正常 symlink/copy 流程。调用方（包括 native/project/remote 各安装路径）对此透明
- **集合（Collection）**：技能分组，支持批量安装和 JSON 导入/导出
- **项目（Projects）**：手动 add 项目根目录，扫描项目下已启用 agent 的 skill 目录（`.claude/skills/` 等），支持装/卸/pin/重命名/移除，主从分离布局（左面板项目列表 + 右面板技能详情）
- **技能市场（Marketplace）**：从 GitHub 仓库远程浏览和安装技能，三 Tab 页面（推荐/官方源目录/skills.sh 搜索）
- **更新中心（Update Center）**：`central_updates` + `skill_update_inventory` 跟踪中央技能与上游 GitHub 仓库的差异，支持检查与同步更新
- **远程目标（Targets）**：管理 SSH/WSL 远程目标，`local_remote_sync` 可将本地中央技能同步到远程机器

### 页面路由

| 路由                                  | 页面                             | 布局模式                                                                                              |
| ------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `/`（重定向到 `/dashboard`）          | —                                | —                                                                                                     |
| `/dashboard`                          | 仪表盘                           | 本地优先的操作总览                                                                                    |
| `/central`                            | 中央技能库                       | 技能卡片列表（两列）                                                                                  |
| `/platform/:agentId`                  | 平台技能视图                     | 技能卡片列表（两列）                                                                                  |
| `/skill/:skillId`                     | 技能详情                         | **双栏布局**：左栏 SKILL.md 预览（全高），右栏 sidebar（metadata + 紧凑图标式安装状态 + collections） |
| `/collections`                        | 技能集合                         | 上方卡片横排选中 + 下方技能列表                                                                       |
| `/discover`, `/discover/:projectPath` | （已废弃，重定向到 `/projects`） | —                                                                                                     |
| `/projects`, `/projects/:projectId`   | 项目级技能管理                   | 左面板项目列表 + 右面板技能详情                                                                       |
| `/obsidian`, `/obsidian/:vaultId`     | Obsidian vault 视图              | vault 扫描与导入                                                                                      |
| `/marketplace`                        | 技能市场                         | 三 Tab（推荐/官方源/skills.sh）                                                                       |
| `/logs`                               | 操作日志                         | 操作日志 + 运行时诊断日志                                                                             |
| `/usage`                              | 技能用量                         | 聚合各 AI 编码工具的技能调用统计                                                                      |
| `/settings/*`                         | 设置                             | 卡片分区（内部按 section 导航，非嵌套路由）                                                           |

### IPC 命令模块（src-tauri/src/commands/）

共 171 个 `#[tauri::command]`，分布在 24 个文件。commands 层是纯壳层：参数翻译、操作日志、错误字符串化，业务逻辑在对应 `services/` 域中。

| 模块                                      | 职责                                                                                    |
| ----------------------------------------- | --------------------------------------------------------------------------------------- |
| `scanner.rs`                              | 扫描目录并解析 SKILL.md 文件的 YAML 前缀                                                |
| `agents/`                                 | 平台 CRUD（36 个内置：coding 30 + lobster 6，外加 central 伪平台与自定义平台）          |
| `linker.rs`                               | 符号链接/复制方式安装和卸载技能                                                         |
| `skills.rs`                               | 技能查询和 Markdown 内容读取                                                            |
| `collections.rs`                          | 集合管理、批量安装、导入导出                                                            |
| `projects.rs`                             | 手动 add 项目 + 扫描 + 装/卸 + pin/重命名/移除                                          |
| `obsidian.rs`                             | Obsidian vault 扫描和源模式导入                                                         |
| `settings.rs`                             | 扫描目录和应用设置的键值存储                                                            |
| `marketplace.rs`                          | GitHub 源同步、远程技能安装、AI 技能解释（Claude/GLM/MiniMax/Kimi/DeepSeek/OpenRouter） |
| `github_import.rs`                        | GitHub 仓库整库技能导入（预览 + 选择导入）                                              |
| `central_metadata.rs`                     | 中央技能 metadata 与 AI 打标                                                            |
| `central_updates.rs` + `central_updates/` | 更新中心：上游 GitHub 更新检查、同步与事件推送                                          |
| `skill_update_inventory.rs`               | 技能更新清单（更新机制 P2）                                                             |
| `central_store_location.rs`               | 中央仓库位置迁移（preview/apply）                                                       |
| `targets.rs`                              | SSH/WSL 远程目标管理与连接测试                                                          |
| `local_remote_sync.rs`                    | 本地仓库/中央技能同步到选定的 SSH/WSL 目标                                              |
| `usage.rs`                                | 技能用量聚合（跨 AI 编码工具）                                                          |
| `logs.rs`                                 | 操作日志与运行时诊断日志                                                                |
| `saved_views.rs`                          | 中央技能库保存的视图                                                                    |
| `tag_groups.rs`                           | 标签组管理                                                                              |
| `portable_state.rs`                       | 便携状态导入/导出                                                                       |
| `bootstrap.rs`                            | 启动聚合载荷（agents + 状态一次取回）                                                   |
| `app_runtime.rs`                          | 应用运行时/平台信息                                                                     |

辅助文件（不含命令）：`central_updates_fs.rs`（更新中心文件落盘 + IPC 边界 `run_blocking_fs` 字符串包装）、`serde_helpers.rs`、`mod.rs`。

### 前端静态数据（src/data/）

| 文件                 | 内容                                                         |
| -------------------- | ------------------------------------------------------------ |
| `officialSources.ts` | 70 个官方 publisher 元数据 + 22 个推荐 skills（含 tag 分类） |
| `aiProviders.ts`     | 7 个 AI 提供商预设（含国内/国际区域端点）                    |

### 共享 UI 模式

- **`UnifiedSkillCard`**（`src/components/skill/UnifiedSkillCard.tsx`）：**所有页面的技能卡片唯一实现**。通过 props 自适应 5 种场景（central/platform/project/marketplace/collection），不要在各页面重建内联卡片组件。统一样式：`rounded-xl` + `ring-1 ring-border` + `bg-card` + `shadow-sm`
- **`InstallDialog`**（`src/components/central/InstallDialog.tsx`）：默认**勾选已链接平台**（反映当前状态），宽度 `sm:max-w-2xl`，平台列表两列网格。`CollectionInstallDialog` 同宽度布局但默认勾选所有 detected 平台（批量首装场景）
- **平台图标切换**：`UnifiedSkillCard` 的 `platformIcons` prop 分 LOBSTER/CODING 两行显示，点击图标即时切换安装/卸载（symlink 方式），走 `centralSkillsStore.togglePlatformLink`

## 代码约定

- **路径别名**：`@/` 映射到 `src/`（在 vite.config.ts 和 tsconfig.json 中配置）
- **状态管理**：每个业务域一个独立的 Zustand store（`src/stores/`），store 内部直接调用 `invoke()` 与后端通信；不要在组件里直接 `invoke()`
- **技能卡片**：只用 `UnifiedSkillCard`，不要新建场景专用卡片组件
- **主题系统**：6 套主题（Catppuccin Mocha/Macchiato/Frappe/Latte + Claude Light/Claude Dark），14 种 accent 配色，通过 `data-theme` 和 `data-accent` HTML 属性切换。`src/index.css` 顶部的 `@custom-variant dark` 把 Tailwind `dark:` 变体映射到 4 套暗色主题（mocha/macchiato/frappe/claude-dark）
- **语义状态色**：success/warning/info/error 四态统一走 `src/lib/statusTone.ts` 的类名工具（token 随主题换肤），禁止写 `dark:text-amber-300` 这类明暗二元适配，也不要直接用原生 Tailwind 调色板表达状态色
- **国际化**：中英双语（`src/i18n/`），所有用户可见文本必须走 i18n
- **测试**：Vitest + jsdom + React Testing Library，setup 在 `src/test/setup.ts`；Tauri `invoke` 在测试中通过 `window.__TAURI_INTERNALS__` mock
- **未使用变量**：ESLint 规则允许 `_` 前缀的未使用参数和变量
- **Rust 后端**：所有 IPC 命令函数签名中通过 `State<AppState>` 注入数据库连接池；不使用 `sqlx::query_as!` 宏（需要 DATABASE_URL），统一使用 `sqlx::query()` + 手动 `Row::get()` 映射
- **错误处理（分层契约）**：`db/repos/*` 返回 `Result<T, sqlx::Error>` 直接透传；`services/<domain>` 返回该域的 thiserror 错误枚举（一域一枚举，定义在 `services/<domain>/error.rs`，db 错误经 `#[from] sqlx::Error` 透传）；`commands/*` 返回 `Result<T, String>`，是唯一允许字符串错误的层（调用点 `.map_err(|e| e.to_string())`）。新增 services 函数禁止返回 `Result<T, String>`；调用方需区分错误类别时加语义化变体用 `matches!` 分支，禁止 `error.contains("...")` 字符串嗅探；`#[error(...)]` 文案逐字保留（前端 toast 直接展示 Display 输出）。详见 `.trellis/spec/backend/domain-error-enums.md`
- **重 IO 必须 spawn_blocking**：async 上下文中的递归遍历、递归拷贝/删除、批量落盘、目录搬迁必须经 `src-tauri/src/fs_util.rs` 的 `run_blocking_fs_with` 包装（全仓唯一包装入口，禁止自创第二种），禁止直接调用同步 `std::fs`；进度/事件发射留在 async 侧，`AppHandle` 不得按值进 blocking 闭包（Windows 测试二进制会崩）。详见 `.trellis/spec/backend/spawn-blocking-io.md`
- **Marketplace GitHub 适配器**：扫描仓库根目录和 `skills/` 子目录，解析 SKILL.md frontmatter 获取 name/description；所有同步到的 skills 缓存在 `marketplace_skills` 表，复用 `sync_registry`/`search_marketplace_skills` 命令
- **AI 解释**：从 settings 表动态读取 provider/api_key/model/api_url，支持 Anthropic 格式和 OpenAI 格式响应，自动跳过 `thinking` 类型 content block
- **安装路径约束**：所有 install/uninstall 路径以中央目录为源（`commands/linker.rs` 是壳层，实现在 `services/installation/`）。添加新的安装方式时，复用 `services/installation/centralize.rs` 的 `ensure_centralized` 保证非中央技能也能被分发

# Superpowers-ZH 中文增强版

本项目已安装 superpowers-zh 技能框架（20 个 skills）。

## 核心规则

1. **收到任务时，先检查是否有匹配的 skill** — 哪怕只有 1% 的可能性也要检查
2. **设计先于编码** — 收到功能需求时，先用 brainstorming skill 做需求分析
3. **测试先于实现** — 写代码前先写测试（TDD）
4. **验证先于完成** — 声称完成前必须运行验证命令

## 可用 Skills

Skills 位于 `.claude/skills/` 目录，每个 skill 有独立的 `SKILL.md` 文件。

- **brainstorming**: 在任何创造性工作之前必须使用此技能——创建功能、构建组件、添加功能或修改行为。在实现之前先探索用户意图、需求和设计。
- **chinese-code-review**: 中文代码审查规范——在保持专业严谨的同时，用符合国内团队文化的方式给出有效反馈
- **chinese-commit-conventions**: 中文 Git 提交规范 — 适配国内团队的 commit message 规范和 changelog 自动化
- **chinese-documentation**: 中文技术文档写作规范——排版、术语、结构一步到位，告别机翻味
- **chinese-git-workflow**: 适配国内 Git 平台和团队习惯的工作流规范——Gitee、Coding、极狐 GitLab、CNB 全覆盖
- **dispatching-parallel-agents**: 当面对 2 个以上可以独立进行、无共享状态或顺序依赖的任务时使用
- **executing-plans**: 当你有一份书面实现计划需要在单独的会话中执行，并设有审查检查点时使用
- **finishing-a-development-branch**: 当实现完成、所有测试通过、需要决定如何集成工作时使用——通过提供合并、PR 或清理等结构化选项来引导开发工作的收尾
- **mcp-builder**: MCP 服务器构建方法论 — 系统化构建生产级 MCP 工具，让 AI 助手连接外部能力
- **receiving-code-review**: 收到代码审查反馈后、实施建议之前使用，尤其当反馈不明确或技术上有疑问时——需要技术严谨性和验证，而非敷衍附和或盲目执行
- **requesting-code-review**: 完成任务、实现重要功能或合并前使用，用于验证工作成果是否符合要求
- **subagent-driven-development**: 当在当前会话中执行包含独立任务的实现计划时使用
- **systematic-debugging**: 遇到任何 bug、测试失败或异常行为时使用，在提出修复方案之前执行
- **test-driven-development**: 在实现任何功能或修复 bug 时使用，在编写实现代码之前
- **using-git-worktrees**: 当需要开始与当前工作区隔离的功能开发或执行实现计划之前使用——创建具有智能目录选择和安全验证的隔离 git 工作树
- **using-superpowers**: 在开始任何对话时使用——确立如何查找和使用技能，要求在任何响应（包括澄清性问题）之前调用 Skill 工具
- **verification-before-completion**: 在宣称工作完成、已修复或测试通过之前使用，在提交或创建 PR 之前——必须运行验证命令并确认输出后才能声称成功；始终用证据支撑断言
- **workflow-runner**: 在 Claude Code / OpenClaw / Cursor 中直接运行 agency-orchestrator YAML 工作流——无需 API key，使用当前会话的 LLM 作为执行引擎。当用户提供 .yaml 工作流文件或要求多角色协作完成任务时触发。
- **writing-plans**: 当你有规格说明或需求用于多步骤任务时使用，在动手写代码之前
- **writing-skills**: 当创建新技能、编辑现有技能或在部署前验证技能是否有效时使用

## 如何使用

当任务匹配某个 skill 时，使用 `Skill` 工具加载对应 skill 并严格遵循其流程。绝不要用 Read 工具读取 SKILL.md 文件。

如果你认为哪怕只有 1% 的可能性某个 skill 适用于你正在做的事情，你必须调用该 skill 检查。

## Agent skills

### Issue tracker

Issues for this repo live in GitHub Issues and should be managed with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

This repo uses the default five-label triage vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain-doc layout. See `docs/agents/domain.md`.
