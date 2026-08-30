# 数据模型

SQLite 是唯一持久化层。`~/.skillsmanage/db.sqlite` 与 target cache 数据库统一通过 path-aware API 打开，并在每条连接上启用 WAL 与外键检查。

## 版本化初始化

```text
以 FK 开启的 pool 打开数据库
  -> 只读 migration preflight
  -> 已有文件且有待迁移工作时创建并验证全库备份
  -> migration 1：legacy baseline
  -> orphan 盘点 / 审计 / 修复
  -> migration 2：owned relation FK rebuild
  -> foreign_key_check
  -> 内置数据 seed
```

`schema_migrations(version, checksum, applied_at)` 记录连续且不可变的迁移。启动在写入前检查 descriptor 连续性、数据库版本断档、未来版本和 SHA-256 checksum。Migration 1 冻结 `v0.10.9` 至 `v0.10.14` 的 legacy 归一化逻辑；migration 2 重建七张 skill-owned relation 表，加入 `FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE`。Observation、project snapshot、调用历史和 usage identity cache 保持独立生命周期。

任何 repair 或 migration 写入之前，有待升级的已有数据库先通过绑定路径的 `VACUUM INTO` 生成一致快照，完成 integrity check、sync 后发布为同目录 `*.pre-migration-v<source>-*.sqlite3`。升级失败会关闭私有 pool、隔离失败文件、复制备份恢复并再次校验，但本次启动仍返回失败。

## Repositories

`db/repos/` 一个逻辑对象一个 repo：

| Repo | 表 |
| --- | --- |
| `skills_repo` | `skills` |
| `installations_repo` | `skill_installations` |
| `observations_repo` | `agent_skill_observations` |
| `agents_repo` | `agents` |
| `collections_repo` | `collections`、`collection_skills` |
| `repositories_repo` | `skill_repositories`、`skill_repository_members` |
| `update_states_repo` | `skill_update_states` |
| `tags_repo` | `skill_tags`、`skill_tag_links`、`skill_ai_tag_reviews` |
| `projects_repo` | `projects`、`project_skill_installations` |
| `scan_dirs_repo` | `scan_directories` |
| `settings_repo` | `settings` |
| `operation_logs_repo` | `operation_logs` |

Repo 收口原始 `sqlx::query()`，上层只接受 `&DbPool` 调 repo 方法。

## 字段参考

字段细节由 `scripts/docs/build-schema-table.mjs` 从 `src-tauri/src/db/schema/*.rs` 扫描生成。

<!--@include: ../../architecture/_generated/data-model.md-->

## 迁移契约

- 已发布 migration source 与 checksum 不可修改；后续 schema/data 变化必须新增连续版本。
- 每个 migration 与自己的 `schema_migrations` 行在同一事务提交；table rebuild 必须带行数守卫与 `foreign_key_check`。
- desktop、`skillport-cli`、SSH cache、WSL cache 统一调用 `open_database*`，生产代码禁止自行组合 raw pool 与 init。
- 旧二进制遇到未来版本必须阻断，不做 downgrade；保留的 pre-migration snapshot 是回滚介质。

Last reviewed: 2026-07-26
