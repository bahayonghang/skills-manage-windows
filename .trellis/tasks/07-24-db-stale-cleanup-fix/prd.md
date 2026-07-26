# 修复全量扫描 stale 清理遗漏与删除事务化

## Goal

消除三张关系表的确定性 orphan 来源，并把多语句删除放进单事务。对应审计 P1-05（🟠）、QW-03。这是 db-schema-versioning-fk（加 FK）的前置。

## 核对证据（2026-07-26 dev 分支，live 复核）

- `src-tauri/src/db/repos/skills_repo.rs:574-608`：`delete_skill` 清理 7 张拥有型关系表但逐条直连 pool，**无事务**——中途失败留下部分删除。
- `skills_repo.rs:619-698`：`delete_skills_not_in_scope` 的空集分支（623-640）与 NOT IN 分支（642-697）均只清理 `skill_update_states`/`skill_repository_members`/`skill_tag_links`/`skill_installations`，**遗漏 `collection_skills`、`skill_ai_tag_reviews`、`skill_explanations`**。
- 关系表无 FK（全仓 FK 仅 `schema/projects.rs:44`、`schema/marketplace.rs:48`），SQLite 不会阻止 orphan。
- 二阶风险（审计 §2.5）：同 ID 重新导入继承旧 collection、AI explanation 错误缓存命中、未来 FK migration 因 orphan 失败。
- 2026-07-26 对 `~/.skillsmanage/db.sqlite` 与 4 个 target cache DB 做只读 LEFT JOIN 盘点：local DB 有 1 条 `skill_explanations` orphan，其余 6 张拥有型关系为 0；4 个 target cache 均为 0。
- 同次盘点发现 local DB 有 33 条 `agent_skill_observations` 没有 `skills` parent。live code 证明 observation 由独立 `row_id` + agent scan keep-set 管理，读取不要求 parent join，因此它不是拥有型关系，不能纳入本任务 cascade/FK preflight。

## Requirements

1. `delete_skills_not_in_scope` 两个分支补齐三张遗漏表的清理。
2. `delete_skill` 与 `delete_skills_not_in_scope` 全部语句放入单个 SQL transaction（参照 `services/scanner/persistence.rs` 的 `persist_scan_batch` 既有事务模式）。
3. 修复上线路径包含一次 orphan 盘点：数据库初始化在 seed 前对所有拥有型 skill 关系表执行 LEFT JOIN 盘点，并返回可序列化 `OrphanRepairReport`。非空报告以表名、skill ID、row count 的脱敏 JSON 写入 `operation_logs.details_json`，该审计记录与 orphan 删除处于同一事务、同成败；桌面入口可在 commit 后额外写 tracing，但 tracing 不是持久契约。
4. 集中定义"skill 拥有型关系表清单"（单一常量/函数），让单删、批删、scanner stale cleanup 与 startup repair 复用。当前清单为 `skill_update_states`、`skill_repository_members`、`collection_skills`、`skill_tag_links`、`skill_ai_tag_reviews`、`skill_explanations`、`skill_installations`。
5. `agent_skill_observations` 由 agent + `row_id` 扫描事实生命周期独立维护；`project_skill_installations` 是可独立存在的项目技能快照。usage history、update inventory、repository pending/skip 是历史、临时或候选记录。这些表不作为 `skills` 行的拥有型关系，本任务不级联删除。

## Acceptance Criteria

- [ ] 删除 skill 后 7 张拥有型关系表 count 为 0（单删、批删、全量扫描三条路径的测试）；observation/project/usage 等独立生命周期数据不被误删
- [ ] 事务中任一语句注入失败时整体回滚，无部分删除（fault injection 测试）
- [ ] 既有 orphan 数据经启动修复路径清零，非空 `OrphanRepairReport` 可稳定序列化为 JSON；对应 operation log 与删除同事务提交，审计写入失败时不删除；拥有型关系的 FK 预演查询为空
- [ ] ID 复用不继承旧 metadata（回归测试）
- [ ] `cd src-tauri && cargo test db::` 全绿，`just ci` 通过

## 非目标 / 依赖

- 不在本任务内加 FK / schema 版本化（属 07-24-db-schema-versioning-fk）。
- 不备份 orphan 完整行；审计 JSON 是诊断证据，不是恢复介质。下一子任务仍须在最终 release 路径中于 repair/migration 前完成 whole-DB 原子备份。
- 无前置依赖，属 Quick Win，可立即执行。

## Key Decision

- 2026-07-26 用户选择轻量自动 repair：先记录表名、skill ID、row count 的持久审计 JSON，再在同一 transaction 删除 orphan；本任务不增加完整行备份格式、sidecar 或恢复 UI。
