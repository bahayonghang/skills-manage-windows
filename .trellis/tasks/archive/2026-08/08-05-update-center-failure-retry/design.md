# 技术设计：更新中心失败项重试与移除决策自动化

## 1. 边界与改动面

| 层 | 文件 | 改动性质 |
| --- | --- | --- |
| services/central_updates/inventory | `mod.rs` | 拆分 compute/persist；常规模式归位接入 |
| services/central_updates/inventory | `relocation.rs` | 抽出共用归位核心；新增快照候选归位 |
| services/central_updates/inventory | `types.rs` | `FailedRepository` 增加 `retry` 分类字段 |
| services/central_updates/inventory | `retry.rs`（新增） | 分片刷新 + 合并 |
| services/central_updates | `error.rs` | 新增归位失败与源路径消失的错误码 |
| commands | `skill_update_inventory.rs` | 新增 `retry_failed_update_repositories` IPC 壳层 |
| 前端 store | `updateCenterStore.ts` | `retryRepositories` + 进行中状态 |
| 前端 UI | `UpdateCenterTabContent.tsx` | Failed 面板动作化 |
| 前端 UI | `UpdateCenterDialog.tsx` | 传递重试 handler |
| i18n | `zh.json` / `en.json` | 新增文案与错误码 |

不改动：`apply_skill_update_decisions_impl`、强制更新/强制镜像、`clear_skill_update_inventory_impl`、DB schema。

## 2. 常规模式自动归位

### 2.1 匹配规则

在快照内按 `skill_id` 定位新路径，规则与增量模式保持同一判定强度：

1. 用 `github_import::build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)` 取整仓候选（快照已在内存，无额外网络请求）。
2. 过滤出 `candidate.skill_id == state.skill_id` 且 `normalize_repo_path(candidate.source_path) != old_path` 的候选。
3. 命中数恰好为 1 时归位；为 0 或 ≥2 时不归位。
4. 若该新路径同时被另一个中央技能占用（`skill_repository_members` 中已存在其它 `skill_id` 指向同路径），不归位。

### 2.2 共用实现

`relocation.rs` 抽出两个函数，增量与常规模式共用：

```rust
pub(super) fn unique_relocation_target(
    skill_id: &str,
    old_path: &str,
    candidates: &[RemoteSkillCandidate],
) -> Option<String>;

pub(super) async fn apply_relocation(
    pool: &DbPool,
    prepared: &PreparedSkillUpdate,
    repo: &GitHubRepoRef,
    repository_id: &str,
    new_path: &str,
    snapshots: &SharedGitHubSnapshots,
) -> Result<SkillUpdateState, RelocationError>;
```

`apply_relocation` 内部仍是 `state_from_relocated_source` + `persist_relocated_skill` 的组合，两条路径行为一致。
`reconcile_relocated_remote_skills` 改为把 `remote_added_items` 转成候选后调用同一对函数，外部行为不变。

### 2.3 接入点

`refresh_skill_update_inventory_impl` 步骤 3 的循环（`mod.rs:238-293`）：

- 循环内不再直接对 `RemoteSkillLoadError::RemoteMissing` + `Regular` 产出 `FailedRepository`，改为把 `(skill_id, repo_cache_key, old_path)` 收集进 `pending_regular_relocations`。
- 整仓候选按 `repo_cache_key` 懒构建并缓存在 `HashMap<String, Arc<Vec<RemoteSkillCandidate>>>`，同一仓库多个待归位技能只扫一次。
- 循环结束后统一处理待归位集合：命中唯一目标则 `apply_relocation`，状态为 `UpdateAvailable` 时推入 `updatable`；未命中或归位失败则推入 `failed_repositories`，`retry` 分类见 §3。
- 落库集中在这一段，`persist_relocated_skill` 的单条事务保持不变（待归位技能数量与仓库数同量级，不做批量事务优化）。

常规模式仍不构建 `remote_missing` 桶、不做任何删除。

## 3. 失败项分类

`types.rs`：

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FailedRepositoryRetry {
    /// 快照获取失败、归位失败、新增收集失败：重试同一范围可能得到不同结果。
    Retryable,
    /// 源路径消失且无法自动归位：需要用户在增量模式下决定保留或删除。
    DecisionRequired,
    /// 本字段出现之前持久化的条目，不提供就地动作。
    #[default]
    Unknown,
}

pub struct FailedRepository {
    pub repository_id: String,
    pub error: String,
    pub error_code: Option<String>,
    #[serde(default)]
    pub retry: FailedRepositoryRetry,
    #[serde(default)]
    pub diagnostics: Option<SkillUpdateDiagnostic>,
}
```

写入点分类：

| 产生位置 | 分类 | error_code |
| --- | --- | --- |
| 快照获取失败（`mod.rs:191-207`） | `Retryable` | 沿用 `failed_repository_reason` |
| 常规模式归位未命中 | `DecisionRequired` | `central_updates.skill_source_missing` |
| 归位执行失败（`relocation.rs:101-110`） | `Retryable` | `central_updates.relocation_failed` |
| `collect_remote_added_skills` per-repo 失败（`mod.rs:354-361`） | `Retryable` | `central_updates.repository_check_failed` |

`mod.rs:247-262` 的英文硬编码文案删除。新错误码经 `ipc_error::public_message_for_code` 提供已审阅英文兜底文案，前端按 `backendErrors.<code>` 本地化（`UpdateCenterTabContent.tsx:47-53` 的现有机制不变）。

`DecisionRequired` 条目仍按 `repository_id` 聚合，同仓库多个技能只保留一条（沿用 `mod.rs:403-404` 的去重）；技能标识不进错误文案，改由 `diagnostics.source_path` 承载。

## 4. 分片刷新与合并

### 4.1 计算与持久化分离

`refresh_skill_update_inventory_impl` 拆成：

```rust
pub(crate) async fn compute_skill_update_inventory(...) -> Result<SkillUpdateInventory, CentralUpdatesError>;
// 原函数 = compute + persist_refresh_inventory，签名与行为不变
```

### 4.2 新服务函数

`inventory/retry.rs`：

```rust
pub(crate) async fn retry_failed_repositories_impl(
    pool: &DbPool,
    fs: &CentralFs,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    base_scope: SkillRefreshScope,
    repository_ids: Vec<String>,
    mode_override: Option<SkillRefreshMode>,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SkillUpdateInventory, CentralUpdatesError>;
```

流程：

1. `repository_ids` 归一去重；为空直接返回基线清单。
2. 基线：`get_skill_update_inventory_impl_scoped(pool, Some(base_scope.clone()))`。
3. 分片范围：`SkillRefreshScope { kind: Repositories, mode: mode_override.or(base_scope.mode), cache_policy: Bypass, repository_ids, .. }`，调 `compute_skill_update_inventory`（**不落库**）。
4. 合并：`merge_inventory_for_repositories(base, partial, &repository_ids)`。
5. 以 `base_scope` 落库：`persist_refresh_inventory(pool, &base_scope, base_mode, cache_policy, &merged)`。`inventory_id` 与 `run.mode` 保持基线值，分片的 `mode_override` 只影响本次计算内容。
6. 返回 `merged`。

### 4.3 合并规则

| 桶 | 规则 |
| --- | --- |
| `updatable` / `remote_missing` | 剔除 `repository_id ∈ set` 的基线条目，并入分片同桶条目 |
| `remote_added` | 同上（`repository_id` 必填） |
| `failed_repositories` | 同上；分片成功时该仓库不再出现，即为重试成功 |
| `unsupported` | 与仓库无关，保留基线 |
| `platform_duplicates` / `deleted_platform_copies` / `orphans` | 仅全量扫描产出，保留基线，忽略分片值 |
| `generated_at` | 取本次合并时间 |

`updatable` / `remote_missing` 中 `repository_id` 为 `None` 的基线条目一律保留。

### 4.4 IPC 壳层

```rust
#[tauri::command]
pub async fn retry_failed_update_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
    repository_ids: Vec<String>,
    mode_override: Option<SkillRefreshMode>,
    operation_id: String,
) -> crate::ipc_error::IpcResult<SkillUpdateInventory>;
```

复用 `refresh_skill_update_inventory` 已有的 `ipc_boundary!` + `with_operation_log` + 进度事件发射结构（同一 `central://skill-update-inventory-progress` 事件与 `operationId` 关联规则），Operation Log details 增加 `repositoryCount` 与 `modeOverride`。

## 5. 前端

### 5.1 store

```ts
retryingRepositoryIds: string[];
retryRepositories(
  repositoryIds: string[],
  options?: { mode?: SkillRefreshMode },
): Promise<{ succeeded: string[]; failed: string[] }>;
```

- 调用期间把目标 id 并入 `retryingRepositoryIds`，`finally` 中移除。
- 后端返回已合并的完整清单，store 整体 `set({ inventory })`，前端不做合并逻辑。
- 成功/失败判定：比较返回清单的 `failedRepositories` 中是否仍含该 `repository_id`。
- 浏览器 fixture 模式（`!isTauriRuntime()`）返回当前 inventory，不发请求。

### 5.2 Failed 面板

`UpdateCenterTabHandlers` 增加：

```ts
retryRepositories: (repositoryIds: string[], mode?: SkillRefreshMode) => void;
```

`FailedRepositoriesPanel` 新增 props：`onRetry`、`retryingRepositoryIds`、`disabled`。

- 顶部："重试全部可重试项 (N)"，N = `retry === "retryable"` 的条目数，N 为 0 时禁用。
- 每行动作：
  - `retryable` → 「重试」按钮，`onRetry([id])`。
  - `decision_required` → 「用增量和删减模式重查该仓库」按钮，`onRetry([id], "sync")`。
  - `unknown` → 无动作按钮。
- 行内进行中：`retryingRepositoryIds.includes(id)` 时按钮显示 `Loader2` 并禁用。
- 全局互斥：`isRefreshing || isApplying || isForcing` 时全部禁用。

### 5.3 提示

- 单条：成功 `central.updateCenter.failed.retrySuccess`，仍失败 `central.updateCenter.failed.retryStillFailing`。
- 批量：`central.updateCenter.failed.retryBatchResult`（`{succeeded}` / `{failed}`），失败数 > 0 用 `toast.error`，否则 `toast.success`。
- 错误：`central.updateCenter.failed.retryError`，走 `formatBackendError`。

## 6. 契约与兼容

- `FailedRepository` 为加字段，旧持久化 payload 反序列化得到 `Unknown`，UI 不提供动作但仍显示错误文案；任意一次完整刷新后恢复为准确分类。
- 新增 IPC 命令进入 codegen 产物，需要更新 `generatedCommandMap.ts`、`docs/architecture/_generated/ipc-commands.md` 与 ipc 契约测试中的命令计数。
- 无 DB schema 变更，无迁移。

## 7. 取舍

- 选择"后端合并后返回完整清单"，而非前端合并：持久化与视图只有一个真实来源，避免 `inventory_id` 记录与界面显示分裂。
- 选择"常规模式在快照内归位"，而非"常规模式也收集远端新增"：前者零额外网络成本，且不引入移除决策桶，保持常规检查的低风险语义。
- `DecisionRequired` 保留在 Failed 桶而不新建标签页：避免再增加一个用户需要理解的分类，动作直接挂在条目上。
- 不做自动重试与退避：失败原因包含认证与配额，自动重试会放大配额消耗。

## 8. 回滚

改动集中在单个分支，无 schema 迁移与数据写回。回滚即撤销分支；已归位的 `skill_repository_members.source_path` 是正确的上游路径，回滚后旧代码按新路径继续工作，不需要反向迁移。
