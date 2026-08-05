# 更新中心失败项重试与移除决策自动化

## Goal

更新中心 Failed 标签页当前只做只读展示，用户对失败的仓库没有任何可执行动作；同时常规检查模式在遇到"远端源路径已消失"时把该技能降级成一条失败记录，并用硬编码文案要求用户手动切换到"增量和删减"模式重跑。本任务给失败项加上单条与批量重试能力，并让常规检查模式在快照内自动完成技能移动/改名的归位，只把确认无法自动定位的情况留给用户决策。

## User Value

- 仓库因网络、认证、限流等原因检查失败时，可以在面板内直接重试单个仓库或一次性重试全部可重试项，不必关闭对话框重跑整轮检查。
- 重试只重算被重试的仓库，其它仓库已有的可更新/新增/移除结果保持可见，不会被清空。
- 上游仓库把技能移动或改名后，常规检查模式直接跟随新路径继续检查更新，不再显示需要切换模式的提示。
- 确实无法自动定位新路径时，失败项给出稳定错误码、可本地化文案，以及"用增量和删减模式重查该仓库"的就地动作，不要求用户手动改两个下拉框。

## Confirmed Facts

### 现状：模式差异

- `refresh_skill_update_inventory_impl` 用 `include_sync_buckets = mode == Sync` 控制分支（`src-tauri/src/services/central_updates/inventory/mod.rs:98`）。远端新增收集、`remote_missing` 桶构建、平台重复扫描、`reconcile_relocated_remote_skills` 都在该分支内。
- 常规模式下，`load_remote_skill_content` 返回 `RemoteSkillLoadError::RemoteMissing` 时被降级为一条 `FailedRepository`，错误文案在 `inventory/mod.rs:243-262` 硬编码为 `"... Switch to incremental and removal mode to decide whether to keep or delete '{skill_id}'."`，`error_code` 为 `None`。
- 该文案的前半句来自 `core/state.rs:180-185` 的 `find_remote_skill_candidate`：按持久化的 `source_path` 在快照中找不到可导入技能。
- 现有自动归位 `reconcile_relocated_remote_skills`（`inventory/relocation.rs:26`）依赖 `remote_added_items`，只在 `include_sync_buckets && !repository_ids.is_empty()` 时执行（`inventory/mod.rs:305`）。因此增量模式下若 scope 为 `skills`（`repository_ids` 为空），自动归位同样不执行。
- 前端 `buildUpdateCheckScope` 在 regular 模式固定返回 `kind: "skills"`（`src/pages/centralUpdateCheckMode.ts:29-35`）。

### 现状：失败项与重试

- Failed 面板 `FailedRepositoriesPanel`（`src/components/central/updateCenter/UpdateCenterTabContent.tsx:175-208`）只渲染仓库标签与错误文案，无任何动作按钮。全仓代码中不存在 retry 相关实现。
- `failed_repositories` 混装三类来源，语义不同：
  1. 快照获取失败（`inventory/mod.rs:191-207`），经 `failed_repository_reason` 归一为稳定 `error_code` + 已审阅公开文案，属于可重试的瞬时故障；
  2. 常规模式的 remote-missing 降级（`inventory/mod.rs:247-262`），重试同一模式必然得到同一结果；
  3. 自动归位失败（`relocation.rs:101-110`）与 `collect_remote_added_skills` 的 per-repo 失败（`inventory/mod.rs:354-361`），`error_code` 均为 `None`。
- 同一 `repository_id` 在 `failed_repositories` 中只保留第一条（`inventory/mod.rs:403-404`）。

### 现状：持久化与视图替换

- `persist_refresh_inventory` 按 `inventory_id = kind:mode:ids`（`inventory/persistence.rs:10-39`）整体写入，底层 `replace_skill_update_inventory` 先 `DELETE FROM skill_update_inventory_entries WHERE inventory_id = ?` 再插入（`src-tauri/src/db/repos/update_inventory_repo.rs:8-18`）。
- 前端 `useUpdateCenterStore.refresh` 用返回值整体 `set({ inventory })`（`src/stores/updateCenterStore.ts:261-266`）。
- 因此直接复用 `refresh({ kind: "repositories", repositoryIds: [id] })` 做单仓库重试会产生两个后果：视图中其它仓库的全部桶被清空；结果写入另一个 `inventory_id`，与当前范围的持久化记录分裂。
- `delete_skill_update_inventory_entries_for_repositories` 已存在（`update_inventory_repo.rs:144`），可用于按仓库替换条目。

### 可行性

- GitHub 快照是整仓 tarball，已在内存中。`build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)` 传 `None` 即扫描整仓候选（用法见 `src-tauri/src/services/marketplace/tests.rs:1134-1139`）。因此常规模式做自动归位不需要额外网络请求。
- 归位后的状态构建可复用 `state_from_relocated_source`（`core/state.rs:274`）与 `persist_relocated_skill`（`relocation.rs:162`）。

## Requirements

### R1. 常规模式自动归位

- 常规检查模式遇到 `RemoteSkillLoadError::RemoteMissing` 时，先在该仓库已下载的快照内按 `skill_id` 查找候选新路径。
- 命中唯一新路径时：更新 `skill_repository_members.source_path`，按新路径重算状态；远端与本地哈希不同则进入 `updatable` 桶，相同则不进入任何可操作桶。
- 命中 0 个或多于 1 个候选时：不做任何写入，记为需要用户决策的失败项。
- 增量模式的现有归位行为（基于远端新增清单匹配）保持不变，且与常规模式归位共用同一匹配与落库实现。
- 常规模式不得自动删除任何技能、不得写入 `remote_missing` 决策桶。

### R2. 失败项分类

- `FailedRepository` 增加可重试性标记，区分：快照获取失败（可重试）、技能源路径消失且无法自动归位（需决策，不可重试）、归位与新增收集失败（可重试）。
- 旧持久化数据反序列化时该字段缺省，缺省值不得让既有失败项显示为可重试。
- 移除 `inventory/mod.rs:247-262` 的硬编码"切换模式"英文文案，改为稳定 `error_code` + 前端本地化文案。

### R3. 单条与批量重试

- Failed 面板每条可重试项提供"重试"按钮；面板顶部提供"重试全部可重试项 (N)"。
- 重试只重算目标仓库，返回合并后的完整清单：目标仓库条目被替换，其它仓库条目保持原值。
- 重试期间对应行显示进行中状态，重试按钮与刷新、应用、强制动作互斥。
- 批量重试逐仓结算，部分成功不中断其余仓库；结束后用一条汇总提示给出成功与失败数量。

### R4. 不可重试项的就地动作

- "技能源路径消失且无法自动归位"的失败项提供"用增量和删减模式重查该仓库"动作，一次点击即完成模式与范围切换并执行重查，结果并入当前清单。
- 该动作不直接删除技能，仅让该技能进入 `remote_missing` 决策桶由用户选择保留或删除。

## Non-Goals

- 不合并常规与增量两种检查模式，常规模式仍不产出移除决策桶。
- 不改动强制更新、强制镜像、平台冗余清理等既有动作的语义。
- 不做失败项的自动定时重试或指数退避。
- 不回填历史缺失的 repository membership。

## Constraints

- 新增错误码、按钮、提示的中英双语文案全部走 i18n。
- 错误文案不得包含 token、完整 URL、本地路径等敏感信息，沿用现有 `public_message_for_code` 约束。
- 重试命令走 `ipc_boundary!` + Operation Log，与现有 refresh 保持一致的错误分类结构。
- services 层新函数返回域错误枚举，不返回 `Result<T, String>`；调用方按 `matches!` 分支判断，不做字符串嗅探。
- 递归遍历、批量落盘等重 IO 继续走 `fs_util::run_blocking_fs_with`。

## Acceptance Criteria

1. 上游仓库将某技能从 `skills/a/x` 移动到 `skills/b/x` 后，常规检查模式不再产生失败项：该技能按新路径完成检查，`skill_repository_members.source_path` 已更新；内容有变更时出现在可更新桶。（Rust 单测，快照 fixture 覆盖）
2. 同一 `skill_id` 在仓库中出现 0 个或 2 个及以上候选时，常规模式不写库，产出带稳定 `error_code` 的不可重试失败项。（Rust 单测）
3. 快照获取失败的仓库标记为可重试；对其执行单条重试后，该仓库条目被刷新，其它仓库的 updatable / added / missing 条目数量与内容不变。（Rust 单测 + 前端 store 测试）
4. 批量重试 3 个仓库、其中 1 个仍失败时，返回结果包含 2 个成功仓库的新条目与 1 条失败记录，前端提示给出成功 2 / 失败 1。（前端组件测试）
5. Failed 面板对可重试项渲染重试按钮，对不可重试项渲染"用增量和删减模式重查该仓库"动作，且两类都不再出现英文硬编码文案。（前端组件测试 + i18n 键存在性）
6. `pnpm typecheck`、`pnpm lint`、`pnpm test`、`cargo test`、`cargo clippy -- -D warnings` 全部通过。
