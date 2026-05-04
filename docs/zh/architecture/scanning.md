# 扫描机制

"在磁盘上找 SKILL.md" 由两个服务承担：`services::scanner` 服务于 Central 仓库，`services::discovery` 服务于项目 / Obsidian 源。两者都生成强类型记录，不直接落 DB。

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

## 项目发现（Discover）

源代码：`src-tauri/src/services/discovery/`。

```text
discovery/
├── roots.rs     按平台解析项目级技能模式
├── scan.rs      遍历根目录，收集 SKILL.md 候选
├── query.rs     Obsidian vault 扫描（仅源模式）
├── import.rs    把 discovered 提升为 Central / 平台技能
└── types.rs     ScanRoot / DiscoveredSkill / ImportRequest
```

Discovery 与 scanner 解决不同问题：scanner 读已安装到 agent 的技能，discovery 读尚未提升到 Central 的项目级 SKILL.md。

### Roots 与模式

`roots.rs` 把每个平台的项目模式去重合并。共享模式（如 `.agents/skills`）即使被多个平台声明，也只发出一次，避免同一 SKILL.md 在 UI 重复 N 份。

### 项目扫描

`scan.rs` 遍历每个 root，解析 frontmatter，按 `(project_path, platform_id)` 写入 `discovered_skills`。UI 的左项目列表 + 右技能详情就是这张表的视图。

### Obsidian 源

`query.rs` 读取 Obsidian vault 注册表。Vault 内按 `.skills > .agents/skills > .claude/skills` 优先级挑选规范 SKILL.md。源扫描结果**不进 `discovered_skills`** 持久缓存：`commands::discover::import_source_skill_to_central` / `import_source_skill_to_platform` 直接接受 `file_path`/`dir_path`，避免污染缓存表。

### 导入流水线

`import.rs` 是 discovered → installed 的唯一写入方：

| 方法 | 效果 |
| --- | --- |
| `symlink` | 默认；走系统符号链接。 |
| `auto` | 先尝试 symlink，权限不足退到 copy（Windows 友好）。 |
| `copy` | 强制目录拷贝。 |

提升到 Central 时，import 调用 `installation::centralize::ensure_centralized`，让 `canonical_path` 与 `is_central` 在安装路径前已就绪。

## 重扫性能

- Scanner 复用 `agent_skill_observations`，全量重扫复杂度是 O(文件数)，不是 O(agent × 文件数)。
- Discovery 主动剪除磁盘上不再存在的 `(project_path, platform_id)` 行，UI 列表保持有界。

Last reviewed: 2026-05-04
