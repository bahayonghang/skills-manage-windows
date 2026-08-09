# Design: Transactional metadata and cache mutations

## 1. Transaction ownership rule

```text
public repository/use-case API
  -> begin transaction
  -> validate all referenced rows
  -> execute all writes/chunks via same connection
  -> commit
```

Helper命名使用 `_in_transaction`，接收 `&mut Transaction<'_, Sqlite>` 或 `&mut SqliteConnection`。只有public top-level开始/提交transaction；helper不访问pool。

## 2. Repository operations

### Repository membership

- `assign_skills_to_repository`：在tx内验证repository和全部skills，批量upsert members。
- `detach_skill_remote_source`：delete update state -> delete member -> prune empty repositories全部在tx内；prune helper改为executor版本。

### Tags and reviews

- 批量一次读取/验证tag IDs与skill IDs。
- `assign_skill_tags_in_transaction`负责bounded multi-values upsert。
- `replace_skill_ai_tags`：validate -> delete old AI only -> insert new，全tx；manual rows不触碰。
- `replace_pending_ai_tag_reviews`：先验证全部existing/proposed条件，再delete pending并bulk upsert。任何trigger/constraint错误rollback到旧集合。

### Parent deletion

- Collection当前没有`collection_id -> collections` FK，保留explicit child+parent但放一个tx。
- Project已有`ON DELETE CASCADE`且pool `after_connect` fail-closed验证FK，改为单条parent delete。测试获取多条connections逐条证明FK enabled。

## 3. Marketplace sync state machine

```text
write attempt timestamp (short DB write)
  -> fetch/parse outside transaction
  -> on fetch error: preserve cache, write error state
  -> on success:
       begin tx
       delete cached rows for registry
       insert complete fresh set with recomputed installed hints
       update last_synced/status/cache metadata
       commit
  -> on tx error: rollback old cache, best-effort write error state
```

Delete-then-insert在一个tx内比动态`NOT IN`更简单：fresh fetch已经是完整snapshot，rollback自动恢复旧cache，也没有variable-limit问题。空snapshot自然只delete并commit。

`last_attempted_sync` 在network前单独持久化是有意行为，不和fresh cache事务合并；这样crash也能看到attempt。`last_sync_status=success`只能和fresh cache同commit。

## 4. Marketplace remove

在tx内读取builtin flag并fail closed；non-builtin则delete child rows和registry。当前FK没有ON DELETE CASCADE，因此不能只删parent。Missing registry保持当前no-op或既有返回，测试pin住。

## 5. Batching

- 使用仓库现有SQLx `QueryBuilder<Sqlite>`模式。
- 一个row所需bind数乘batch size不得超过集中定义的安全阈值；按chunk执行。
- 所有chunk共享一个tx。故障注入放在后段chunk，证明前段rollback。
- validation query同样chunk，但在任何mutation前完成。

## 6. Failure injection

在single-connection memory DB内创建TEMP/普通trigger，对特定sentinel ID执行`RAISE(ABORT, ...)`。测试流程：seed old state -> create trigger -> call API -> assert error ->逐表assert byte/row-equivalent -> drop trigger -> retry success。

不要靠传入本来就会在第一步失败的值来证明中间rollback。

## 7. Compatibility and errors

Repository仍返回`sqlx::Error`，service转换为domain error，command转换stable IPC。Validation顺序与message保持；transaction加入的commit error不泄漏SQL/路径/secret。

## 8. Rollback

按domain分阶段：repositories -> tags -> collection/project -> Marketplace。每阶段可独立回滚。Marketplace delete/insert/status必须作为一个阶段，不能只落stale delete而没有rollback tests。
