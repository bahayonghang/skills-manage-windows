# Design: Bounded GitHub snapshot lifecycle

## 1. 两种状态的职责

| 状态 | 内容 | 生命周期 | mutation lease |
| --- | --- | --- | --- |
| Central update cache | repository bytes | check/update 间短期复用 | 无 |
| Preview registry | target-bound immutable snapshot或 remote workspace ownership | preview -> read/import/discard/expiry | 单 holder import lease |

二者共享计量 helper，但不能合并 registry：preview 的 binding/lease/cleanup 语义更严格。

## 2. Shared snapshot bytes

将 Central cache value 与 request map 改为 `Arc<GitHubRepoSnapshot>`。为 snapshot 增加 checked `retained_bytes()`，按每个 `Vec<u8>.len()` 求和，overflow 返回 typed budget/internal error。

```rust
struct CachedSnapshot {
    snapshot: Arc<GitHubRepoSnapshot>,
    retained_bytes: u64,
    cached_at: DateTime<Utc>,
    last_access_seq: u64,
}
```

使用单调 access sequence 保证测试和 LRU 不依赖 wall-clock tie。lock 内只更新 metadata/Arc，不遍历或 clone bytes。

## 3. Central cache policy

`SnapshotCacheLimits { max_entries: 8, max_bytes: 256 MiB, ttl: 10 min }` 为 production default，测试可注入小值和 clock/access sequence。

Insert 顺序：

1. 计算 bytes，先 prune expired。
2. 如果单项超过 cache aggregate cap，返回“当前请求可用但未缓存” outcome。
3. 替换同 key 时先扣旧 bytes。
4. 从 oldest non-current entry 淘汰，直到新值满足 count+bytes。
5. 插入 Arc，更新计数。

API 返回 cache outcome 供安全诊断，不把容量拒绝升级为 update failure。

## 4. Preview registry state machine

```text
Ready --lease--> Importing --failure--> Ready
  |                  |--success--> CleanupPending/Removed
  |--expire/evict/discard--> CleanupPending/Removed
CleanupPending --cleanup fail--> CleanupPending
CleanupPending --cleanup ok--> Removed
Importing --discard--> Importing(discard_pending) --release--> CleanupPending/Removed
```

Local storage drop 是同步成功清理，可直接 Removed。Remote storage 必须经过 owning target adapter；进入 CleanupPending 后 token 不可再读取或 import。

## 5. Target ownership

- Registry selection/prune API 必须接收 `target_id`，只返回该 target 的 ready victims。
- 当前 target connection 只清理这些 victims；不得调用 global prune 后忽略 foreign target victims。
- 每个 remote victim 在 `remove_tree` 成功后 `ack_cleanup(id)`；失败则保留 cleanup-pending entry。
- 当切回 target、创建新 preview、explicit discard 或 target delete cleanup 时重试 pending entries。
- global cap 满且 victims属于其他 target时 fail closed，保留 ownership。错误引导用户关闭旧 preview/切换 target 触发清理，但公共文本不含 target id。

## 6. Register failure

Remote acquisition 先创建 workspace、构造 snapshot，再向 registry register。若 per-target/global cap 无法接纳：

1. 尝试 owning connection 删除刚创建的 workspace。
2. 删除成功则返回 capacity error。
3. 删除失败则把新 snapshot登记为 CleanupPending，再返回 capacity/cleanup error；不能遗失 path。

## 7. Concurrency

- Registry mutex 只做状态 transition，不在 lock 内 await remote cleanup。
- transition 返回 immutable cleanup ticket；ack 使用 id + generation，避免旧 cleanup completion 删除同 id 的新 entry。
- import lease transition 与 eviction/prune 在同一 mutex 下原子检查。
- Central cache lock 同样不包含 network/bytes clone。

## 8. Compatibility and errors

现有六个 preview lifecycle IPC code保持；新增 capacity/cleanup pending code 使用固定摘要并映射到“关闭旧预览后重试/重新预览”。若 frontend 需要新 code 文案，同步中英文 i18n 和 contract tests。

## 9. Rollback

先引入 Arc/计量和测试，再加 Central eviction；preview registry state machine 单独落地。Remote cleanup transition 不能半迁移：只有在所有 call sites 都 ack/retry 后才能删除旧 prune API。
