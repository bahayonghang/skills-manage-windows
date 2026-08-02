# 为元数据与 Marketplace 缓存批量写入补齐事务

## Goal

把当前返回单一 `Result` 的多语句 metadata/cache mutation改为真正的 all-or-nothing transaction，并让成功的 Marketplace sync 表示一个完整远端 snapshot：旧 cache在任何中间失败时保留，成功时删除远端已经移除的 rows。

## Evidence

- `repositories_repo.rs:240-250` detach三步直连pool；`:571-605` 多skill assignment逐条commit。
- `tags_repo.rs:127-167` skill x tag逐条commit；`:193-215` delete-then-insert AI links；`:218-269` delete pending后才循环验证/insert。
- `collections_repo.rs:80-90` 先删memberships再删collection；collection side没有parent FK cascade。
- `projects_repo.rs:102-115` pool已强制foreign_keys时仍显式两步删除。
- `marketplace/mod.rs:238-266` registry两步删除；`:363-403` sync逐条upsert，最后才写success且不删stale rows。
- 仓库已有 `skills_repo`、repository provenance、tag review accept等transaction模板。

## Requirements

1. 下列公开API各自拥有一个顶层transaction；成功全部commit，任何 validation/SQL/trigger/commit失败全部rollback：
   - `detach_skill_remote_source`
   - `assign_skills_to_repository`
   - `assign_skill_tags`
   - `replace_skill_ai_tags`
   - `replace_pending_ai_tag_reviews`
   - `delete_collection`
   - `delete_project`（或证明单条parent delete + enforced cascade即可）
   - `remove_registry_impl`
   - 成功fetch后的 `sync_registry_impl` cache replacement/status update
2. 需要复用的内部操作抽为 transaction-scoped helper，接受 `&mut SqliteConnection`/`Transaction` executor；顶层API不得调用另一个会自行`pool.begin()`的API造成nested transaction或pool connection切换。
3. 所有可预验证的 repository/skill/tag/review IDs 在同一transaction snapshot中批量验证后再写。保持现有错误文本和顺序；混合valid/invalid输入不能留下valid部分。
4. 批量insert/update使用 `QueryBuilder` 或等价bounded batching减少round trips；每chunk遵守SQLite bind budget，所有chunks仍由一个transaction包住。
5. `detach_skill_remote_source` 的 update state删除、membership删除和empty repository prune使用同一transaction；并发写不能在步骤间插入。
6. `replace_skill_ai_tags` 只替换source='ai' links，manual links保持；`replace_pending_ai_tag_reviews` 出错时旧pending集合完整保留。
7. `delete_collection` 在不扩大schema的前提下transaction包住child+parent。`delete_project` 优先使用单条parent delete和已验证的`ON DELETE CASCADE`，同时保留每条pool connection启用FK的contract test。
8. Marketplace network fetch/parse在transaction外完成。成功后短transaction原子替换该registry的全部cache rows并更新success metadata；最简单安全形状为transaction内delete registry rows -> insert fresh snapshot -> update registry。
9. Marketplace fresh snapshot为空时也要原子清空旧rows并记success；insert/status失败保留整个旧cache。fetch失败保持旧cache并更新attempt/error，不能先删cache。
10. `remove_registry_impl` 对missing row保持现有语义；builtin check、cache delete和registry delete在同一transaction，FK/trigger失败全回滚。
11. Marketplace install/`is_installed` post-import语义归P0 child；本任务只拥有registry sync/remove中的derived installed值。
12. 不改变公开函数签名、IPC DTO、partial-result语义或用户数据。若发现某API确实需要partial success，必须先另行修改产品contract，不能静默保留部分commit。

## Acceptance Criteria

- [ ] 每个scope API至少一条中间步骤SQLite trigger故障注入测试，断言error后所有前置row与旧集合逐项不变。
- [ ] mixed valid/invalid skill/tag/repository输入的批量测试证明零partial rows；错误文本与现状一致。
- [ ] AI replace测试覆盖manual preservation、invalid tag、第二条insert trigger failure和retry success。
- [ ] collection/project/registry parent delete failure保留children/cache；project单delete证明每条acquired connection FK enabled且cascade生效。
- [ ] Marketplace sync A,B -> fresh B,C后结果恰为B,C；empty结果清空；第二条insert或status update失败后仍完整为A,B且registry不显示success。
- [ ] Marketplace fetch/parse期间不持有DB transaction；transaction测试可观测边界或代码结构证明锁时间只覆盖DB mutation。
- [ ] 大batch按bind budget chunk且一个后段chunk失败时前段也rollback；无N x M逐条autocommit。
- [ ] 没有新增schema时generated docs不变化；如实现需要schema，则先更新本task design并按migration/docs契约执行。
- [ ] focused db/marketplace tests、Rust fmt、all-targets locked Clippy、locked tests和 `just ci` 通过。

## Non-Goals

- 不把Central filesystem mutation塞进SQLite transaction；Central FS+DB一致性继续由operation journal负责。
- 不重构全部repositories或移除`db::*`兼容re-export。
- 不在本任务处理Marketplace install direct writer或snapshot cache容量。

## Dependency

DB repository部分可独立实施。Marketplace sync/remove阶段需避开P0 Marketplace install task对同一`mod.rs`的并行编辑；功能边界互不接管。
