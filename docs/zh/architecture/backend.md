# 后端

Rust 端是 Tauri v2 应用，分为薄 IPC 处理、业务服务、目标适配、sqlx-SQLite 持久化四层。

## Crate 布局

```text
src-tauri/src/
├── lib.rs             Tauri builder + invoke_handler
├── main.rs            入口，调用 lib::run()
├── path_utils.rs      跨平台路径助手
├── paths.rs           稳定应用路径（~/.skillsmanage）
├── central_migration.rs  历史 ~/.agents/skills → 私有库迁移
├── commands/          按域分组的 IPC 处理函数
├── services/          纯业务逻辑
├── targets/           本地 + SSH 执行
└── db/                Pool + schema + repos
```

## AppState

`AppState`（`lib.rs`）通过 `tauri::State<AppState>` 共享：

| 字段 | 用途 |
| --- | --- |
| `db: DbPool` | 始终是本机 sqlite 池 |
| `targets: TargetRegistry` | 活动目标（Local / SSH）+ 远端池缓存 |
| `ai_tag_jobs: AiTagJobRegistry` | AI 标注任务的协作取消标志 |

`AppState::active_db()` 返回与当前目标匹配的池，无论本地还是 SSH，命令调用方式一致。

## Commands 层

每个 `#[tauri::command]` 都在 `src-tauri/src/commands/` 下；总注册表在 `lib.rs::run()` 的 `tauri::generate_handler!` 中：

| 文件 | 域 |
| --- | --- |
| `bootstrap.rs` | dashboard 冷启动快照 |
| `targets.rs` | 活动目标 + SSH 目标 CRUD |
| `logs.rs` | Operation Log list / get / clear / export + Runtime Log 诊断 |
| `scanner.rs` | 按需 `scan_all_skills` |
| `agents.rs` | 27 个内置 + 自定义 agent |
| `linker.rs` | 安装 / 卸载 / 批量安装 |
| `skills.rs` | 技能详情 / 内容 / 文件树 / 资源管理器 |
| `central_metadata.rs` | 仓库、标签、AI 标签建议 |
| `central_updates.rs` | 远端更新状态 + 批量应用 |
| `collections.rs` | 集合 CRUD + 导入导出 |
| `settings.rs` | 键值 + 扫描目录 + GitHub PAT |
| `discover.rs` | （0.10.x 移除——拆分为 `projects.rs` + `obsidian.rs`） |
| `projects.rs` | 项目 add / list / rename / pin / scan / install / uninstall / remove |
| `obsidian.rs` | Obsidian vault 扫描 + 源模式导入 |
| `github_import.rs` | 仓库预览 + 导入 + raw fetch |
| `marketplace.rs` | 源 + 缓存 + AI 解释 |
| `portable_state.rs` | SkillPort 状态导入导出 |

完整命令清单参见 [IPC 命令字典](./ipc-commands.md)。

## Services 层

业务逻辑在 `src-tauri/src/services/`，commands 保持薄壳：

```text
services/
├── scanner/             读盘 SKILL.md frontmatter
├── projects/            项目级 skill 管理（add / scan / install / uninstall）
├── obsidian/            Obsidian vault 扫描 + 源模式导入
├── installation/        centralize / native / project / remote / batch
├── central_skills/      中央仓库 query / delete / 文件树
├── github_import/       归档下载、预览工作区、原始 HTTP
├── marketplace/         源同步 + 缓存
└── ai_provider/         Claude + OpenAI 兼容流式
```

较大服务继续按职责拆文件，不堆积单文件 mod.rs。

## Targets

`targets/` 抽象本地与 SSH 执行：

| 文件 | 作用 |
| --- | --- |
| `model.rs` | 持久化的目标行 |
| `registry.rs` | 活动目标解析 + 远端池缓存 |
| `exec.rs` | 本地或 ssh 执行命令 |
| `cred.rs` | 加密密码存储 |
| `askpass.rs` | ssh 密码 helper |
| `commands.rs` | IPC 命令（重导出为 commands::targets） |

服务不写 `if remote {}` 分支，统一调用 `targets::exec`。

## 持久化

`db/` 拆分如下：

```text
db/
├── pool.rs             create_pool()，启用 WAL
├── types.rs            共享结构体
├── schema/             按业务域拆分的建表语句
├── migrations.rs       ensure_column 增量 ALTER
├── repos/              一个业务对象一个 repo
├── seed.rs             agent 注册表 seed
└── tests.rs            集成式 db 测试
```

参见[数据模型](./data-model.md)了解表布局。

## 日志与错误

- **操作日志。** `operation_logs` 长生命周期结构化行，显示在日志页的 Operation layer。
- **运行时日志。** `skillport-YYYY-MM-DD.log` 短生命周期日文件，由后端 tracing 与前端诊断写入，经白名单 IPC 读取 / 导出，并按保留周期清理。
- **错误。** 所有命令返回 `Result<T, String>`。服务把错误上下文留在内部，到 IPC 边界才坍塌为字符串。

Last reviewed: 2026-06-03
