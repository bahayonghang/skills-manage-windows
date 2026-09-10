# Design: Central skill card project destinations

## Boundaries

- **Backend persistence:** `services/installation` 在项目安装 Installed / Skipped 之后调用 `services/projects` 的登记 helper；SQL 只进 `db/repos/projects_repo.rs`。不把 SQL 写进 `central_skills` 或 `installation`。
- **Backend read path:** `central_skills` 在现有 `skills_with_links_from_rows` enrichment 中批量读取 psi×projects，填 `SkillWithLinks.linked_projects`。不新增 IPC 命令；`get_central_skills` / `get_central_skills_page` 共用同一 enrichment。
- **Frontend:** store 继续 `get_central_skills` 刷新。`buildCentralSkillCardProps` 是 central 卡片唯一构造方，把 `linked_projects` 传进 `UnifiedSkillCard`。组件不 `invoke()`。
- **Not this task:** `Sidebar.tsx` CODING 区、`SkillCardMeta.projectBadge`、Projects 页信息架构。

## Data contract

```ts
interface LinkedProject {
  project_id: string;
  path: string;
}

interface SkillWithLinks {
  // existing fields...
  linked_agents: string[];
  linked_projects: LinkedProject[]; // serde default []
}
```

- Rust：`SkillWithLinks` 增加 `linked_projects: Vec<LinkedProject>`，`#[serde(default)]`。`LinkedProject` 含 `project_id`、`path`（与现有 `SkillWithLinks` 一样用 snake_case）。
- 卡片可见名 = 前端 `getPathBasename(path)`；空末段时回退 `project` 字面量不进 i18n 业务名，改用 i18n 的 generic project label。
- Tooltip = `formatPathForDisplay(path)`。
- 去重：同一 `skill_id` 下按 `project_id` 去重。排序：pinned DESC，然后 path basename case-insensitive，再 `project_id`（稳定）。pinned 需要 JOIN `projects.pinned`，DTO 可不暴露 pinned。
- 分页：空页不发 `IN` 查询；`IN` 列表 500 一组，对齐 `central-skills-pagination.md`。
- 不过滤、不排序 Central 列表：`linked_projects` 只 enrichment，不改变 SQL 分页谓词。`install_state` 仍只看全局 `linked_agents`。
- CLI `SkillWithLinks`：`serde(default)` 空 vec，不改 CLI 文档合同。

## Persistence after Central project install

Call sequence for `batch_install_central_skills` when `project_path` is set, per (skill, agent) after FS outcome:

1. Local transport:
   - `add_project_impl`（已存在则返回旧行）。
   - `upsert_project_skill_installation`，`source_origin = "central"`，`installed_path` / `link_type` / `symlink_target` 来自本次 outcome 或 skip 检测的已有目标。
2. Remote transport:
   - 新 helper：按规范化远端 POSIX 路径插入或复用 `projects` 行，**禁止** `Path::is_dir()`。
   - 同样 upsert psi。
3. DB 写失败：按 `transactional-mutations.md` 补偿——新物化目标删除；若本次是替换 symlink 则恢复旧目标。Skip 且未改 FS 时，DB 失败只返回错误、不删用户已有目录。
4. 不把 Central 项目安装改道到 `install_skill_to_project_impl`（那条是已有 `project_id` 的本地 Projects 页路径，且会再做一遍 FS）。

Helper 放在 `services/projects`（例如 `record_project_skill_from_central_install`），installation 只调服务函数，不碰 repo SQL。

## Card UI

- 在 central 场景增加 `linkedProjects: LinkedProject[]`（可空省略）。
- `toModel` 拷贝该字段；渲染层不读 `variant`。
- 位置：描述/标签下方、footer 上方，一行 `flex-wrap` chips。list 与 grid 都走这条，这样搜索 list 也能看见。
- Chip：`Folder`（lucide，与 Projects 列表一致）+ basename。无 click handler。`title` 为完整路径。`aria-label` 走 i18n（`central.linkedProjectLabel`，`{ name }`）。
- 不复用 `projectBadge`（单值、Obsidian）。
- compact 密度：chips 保持可见（不要像平台图标那样 idle 隐藏），否则搜索 list 的核心信息会丢。
- 空数组：不渲染容器。

## Compatibility

- 无 schema 变更，无 migration。
- 旧客户端忽略未知 JSON 字段无妨；新前端对缺字段按 `[]`。
- 安装成功后现有 `installSkill` / `batchInstallSkills` 已 `get_central_skills` 刷新，只要 DTO 带上 `linked_projects` 即可，不必改刷新时序。
- `pnpm ipc:codegen`：`SkillWithLinks` 目前无 `specta::Type`，手写 `src/types/index.ts` + commandMap 结果类型即可。不要为这一字段给 command 换名。
- 若 `docs:gen` 扫到该 DTO 则纳入提交；schema 文档不变。

## Trade-offs

| Choice | Why |
|---|---|
| Enrich list DTO vs 卡片内 `list_projects_using_skill` | 142 张卡片会打爆 IPC；违反 store 边界 |
| 共用 psi 表 vs 新表 | 详情、Projects 页、卡片同一事实源 |
| 成功/skip 都登记 | 否则截图 2 的历史 copy 永远不出现 |
| 显示 basename 而非 `projects.name` | 用户明确要求路径末段 |
| 只读 chip | 用户说「即可」；toggle 会与平台图标抢交互 |

## Rollback

- 只读展示 + 幂等 upsert：回滚代码后旧 psi 行无害。
- 若登记 helper 有 bug，最坏是 Projects 页多出登记项或卡片缺 chip；不改 Central 文件本体以外的卸载语义。
