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
- `repository_started` 必须在获得 4 路 semaphore permit 后发出；每个已开始仓库必须
  以 completed 或 failed 结算。完成顺序不得成为业务或测试假设。
- `repositoryKey` 用于集合增删，`repositoryName` 仅显示 `owner/repo`。事件禁止携带
  token、完整 URL、本地路径或后端错误详情。
- `finalizing` 在 snapshot 全部成功后、比较和持久化前发出；命令返回值仍是最终成功/
  失败的唯一权威。

## 4. Validation & Error Matrix

| 条件 | 必须行为 |
| --- | --- |
| 尚未收到 `started` 或 `total == 0` | 显示准备态/不确定进度，不设置 `aria-valuenow` |
| 收到其他 operation 的事件 | 忽略，不改变当前进度 |
| 重复 started/settled 事件 | 按 repository key 幂等合并，不重复显示活跃仓库 |
| listener 建立失败 | 不调用后端；走现有内联错误、toast 和重试路径 |
| event emit 失败 | best effort；不得让库存刷新失败 |
| 仓库下载失败 | 发 `repository_failed` 后让原命令失败；清理 listener 和临时进度 |
| 命令成功 | 清理 listener/临时进度，按 inventory 首选 tab 打开 Update Center |

## 5. Good / Base / Bad Cases

- Good：4 个仓库并发下载，活跃集合显示全部 4 个 `owner/repo`，任意顺序完成后移除。
- Base：缓存命中仓库直接计入 completed；无仓库时稳定停留在不确定状态直到 finalizing。
- Bad：订阅建立在 invoke 之后、用 timer 伪造百分比、复用 skill 级 progress event、
  或为线性进度把下载改成串行。

## 6. Tests Required

- Rust：去重 total、缓存命中、真实 permit 后 started、成功/失败 settle、completed 单调且
  不依赖完成顺序。
- Store：listen 先于 invoke、operation 过滤、活跃集合幂等合并、成功/失败清理、重试
  从零开始、unlisten 始终调用。
- Component/controller：准备/确定/finalizing 状态、1-4 个活跃仓库、长名称、progressbar
  ARIA、失败恢复选择和 toast、成功打开正确 tab。
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
