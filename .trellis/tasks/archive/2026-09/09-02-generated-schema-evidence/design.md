# Design

## Change List

- `src-tauri/src/db/schema/core.rs::init`：让 `skills` 的 base DDL 与当前 `uid` 最终状态一致；保留 `ensure_column`、回填和 `idx_skills_uid` 建立逻辑。[R1]
- `src-tauri/src/db/schema/metadata.rs::init`：把 `group_id`、`proposed_name`、`proposed_description`、`last_synced_at` 放入各自 base DDL；保留四条 `ensure_column` 旧库路径。[R1]
- `scripts/docs/build-schema-table.mjs::{parseColumns,parseFile,render,generateSchemaDocs}`：以模块内语句顺序构建最终 table/index model，支持有限 ALTER、UNIQUE 与 DROP，并渲染索引唯一性。[R2][R3][R4][R5]
- `src/test/scripts/docsGeneration.test.ts`：增加 ALTER、UNIQUE、DROP/recreate、未知 DDL、稳定字节与仓库已知缺口 fixture。[R2][R3][R4][R5]
- `src-tauri/src/db/tests.rs`：用既有 test pool/`PRAGMA table_info` 与 `PRAGMA index_list` 验证 AC1/AC4 的运行时最终状态，不建立第二套迁移入口。[R1][R6]
- `docs/architecture/_generated/data-model.md`：仅由 `pnpm docs:gen` 刷新。[R3][R5]

## Contract

1. Canonical ownership：新库最终列由 base `CREATE TABLE` 声明；旧库升级由同模块现有 `ensure_column` 声明。两者描述同一最终语义，不改变 `schema::init` 的调度顺序。[R1]
2. Parser state：`parseFile` 对每个模块维护 `tablesByName` 与 `indexesByName`；ADD COLUMN 写入目标表，DROP INDEX 删除索引，后续 CREATE 可重建，重复最终实体报含源位置的冲突。[R2]
3. Parser boundary：只解释 Rust 字符串中的 schema DDL 子集。发现 schema-affecting 前缀但无法解析时 fail closed；`UPDATE`、`INSERT` 等 DML 明确忽略。[R4]
4. Render contract：索引行固定输出名称、列序列和 `unique`/`non-unique`；表/列/索引按确定性顺序渲染，写入仍由 `writeOrCheckGeneratedFile` 负责。[R3][R5]
5. Evidence split：Node fixture 证明生成模型，Rust test 证明实际 SQLite 初始化。两类测试不宣称验证真实用户数据库升级。[R6]

## Compatibility

- `CREATE TABLE IF NOT EXISTS` 对旧库无写入作用；保留的 `ensure_column` 继续为旧库加列，因此升级路径不因文档修复而删除。
- 新库直接得到完整列集合，随后 `ensure_column` 成为空操作；回填和 unique index 的执行次序保持现状。
- 文档格式只增加准确的列和索引唯一性标记；`docs:gen:check` 的只读语义不变。

## Verification Boundary

- 自动验证：有限 DDL parser 的状态转换、生成字节稳定性、stale 检测、当前仓库文档已知条目、SQLite 新库/缺列旧库的 PRAGMA 结果。[AC1-AC14]
- 人工检查：审阅生成 diff，确认没有业务 schema 语义变化或未解释的表/索引消失。
- 外部/缺失证据：真实历史数据库文件、跨版本安装升级和生产数据回填均为 `UNVERIFIED`。[AC15]

## Rollback

- Rollback A：若 parser fixture 失败，回退 `build-schema-table.mjs` 与对应 Node 测试/生成文档；Rust schema 文件保持不动。
- Rollback B：若新库或旧库 PRAGMA 回归，成组回退 `core.rs`/`metadata.rs` base DDL 与 Rust 测试，不移除原 `ensure_column`。
- Rollback C：最终提交以“schema 声明 + parser + 两侧测试 + 生成文档”为一个原子 task commit；整任务回退不需要数据迁移或旧数据 backfill。

## Considered but Not Chosen

- 不引入通用 SQL parser 或 Node SQLite dependency：仓库 DDL 子集有限，新增依赖扩大交付面。
- 不运行应用后把临时 SQLite dump 当文档来源：这会让文档生成依赖二进制/平台，并模糊源码 ownership。
- 不删除 `ensure_column`：它是旧数据库兼容路径，本任务只修复最终 schema 证据。
