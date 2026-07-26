# Schema 版本化迁移与关系表 FK CASCADE

## Goal

把当前不可审计的 SQLite 启动建表/探测式演进升级为可验证、可恢复的版本化迁移，并用数据库 FK CASCADE 强制七张 skill owned relations 的父子完整性。这样桌面端、`skillport-cli` 和 SSH/WSL target cache 都能从受支持的旧版本安全升级，失败时恢复升级前数据库，而不是留下半迁移状态。

## Background

- 2026-07-26 `dev`（`ac52e67a`）现场仍由 `src-tauri/src/db/schema/` 9 个模块和 `migrations.rs` 执行 `CREATE TABLE IF NOT EXISTS`、`PRAGMA table_info`、`ALTER TABLE`；没有 `schema_migrations`、版本连续性或 checksum。
- skills 父子关系没有 DB FK；只有应用层手工 cascade。前置任务 `07-24-db-stale-cleanup-fix` 已以 `21eb82a9` 完成七表 orphan repair、事务化删除和审计，为 FK rebuild 清零历史数据。
- 当前启动顺序是 `schema::init -> repair_orphan_skill_relations -> seed`。本任务必须把 whole-DB backup 放在任何 repair 或 migration 前。
- 本地桌面、`skillport-cli`、SSH/WSL target cache 均复用 `create_pool -> init_database*`，但路径和初始化当前分离，存在调用方绕过备份的结构性风险。
- 当前版本为 `0.10.14`。用户决定升级兼容窗口覆盖 Windows 发行线五个 tag：`v0.10.9`、`v0.10.10`、`v0.10.12`、`v0.10.13`、`v0.10.14`。
- 仓库没有发布版本 DB fixture；现有测试只手工构造个别 legacy table。新 fixture 必须由对应 tag 的 schema 冻结生成，不能用当前 schema 冒充旧版本。

## Requirements

1. 引入 `schema_migrations(version, checksum, applied_at)`；迁移版本从 1 连续递增，每个 migration 的 schema/data 变更和版本记录在同一事务提交。
2. 启动时在任何写入前校验已应用版本连续、无未知未来版本、checksum 与当前不可变 migration source 一致；任一不符即阻断启动。
3. 所有已有文件数据库在首次 legacy baseline 或待执行 migration 前创建一致、可验证的 whole-DB 备份；新建空库不备份。repair/migration 失败后关闭 pool 并恢复原库，启动仍返回失败，避免带病继续。
4. 最终初始化顺序固定为 `open pool with FK -> migration preflight -> backup if needed -> legacy baseline -> orphan repair/audit -> FK migration -> foreign_key_check -> seed`。backup 必须先于前置任务引入的 repair。
5. 七张 owned relations 通过 SQLite table rebuild 增加 `FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE`：`skill_update_states`、`skill_repository_members`、`collection_skills`、`skill_tag_links`、`skill_ai_tag_reviews`、`skill_explanations`、`skill_installations`。
6. 每个 pooled connection 都在 `after_connect` 中开启并回读验证 `PRAGMA foreign_keys = ON`；不得依赖单次 pool-level PRAGMA。
7. FK 落地后，单删、批量 reconciliation 和 scanner stale cleanup 只删除 `skills` parent 并依赖 cascade；七表 compile-time ownership list 继续作为 migration、repair 和测试断言的单一来源。
8. 为五个批准 tag 冻结可审计的 legacy schema fixture，物化为真实临时 SQLite 文件后升级；验证迁移版本、sentinel data、七表 FK、备份和 `foreign_key_check`。
9. 本地桌面、`skillport-cli` 和按需 target cache 必须走同一 path-aware open/init API；不得保留任一生产入口的无备份初始化旁路。
10. 更新中英文数据库架构文档与 backend code-spec，删除“没有版本化 migration”这一过时契约。

## Acceptance Criteria

- [ ] 五个 tag fixture 均能升级到最新连续版本，`schema_migrations` checksum 正确，sentinel data 保留，`PRAGMA foreign_key_check` 为空
- [ ] migration source 被改写、版本断档或数据库包含未来版本时，启动在任何 schema/data 写入前失败
- [ ] migration 中途注入失败后，七表 rebuild、repair audit 和其他 schema 变化均不残留；原 DB 自动恢复且 `PRAGMA integrity_check` 通过
- [ ] 已有文件库在 repair/migration 前生成一致备份；空库和已是最新版本的库不重复制造备份；成功升级保留一份对应 source version 的 last-known-good 备份
- [ ] pool 的多个独立连接均返回 `PRAGMA foreign_keys = 1`
- [ ] 删除 skill parent 后七张 owned relations 由 CASCADE 清零；observation、project snapshot、usage history 等独立生命周期数据保留
- [ ] local desktop、CLI、SSH cache、WSL cache 的生产入口均命中相同 migration/backup contract
- [ ] `cargo test db:: --locked`、全量 Rust gates 和 `just ci` 全绿

## Key Decisions

- 兼容窗口：覆盖 `v0.10.9` 至 `v0.10.14` 五个实际 tag，不把缺失的 `v0.10.11` 伪造成发布基线。
- 恢复语义：自动恢复数据后仍返回启动失败；不在同一进程静默重试 migration。
- 备份保留：每次 pending attempt 都先生成新的唯一快照；成功发布后，每个 DB/source schema version 只保留最新一份 last-known-good sibling backup。旧 backup 不能仅凭完整性通过就盲目复用。
- fixture 形式：提交可读的 tag-pinned SQL/manifest，测试时物化为真实 SQLite 文件，避免不可审查的二进制 blob。

## Out of Scope

- 不设计 `fs-db-operation-journal` 的 operation journal/Saga 表；该子任务在本迁移体系上增量加表。
- 不提供备份浏览、手工恢复或 downgrade UI。
- 不给 `agent_skill_observations`、project snapshot、usage history 等独立生命周期表添加 skill-parent FK。
- 不重构无关 repository/service 代码，不处理其他审计子任务。
