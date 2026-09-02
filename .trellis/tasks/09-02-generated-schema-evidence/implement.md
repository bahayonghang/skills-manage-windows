# Implementation Plan

本文件只定义后续实施步骤；当前任务保持 `planning`，不修改 schema、生成器或文档。

## Steps

1. Parser regression first [R2][R3][R4]
   - Files/symbols：`src/test/scripts/docsGeneration.test.ts` 的 schema fixture；`scripts/docs/build-schema-table.mjs::{parseFile,render}`。
   - 先增加 ALTER ADD、CREATE UNIQUE、DROP/recreate、未知 DDL 与 DML-ignore fixture，再实现最小状态归并器。
   - 定向验证：`pnpm exec vitest run src/test/scripts/docsGeneration.test.ts`。
   - Rollback point：只回退上述两个文件；不得保留跳过 fixture 的静默解析分支。

2. Align base DDL with compatibility paths [R1][R6]
   - Files/symbols：`src-tauri/src/db/schema/core.rs::init`、`src-tauri/src/db/schema/metadata.rs::init`、`src-tauri/src/db/tests.rs`。
   - 把四个 ALTER-only 列写入对应 base CREATE，保留所有 `ensure_column`、UID 回填和索引语句；增加新库和缺列旧库 PRAGMA 测试。
   - 定向验证：`cargo test --manifest-path src-tauri/Cargo.toml --locked db::tests`。
   - Rollback point：成组回退 base DDL 与新增 Rust test；旧库兼容语句始终保留。

3. Render and known-gap contract [R3][R5]
   - Files/symbols：`build-schema-table.mjs::render`、`docsGeneration.test.ts`、`docs/architecture/_generated/data-model.md`。
   - 固定 unique/non-unique 表达并断言四个已知列与 `idx_skills_uid`；只用 `pnpm docs:gen` 生成产物。
   - 定向验证：`pnpm docs:gen && pnpm docs:gen:check`；随后 `git diff --check -- scripts/docs/build-schema-table.mjs src/test/scripts/docsGeneration.test.ts src-tauri/src/db/schema/core.rs src-tauri/src/db/schema/metadata.rs src-tauri/src/db/tests.rs docs/architecture/_generated/data-model.md`。
   - Rollback point：生成文件必须与生成器同进退，禁止手工修补 Markdown。

4. Close the task evidence [R5][R6]
   - 连续运行两次 `pnpm docs:gen`，第二次后确认 `git diff` 无新增字节变化；运行总验证块并记录通过/失败/跳过。
   - 人工比对生成文档的已知表、列和索引；记录真实历史 DB/安装升级 `UNVERIFIED`。

## Total Verification

```powershell
pnpm exec vitest run src/test/scripts/docsGeneration.test.ts
cargo test --manifest-path src-tauri/Cargo.toml --locked db::tests
pnpm docs:gen
pnpm docs:gen:check
pnpm docs:build
just ci
```

## Human and External Evidence

- 人工：检查 `data-model.md` diff 只增加/修正实际最终 schema，确认没有表或索引无原因消失。
- 外部：至少一个真实历史用户数据库的升级、Windows 安装升级和生产数据完整性不在自动测试覆盖内，必须报告 `UNVERIFIED`，不得用 fixture 或新建库测试替代。

## Final Rollback Point

本 task 的最终提交只包含 Change List 中的文件；出现任何运行时 schema 回归时整体 revert 该提交即可，禁止通过新增迁移版本、数据 backfill 或兼容 shim 修补本任务。
