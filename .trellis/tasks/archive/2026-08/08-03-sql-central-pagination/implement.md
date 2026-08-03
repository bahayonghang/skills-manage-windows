# Implementation Plan: SQL-backed Central pagination

## Step 1 - Reference and performance baseline

- [ ] 提取/保留in-memory filter/sort为test-only reference，加入stable id tie规则。
- [ ] 创建覆盖source/tag/install/shared-root/timestamp edge cases的deterministic fixture。
- [ ] 创建5k+ benchmark fixture与enrichment input counter。
- [ ] 记录现状p50/p95、全量rows/enrichment和query plan到`research/pagination-baseline.md`。

## Step 2 - Typed filter and repository query

- [ ] 实现request normalization、enum parsing、dedupe与100值limit。
- [ ] 用共享predicate builder生成count/page SQL与binds。
- [ ] 实现literal contains、source/tag/uncategorized/install EXISTS语义。
- [ ] 实现stable whitelist order和checked total conversion。

Gate: repository query与reference在small fixtures完全等价。

## Step 3 - Page-only enrichment

- [ ] page impl先调用repository query，再把page rows交给enrichment。
- [ ] 确保relation helpers只接收<=500 IDs，empty page无IN query。
- [ ] 为paged list添加persisted timestamp helper，移除hot-path stat。
- [ ] 保持unpaged/detail调用方行为不变。

Gate: 5k fixture page size25只enrich25 rows，structural assertion通过。

## Step 4 - Query plan/index decision

- [ ] 收集name/time/source/tag/install/contains的`EXPLAIN QUERY PLAN`。
- [ ] 只在证据支持时添加最小index和versioned migration。
- [ ] 对old DB升级、new DB和index存在性写migration tests。
- [ ] 记录哪些contains/count仍预期scan。

## Step 5 - Benchmark and docs

- [ ] release build同fixture多轮warm-up，记录before/after p50/p95、query/enrichment counts。
- [ ] 更新skill timestamp/list contract与architecture docs。
- [ ] 若schema变化，运行`pnpm docs:gen`并提交两份generated docs。

## Step 6 - Validation

- [ ] focused `central_skills` / repository / migration tests。
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] `pnpm docs:gen:check`（schema变化时先`docs:gen`）。
- [ ] `just ci`
- [ ] final diff检查无frontend DTO/command drift。

## Rollback points

- Reference/baseline纯测试可独立保留。
- Repository query在route切换前无行为影响。
- Index migration只新增index，回滚代码时可保留，不删除用户数据。
