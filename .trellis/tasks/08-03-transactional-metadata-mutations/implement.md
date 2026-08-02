# Implementation Plan: Transactional metadata and cache mutations

## Step 1 - Failure matrix

- [ ] 为每个scope API记录当前statement sequence和预期atomic set。
- [ ] 添加mixed valid/invalid与中间trigger failure red tests。
- [ ] Marketplace添加A,B -> B,C、empty、second insert/status failure fixtures。
- [ ] 添加large chunk后段failure rollback test。

## Step 2 - Transaction helpers

- [ ] 抽取共享 validation/batch helpers，接受transaction executor。
- [ ] 集中SQLite safe bind budget/chunk calculation，checked处理乘法/空输入。
- [ ] 保持public signatures和error text。

## Step 3 - Repositories and tags

- [ ] 迁移detach/assign repository operations。
- [ ] 迁移assign tags、replace AI tags、replace pending reviews。
- [ ] 确认不存在nested transaction或helper回到pool。

Gate: db focused rollback和success tests通过。

## Step 4 - Parent deletion

- [ ] Collection child+parent放一个tx。
- [ ] Project改为single parent delete + cascade，补multi-connection FK/cascade/trigger tests。
- [ ] 检查missing parent与rows_affected现有语义。

## Step 5 - Marketplace cache snapshot

- [ ] 保持fetch/parse在tx外。
- [ ] 实现fresh set delete+batch insert+success metadata同tx。
- [ ] transaction failure后保留旧cache并记录error attempt。
- [ ] remove registry builtin check/delete同tx。
- [ ] 与P0 child协调`marketplace/mod.rs`，不修改install ownership。

## Step 6 - Docs and validation

- [ ] 更新data/marketplace architecture docs中transaction与snapshot语义。
- [ ] focused db/marketplace tests。
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] schema若变化：`pnpm docs:gen` + `pnpm docs:gen:check`。
- [ ] `just ci`
- [ ] final grep审计scope APIs无循环autocommit或delete-then-write旁路。

## Rollback points

- 四个domain阶段分别保持green，可独立回滚。
- Marketplace snapshot replacement整体提交/回滚；不要保留单独stale delete。
- 已写入的用户metadata不通过手工清表恢复，rollback只使用代码/transaction语义。
