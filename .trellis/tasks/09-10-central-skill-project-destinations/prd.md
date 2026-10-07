# Central Skills 展示已安装项目

## Goal

在 Central Skills 列表里，每个技能除全局 agent 安装目标外，还显示它已安装到的项目：文件夹图标 + 路径末段名称，不展示完整路径。用户从 Central Install 对话框装到项目目录后，不必打开详情或 Projects 页也能确认落点。

## User Value

从 Central 把技能装进项目（例如 `PromptHub`）后，列表卡片立刻能看见该项目；刷新后仍然在。全局 agent 图标行为保持不变。

## Background

主侧栏 CODING 区只列出全局 agent（`Sidebar.tsx:337-360`）。Central 网格卡片用 `linked_agents` 画平台图标（`CentralSkillListContent.tsx:142-149`）；该字段只来自 `skill_installations` + shared-root（`central_skills/query.rs:223-234`）。搜索会强制 list 视图，list 卡片当前不传 `platformIcons`。

项目安装有两条链路：Projects 页会写 `project_skill_installations`（`projects/crud.rs:485-500`）；Central / Batch 对话框带 `project_path` 时只做文件系统操作（`installation/project.rs:345-412`，`installation/batch.rs:104-124`）。详情页已有「已装在哪些项目」（`SkillDetailSidebar.tsx:651-695`），列表没有。`getPathBasename`（`src/lib/path.ts:103-111`）已能取路径末段。`SkillCardMeta.projectBadge` 是 Obsidian 单字符串徽章，不复用为 Central 多项目列表。

## Requirements

- **R1.** Central Skills 的 list 与 grid 卡片都要展示该技能已安装到的去重项目。全局 agent 展示维持现状（网格 footer 图标；不把 list 视图补成平台 toggle）。
- **R2.** 展示形态：文件夹图标 + 路径末段（`getPathBasename(path)`）。完整路径只作为 `title` / tooltip。两个不同路径末段相同（两个 `PromptHub`）时靠 tooltip 区分，不在默认文案里拼完整路径。
- **R3.** 同一项目装到多个 agent 时，卡片上该项目只出现一次。去重键是 `project_id`，不是显示名。
- **R4.** 无项目安装时不占位。有项目安装时刷新 / 重进 Central 后仍在。
- **R5.** Central Install 对话框（Project directory）成功后，必须登记 `projects` 并写入 `project_skill_installations`，与 Projects 页共用表，不建第三张表。本地路径走现有幂等 `add_project_impl`。远程 target 不得用本机 `Path::is_dir` 校验远端路径；在对应 target 库里按规范化远端路径登记。若安装结果是「目标已存在」而跳过，仍要补登记，以便治愈历史只落盘未入库的安装。
- **R6.** 项目项只读：不 toggle 安装/卸载。点击不要求跳转 Projects 页（完整路径 tooltip 即可）。卸载仍走 Install 对话框、详情或 Projects 页。
- **R7.** 用户可见文案走 `src/i18n/`（en + zh）。图标必须有可访问名称。卡片不直接 `invoke()`；数据来自 store 里的 `SkillWithLinks`。
- **R8.** 远程 target 上的项目安装使用同一 DTO 与同一末段名规则。
- **R9.** 可见名称始终用路径末段，不用 `projects.name`（rename 后 Projects 页可以显示自定义名，卡片仍显示末段，避免违背「不要完整路径、用末段名」）。

## Acceptance Criteria

- [ ] **AC1.** 中央技能已装到 `D:\Documents\Code\Rust\Exp\PromptHub`（及 POSIX 等价路径）时，Central 列表（含搜索 list 视图）显示文件夹图标 + `PromptHub`，不显示盘符或中间目录。
- [ ] **AC2.** 同一技能在同一项目下装了 N 个 agent，卡片仍只显示一个项目项。
- [ ] **AC3.** 仅有全局 agent、无项目安装时，不出现项目文件夹项；网格上的全局 agent 图标不回退。
- [ ] **AC4.** 经 Central Install 选择 Project directory 安装成功并关闭对话框后，无需手动 Add Project，该技能卡片出现对应项目名；刷新后仍在。
- [ ] **AC5.** 经 Projects 页登记并安装的项目同样出现在 Central 卡片上。
- [ ] **AC6.** 详情「已装在哪些项目」的项目集合与卡片去重后的项目集合一致（详情仍可按 agent 分行）。
- [ ] **AC7.** 磁盘上已有项目安装但 psi 缺失时，再次走同一 Central 项目安装（skip 已存在目标）后卡片能出现该项目。
- [ ] **AC8.** 前端测试：卡片渲染（去重、末段名、空态、list+grid）、Install 后列表刷新。后端测试：Central 项目路径安装登记 project+psi；skip 路径也补登记；Central list/page enrichment 按页内 skill id 批量查出 `linked_projects`。`unifiedSkillCardVariants` 互斥负例随新 central prop 更新。
- [ ] **AC9.** 无 schema 迁移。若 IPC/DTO 文档需要同步则 `pnpm docs:gen`。`just ci` 通过。Windows 安装包、真实 SSH/WSL 项目安装本任务不作为完成门，标记 UNVERIFIED。

## Out of Scope

- 主侧栏 CODING 区列出项目，或改 `/projects` 信息架构。
- 卡片项目芯片 toggle 安装/卸载，或点击跳转 `/projects/:id`。
- 默认展示完整路径。
- 改 Install 对话框 Target / Copy vs Symlink 交互。
- 扫描磁盘自动发现未登记项目。
- 给 list 视图补上全局 agent toggle 图标。
- 改 Obsidian `projectBadge` 语义。
- CLI 输出新字段（Rust DTO 用 `serde(default)` 保持兼容即可，不扩展 CLI 合同）。

## Key Decisions

- 展示位置：Central 技能卡片（用户确认 A），不是主侧栏 CODING。
- 名称：路径末段，不用 rename 后的 `projects.name`。
- 数据：扩展 `SkillWithLinks.linked_projects`；Central list/page 在现有 enrichment 里批量 JOIN，组件不 `invoke('list_projects_using_skill')`。
- 持久化：Central 项目安装成功或 skip-already-exists 后写入现有 `projects` + `project_skill_installations`。这会让该路径出现在 Projects 页，是可见副作用，也是列表能刷新仍在的前提。
- 远程：登记远端路径，禁止本机目录存在性检查。

## Risks

- 历史 Central 项目安装若从未再走安装路径，仍不可见，直到 skip/reinstall 或用户从 Projects 页 Add + scan。AC7 只覆盖再次安装/skip。
- 幂等 `add_project` 会把该目录加入 Projects 目录树；不另做「隐藏项目」开关。
- 远程路径登记若规范化不一致，可能与日后手动 Add Project 产生重复行；normalize 必须走现有 `normalize_project_path` / remote POSIX normalize。
