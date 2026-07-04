# Implement：Update Center 落 service 域

> 前置：`prd.md` + `design.md`。按阶段推进，每阶段结束跑门禁并独立提交（= 回滚点）。任何阶段门禁红 → 修复或 `git revert` 该阶段提交，不带病进入下一阶段。

## Phase A：`services/central_store_location`（先证模式）

- [x] A1 新建 `services/central_store_location/error.rs`：`CentralStoreLocationError`（哨兵码 5 变体 + Io/Db/TaskJoin + Scanner/Installation transparent + NotASymlink/InvalidSymlinkOwner/PathPrefix/CentralAgentNotFound），Display 逐字对齐现文案
- [x] A2 新建 `services/central_store_location/mod.rs`：迁入 `preview/apply_central_store_location_change_impl` 及全部私有 helper（validated_roots/normalize_local_root/is_nested_path/skill_dir_ids/update_central_root/rebuild_symlinks.../replace_symlink/remove_existing_path/update_symlink_row/stored_path_string + `ensure_local_target(&ActiveTarget)`），错误全部改typed；`run_blocking_fs` 改 `fs_util::run_blocking_fs_with(label, task, CentralStoreLocationError::task_join)`
- [x] A3 `services/mod.rs` 挂域；`commands/central_store_location.rs` 收缩为 2 壳（保留请求/响应结构体定义位置裁决：IPC 载荷类型随逻辑迁入 service `types` 区，壳 `pub use` 回原路径）
- [x] A4 4 条内联测试迁 `services/central_store_location/tests.rs`，哨兵码断言改 `.to_string()` 比对
- [x] A5 门禁：`cd src-tauri && cargo test central_store_location && cargo test && cargo clippy -- -D warnings`
- [x] A6 提交（refactor(central-store-location)）

## Phase B：`services/central_updates` 骨架 + core + fs

- [x] B1 `error.rs`：`CentralUpdatesError` 全量变体（design D2 清单），含 GithubImport/Installation/CentralSkills transparent、Remote(String)、Json(String)、TaskJoin
- [x] B2 `types.rs`：SkillUpdateStatus/进度载荷/Result·Failure·Skip/PreparedSkillUpdate/RemoteSkillContent/UpdateCounters/RemoteSkillLoadError/SnapshotCachePolicy/GitHubUpdateSource 平移
- [x] B3 `snapshots.rs`：`CentralUpdateSnapshotCache` 自 lib.rs 迁入（lib.rs 改 `pub use`）+ prepare_snapshots*/repo_cache_key/snapshot_cache_ttl，错误 typed
- [x] B4 `fs.rs` + `fs/remote_scripts.rs`：central_updates_fs.rs 平移，CentralFs 方法与自由函数改 `CentralUpdatesError`；删字符串版 `run_blocking_fs`
- [x] B5 `core.rs`：其余内核平移 + 抽取 `check_central_skill_updates_impl` / `update_central_skills_impl`（app: Option<&AppHandle>、cancel: &AtomicBool 显式收参）+ `emit_update_progress` 改 Option
- [x] B6 `commands/central_updates.rs` 收缩为 5 壳 + `pub mod repository_sync;` 保留 + interim `pub(crate) use` 面（repository_sync/inventory 仍在 commands 期间的编译桥）
- [x] B7 repository_sync.rs 与 skill_update_inventory/*（仍在 commands）`use` 行改指 `services::central_updates`；删除 `commands/central_updates_fs.rs`；fs_util.rs doc 修句
- [x] B8 测试迁移：`services/central_updates/tests.rs`（15）+ `fs/tests.rs`（7），import/断言调整
- [x] B9 门禁：`cargo test central_updates && cargo test && cargo clippy -- -D warnings`
- [x] B10 提交

## Phase C：repository_sync 归位

- [x] C1 `services/central_updates/repository_sync.rs`（+`repository_sync/summaries.rs`）平移；抽 `check_central_repository_sync_impl` / `apply_central_repository_sync_impl`；collect_remote_added_skills/load_syncable_github_repositories 错误 typed
- [x] C2 两壳拍平进 `commands/central_updates.rs`（`#[deprecated]` 保留），删 `commands/central_updates/` 目录；lib.rs 两条注册路径更新
- [x] C3 门禁 + 提交

## Phase D：inventory 归位

- [x] D1 `services/central_updates/inventory/` 9 子模块 + mod.rs（refresh/get/clear impl）平移，错误 typed
- [x] D2 抽 `apply_skill_update_decisions_impl`：步骤 5 改调 `update_central_skills_impl`（命令套命令消失）
- [x] D3 force.rs/apply_steps.rs 等内部 String 签名全部 typed（design D2 变体清单）
- [x] D4 `commands/skill_update_inventory.rs` 收缩为 8 壳，删子目录；B6 的 interim `pub(crate) use` 桥拆除
- [x] D5 49 条测试迁 `services/central_updates/inventory/tests.rs`
- [x] D6 门禁 + 提交

## Phase E：收尾审计

- [x] E1 AC grep 三连（design §6）：commands 无 sqlx/std::fs；services 两域无 `Result<_, String>`；`grep -rn "central_updates_fs" src-tauri/src` 仅剩 0 命中
- [x] E2 `pnpm test`（前端零改动旁证）+ `just ci`
- [x] E3 spec 登记：domain-error-enums.md 域清单追加两域；必要时补 seam 说明
- [ ] E4 提交 + 归档

## 回滚点

A6/B10/C3/D6 四个提交各自独立可 revert；B 是 C/D 的前置，revert B 须连带 revert C/D。

## 验证命令备忘

```bash
cd src-tauri && cargo test                      # 全量（739+2 基线不减）
cd src-tauri && cargo clippy -- -D warnings     # lint 门禁
grep -rn "Result<.*, String>" src-tauri/src/services/central_updates src-tauri/src/services/central_store_location | grep -v tests
grep -n "sqlx::query\|std::fs" src-tauri/src/commands/central_updates.rs src-tauri/src/commands/central_store_location.rs src-tauri/src/commands/skill_update_inventory.rs
pnpm test && just ci
```
