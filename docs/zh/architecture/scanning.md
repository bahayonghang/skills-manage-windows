# 扫描机制

"在磁盘上找 SKILL.md" 由两个服务承担：`services::scanner` 服务于 Central 仓库，`services::projects` 服务于项目级 skill 库。两者都生成强类型记录，不直接落 DB。

## Central Scanner

源代码：`src-tauri/src/services/scanner/`。

```text
[command::scan_all_skills] ──► services::scanner::scan_all
                                       │
                                       ▼
                  遍历每个注册的扫描目录
                                       │
                                       ▼
                  解析 SKILL.md YAML frontmatter（name / description）
                                       │
                                       ▼
                  通过 repo upsert skills + observations（每个 agent 一行）
```

- 扫描目录存在 `scan_directories`：内置（按各 agent 的 `global_skills_dir` 解析）和用户自添加都被采纳。
- `claude_plugin.rs` 处理嵌套的 `~/.claude/plugins` 布局——每个插件有自己的 `skills/` 目录。
- 每个 agent 独立写一份 observations，UI 显示 "此技能也安装在 X / Y / Z" 时无需重新走盘。

## 项目级扫描

源代码：`src-tauri/src/services/projects/`。

```text
projects/
├── crud.rs   add / list / rename / pin / remove + install / uninstall
├── scan.rs   遍历已启用 agent 的项目级 skill 目录
├── types.rs  ProjectDto / ProjectSkillDto
└── tests.rs  单元测试
```

项目扫描与 Central scanner 解决不同问题：scanner 读 agent 全局路径来填中央库，项目扫描读每个项目下的 agent 专属路径（`<project>/.claude/skills/` 等）来收集本地 SKILL.md。

### 根目录

没有隐式 root，每个项目根都由用户通过 `pick_project_folder` → `add_project` 显式注册。路径会被规范化、统一分隔符，然后用 `sha256` 前 16 位 hex 算出稳定的 `project_id`。

### 扫描流程

`scan.rs` 遍历**已启用的 agent**（`SELECT id, project_skills_dir FROM agents WHERE is_enabled = 1 AND id != 'central'`），把每个 agent 的项目级 skill 目录路径解析出来，复用 `services::scanner::scan_directory` 一级扫，按 `(project_id, skill_id, agent_id)` UPSERT 进 `project_skill_installations`。`symlink_metadata` 判断 `link_type`（`symlink` / `copy`）；磁盘上已不存在的孤儿 psi 行在同一轮里 DELETE。

### 装 / 卸

`crud::install_skill_to_project_impl` 和 `uninstall_skill_from_project_impl` 是装卸链路的唯一写入方。安装要求 skill 已经中央化（`is_central = true && canonical_path IS NOT NULL`），`method` 接受 `symlink`（默认）或 `copy`。Windows 未开发者模式时 symlink 失败原样抛错误字符串，由前端 toast 透传。

### 移除项目

`remove_project_impl(id, uninstall_skills)` 删项目行（连带 psi 行）。`uninstall_skills = true` 时先遍历 psi 删盘上的 symlink/copy，再删项目；单行删盘失败仅 log，不阻塞整个 remove。

## 从 Discovery 模块迁移

旧的 `services::discovery` 模块（全盘爬 + 写死扫描候选根 + `discovered_skills` 表）在 0.10.x 已删。Schema migration 首次启动时会 DROP `discovered_skills` 并清掉 `settings.discover_scan_roots_config`。原先复用 discovery 的 Obsidian vault 扫描逻辑迁到了 `services::obsidian/`。

## 重扫性能

- Scanner 复用 `agent_skill_observations`，全量重扫复杂度是 O(文件数)，不是 O(agent × 文件数)。
- 项目扫描在每次 rescan 后剪除磁盘上不再存在的 `(skill_id, agent_id)` 行，UI 列表保持有界；IPC 层 emit `project:scanned`，前端不需要整页刷新。

Last reviewed: 2026-05-14
