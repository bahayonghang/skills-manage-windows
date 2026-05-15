# 架构总览

SkillPort 是一个三层桌面应用：React UI 通过 Tauri IPC 与 Rust 后端通信，后端写入本地 SQLite 文件，少量 HTTP 客户端访问 GitHub 与 AI 提供方。

## 分层图

```text
┌──────────────────────────────────────────────────────────────────┐
│ React 19 + TypeScript (src/)                                     │
│   路由（react-router v7）                                         │
│   页面（src/pages/*）                                             │
│   Stores（src/stores/* — Zustand）                                │
│   组件（src/components/*）                                        │
└──────────────┬───────────────────────────────────────────────────┘
               │  invoke() / 事件 listen()
┌──────────────▼───────────────────────────────────────────────────┐
│ Rust + Tauri v2（src-tauri/src/）                                 │
│   commands/*  薄 IPC 处理函数                                     │
│   services/*  业务逻辑（scanner / installation / projects）       │
│   targets/*   本地 + SSH 执行适配                                  │
│   db/*        sqlx 池 + schema + repos                            │
└──────────────┬─────────────────────────┬─────────────────────────┘
               │                         │
       ┌───────▼──────┐         ┌────────▼────────┐
       │ SQLite (WAL) │         │ HTTP（reqwest） │
       │ ~/.skillsmanage/db.sqlite │ GitHub + AI │
       └──────────────┘         └─────────────────┘
```

## 模块边界

| 层 | 职责 | 可访问 |
| --- | --- | --- |
| `src/pages/` | 路由级视图 | 仅 stores |
| `src/stores/` | UI 状态 + IPC 调用 | `invoke()` 和 Tauri 事件 |
| `src/components/` | 复用 UI | 通过 hook 访问 stores |
| `src-tauri/src/commands/` | `#[tauri::command]` 处理函数 | `services/*`、`db/*`、`targets/*` |
| `src-tauri/src/services/` | 纯业务逻辑 | `db/repos`、OS、HTTP |
| `src-tauri/src/targets/` | 本地 + SSH 执行 | OS、`ssh` 二进制 |
| `src-tauri/src/db/` | Schema + sqlx repos | SQLite 池 |

组件不直接调用 `invoke()`。Stores 独占 IPC 入口，测试 mock 只挂一层即可。

## 数据主路径

```text
[用户操作] ──► page ──► store action ──► invoke('xxx')
                                       │
                                       ▼
                               commands::xxx
                                       │
                                       ▼
                               services::xxx
                                       │
                                ┌──────┴──────┐
                                ▼             ▼
                          db::repos      OS / HTTP
```

两个分隔点保证后端可测：`commands::xxx` 保持薄壳，`services::xxx` 不依赖 Tauri 运行时；service 层借用 `&DbPool`，单测使用临时 SQLite 文件。

## 横切关注点

- **状态同步。** 启动时后端 emit `system://migration-progress`，UI 显示横幅，无需轮询。
- **操作日志。** 每次安装/卸载写入结构化 `operation_logs` 行，含 `level`、`target_kind`、`category`、`action`、`summary`。日志页通过 `commands::logs` 回读。
- **活动目标。** 所有命令优先解析 `AppState::active_db()`，SSH 模式落到远端 SQLite 缓存。

Last reviewed: 2026-05-04
