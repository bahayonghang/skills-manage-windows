# Central Update Inventory Progress Contract

## 1. Scope / Trigger

当 Update Center 库存刷新需要向前端展示仓库级真实进度时使用本契约。它覆盖 Rust
snapshot 生命周期、Tauri event、Zustand 合并状态和模式弹窗；不改变库存响应、缓存
格式、检查范围或最多 4 路仓库下载并发。

## 2. Signatures

```rust
refresh_skill_update_inventory(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
    operation_id: String,
) -> Result<SkillUpdateInventory, String>

type SnapshotProgressReporter =
    Arc<dyn Fn(SnapshotProgressEvent) + Send + Sync + 'static>;

pub struct SkillUpdateInventory {
    // existing actionable and diagnostic buckets
    #[serde(default)]
    pub unsupported: Vec<UnsupportedSkill>,
}

pub struct UnsupportedSkill {
    pub skill_id: String,
    pub reason_code: UnsupportedSkillReasonCode,
}

pub enum UnsupportedSkillReasonCode {
    UnknownSource,
    UnsupportedSourceType,
    MissingSourcePath,
    UnsupportedSource,
}
```

```ts
listen<SkillUpdateInventoryProgressPayload>(
  "central://skill-update-inventory-progress",
  handler,
);
```

## 3. Contracts

- 前端先生成 `operationId` 并完成 `listen`，再调用命令；事件按 `operationId` 过滤。
- payload 使用 camelCase：`operationId`、`status`、`total`、`completed`、可选
  `repositoryKey` 和 `repositoryName`。
- `status` 仅允许 `started`、`repository_started`、`repository_completed`、
  `repository_failed`、`finalizing`。
- `total` 是实际去重后的 `GitHubRepoRef` 数；`completed` 单调递增并统计缓存命中、
  成功和失败。缓存命中不进入活跃列表。
- scope skill 数与 repository progress 是两个独立维度：refresh 必须先解析并分类 scope
  内的每个 Central skill，而 `total` 只表示其中可查询且去重后的远端仓库数。UI 必须同时
  保留 scope 文案（例如 141 skills）并把 `total=1` 明确写成可查询的去重仓库，不能把它
  表述成只选择或只检查了一个 skill。
- 每个 scope skill 都必须完成结构化分类。没有 repository membership、来源类型不支持，
  或 GitHub `source_path` 缺失/无法通过仓库路径规范化的 skill 进入 `unsupported`；无效
  `source_path` 不得进入 snapshot repository 集合或触发网络请求。
- `unsupported` 是只读 inventory bucket，必须与 run/其它 entries 同事务持久化并能 reload；
  旧 payload 或旧 run 缺少该 bucket 时按空集合读取。前端只根据 `reason_code` 显示固定
  i18n 文案，不显示动态 source、URL、路径或后端错误文本。
- refresh 不创建、更新或删除 `skill_update_states`。该表只保存成功 apply/update 后的安装
  baseline；`unsupported` 不得伪造 repository identity、branch、hash 或历史 assignment。
  已查询且 up-to-date 的 skill 仍可不进入 actionable inventory，但不能用它掩盖
  `unsupported`。
- `repository_started` 必须在获得 4 路 semaphore permit 后发出；每个已开始仓库必须
  以 completed 或 failed 结算。完成顺序不得成为业务或测试假设。
- 首轮仍最多 4 路并发且必须全部 settled 后，才可对 typed classifier 标记为安全可重试的
  timeout/request/body/5xx-exhausted 仓库做一次稳定顺序的串行补偿。补偿成功写入正常 cache；
  每个唯一仓库只在最终结果时结算一次，内部 retry 不得让 `completed > total`。
- GitHub archive 的合法 `302 -> codeload` 第二跳属于同一次仓库 snapshot：只有
  bounded archive 读取与 snapshot 构建成功后才发 `repository_completed`。redirect
  校验拒绝、第二跳 3xx 或下载失败必须发 `repository_failed`，且 completed 只递增一次。
- `repositoryKey` 用于集合增删，`repositoryName` 仅显示 `owner/repo`。事件禁止携带
  token、完整 URL、本地路径或后端错误详情。
- `finalizing` 在 snapshot 阶段结束后、比较和持久化前发出；该阶段允许存在失败仓库，
  命令返回值仍是最终成功/失败的唯一权威。
- 仓库 snapshot 获取失败按仓库结算，不终止整轮 refresh：失败仓库进入
  `failed_repositories`，其余仓库照常比较并持久化 inventory。检查范围覆盖全部可同步
  GitHub 仓库，一个仓库不可达就丢弃整轮结果会让检查无任何产出。
- `failed_repositories` 以 `(bucket, repository_id)` 为主键持久化，同一 repository 只保留
  一条。snapshot 获取失败先于它派生的下游原因写入，因此根因胜出。
- 已分类的失败必须写 `error_code` + 经审阅的固定文案；域错误 Display、URL、token、路径
  一律不得进入该条目。旧持久化条目缺少 `error_code` 时按 `None` 读取，前端回落到已存
  文案。
- `FailedRepository.diagnostic_category` 与 inventory 的 retry attempted/recovered 字段均为
  optional/default 兼容字段。category 与 retry eligibility 必须来自同一个 typed classifier。

## 4. Validation & Error Matrix

| 条件                                                                | 必须行为                                                                                            |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 尚未收到 `started` 或 `total == 0`                                  | 显示准备态/不确定进度，不设置 `aria-valuenow`                                                       |
| 收到其他 operation 的事件                                           | 忽略，不改变当前进度                                                                                |
| 重复 started/settled 事件                                           | 按 repository key 幂等合并，不重复显示活跃仓库                                                      |
| listener 建立失败                                                   | 不调用后端；走现有内联错误、toast 和重试路径                                                        |
| event emit 失败                                                     | best effort；不得让库存刷新失败                                                                     |
| 单个仓库下载失败                                                    | 发 `repository_failed`；写入该仓库的 `failed_repositories` 条目；其余仓库继续比较并持久化 inventory |
| archive redirect 被拒绝或第二跳失败                                 | 同上；条目携带稳定 `error_code`，不写入域错误 Display 文本                                          |
| 首轮 typed transient 失败且补偿成功                                  | 最终不写 failed repository；progress 只结算 completed 一次；retry recovered 加一                   |
| invalid ref、redirect、denial、not found、parse/integrity 或 budget 失败 | 不自动重试；保留最终 typed category                                                                |
| 同一 repository 产生多个失败原因                                    | 只保留第一条（snapshot 获取失败优先），避免 entry 主键冲突                                          |
| archive redirect 第二跳成功并构建 snapshot                          | 发 `repository_completed`；继续比较与最终持久化                                                     |
| scope 有 141 skills，但只有 7 个 skills 归属 1 个 GitHub repository | progress `total=1`；仍分类全部 141 个 skills，并把其余无法查询项持久化为 `unsupported`              |
| GitHub membership 的 `source_path` 缺失或规范化失败                 | 分类为 `missing_source_path`；不下载该 repository；其它 scope skills 继续处理                       |
| inventory entry 插入失败                                            | run 与全部 entries 一起回滚；`skill_update_states` 逐字段不变                                       |
| 其它 bucket 为空且 `unsupported` 非空                               | Update Center 默认打开“无法检查”tab；不得显示“全部最新”                                             |
| 命令成功                                                            | 清理 listener/临时进度，按 inventory 首选 tab 打开 Update Center                                    |

## 5. Good / Base / Bad Cases

- Good：4 个仓库并发下载，活跃集合显示全部 4 个 `owner/repo`，任意顺序完成后移除。
- Good：全选 141 个 skills 时只查询 1 个去重仓库，同时 reload 后仍能看到 134 个
  `unsupported` 条目及固定原因。
- Base：缓存命中仓库直接计入 completed；无仓库时稳定停留在不确定状态直到 finalizing。
- Bad：订阅建立在 invoke 之后、用 timer 伪造百分比、复用 skill 级 progress event、
  或为线性进度把下载改成串行。
- Bad：只持久化 actionable bucket，让无法查询的 skills 从结果消失；或根据历史快照、
  skill 名称和 `source` 字符串自动恢复 repository membership。

## 6. Tests Required

- Rust：去重 total、缓存命中、真实 permit 后 started、成功/失败 settle、completed 单调且
  不依赖完成顺序。
- Rust redirect fixture：合法一跳产生 snapshot 后仓库结算 completed 并允许 inventory/state
  持久化；拒绝或第二跳失败结算 failed 并写入带 `error_code` 的 `failed_repositories` 条目，
  `skill_update_states` 仍逐字段不变。
- Rust 快照聚合：一个仓库失败时其余仓库快照必须保留；fail-fast 包装器仍对首个失败返回
  Err，供沿用旧契约的调用方使用。
- Rust retry matrix：timeout/request/body/5xx 最多补偿一次且串行峰值为 1；policy/auth/not-found/
  parse/integrity/budget 调用次数保持 1；稳定顺序与最终一次性 progress settlement 必须断言。
- Rust inventory：同一 scope 同时包含 queryable、up-to-date、unassigned、unsupported-source
  和 invalid-source-path skills；断言 repository `total` 去重、unsupported reason、无错误网络
  请求、run/entry reload，以及 refresh 前后 `skill_update_states` 逐字段一致。
- Rust transaction：用 SQLite trigger 阻断后续 inventory entry，断言旧 run/entries 完整保留、
  新 run/entries 不部分发布，baseline 不变。
- Store：listen 先于 invoke、operation 过滤、活跃集合幂等合并、成功/失败清理、重试
  从零开始、unlisten 始终调用。
- Component/controller：准备/确定/finalizing 状态、1-4 个活跃仓库、长名称、progressbar
  ARIA、scope skills 与 queryable repository 文案、unsupported tab/count/reason、失败恢复选择
  和 toast、成功打开正确 tab。
- 收尾：定向测试后运行 `just ci`；Windows Tauri 手动验证真实多仓库进度与失败重试。

## 7. Wrong vs Correct

```ts
// Wrong: first backend event can be lost, and listeners can leak.
invoke("refresh_skill_update_inventory", args);
listen(EVENT, handler);

// Correct: subscribe first, scope events, and always dispose.
const unlisten = await listen(EVENT, (event) => {
  if (event.payload.operationId === operationId) merge(event.payload);
});
try {
  await invoke("refresh_skill_update_inventory", { scope, operationId });
} finally {
  unlisten();
}
```

```rust
// Wrong: reconstructing a queryable repository from incomplete assignment fields can
// schedule a request even when source_path is invalid.
let repos = prepared.iter().filter_map(|item| repo_ref_for_repository(&item.assignment.repository));

// Correct: only a successfully parsed PreparedSkillUpdate source is queryable.
let repos = prepared.iter().filter_map(|item| item.source.as_ref().map(|s| s.repo.clone()));
```
