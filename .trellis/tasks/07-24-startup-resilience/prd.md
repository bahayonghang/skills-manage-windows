# 启动健壮性：去除启动路径 expect panic

## Goal

DB 损坏、目录权限、磁盘满或 schema 初始化失败时，桌面应用不再 panic 或静默退出；主窗口进入完整的前端启动恢复页，向用户提供安全的重试、备份并重建和退出路径。对应审计 P2-07。

## Background

- `src-tauri/src/lib.rs:230-268` 在文件日志启用后，仍以 `fs::create_dir_all(...).expect(...)` 和 `db::open_database(...).expect(...)` 处理目录、数据库打开与迁移失败；任一失败都会 panic，窗口无法进入可恢复状态。
- `src-tauri/src/db/migrations/backup.rs` 已提供 `PRAGMA integrity_check`、迁移前原子备份、失败数据库隔离与恢复原语；启动恢复必须复用或下沉这些机制，不能另建语义冲突的 SQLite 备份协议。
- `src/main.tsx` 在 React 渲染前加载主题、字体偏好与运行日志；`src/components/layout/AppShell.tsx` 挂载后才启动依赖 DB 的 target/platform store，因此可在 `AppShell` 之前建立不依赖 DB 的启动门。
- Rust 已启用 `tauri-plugin-process`。用户于 2026-07-27 选择完整前端恢复页，不采用仅原生错误对话框的降级体验。

## Requirements

1. 启动状态必须是结构化且可序列化的显式状态机，至少包含 `checking`、`ready`、`recovery_required`、`fatal`；IPC 不得返回原始路径、SQL、凭据或未脱敏内部错误。
2. 数据目录不可创建或不可写时进入 `fatal`，显示稳定的失败类别、建议动作、重试与退出；不得调用任何依赖 `AppState`/DB 的命令。
3. DB 打开、完整性或 schema 初始化失败时进入 `recovery_required`。后台执行 `PRAGMA integrity_check` 类诊断并写 Runtime Log，前端显示本地化的错误类别与恢复说明。
4. “重试”只重新执行原启动初始化，不改写现有数据库；失败时保持恢复页并刷新安全诊断状态。
5. “备份并重建”必须先把 `db.sqlite` 及存在的 WAL/SHM companion 作为一个恢复集合移动到唯一、不可覆盖的备份位置；所有移动成功后才创建新数据库。部分移动失败必须回滚或明确 fail closed，禁止静默丢弃旧文件。
6. 重建成功后在同一进程内安装现有 `AppState`、启动正常后台任务并进入主应用；并发重试/重建必须串行化，不能重复安装状态或重复执行后台迁移。
7. 前端启动恢复行为由 Zustand store 持有；组件不得直接调用 Tauri `invoke`。浏览器演示态必须注册 `ready` fixture，不能因新增启动门失效。
8. 恢复页使用现有设计 token、Lucide 图标和中英文 i18n；加载、可恢复失败、不可恢复失败、操作中和二次失败均有可访问、无重叠的明确状态。
9. 健康启动路径不增加第二次 DB 打开、完整性扫描或文件复制，只增加一次本地启动状态 IPC；现有启动后恢复、secret 迁移和 legacy Central migration 仍只在 ready 后运行一次。

## Acceptance Criteria

- [ ] 目录不可创建、损坏 DB、schema 初始化失败三类故障注入测试证明应用初始化不 panic，并分别产生 `fatal` / `recovery_required` 的稳定安全状态。
- [ ] 完整性诊断结果进入 Runtime Log，但启动状态 IPC 和前端 DOM 不含绝对 DB 路径或原始 SQLx 错误。
- [ ] 损坏 DB、WAL、SHM 在重建前被保留；故障注入覆盖部分移动失败且证明不会启动到新旧混合状态。
- [ ] 重试不改写旧 DB；备份并重建成功后状态变为 `ready`，主应用初始化一次且备份仍可恢复。
- [ ] 前端测试覆盖 loading、ready、recovery、fatal、重试失败、重建成功、操作防重入和浏览器 fixture；错误页出现时 `AppShell` 不挂载。
- [ ] 健康路径只打开一次 DB、只启动一次 ready 后任务；Tauri 开发态冷启动 smoke 无可感知回归。
- [ ] `cargo fmt --all -- --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked`、`pnpm typecheck`、`pnpm lint`、相关 Vitest 与 `just ci` 全部通过。

## Out Of Scope

- 不做完整 health diagnostics 面板（L-04）。
- 不提供任意文件浏览、手工选择 DB 或导入备份的 UI。
- 不改变 `Result<T, String>` 的 commands 层错误契约；Typed IPC 全面迁移由 `07-24-typed-ipc-migration` 负责。
- 不处理日志初始化自身失败的恢复 UI，也不引入新的生产依赖。

## Dependencies And Deferred Items

- 复用已归档 `07-24-db-schema-versioning-fk` 的版本化迁移与备份契约。
- 启动恢复页稳定后，再由后续 health diagnostics 工作扩展日志导出或高级修复入口。
