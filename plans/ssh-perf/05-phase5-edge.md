# 阶段 5 — 边角优化

## 目标

把剩下的小石头一并搬走。各任务独立、可单独 PR。

## 任务清单

### 5.1 Bootstrap SQL 并发

文件：`src-tauri/src/commands/bootstrap.rs` `get_dashboard_central_summary_impl`

现状：5+ 次串行 `SELECT COUNT(*)`，每次约 1-5ms（本地 SQLite），总计 10-30ms。

改造：

```text
let (central, updates, ai, uncategorized, unassigned) = tokio::try_join!(
    get_count(pool, SQL_CENTRAL),
    get_count(pool, SQL_UPDATES),
    get_count(pool, SQL_AI),
    get_count(pool, SQL_UNCATEGORIZED),
    get_count(pool, SQL_UNASSIGNED),
)?;
```

注意：SQLite 默认串行，并发查询取决于 connection pool size。check `src-tauri/src/db/pool.rs` 的 max_connections，必要时 ≥ 5。

### 5.2 scan_ssh_directory 内合并 exists+read

阶段 2 已经从根本上重写，本任务在阶段 2 之后做：保留代码里如果还有路径用旧的 exists + read_file 串调，合并为单次 cat 容错（cat 失败 = 文件缺失）。

### 5.3 评估默认启用的 agent 数量

`src-tauri/src/db/seed.rs` 中默认 27 个内置 agent。截图右栏 `Enabled platforms · 0 of 0` 说明远端 dckj 上没有任何 agent 目录被探测到，但前端列表仍可能列出全部。

要做：
- 添加 settings 项 `default_enabled_agents`（id 列表）
- 默认仅启用 claude-code / codex / openclaw / central，其他默认 disabled
- 用户可在设置页勾选

减少扫描默认范围 = 减少远端 SSH 工作量。**这是配置优化，不是删 agent**。

### 5.4 SFTP 评估

阶段 3 用 channel exec + bash 批读。如果实测 SFTP 在批读小文件时更快（少一层 shell 解析），切换。

预测：100 个 SKILL.md（每个 < 10KB）批读：
- bash cat：1 channel + 1 脚本，O(1) 控制开销
- SFTP：N 个 file open/read 调用，每个 1 round-trip，但 russh-sftp 支持 pipeline

实测决定。

### 5.5 操作日志检索性能

`operation_logs` 表随时间膨胀，前端 dashboard 显示 recent logs 用 `LIMIT N`，但表无 created_at 索引时全表扫描。

`src-tauri/src/db/schema/core.rs` 检查现有索引，缺 `(created_at DESC)` 加上。

### 5.6 前端 list virtualization 复核

`src/components/ui/virtualized-list.tsx` 已存在。检查 Central Skills、Discover、Marketplace 三个长列表是否都接入；没有的接入。

## 文件改动清单

```text
src-tauri/src/commands/bootstrap.rs        SQL 并发
src-tauri/src/db/pool.rs                   max_connections 调整
src-tauri/src/db/seed.rs                   默认启用列表收窄
src-tauri/src/db/schema/core.rs / migrations.rs  索引补充
src/components/settings/PlatformVisibilitySettingsSection.tsx  默认勾选行为
src-tauri/src/db/migrations.rs             加 idx_operation_logs_created_at
+视情况：列表 virtualization 接入
```

## 风险

| 风险                          | 缓解                                       |
|-------------------------------|--------------------------------------------|
| SQLite 并发查询连接不足        | pool size ≥ 5；测连接耗尽行为              |
| 缩减默认 agent 影响升级用户    | 仅影响新装；老用户配置不动                  |
| 索引迁移在大表上慢             | 加索引一次性 + 在 migration 里超时保护      |

## 估时

0.5-1 天。可拆成 6 个独立 micro-PR 也可打包。

## 验收

- bootstrap snapshot 调用耗时下降 ≥ 30%
- 默认装新用户首次启动只扫 5 个 agent 而非 27
- operation_logs 量到 10 万条仍能在 100ms 内拿到 recent 50
