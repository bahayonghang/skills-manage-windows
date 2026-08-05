# Update Inventory Retry and Relocation Contract

更新中心失败项的分类、重试合并语义，以及技能移动/改名的自动归位规则。

## 1. 失败项分类

`FailedRepository.retry`（`services/central_updates/inventory/types.rs`）决定前端在 Failed 标签页给出什么动作，写入方必须显式指定：

| 分类                | 语义                                                   | 产生位置                                     |
| ------------------- | ------------------------------------------------------ | -------------------------------------------- |
| `retryable`         | 重跑同一范围可能得到不同结果                           | 快照获取失败、归位执行失败、远端新增收集失败 |
| `decision_required` | 源路径消失且无法唯一定位新位置，需要用户决定保留或删除 | 常规模式归位未命中                           |
| `unknown`           | 本字段出现之前持久化的条目                             | 仅反序列化默认值                             |

`unknown` 是 `Default`，前端对它不提供任何动作。新增失败项写入点必须给出前两类之一，不得依赖默认值。

失败项文案只允许是 `ipc_error::public_message_for_code` 返回的已审阅句子，不得把域错误 Display、传输细节、仓库 URL、本地路径或 token 放进 `error`。技能标识不进文案，需要定位时放 `diagnostics.source_path`。

## 2. 重试的合并语义

`refresh_skill_update_inventory_impl` 按 `inventory_id = kind:mode:ids` 整体覆盖持久化，前端也整体替换视图。因此**分片刷新必须由后端合并后返回完整清单**，不得让前端自行拼接，也不得用一个更窄的 scope 直接调 refresh 来重试。

`retry_failed_repositories_impl`（`inventory/retry.rs`）的固定顺序：

1. 读基线清单（按 `base_scope`）；`repository_ids` 为空时直接返回基线，不触网、不写库。
2. 用 `compute_skill_update_inventory`（compute/persist 已分离）算出仓库分片，**不落库**。
3. `merge_inventory_for_repositories` 按 `repository_id` 替换 `updatable` / `remote_added` / `remote_missing` / `failed_repositories`；`unsupported` 与平台桶（platform duplicates / deleted platform copies / orphans）只由全量扫描产出，一律保留基线值。`repository_id` 为 `None` 的基线条目保留。
4. 以 `base_scope` 与基线 mode 落库，`inventory_id` 与 `run.mode` 不受 `mode_override` 影响。

`mode_override` 只改变本次分片查找的内容（例如让 `decision_required` 的技能进入 `remote_missing` 决策桶），不改变面板自身所处的模式。

分片继承基线的 cache policy，使重试看到的与再刷一次该面板一致。

## 3. 自动归位规则

两种检查模式共用 `unique_relocation_target` + `apply_relocation`（`inventory/relocation.rs`），归位只在**恰好一个**候选满足「同 `skill_id` 且路径不同」时执行：

- 增量模式的候选来自远端新增清单（`collect_remote_added_skills`），并额外校验 addition 的 conflict 不指向别的技能。
- 常规模式没有新增清单，候选来自该仓库已下载的整仓快照：`build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)`。快照已在内存中，**不产生额外网络请求**；同一仓库的多个待归位技能共用一次候选扫描。
- 归位前必须确认新路径没有被别的中央技能占用（`db::get_skill_id_for_repository_source_path`）。
- 0 个候选、多个候选、新路径被占用：不写库，产出 `decision_required` 失败项。

常规模式不构建 `remote_missing` 决策桶、不删除任何技能；这是"常规检查低风险"的边界，归位只做路径跟随。

## 4. 现有实现

- `services/central_updates/inventory/mod.rs` — `compute_skill_update_inventory` / `refresh_skill_update_inventory_impl`，步骤 3 收集 `pending_relocations`
- `services/central_updates/inventory/relocation.rs` — 匹配与落库
- `services/central_updates/inventory/retry.rs` — 分片刷新与合并
- `commands/skill_update_inventory.rs` — `retry_failed_update_repositories`，复用 refresh 的进度事件与 Operation Log 结构
- `src/stores/updateCenterStore.ts` — `retryRepositories`（整体替换后端返回的合并清单）
