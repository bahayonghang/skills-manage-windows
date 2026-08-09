# 将 Central 技能分页过滤下推到 SQLite

## Goal

重写 `get_central_skills_page_impl` 的执行顺序：SQLite 先完成 filter/count/order/limit/offset，service 只对当前页最多 500 个 skill做关联富化。用当前 in-memory evaluator作为 reference oracle，锁住查询、source/tag/install-state、排序和 total语义，并建立可重复的 5k+ fixture 性能证据。

## Evidence

- `central_skills/query.rs:191-214` 读取全部 Central rows和全部关联数据。
- `query.rs:215-228` 富化每一行；`skill_time.rs:40-64` 对缺失缓存时间的行同步 stat filesystem。
- `query.rs:281-293` 最后才在内存 filter/sort/skip/take。
- installations/repositories/tags batch helpers按全部 skill IDs生成动态 `IN` binds。
- 当前 page test `central_skills/tests.rs:508-537` 只有两条 Central rows。

## Requirements

1. 新增 repository query API 返回 `(page_skill_rows, total)`；所有用户输入使用 bind，sort字段/direction和install-state先解析为enum，不拼接任意 SQL token。
2. 保持请求默认与clamp：offset默认0且负数归0；limit默认100并限制1..500；unknown sort fallback `name:asc`；现有 install aliases保持。
3. 文本搜索保持 ASCII case-insensitive literal substring语义。使用 `instr(lower(column), normalized_query)` 或等价表达，`%`、`_`、`\\` 不得变成 wildcard/escape。
4. Source filter保持 OR语义：明确 repository IDs、`unassigned`、unknown repository和无member fallback与当前 evaluator一致。
5. Tag filter保持 OR语义；`uncategorized` 匹配无tag或只有 system uncategorized tag的 skill，并可与普通tag同时选择。
6. Install filter保持当前 `linked_agents` 语义：有直接 installation，或存在 shared-root agent时视为 linked/installed；不能只查 installation表而漏掉 shared root。
7. 排序支持 name、createdAt/created_at、updatedAt/updated_at与 asc/desc。分页必须稳定：在原排序键和name后增加 `id` tie-breaker，reference oracle同步采用该确定性规则。
8. Paginated list 的 created/updated authority改为 persisted `fs_created_at`/`fs_updated_at`，缺失时回退 `scanned_at`。page hot path不得为排序或展示同步 stat全库/本页；该列表语义需写入 `skill_time` doc/spec并用legacy-null fixture验证。
9. 先取当前页 `Skill` rows，再调用现有/收窄后的 enrichment helper；installations/repository/tags查询的 ID 数量不超过 page size，且空 page不发动态 `IN ()`。
10. Source/tag normalized filter各最多100个非空唯一值；超限返回 typed input error，避免超过 SQLite bind budget。该限制如暴露到UI需同步i18n。
11. 检查现有 indexes 与 `EXPLAIN QUERY PLAN`。只添加有 before/after plan证据的最小 index；若添加 index，必须走 versioned migration并运行 docs generation。
12. `get_central_skills_impl`、detail和其他 unpaged API不在本任务删除；frontend DTO/command名保持不变。

## Acceptance Criteria

- [ ] deterministic fixtures覆盖 query中的ASCII大小写、literal `%/_/\\`、source OR/unassigned、tags OR/uncategorized、install aliases/shared root、所有sort/direction、negative/large offset/limit和empty page。
- [ ] reference-equivalence test对一组组合请求逐项比较旧 evaluator与SQL的 `ids + total`；唯一有意变化是显式id tie-break与persisted timestamp authority，并有专门断言。
- [ ] 5k+ Central fixture、每skill多 installations/tags/repository 的 page size 25查询只把25个 IDs交给 enrichment helpers；structural test禁止 page impl调用 `get_central_skills_impl`。
- [ ] batch helper bind数量不超过500；filter超100值fail typed validation，不接近 SQLite variable limit。
- [ ] `EXPLAIN QUERY PLAN` evidence记录 unfiltered name/time sort、source、tag、install和contains search；明确哪些查询合理使用scan，不能伪报全部走index。
- [ ] release build固定fixture的before/after多轮p50/p95与rows/enrichment/query count写入task research；CI不使用跨机器wall-clock硬阈值。
- [ ] paginated hot path中不存在 `std::fs::metadata`，displayed timestamps与SQL排序key一致。
- [ ] schema如变化则migration old/new DB tests、`pnpm docs:gen`两份generated docs、`docs:gen:check`通过。
- [ ] focused central_skills/db tests、Rust fmt、all-targets locked Clippy、locked tests和 `just ci` 通过。

## Non-Goals

- 不引入SQLite FTS或改变contains产品语义；contains query/count在大库仍可能scan，但不再全量hydrate/enrich。
- 不改frontend virtualization、page size或缓存策略。
- 不用不稳定的“必须快X倍”作为CI门禁。

## Dependency

无代码前置依赖。实施前先建立reference oracle与benchmark，随后才能改query。
