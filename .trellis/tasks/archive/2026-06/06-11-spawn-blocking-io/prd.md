# PRD：重 IO 路径 spawn_blocking 改造（子任务 A）

> 父任务：`06-11-analysis-driven-fixes` ｜ 执行顺序：第 2 位
> 依赖：建议在 D（工程卫生快修包）之后执行；必须在 C 批次（thiserror）之前完成，避免函数签名改动叠加冲突。
> 来源：分析报告条目 #1

## Goal

消除 async 上下文中的同步文件 IO 阻塞：17 个文件的 async fn 直接调用 `std::fs`，其中递归拷贝/删除/搬迁类操作在大技能库下会阻塞 Tauri runtime 工作线程，表现为 UI 整体卡顿。

## Requirements

1. 复用 `services/installation/fs_util.rs:14` 的 `spawn_blocking` 包装模式（必要时将该包装提升为跨域共享工具，如 `src-tauri/src/fs_util.rs` 或 `services/fs_util.rs`）。
2. **必改（重 IO：递归拷贝/删除/搬迁/批量落盘）**：
   - `commands/central_updates_fs.rs`（8 个 async fn / 17 处同步 fs）
   - `services/github_import/import.rs`（10 / 15）
   - `commands/central_store_location.rs`（9 / 10，中央目录搬迁含递归拷贝）
   - `services/projects/crud.rs`（10 / 10）
   - `services/central_skills/delete.rs`（17 / 6）
   - `services/central_skills/files.rs`（8 / 6）
3. **逐项评估（单文件小读写，可保持现状但需在 PR 说明中记录评估结论）**：
   `skill_update_inventory/force.rs`、`services/local_remote_sync.rs`、`services/ai_tagging/prompt.rs`、`services/installation/centralize.rs`、`services/installation/native.rs`、`services/installation/project.rs`、`services/installation/skip.rs`、`services/marketplace/skills_sh.rs`、`services/obsidian/import.rs`、`services/obsidian/query.rs`、`services/usage/fs_backend.rs`。
4. 不改变任何 IPC 命令的对外行为与错误消息格式（本任务不动错误类型，留给 C 批次）。

## Acceptance Criteria

- [ ] 「必改」清单中 6 个文件的重 IO 路径全部经 `spawn_blocking`（或等效异步包装）执行，代码 review 可逐函数核对。
- [ ] 「逐项评估」清单每个文件有明确结论（改造 / 保持现状 + 理由）。
- [ ] `cd src-tauri && cargo test` 全绿（709+ 测试）；`cargo clippy -- -D warnings` 零警告。
- [ ] `just ci` 全绿。
- [ ] 手动冒烟：技能扫描、安装/卸载、中央目录操作、GitHub 导入各执行一次无回归。
