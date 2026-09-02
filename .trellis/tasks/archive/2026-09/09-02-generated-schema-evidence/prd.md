# 数据库 schema 生成证据完整性

## Goal

让生成的数据库架构文档真实反映当前代码初始化后应得到的最终 schema，同时保留旧数据库的增量迁移兼容路径。

## Findings

- `QUAL-001`（Medium / M）：`scripts/docs/build-schema-table.mjs:20,86` 仅解析 `CREATE TABLE` 和非唯一 `CREATE INDEX`，遗漏 ALTER-only 列与 unique index。
- 已确认的缺口包括 `src-tauri/src/db/schema/core.rs:56` 的 `idx_skills_uid`，以及 `src-tauri/src/db/schema/metadata.rs:163-168,208-219,242-247` 的 `skill_tags.group_id`、AI tag review proposed 字段和 `skill_repositories.last_synced_at`。

## Requirements

- R1： [QUAL-001] `src-tauri/src/db/schema/*.rs` 中每张表的 base `CREATE TABLE` 声明当前新库的最终列集合；`ensure_column` 继续只承担旧库升级，不移除或重写现有迁移框架。
- R2： [QUAL-001] `build-schema-table.mjs` 按单个 schema 模块内的源码顺序归并仓库实际使用的 `CREATE TABLE`、`ALTER TABLE ... ADD COLUMN`、`CREATE [UNIQUE] INDEX` 和 `DROP INDEX`，得到每张表/索引唯一的最终记录。
- R3： [QUAL-001] 生成文档分别标识 unique 与 non-unique index，并保留列的最终 type、nullable、default 和 primary-key 语义。
- R4： [QUAL-001] 遇到以 `CREATE TABLE`、`ALTER TABLE`、`CREATE INDEX`、`CREATE UNIQUE INDEX` 或 `DROP INDEX` 开头、但不属于 R2 支持子集的 schema DDL 时，生成器必须带源文件位置失败；普通数据回填 DML 不属于该检查。
- R5： [QUAL-001] `docs:gen` 是唯一写入入口；`docs:gen:check` 只读检测漂移，重复生成得到稳定字节。
- R6： [QUAL-001] 自动证据必须同时覆盖解析器最终状态和运行时 SQLite 最终状态；历史用户数据库实际升级成功仍作为外部证据边界。

## Acceptance Criteria

- [x] AC1（R1）：新建数据库的 `PRAGMA table_info` 包含 `skill_tags.group_id`、`skill_ai_tag_reviews.proposed_name`、`skill_ai_tag_reviews.proposed_description` 和 `skill_repositories.last_synced_at`。
- [x] AC2（R1）：缺少上述列的旧库经过既有初始化后，`PRAGMA table_info` 包含全部四列。
- [x] AC3（R1）：四列在新库与升级库中的 type/null/default 都与 base DDL 一致。
- [x] AC4（R2）：fixture 中 `CREATE` 后 `ALTER ADD COLUMN` 的列只出现一次，并采用 ALTER 后的最终定义。
- [x] AC5（R2）：fixture 中 `DROP INDEX` 后的索引不出现在最终模型。
- [x] AC6（R2）：fixture 中同名索引被 drop 后重建时，最终模型只保留重建定义。
- [x] AC7（R3）：fixture 与仓库生成文档都把 `idx_skills_uid` 标为 unique。
- [x] AC8（R3）：fixture 与仓库生成文档都把普通索引标为 non-unique。
- [x] AC9（R3）：生成文档出现 AC1 的四个已知增量列，并准确展示其 nullable/default/type。
- [x] AC10（R4）：不支持的 schema DDL fixture 以非零结果失败，错误包含 repo-relative 源文件和 DDL 类别。
- [x] AC11（R4）：普通 `UPDATE` 回填 fixture 不触发 unknown-DDL 失败。
- [x] AC12（R5）：连续两次 `pnpm docs:gen` 产物字节相同。
- [x] AC13（R5）：`pnpm docs:gen:check` 不改文件，并在 stale artifact 上失败。
- [x] AC14（R6）：Node 解析 fixture 与 Rust PRAGMA schema tests 分别通过并各自报告验证边界。
- [x] AC15（R6）：任务交付明确记录“历史用户数据库迁移与真实安装升级 `UNVERIFIED`”，不得用 fixture 或新建库测试替代该证据。

## Out of Scope

- 重写数据库迁移框架、合并 migration version 或回填旧数据。
- 修改业务表、列、索引的既有运行时语义。
- 引入 SQLite Node 依赖、通用 SQL parser 或新的 schema DSL。
