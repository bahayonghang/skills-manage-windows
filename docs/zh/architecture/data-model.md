# 数据模型

SQLite 是唯一的持久化层。`~/.skillsmanage/db.sqlite` 启动时以 WAL 模式打开，按增量方式迁移——不存在 Diesel 风格的 migration 目录。

## Schema 初始化顺序

```text
core         skills / skill_installations / agent_skill_observations / agents
 └─ collections    collections / collection_skills
    └─ metadata    repositories / update_states / tags / tag_links / ai_reviews
       └─ discovery   scan_directories
          └─ projects   projects / project_skill_installations
             └─ settings settings / operation_logs（+6 索引）
                └─ marketplace registries / skills / explanations（+8 ALTERs）
```

所有 `CREATE TABLE` 都带 `IF NOT EXISTS`，增量列通过 `migrations::ensure_column` 添加，跨版本幂等。

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

字段细节由 `scripts/build-schema-table.mjs` 从 `src-tauri/src/db/schema/*.rs` 扫描生成。

<!--@include: ../../architecture/_generated/data-model.md-->

## 迁移契约

- 新增列：在 schema 的 `init.rs` 加列，并追加一个 `migrations::ensure_column`，老库就地升级。
- 重命名：写一段 Rust 迁移做 copy + drop；SQLite 在某些发行版本上 rename column 不可靠。
- 删除列：UI 还在读的列绝不删。按发布周期降级：先停写 → 迁移读取方 → 下一个版本再删。

Last reviewed: 2026-05-04
