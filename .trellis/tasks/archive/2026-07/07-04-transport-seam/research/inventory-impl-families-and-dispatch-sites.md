# 盘点：`*_impl` 平行函数族 + 命令层 active_target 分发点（2026-07-05）

> 由探查 agent 产出，行号基于 dev 分支当日快照。

## Part 0: 核心类型与 active target 解析链

| 类型                         | 定义                   | 形状                                                                                                                                                                                    |
| ---------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `TargetKind` enum            | `targets/model.rs:25`  | `Local` / `Ssh` / `Wsl`（Copy，serde lowercase）——持久化判别                                                                                                                            |
| `ActiveTarget` enum          | `targets/model.rs:207` | `Local` / `Ssh(Box<RemoteTargetConfig>)` / `Wsl(Box<WslTargetConfig>)` ——运行时分发枚举。helpers: `is_remote_like()`(216) `remote_home()`(218) `id()`(226) `label()`(234) `kind()`(242) |
| `RemoteTargetConfig`         | `targets/model.rs:59`  | SSH 目标行                                                                                                                                                                              |
| `WslTargetConfig`            | `targets/model.rs:82`  | WSL 目标行                                                                                                                                                                              |
| `ConnectedRemoteTarget` enum | `targets/remote.rs:23` | `Ssh(ConnectedSshTarget)` / `Wsl(ConnectedWslTarget)` ——活连接，由 `connect_remote_target()` 产出                                                                                       |

解析链：设置键 `"active_target_id_v1"`（`model.rs:4`）→ `TargetRegistry::active_target`（`registry.rs:356`）→ `AppState::active_target()`（`lib.rs:78`）。`connect_remote_target(&ActiveTarget)`（`targets/remote.rs:9`）是 Ssh/Wsl 变成活连接的唯一入口。**命令层所有分发都是 `Local` vs `Ssh(_)|Wsl(_)` 二元**；SSH/WSL 之分只在 `connect_remote_target` / `ConnectedRemoteTarget` 内部重现。

## Part 1: 平行函数族（8 `_ssh_impl` + 5 `_remote_impl`，7 族）

### Pattern A —— mutation 族：`_impl`（local）+ `_remote_impl`（活远程）+ `_ssh_impl`（**死适配器**）

5 个 mutation `_ssh_impl` 全部**零调用点**（仅被 `commands/linker.rs:36,38`、`commands/skills.rs:24,25` 的 `pub use` 桥保活，含测试在内无人调用）。

1. **install_skill_to_agent**（services/installation/）：local `native.rs:58/188/149`（_impl/_copy_impl/_auto_impl）；`install_skill_to_agent_remote_impl` `remote.rs:239`（调用点 `linker.rs:74`、`collections.rs:268`）；`install_skill_to_agent_ssh_impl` `remote.rs:224` **死**。共享核心 `install_skill_to_agent_ssh_with_connection` `remote.rs:252`（`_remote_impl` 与批量路径 `linker.rs:365` 共用）。
2. **uninstall_skill_from_agent**：local `native.rs:409/431`、batch `batch.rs:107`；`_remote_impl` `remote.rs:343`（调用点 `linker.rs:138/196`）；`_ssh_impl` `remote.rs:333` **死**。
3. **delete_central_skill**（services/central_skills/delete.rs）：local `:509`；`_remote_impl` `:287`（调用点 `skills.rs:103`）；`_ssh_impl` `:368` **死**。
4. **delete_central_skills（批量）**：local `:583`；`_remote_impl` `:378`（调用点 `skills.rs:155`、`repository.rs:150`）；`_ssh_impl` `:429` **死**。
5. **delete_skill_repository**（delete/repository.rs）：local `:111`；`_remote_impl` `:140`（调用点 `skills.rs:245`）；`_ssh_impl` `:171` **死**。

### Pattern B —— preview（读）族：`_impl` + `_ssh_impl`（**活远程终端**，纯 DB+路径，无连接），无 `_remote_impl`

6. **preview_delete_central_skill(s)**：local `delete.rs:438/484`；`preview_delete_central_skill_ssh_impl` `delete.rs:197`（私有，被批量 `:256` 调）；`preview_delete_central_skills_ssh_impl` `delete.rs:243` **活**（调用点 `skills.rs:82`、`repository.rs:103`）。
7. **preview_delete_skill_repository**：local `repository.rs:83`；`_ssh_impl` `repository.rs:97` **活**（调用点 `skills.rs:224`）。

> 命名不一致：`_ssh_impl` 在 mutation 族=死适配器，在 preview 族=活远程实现。统一 seam 时应连带处理命名，5 个死 mutation 适配器可直接删除。

## Part 2: 命令层分发点（三类）

### 2a. 真分发（显式 match/if 选 local-vs-remote 实现）—— 8 文件 19 处

| file:line                  | fn                                | 分支                                                                    |
| -------------------------- | --------------------------------- | ----------------------------------------------------------------------- |
| `linker.rs:58`             | install_skill_to_agent            | Local→按 method 三选一；Ssh\|Wsl→`_remote_impl`                         |
| `linker.rs:127`            | uninstall_skill_from_agent        | Local→`with_row_impl`；远程→`_remote_impl`                              |
| `linker.rs:188`            | batch_uninstall_skills_from_agent | Local→batch impl；远程→循环 `_remote_impl`                              |
| `linker.rs:295`+`:339`     | batch_install_to_agents           | 预连一次 + 循环内二分（Local→by_method；远程→`ssh_with_connection`）    |
| `linker.rs:436`            | batch_install_central_skills      | `is_remote_like()`→连一次+循环；else local batch impl                   |
| `skills.rs:77`             | preview_delete_central_skills     | Local/远程 preview 二选一                                               |
| `skills.rs:98`             | delete_central_skill              | 二选一                                                                  |
| `skills.rs:152`            | delete_central_skills             | 二选一                                                                  |
| `skills.rs:219`            | preview_delete_skill_repository   | 二选一                                                                  |
| `skills.rs:240`            | delete_skill_repository           | 二选一                                                                  |
| `agents/mod.rs:356`        | get_agents                        | Local→活扫描；远程→DB 缓存                                              |
| `agents/mod.rs:376`        | detect_agents                     | 二选一                                                                  |
| `scanner.rs:52`            | scan_all_skills                   | 远程包 90s 超时                                                         |
| `github_import.rs:31`      | preview_github_repo_import        | 二选一                                                                  |
| `github_import.rs:64`      | import_github_repo_skills         | 二选一                                                                  |
| `collections.rs:259`       | batch_install_collection          | `is_remote_like()`→循环 `_remote_impl(…,"copy")`；else local batch impl |
| `usage.rs:166`             | refresh                           | Local→Scope::Local；远程→连接+Scope::Remote                             |
| `local_remote_sync.rs:110` | selected_remote_target            | 守卫（要求远程）                                                        |
| `local_remote_sync.rs:121` | refresh_synced_target_cache       | **唯一 Ssh vs Wsl 分裂点**（remote_db / remote_db_for）                 |

### 2b. 守卫（拒绝/要求某 target 类型，无远程实现分发）

- `obsidian.rs:13,26,36,66,81`：远程一律空/报错（Obsidian 仅本地）。
- `central_store_location.rs:36,49`：`ensure_local_target` 守卫。

### 2c. 委托（命令把 ActiveTarget 下传，真分叉在 service）

- `skills.rs:346,362,378,394` → `central_skills/files.rs:57,93,136,~338` 内部 match。
- `marketplace.rs:113,175` → `marketplace/mod.rs:524`、`skills_sh.rs:247` 内部 match。
- `portable_state.rs:289` → service 侧；命令只读 `.kind()` 记日志。
- `central_updates.rs:41,74,107,133`、`skill_update_inventory.rs:28,78,107,136` → **`CentralFs::from_active_target`**（`services/central_updates/fs.rs:52`）façade——「解析一次、之后不再分支」的在库范本（`fs.rs:46`：`Local` / `Remote(Box<ConnectedRemoteTarget>)`）。

**基线口径**：真分发 8 文件 19 处；守卫 2 文件（合法保留）；委托 4 文件（已是目标形态）。
