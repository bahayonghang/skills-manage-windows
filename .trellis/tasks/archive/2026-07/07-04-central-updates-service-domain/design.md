# Design：Update Center 落 service 域

> 前置：`prd.md`。本文档裁决 PRD 留白的四件事：域切分、错误枚举形状、State 解耦方式、分阶段迁移与回滚点。
> 证据基线（2026-07-04 本任务实测）：非测试代码 **6,611 行**（central_updates.rs 1212 + repository_sync 683+156 + central_updates_fs 783+87 + central_store_location 681 + skill_update_inventory 698+2398），测试 **75 条**（central_updates 15 + fs 7 + inventory 49 + store_location 内联 4）。17 个 IPC 命令。域外引用仅 lib.rs 注册宏 + fs_util.rs 一行 doc 注释——域自封闭，迁移无涟漪。

## 1. 现状结构速览

| 文件                                                        | 行数 | 内容                                                                                             |
| ----------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------ |
| `commands/central_updates.rs`                               | 1212 | 5 命令 + 更新编排内核（prepare/snapshot/state 机/update_one/copy 刷新/进度发射）                 |
| `commands/central_updates/repository_sync.rs`（+summaries） | 839  | 2 命令（已 deprecated）+ remote-added 收集 + per-repo 汇总                                       |
| `commands/central_updates_fs.rs`（+remote_scripts）         | 870  | `CentralFs` Local/SSH 双模文件系统 façade + 哈希/原子写/copy 刷新 + `run_blocking_fs` 字符串包装 |
| `commands/skill_update_inventory.rs` + 9 子模块             | 3096 | 8 命令（P2 统一面板）：refresh/get/clear/apply/force×2/scan×2                                    |
| `commands/central_store_location.rs`                        | 681  | 2 命令：中央仓库位置迁移 preview/apply（独立能力）                                               |

关键耦合事实：

- inventory 与 repository_sync 大量复用 central_updates 内核（`prepare_skill_updates`、`load_remote_skill_content`、4 个 state 构造器、`collect_remote_added_skills`、`CentralFs`）——**三者是同一个业务能力**（更新中心），拆域会制造巨大的跨域 pub 面。
- `apply_skill_update_decisions` 直接调用旧命令 `update_central_skills(app.clone(), state.clone(), …)`——命令套命令，是 State 耦合的病灶。
- `central_store_location` 只与更新中心共享 `run_blocking_fs` 字符串包装，业务、错误词汇（`central_store_location_*` 哨兵码）、生命周期完全独立。
- `CentralUpdateSnapshotCache` 定义在 lib.rs，是更新中心私有的缓存类型（AppState 持有）。
- `commands/github_import.rs` 已是纯壳（业务在 `services/github_import`），是本次迁移的形状模板；所有 `commands::github_import::X` 引用实为 service 类型 re-export，迁移后直接 `use crate::services::github_import::X`。

## 2. 决策

### D1 域切分：两域 —— `services/central_updates` + `services/central_store_location`

- **`services/central_updates`** 承接 check/update/cancel、repository_sync、skill_update_inventory（含 force/scan）与 `CentralFs` façade。`skill_update_inventory` 归属该域的 `inventory/` 子模块：它是同一能力的 P2 门面，复用内核 80%，独立成域只会让内核全部 pub 化。
- **`services/central_store_location`** 独立小域：仓库位置迁移。与更新中心零业务耦合，错误词汇独立（哨兵码直出前端 i18n 匹配）。
- **否决**「单一大域含 store_location」：迁移域塞进更新域会让 `CentralUpdatesError` 混入 5 个哨兵码变体，两套错误词汇互相污染。
- **否决**「inventory 独立成域」：`prepare_skill_updates`/`RemoteSkillLoadError`/state 构造器全要跨域导出，interface 面暴涨，违背 deep module 目标。

### D2 错误枚举：`CentralUpdatesError`（~40 变体）+ `CentralStoreLocationError`（~14 变体）

遵循 `.trellis/spec/backend/domain-error-enums.md` 骨架（Io{context,source} / Db(#[from] sqlx) / Remote(String) / TaskJoin + 语义变体，文案逐字保留）。本域特有裁决：

- **`RemoteSkillLoadError`（RemoteMissing/Other）原样保留**：它不是域错误，是数据流分类器——其 String 载荷最终落库为 `SkillUpdateState.error`。域错误在其上游（如 `collect_remote_skill_files` 返回 `CentralUpdatesError`，调用点 `.map_err(|e| RemoteSkillLoadError::remote_missing(e.to_string()))`）。
- **数据侧错误字段保持 String**（spec §3）：`SkillUpdateState.error`、`SkillUpdateApplyFailure.error`、`CentralRepositorySyncFailure.error`、`ForceSkillUpdateFailure.error`、`CentralStoreLocationSymlinkFailure.error` 等 partial-success 载荷，构造点 `e.to_string()`。
- **跨域透传**：`#[error(transparent)] GithubImport(#[from] GithubImportError)`、`Installation(#[from] InstallationError)`、`CentralSkills(#[from] CentralSkillsError)`；store_location 另有 `Scanner(#[from] ScannerError)`。targets 传输错误照例调用点 `.to_string()` 入 `Remote(String)`。
- **serde_json**：`#[error("{0}")] Json(String)`，调用点 map（persistence.rs 的 payload 序列化）。
- `CentralStoreLocationError` 哨兵码变体 Display 必须逐字等于现值：`central_store_location_requires_overwrite` / `_empty_path` / `_same_path` / `_nested_path` / `_unsupported_target`（`ensure_local_target` 检查随迁入 service，接 `&ActiveTarget`）。

### D3 State 解耦：impl 显式收参，杀掉命令套命令

服务函数签名统一收显式依赖，不见 `State<AppState>` / `AppHandle`（按值）：

```rust
// 新抽取（现为命令体内联逻辑）：
check_central_skill_updates_impl(app: Option<&AppHandle>, pool, fs, cancel: &AtomicBool,
    auth: Option<&str>, client, snapshots_cache, skill_ids) -> Result<Vec<SkillUpdateState>, CentralUpdatesError>
update_central_skills_impl(app: Option<&AppHandle>, pool, fs, cancel: &AtomicBool,
    auth, client, snapshots_cache, skill_ids) -> Result<CentralSkillUpdateResult, CentralUpdatesError>
check_central_repository_sync_impl(...同上 + repository_ids, skill_ids)
apply_central_repository_sync_impl(app: Option<&AppHandle>, pool, active_target, auth, decisions)
apply_skill_update_decisions_impl(app: Option<&AppHandle>, pool, active_target, fs, cancel,
    auth, client, snapshots_cache, decisions)   // ← 内部改调 update_central_skills_impl，命令套命令消失
```

- 进度发射走 `emit_update_progress(app: Option<&AppHandle>, …)`，留在 async 侧（先例：`services/github_import/progress.rs`；`force_mirror_central_repositories_impl` 已是 `Option<&AppHandle>` 形状）。blocking 闭包只接纯 fs 工作（spec spawn-blocking-io Windows 坑）。
- 取消旗语传 `&AtomicBool`（壳层传 `&state.central_update_cancel`）；重置/轮询语义逐行保持。
- 已有 `*_impl`（inventory refresh/get/clear、force×2、scan×2、keep_remote_missing、store_location preview/apply）只改错误类型，签名结构不动。

### D4 `CentralFs` façade 与字符串包装退役

- `central_updates_fs.rs` 整体迁为 `services/central_updates/fs.rs`（+`fs/remote_scripts.rs`），方法改返 `CentralUpdatesError`。它本就自述为「typed service domains 的 commands 边界对应物」——迁移即归位。
- `run_blocking_fs`（字符串版包装）**退役**：两个消费者（本域 + store_location）都改用 `crate::fs_util::run_blocking_fs_with(label, task, XxxError::task_join)`。fs_util.rs doc 里提及 commands 包装的那句同步删除。
- `commands/central_updates_fs.rs` 文件删除；scan.rs 单点 `symlink_metadata`、force.rs 单点 `create_dir_all` 属 spec 豁免的单文件小操作，随逻辑迁入 services 原样保留（迁移是行为保持型，不新增包装）。

### D5 `CentralUpdateSnapshotCache` 归位域内

类型移到 `services/central_updates/snapshots.rs`，lib.rs 改 `pub use services::central_updates::CentralUpdateSnapshotCache;`——AppState 字段类型与全部 `crate::CentralUpdateSnapshotCache` 引用零改动，域拥有自己的缓存。

### D6 壳层终态与 lib.rs 路径

- `commands/central_updates.rs`：7 个壳（5 + repo-sync 2，子目录拍平），`#[deprecated]` 注记保留在壳上（IPC 面语义），service impl 不带 deprecated —— 壳直调 impl 后，现存 `#[allow(deprecated)]` 内部调用点全部消失。
- `commands/skill_update_inventory.rs`：8 个壳；`commands/central_store_location.rs`：2 个壳。
- lib.rs 注册宏里 `commands::central_updates::repository_sync::{check,apply}_central_repository_sync` 两条路径改为拍平后的新路径（**IPC 命令名不变**，宏取 fn 名）。

## 3. 目标布局（文件 1:1 映射，diff 最小化）

```
services/central_updates/
  mod.rs                  ── 域 doc + 子模块声明 + pub 面收口
  error.rs                ── CentralUpdatesError
  types.rs                ── SkillUpdateStatus、进度/结果/失败/跳过载荷、PreparedSkillUpdate、
                             RemoteSkillContent、UpdateCounters、RemoteSkillLoadError、
                             SnapshotCachePolicy、GitHubUpdateSource（central_updates.rs 头部拆出）
  snapshots.rs            ── CentralUpdateSnapshotCache（自 lib.rs）+ prepare_snapshots* + repo_cache_key + TTL
  core.rs                 ── central_updates.rs 主体其余：load_selected/prepare_skill_updates/
                             load_remote_skill_content/state 构造器/update_one_skill*/refresh_copies/
                             keep_remote_missing_impl/check_impl/update_impl/emit_update_progress
  fs.rs + fs/remote_scripts.rs + fs/tests.rs      ── 自 central_updates_fs.rs（含 7 测）
  repository_sync.rs + repository_sync/summaries.rs ── 自 commands 同名（抽 2 个 impl）
  inventory/{mod,apply_steps,force,persistence,relocation,repositories,scan,scope,types,view,tests}.rs
                          ── 自 commands/skill_update_inventory（mod.rs 收 refresh/apply 等 impl；49 测）
  tests.rs                ── 自 commands/central_updates/tests.rs（15 测）

services/central_store_location/
  mod.rs + error.rs + tests.rs   ── 自 commands/central_store_location.rs（4 测）
```

> core.rs 约 900 行——刻意不再细拆：内部函数互相咬合紧密（prepare→snapshot→load→state），拆文件只会制造 pub(super) 网；「不按文件大小机械拆分」是父任务红线。

## 4. 分阶段迁移与回滚点（每阶段独立提交 = 回滚点，全程 `cargo test` 绿）

| 阶段  | 内容                                                                                                                                                                                                                                                   | 门禁                                                     |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| **A** | `services/central_store_location`（域最小、零耦合，先证模式）：error.rs + 逻辑迁移 + 壳收缩 + 4 测随迁（断言改 `.to_string()` 比对）                                                                                                                   | `cargo test central_store_location` + 全量 test + clippy |
| **B** | `services/central_updates` 骨架：error/types/snapshots(含缓存搬家)/core/fs 迁移 + 5 壳收缩 + check/update impl 抽取 + interim：repository_sync 与 inventory（仍在 commands）的 `use` 行改指 services + 删 `commands/central_updates_fs.rs` + 22 测随迁 | 同上（scanner 域回归 + 全量）                            |
| **C** | repository_sync 迁移：抽 check/apply 两 impl，壳拍平进 `commands/central_updates.rs`，lib.rs 两条路径更新                                                                                                                                              | 全量 test + clippy                                       |
| **D** | inventory 迁移：9 子模块 + apply impl 抽取（改调 `update_central_skills_impl`）+ 8 壳收缩 + 49 测随迁                                                                                                                                                  | 全量 test + clippy                                       |
| **E** | 收尾审计：AC grep 门禁（见 §6）、fs_util.rs doc 修句、spec 登记、`just ci`                                                                                                                                                                             | 全部 AC                                                  |

依赖说明：B 先于 C/D（内核是被复用方）；A 独立可先行。C、D 之间无序（D 依赖 B 的 impl 抽取即可），按 C→D 执行以缩小单阶段 diff。

## 5. 测试迁移与断言调整清单

- 75 条测试全部随逻辑平移（文件级搬家 + import 改路径），**不允许净减少**。
- 断言调整仅一类：原 `Result<_, String>` 的 `unwrap_err()` 直接字符串比对 → 改 `unwrap_err().to_string()` 比对（文案逐字不变）。涉及：store_location 3 处哨兵码断言、central_updates/inventory 各 err 文本断言若干——逐条在提交说明列出。
- 测试基建继续用 `crate::test_support`（上一任务产物），`mem_pool`/`mem_pool_with_home`/`central_skill_row` 均可直用。

## 6. 验收映射（对 PRD AC）

| PRD AC                                     | 落点                                                                                                                                                                                                |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| commands 无 `sqlx::query` / `std::fs` 直调 | `grep -n "sqlx::query\|std::fs" src-tauri/src/commands/central_updates.rs src-tauri/src/commands/central_store_location.rs src-tauri/src/commands/skill_update_inventory.rs` → 0 命中（文件仅剩壳） |
| services 域无 `Result<T, String>`          | `grep -rn "Result<.*, String>" src-tauri/src/services/central_updates src-tauri/src/services/central_store_location \| grep -v tests` → 0 命中                                                      |
| 全量测试 + clippy                          | `cd src-tauri && cargo test`（739+ 基线不减）+ `cargo clippy -- -D warnings`                                                                                                                        |
| 前端零改动                                 | `git status` 不含 src/；IPC 命令名/参数/返回结构不动（壳层只译参）                                                                                                                                  |

行为保持红线：GitHub 请求路径（auth/client/snapshot 下载并发=4）、事件名 `central://skill-update-progress` 与载荷、取消旗语语义、`#[deprecated]` note 文本、全部错误文案——逐字不变。
