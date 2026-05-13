# 项目级 Skill 管理 — 设计规格

| 项     | 内容                                                              |
| ------ | ----------------------------------------------------------------- |
| 日期   | 2026-05-13                                                        |
| 状态   | Draft（已通过设计审查，待 plan 阶段拆任务）                       |
| 关联   | [bahayonghang/skills-manage-windows#7](https://github.com/bahayonghang/skills-manage-windows/issues/7) |
| 涉及   | 后端 `commands/` `services/` `db.rs`，前端 `pages/` `stores/`     |

## 1. 背景

Issue #7 用户报"添加扫描目录无效"——D 盘项目即使打开 UI 上的扫描目录也扫不到。Owner 自述："暂时没有项目级 skills 的管理功能，只支持安装，正在添加相关功能"。

### 1.1 Bug 根因

`src-tauri/src/services/discovery/roots.rs::default_scan_roots()` 把候选扫描根硬编码为 9 个固定路径，全部在 `~` 目录下（`~/projects`、`~/Documents`、`~/Developer`、`~/Desktop` 等）加上 `/Applications`。`get_scan_roots_impl` 只在这 9 个候选上叠加 `enabled` 状态。**代码里完全没有"添加自定义扫描目录"的能力**——UI 上的"添加扫描目录"操作要么不存在，要么仅持久化 enabled，无法落到候选之外的任意路径。

### 1.2 现有能力 vs 缺失

| 维度       | 现状                                          | 缺失                                      |
| ---------- | --------------------------------------------- | ----------------------------------------- |
| 发现       | 全盘扫描固定 9 个根，深度上限 8               | 跨盘任意路径、单项目精确扫描              |
| 安装       | 中央 → 平台全局（`~/.claude/skills/` 等）     | 中央 → 项目本地（`<project>/.claude/...`）|
| 管理       | 全局技能装/卸                                 | 项目本地技能装/卸/列出/预览               |
| 持久化     | `discovered_skills` 表（无主项目实体）        | `projects` 一等实体，可 pin、可重命名     |

## 2. 目标与非目标

### 2.1 目标（本规格覆盖）

- **绕过 Bug #7**：彻底放弃"扫描根 → 全盘发现"模式，改为"手动 add 项目 → 单点扫描"。任意盘符、任意深度都可被管理。
- **项目作为一等实体**：`projects` 表持久化，支持 pin、重命名、移除（含可选连带卸载）。
- **项目本地 skill 装卸**：中央 → 项目下平台特定目录（默认 symlink，可选 copy）。
- **预览**：复用现有 `SkillDetailView` 渲染项目本地 skill 的 SKILL.md。
- **UI**：新主入口 `/projects`，左右分栏（左项目列表 + 右项目详情）。

### 2.2 非目标（本规格不做）

- **应用内编辑** SKILL.md（用户决策：后续统一加）。
- **双向同步** copy 模式下中央改动到项目副本的自动同步（"重装"即可达成同等效果）。
- **跨项目批量操作**（如"把 X skill 同时装到所有项目"），等真实需求出现再加。
- **保留旧 `/discover` 视图**作为兼容（用户决策：严格执行删除）。

## 3. 决策摘要

| 决策项          | 选项                                          | 理由摘要                                                                 |
| --------------- | --------------------------------------------- | ------------------------------------------------------------------------ |
| 范围            | A 扫描 + B 安装 + C 本地管理（C 不含编辑）    | 一次性闭合"扫得到、装得上、卸得下"完整链路                               |
| 本地存放        | 平台特定路径（`.claude/skills/` 等）          | agent 只读它认识的目录，放别处等于白装                                   |
| 项目注册        | 纯手动 add；废弃全盘扫描；GUI 选文件夹        | 绕过 Bug #7 与深度上限；用户对"哪些项目被管理"完全可控                   |
| 安装方式        | 默认 symlink，可选 copy                       | 项目下 `.claude/skills/` 不入 git，symlink 无死链；copy 兜底跨机迁移     |
| 编辑入口        | 无                                            | 用户明确"暂时不加，后续统一"                                             |
| 移除项目        | 弹窗确认；默认保留磁盘，可选一并卸载          | 显式询问优于隐式留尾巴；激进操作放次按钮                                 |
| `managed_by_app`| 不需要                                        | 所有扫到的 skill 一视同仁，包括 npx skills 等外部工具装的                |
| 兼容性          | 严格删除旧 Discover 代码与表                  | 双轨成本高于价值；用户明确选严格执行                                     |

## 4. 数据模型

### 4.1 新增表

```sql
CREATE TABLE projects (
  id              TEXT PRIMARY KEY,           -- sha256(规范化 path) 前 16 位
  path            TEXT NOT NULL UNIQUE,
  name            TEXT NOT NULL,              -- 默认 basename，可重命名
  pinned          BOOLEAN NOT NULL DEFAULT 0,
  added_at        TEXT NOT NULL,              -- rfc3339
  last_scanned_at TEXT
);

CREATE TABLE project_skill_installations (
  project_id      TEXT NOT NULL,
  skill_id        TEXT NOT NULL,
  agent_id        TEXT NOT NULL,
  installed_path  TEXT NOT NULL,              -- 绝对路径
  link_type       TEXT NOT NULL,              -- 'symlink' | 'copy'
  symlink_target  TEXT,                       -- link_type='symlink' 时填
  created_at      TEXT NOT NULL,
  PRIMARY KEY (project_id, skill_id, agent_id),
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_psi_project ON project_skill_installations(project_id);
```

### 4.2 旧数据处理

```sql
DELETE FROM discovered_skills;                                  -- 清空，表保留（防回退）
DELETE FROM settings WHERE key = 'discover_scan_roots_config';  -- 清旧扫描配置
```

`discovered_skills` 表本身不 DROP，下个稳定版本单独发一次迁移删除。

## 5. IPC 命令

新增模块 `src-tauri/src/commands/projects.rs`：

| 命令                                                 | 作用                                                              |
| ---------------------------------------------------- | ----------------------------------------------------------------- |
| `pick_project_folder()`                              | 调 `tauri-plugin-dialog` 弹原生文件夹选择对话框，返回绝对路径     |
| `add_project(path: String)`                          | 规范化路径 → 哈希取 id → INSERT projects → 异步触发扫描           |
| `list_projects()`                                    | 返回所有项目（pinned 在前，last_scanned_at 倒序）                 |
| `rename_project(id, name)`                           | UPDATE projects.name                                              |
| `set_project_pinned(id, pinned)`                     | UPDATE projects.pinned                                            |
| `remove_project(id, uninstall_skills: bool)`         | 可选先卸载所有 skill；DELETE FROM projects（CASCADE 清 psi）      |
| `rescan_project(id)`                                 | 遍历已启用 agent，扫描其目录，UPSERT psi + reconcile              |
| `get_project_skills(id)`                             | 返回该项目下所有 ProjectSkill                                     |
| `install_skill_to_project(skill_id, project_id, agent_id, method)` | 中央 → 项目；复用 `services/installation` 工具         |
| `uninstall_skill_from_project(project_id, skill_id, agent_id)`     | 仅走表：表里没记录直接报错；删 symlink/copy + 删表    |

废弃命令（直接删除，不保留兼容）：

```
discover_scan_roots, get_scan_roots, set_scan_root_enabled,
start_project_scan, stop_project_scan,
get_discovered_summary, get_discovered_skills, clear_discovered_skills,
import_discovered_skill_to_central, import_discovered_skill_to_platform,
import_source_skill_to_central, import_source_skill_to_platform,
get_obsidian_vaults, get_obsidian_vault_skills
```

对应 `src-tauri/src/services/discovery/` 整个模块删除。

### 5.1 新增类型

```rust
pub struct Project {
    pub id: String,
    pub path: String,
    pub name: String,
    pub pinned: bool,
    pub added_at: String,
    pub last_scanned_at: Option<String>,
}

pub struct ProjectSkill {
    pub project_id: String,
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub agent_id: String,
    pub agent_display_name: String,
    pub installed_path: String,
    pub link_type: String,         // 'symlink' | 'copy'
    pub symlink_target: Option<String>,
}
```

### 5.2 事件

| 事件             | Payload                                | 触发                                |
| ---------------- | -------------------------------------- | ----------------------------------- |
| `project:scanned`| `{ project_id, skill_count }`          | `rescan_project` 完成后异步 emit    |

## 6. 核心流程

### 6.1 add 项目

1. 前端 `pick_project_folder()` 拿到绝对路径 P。
2. `add_project(P)`：规范化 → 哈希取 id → 已存在 path 直接返回旧 Project → 否则 INSERT。
3. 立即返回 Project，前端跳转 `/projects/:id`。
4. 后端在 tauri::async_runtime 上调 `rescan_project(id)`；扫描完成 emit `project:scanned`。
5. 前端监听事件刷新右侧详情。

### 6.2 rescan_project

1. 查 projects 拿 path P。
2. 加载"已启用的 agent 列表"：以 `db::builtin_agents()` 为基础集，与 `agents` 表中 `enabled = true` 的记录取交集，再排除 `id = "central"`。enabled 字段来自用户在设置页中的开关，不是静态内置数据。
3. 对每个 agent：
   - 计算 `skill_dir = P / agent.project_skills_dir`（剥离 home 依赖，复用 `roots.rs::platform_skill_patterns` 的字符串处理逻辑）。
   - 不存在则跳过。
   - `scan_directory(skill_dir)` 收集 skill 列表。
   - 对每个 skill：`symlink_metadata` 判断 symlink/copy；symlink 则 readlink；UPSERT 进 psi。
4. DELETE psi 中 (project_id 匹配) 但磁盘上 installed_path 不存在的孤儿。
5. UPDATE projects.last_scanned_at = now。

### 6.3 install_skill_to_project

1. 查中央 skills 表得 `canonical_path`（必须 is_central=true 且 canonical_path 非空，否则报错）。
2. 查 projects 拿 P；查 builtin_agents 拿 `agent.project_skills_dir`。
3. `target_dir = P / agent.project_skills_dir`，确保存在。
4. `target_path = target_dir / skill_dir_name`，已存在则报错"已安装"。
5. method=symlink → `create_symlink(canonical_path, target_path)`；method=copy → `copy_dir_all_blocking`。
6. INSERT 到 psi。

### 6.4 uninstall_skill_from_project

1. SELECT installed_path, link_type FROM psi WHERE 三元组匹配；不存在直接 `Err("skill 未在表中登记")`。
2. symlink → `remove_file`；copy → `remove_dir_all`。
3. DELETE 该行。

### 6.5 remove_project

1. 前端弹窗：复选框"同时卸载本项目下所有已装 skill"（默认未勾选）。
2. 调 `remove_project(id, uninstall_skills)`。
3. 若 `uninstall_skills=true`：遍历 psi 调用单条 uninstall 逻辑，磁盘清理。
4. DELETE FROM projects WHERE id（ON DELETE CASCADE 自动清 psi 残留）。

## 7. UI 形态

### 7.1 路由

```
旧 /discover、/discover/:projectPath  →  跳转 /projects 并提示重新添加
新 /projects                          →  项目列表 + 默认详情骨架
新 /projects/:projectId               →  指定项目详情
```

侧边栏导航条目改名：`nav.discover` → `nav.projects`，i18n 同步更新。

### 7.2 主布局

复用 `DiscoverShell` 改名为 `ProjectsShell`，左 240px 项目列表 + 右自适应详情：

```
┌────────────────────────────┬─────────────────────────────────────┐
│ [+ 添加项目]               │ skills-manage-windows               │
│ [🔍 搜索...]               │ D:\...\skills-manage-windows  [📂]  │
│ ─────────────────          │ 上次扫描 2 分钟前   [🔄 重扫]       │
│ ★ skills-manage-windows    │ ─────────────────────────────────   │
│   D:\Documents\Code\...    │ [+ 从中央库安装]  [🔍 搜索 skill]   │
│   12 skills          [···] │ ─────────────────────────────────   │
│ ○ my-app                   │ ┌─────────────────────────────────┐ │
│   D:\code\my-app           │ │ brainstorming         [symlink] │ │
│   3 skills           [···] │ │ Claude Code · .claude/skills/   │ │
│                            │ │ 创造性工作前必须使用...         │ │
│                            │ │ [预览] [卸载]                   │ │
│                            │ └─────────────────────────────────┘ │
└────────────────────────────┴─────────────────────────────────────┘
```

### 7.3 项目卡片元素

- `★/○` pin 切换图标
- name + 路径（中段省略）+ skill 数 badge
- `[···]` 上下文菜单：重命名、重新扫描、移除
- 整卡可点击，激活右侧详情

### 7.4 Skill 卡片元素

复用 `UnifiedSkillCard` 扩展 `scenario="project"`：

- 名称 + agent 图标
- `[symlink]` 绿色 / `[copy]` 琥珀色 标签
- description 摘要
- `[预览]` → SkillDetailView drawer，sidebar 显示"装在哪个项目"
- `[卸载]` → 直接调 `uninstall_skill_from_project` + toast

### 7.5 "从中央库安装"对话框

复用现有 `InstallDialog` 模式：

- 目标侧改为"该项目下的哪些 agent 目录"——勾选 `.claude/skills/`、`.kiro/skills/` 等
- 列出**所有已启用的 agent**（与 6.2 步骤 2 同一来源），不要求项目下该 agent 目录已存在。若目录不存在，由 `install_skill_to_project` 负责自动创建（6.3 步骤 3 已覆盖）
- UI 用次要文案标注每个 agent 目录的当前状态：`✓ 已存在` / `+ 将自动创建`，让用户对落盘后的目录结构有预期
- 底部安装方式 radio：`◉ symlink（默认） ○ copy`

### 7.6 移除项目弹窗

```
┌──────────────────────────────────────────────┐
│ 移除项目 "skills-manage-windows"             │
├──────────────────────────────────────────────┤
│ 项目将从列表中移除。                         │
│                                              │
│ ☐ 同时卸载本项目下所有已装 skill             │
│   未勾选时磁盘上的 .claude/skills/ 保留      │
│                                              │
│           [取消]   [移除]                    │
└──────────────────────────────────────────────┘
```

### 7.7 项目根目录快捷入口

详情页头部路径旁 `📂` 图标，点击调现有 `open_in_file_manager` 命令。

## 8. 兼容性与迁移

### 8.1 路由跳转

- `/discover` 和 `/discover/:path` 都重定向到 `/projects`。
- 跳转后顶部条幅一次性提示：`i18n: notice.discoverDeprecated = "项目技能库已升级为'项目'管理，请重新添加项目"`。
- 关闭后不再显示（用 settings 表的 boolean 记一下已读）。

### 8.2 代码删除清单

后端：

```
src-tauri/src/commands/discover.rs                   整个删除
src-tauri/src/services/discovery/                    整个删除
src-tauri/src/db.rs 中 discovered_skills 相关查询    删 get/insert/delete_discovered_skills 等
src-tauri/src/lib.rs invoke_handler 注册             删废弃命令
```

前端：

```
src/pages/DiscoverView.tsx                           删
src/pages/discoverBindings.ts                        删
src/pages/discoverViewModel.tsx                      删
src/stores/discoverStore.ts                          删
src/components/discover/DiscoverShell.tsx            改名为 ProjectsShell
src/types/ DiscoveredProject、DiscoveredSkill         删，新增 Project、ProjectSkill
i18n discover.* 键                                   迁到 projects.*
```

文档：

```
docs/guide/discover.md  和 docs/zh/guide/discover.md  改为 projects.md
docs/architecture/scanning.md  和 zh 版本             同步更新到新模型
```

## 9. 测试策略

| 类型        | 覆盖                                                                 |
| ----------- | -------------------------------------------------------------------- |
| Rust 单测   | `projects` CRUD；`psi` reconcile；symlink/copy 落盘与卸载            |
| Rust 集成   | add → scan → install → uninstall → remove 全链路，symlink/copy 各一遍|
| 前端单测    | `projectsStore` 状态转移；Add 对话框；卸载 toast 与错误路径          |
| E2E（手动） | 真机选 D 盘项目 add，验证 `.claude/skills/<x>` 出现 symlink 节点     |

Rust 集成测试通过临时目录构造模拟项目，覆盖关键失败路径（中央 skill 不存在、目标已存在、跨设备 symlink fallback）。

## 10. 分阶段交付

| 阶段 | 内容                                                            | 交付物                                  |
| ---- | --------------------------------------------------------------- | --------------------------------------- |
| I    | DB 迁移 + projects CRUD + `pick_project_folder` + 扫描入库      | 能 add、能看到 skill 列表，无法装卸     |
| II   | install/uninstall 链路 + Skill 卡片 + InstallDialog              | 核心闭环：装、卸、看 link_type          |
| III  | 移除项目带连带卸载 + pin/重命名 + 路由迁移 + 旧码与表清理        | 旧 Discover 完全替换                    |

每阶段独立 commit / PR。阶段 I 上线后即使 II/III 推迟也不破坏现有功能——旧 Discover 还在，新 `/projects` 已经空着可用。
