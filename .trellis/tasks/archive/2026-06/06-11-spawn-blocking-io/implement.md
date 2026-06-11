# Implement：重 IO 路径 spawn_blocking 改造（子任务 A）

技术模式见父任务 `design.md` 第 2 节。前置：D（06-11-eng-hygiene-quickfixes）已归档。

## 执行清单

1. [x] 提升 `services/installation/fs_util.rs` 的 spawn_blocking 包装为跨域共享模块（`src-tauri/src/fs_util.rs`），installation 原路径 re-export → 验证：`cargo test` 绿，installation 测试不变。
2. [x] 改造 `commands/central_updates_fs.rs`（17 处）→ 验证：该模块相关测试绿（hash_directories / write_skill_dir_atomic / refresh_copy_install 三个 Local 分支全部经 run_blocking_fs）。
3. [x] 改造 `services/github_import/import.rs`（15 处）→ 验证：github_import tests.rs 绿（批量落盘 write_snapshot_source_to_target 转 async + 逐文件 spawn_blocking；staging/backup 递归删除转 async 包装；单文件 SKILL.md 读取与 rename 保持同步豁免）。
4. [x] 改造 `commands/central_store_location.rs`（10 处，含递归搬迁）→ 验证：4 个模块测试绿；preview 扫描、搬迁拷贝循环（含 create_dir_all + 逐 skill 覆盖）、symlink 重建逐行包装，失败恢复语义逐行比对未变（错误中断点、failures 收集、DB 更新顺序一致）。
5. [x] 改造 `services/projects/crud.rs`（10 处）→ 验证：projects tests.rs 绿（remove_project_impl 删除循环整体包装，吞错+log 语义不变；install/uninstall 链路此前已包装）。
6. [x] 改造 `services/central_skills/delete.rs` + `files.rs`（共 12 处）→ 验证：central_skills tests.rs 绿（delete 的安装清理循环 + 中央目录递归删除整体包装；files 的内容读取/文件读取/递归目录树 Local 分支包装）。
7. [x] 逐项评估清单（11 个文件）：结论见下方附录。
8. [x] 全量验证：`cargo test` 697 passed / 2 ignored；`cargo clippy -- -D warnings` 零警告；`pnpm typecheck` + `pnpm lint` 绿。`just ci` 中 web:test 有 1 个**预先存在**的失败用例（`CentralSkillsView.github-import-error.test.tsx`，已在干净 HEAD 上复现，与本任务 Rust-only 改动无关）。

## 风险文件与回滚

- 风险最高：`central_store_location.rs`（目录搬迁含中途失败恢复逻辑），改造时注意闭包捕获导致的所有权调整不得改变失败恢复语义。
- 回滚：单 commit revert。

## 启动前检查

- [x] D 已归档，工作区干净，基于最新 dev 分支。

## 附录：评估结论（执行时填写）

| 文件 | 结论 | 理由 |
| ---- | ---- | ---- |
| `commands/skill_update_inventory/force.rs` | 豁免 | 仅 1 处单次 `create_dir_all`；重 IO 全部经 CentralFs / github_import 路径（本任务已包装） |
| `services/local_remote_sync.rs` | 改造 | `collect_repo_snapshot` / `collect_skill_snapshots` 为递归遍历 + 全量读文件，已在 `build_sync_plan` 中包装 |
| `services/ai_tagging/prompt.rs` | 豁免 | 每次 AI 请求仅 1 次小文件 SKILL.md 读取，耗时被网络调用主导 |
| `services/installation/centralize.rs` | 豁免（已合规） | 全部 fs 路径此前已走 `run_blocking_fs` / `copy_dir_all_blocking`；`ensure_replaceable_target_sync` 仅在包装内调用 |
| `services/installation/native.rs` | 豁免（已合规） | 目录创建/卸载删除已包装；剩余 `canonicalize` 为单次元数据解析 |
| `services/installation/project.rs` | 豁免（已合规） | 目录检查/创建/symlink/copy 全部已包装 |
| `services/installation/skip.rs` | 豁免（已合规） | 存在性/symlink/递归内容比对检测全部已包装 |
| `services/marketplace/skills_sh.rs` | 豁免 | 仅 1 处单次 `create_dir_all`；导入重 IO 委托 github_import（本任务已包装） |
| `services/obsidian/import.rs` | 豁免（已合规） | 检查/创建/symlink/copy 已包装；仅剩 `parse_skill_md` 单次小文件读取 |
| `services/obsidian/query.rs` | 改造 | vault 发现 + 多目录扫描（registry 读取、fallback roots 一层遍历、每 vault 3 个 skills 目录批量解析 SKILL.md），两个 impl 整体包装 |
| `services/usage/fs_backend.rs` | 改造 | LocalFsBackend 的 `walk_jsonl`（walkdir 递归）、`read_many_to_strings`（批量读，jsonl 可达 MB 级）、`read_to_string`、`list_entries` 包装；Remote `fetch_to_local` 的临时文件写入（SQLite 可达 MB 级）包装；`exists` 单次元数据探测豁免 |

### 执行备注

- 共享包装位于 `src-tauri/src/fs_util.rs`（`run_blocking_fs`），`services/installation/fs_util.rs` 原路径 `pub(crate) use` re-export 保持兼容，未引入第二种包装模式。
- **Windows 测试二进制坑**：blocking 闭包内**不得按值持有 `AppHandle`**（含 `Option<AppHandle>`）。AppHandle 的 drop-glue 会把 tauri/muda 的菜单与对话框代码链入测试二进制，引入 `comctl32.dll!TaskDialogIndirect` 导入；测试二进制无 comctl32 v6 manifest，进程加载直接 `STATUS_ENTRYPOINT_NOT_FOUND`。progress 发射保留在 async 侧（按引用持有 AppHandle），`progress.rs` 内有注释说明。
