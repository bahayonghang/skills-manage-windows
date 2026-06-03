# 术语表

UI、源码、文档中使用的术语。

| 术语 | English | 含义 |
| --- | --- | --- |
| 技能 | Skill | 含 `SKILL.md` 的目录及其可选 helper；SkillPort 管理的最小单位 |
| 中央技能库 | Central Skills | 位于 `~/.skillsmanage/skills/` 的规范仓库 |
| 平台 | Platform | 从已知目录读取技能的 agent（Claude Code、Cursor…） |
| Agent | Agent | 源码中等价于"平台"（`agent_id`、`agents` 表） |
| 龙虾 | Lobster | UI 分组：中文厂商系编码 agent（OpenClaw、QClaw…） |
| 通用平台 | Universal Agents | 全局共享 `~/.agents/skills/`，被 Codex CLI / Cursor / OpenCode / Amp / Copilot 等 universal agents 读取；项目范围内 Antigravity 与 Antigravity CLI 也共享 `.agents/skills/` |
| 安装 | Install | 把 Central 技能落到平台技能目录 |
| 符号链接 | Symlink | Linux / macOS 与 Windows 开发者模式下的默认安装方式 |
| 拷贝 | Copy | 复制目录的安装方式；Windows fallback 默认 |
| 自动 | Auto | 优先 symlink，权限不足回退 copy |
| 集中化 | Centralize | 把非 Central 技能拷入 `~/.skillsmanage/skills/` |
| 项目发现 | Discover | 遍历项目目录查找尚未提升到 Central 的 SKILL.md |
| 市场 | Marketplace | 远端技能源（GitHub repo / 镜像）的策展列表 |
| 源 | Registry | `skill_registries` 中的一行，对应一个远端来源 |
| 集合 | Collection | 用户自定义的技能分组，用于批量安装与导入导出 |
| 技能仓库 | Repository | 按源仓库分组 Central 技能的本地元数据 |
| 标签 | Tag | 本地分类项，可手动也可由 AI 建议 |
| 操作日志 | Operation Log | `operation_logs` 中的结构化行，记录安装、卸载、扫描、设置、target 切换、导入、导出等用户可见操作 |
| 运行时日志 | Runtime Log | 有界日文件 `skillport-YYYY-MM-DD.log`，用于前后端诊断；与 Operation Log 分离 |
| 可观测性控制台 | Observability Console | `/logs` UI，包含独立的 Operation 与 Runtime 模式 |
| 目标 | Target | Local 或 SSH 主机；命令解析的目的端 |
| 活动目标 | Active Target | SSH 横幅当前选中的 Target |
| Vault | Vault | Obsidian 管理的目录；SkillPort 在 `/obsidian` 下做仅源扫描 |
| 启动快照 | Bootstrap | 启动时缓存的快照，让 Dashboard 在扫描完成前就能渲染 |
| 回填 | Backfill | 一次性数据迁移，把 `datetime('now')` 等填入新增列 |

## 命名约定

| 概念 | 约定 |
| --- | --- |
| 技能 ID | 目录名（如 `python-style`） |
| Agent ID | 全小写蛇形（如 `claude_code`） |
| 集合 ID | UUID v4 |
| 标签 ID | UUID v4 |
| 源 ID | UUID v4 |

## 交叉引用

- 技能协议：见[技能协议](./skill-protocol.md)
- 平台 → 目录：见[平台路径](./platform-paths.md)
- 安装方式语义：见[架构 → 安装引擎](../architecture/installation-engine.md)

Last reviewed: 2026-06-03
