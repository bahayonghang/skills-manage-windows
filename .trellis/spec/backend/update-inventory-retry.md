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

Snapshot acquisition 写入的失败项还必须带 optional/default `diagnostic_category`，并由同一 typed classifier 同时决定 category 与自动重试资格。refresh/retry 结果携带 optional retry attempted/recovered 计数；旧持久化 inventory 缺少这些字段时读取为 `None`。

refresh/retry Operation Log 最多保存 50 个 `{repositoryId,errorCode,errorCategory}`，保持结果顺序并记录截断数；repository ID 通过有限长度 ASCII allowlist，不安全值降级为固定 `batch`。Runtime Log 只保存排序去重 code/category 与 retry counts，且 refresh/retry 使用各自准确的 action。两层均不得读取 failed row 的动态 `error` 生成诊断。

## 2. 仓库归属与重试合并语义

### 2.1 Actionable inventory 的仓库归属

检查 scope 只决定本轮检查哪些技能、是否扫描远端新增项，不改变技能已有的仓库归属。凡是从 `PreparedSkillUpdate` 生成的仓库型 `updatable` / `remote_missing`，都必须直接携带 `assignment.repository.id`：

- Skills / Platform scope 即使没有显式 `repository_ids`，也必须写入 `Some(repository_id)`。
- Inventory 计算与 relocation 之间用仓库 ID 和 state 的组合值传递，不得从 state 的 URL / branch 反向猜归属。
- 公开载荷继续使用 `Option<String>` 以读取旧 inventory；新生产的仓库型 actionable item 不得写 `None`。

### 2.2 分片重试

`refresh_skill_update_inventory_impl` 按 `inventory_id = kind:mode:ids` 整体覆盖持久化，前端也整体替换视图。因此**分片刷新必须由后端合并后返回完整清单**，不得让前端自行拼接，也不得用一个更窄的 scope 直接调 refresh 来重试。

`retry_failed_repositories_impl`（`inventory/retry.rs`）的固定顺序：

1. 读基线清单（按 `base_scope`）；`repository_ids` 为空时直接返回基线，不触网、不写库。
2. 用 `compute_skill_update_inventory`（compute/persist 已分离）算出仓库分片，**不落库**。
3. 从 `skill_repository_members` 读取目标仓库当前的 Central member skill ids，供旧 inventory 兼容使用。
4. `merge_inventory_for_repositories` 替换 `updatable` / `remote_added` / `remote_missing` / `failed_repositories`；`unsupported` 与平台桶（platform duplicates / deleted platform copies / orphans）只由全量扫描产出，一律保留基线值。
   - `repository_id = Some(id)`：只按显式 id 是否命中目标仓库判断；显式属于其它仓库的条目不得因 skill id 相同而删除。
   - `repository_id = None`：仅当当前 member skill ids 能证明该技能属于目标仓库时替换；无法证明的旧条目保守保留。
   - 先删除目标基线项再追加分片；分片已变为 up-to-date、没有 replacement 时，旧 actionable row 必须消失。
5. 合并后的 entry keys 必须在进入数据库事务前通过唯一性校验，然后继续使用严格 INSERT 持久化。重复 `(inventory_id, bucket, entity_key)` 返回固定的 `central_updates.inventory_invariant`，不得使用 `INSERT OR REPLACE`、静默 dedup 或“最后一项获胜”。
6. 以 `base_scope` 与基线 mode 落库，`inventory_id` 与 `run.mode` 不受 `mode_override` 影响。

`mode_override` 只改变本次分片查找的内容（例如让 `decision_required` 的技能进入 `remote_missing` 决策桶），不改变面板自身所处的模式。

分片继承基线的 cache policy，使重试看到的与再刷一次该面板一致。

### 2.3 Remote addition 的不可变 Apply 权威

- Refresh 从同一 pinned repository snapshot 生成新增项，并在每个 pending row
  写入同一 full commit SHA 和 repository digest。
- Apply 先按 repository 合并 selections，再读取 selected pending rows；缺失、
  legacy `NULL`、格式错误或混合 identity 只失败该 repository，并保留其 rows。
- Local exact cache hit不得触网；cache miss 只获取持久化 full SHA 并校验摘要。
  SSH / WSL 同样以 full SHA 创建 workspace并校验完整 repository manifest。
- Apply 禁止重新解析 display branch。成功后只删除成功 imported 或明确 skip 的
  pending rows，不影响同批其它 repository 的 partial success。

Inventory invariant 是内部 fail-closed 错误：IPC code 固定为 `central_updates.inventory_invariant`、`retryable=false`，Operation Log 只记录固定 category / phase，不得写重复 key、skill id、仓库 URL、本地路径或 SQLite 错误文本。全局 legacy unique-constraint 映射不作为 inventory 的错误分类路径。

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
- `services/central_updates/inventory/persistence.rs` — inventory entry key 唯一性校验与严格持久化
- `commands/skill_update_inventory.rs` — `retry_failed_update_repositories`，复用 refresh 的进度事件与 Operation Log 结构
- `src/stores/updateCenterStore.ts` — `retryRepositories`（整体替换后端返回的合并清单）

## 5. 测试要求

- Skills + Regular 与 Platform + Regular 基线中，同一仓库同时有 updatable 和 missing skill，再以 Sync repository slice 重试；结果不得重复并可从原 scope inventory reload。
- Skills / Platform 新生产的 updatable 与 remote-missing 必须断言精确 repository id。
- 用旧 payload 形状（`repository_id = null`）覆盖：目标替换、目标变为 up-to-date 后 stale row 删除、非目标 null row 保留。
- 人为构造重复 entry key 时断言 typed invariant、固定 IPC envelope 和旧 run 无修改；错误载荷不得出现数据库结构或动态标识。
- 既有 All / Repositories retry、relocation、platform buckets 和 empty-target no-op 用例保持通过。
