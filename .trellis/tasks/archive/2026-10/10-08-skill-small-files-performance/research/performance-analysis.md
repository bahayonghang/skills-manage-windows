# 大量小文件生命周期性能分析

状态：`STATIC_ANALYSIS`。起点 `063c8d3236197a06ff17083c28a9738487c07510`；`dev` 工作树初始干净、领先 `origin/dev` 3 个提交。未下载真实仓库、未读写用户 Central/Platform 数据、未测运行时耗时。

## 1. 用户证据

截图提供 1 个 Skill、13,016 个文件和 `Writing files`，示例进度 `5054 / 13016 files`、`51%`。当前组件有字节计数时优先按字节计算百分比（`src/components/marketplace/githubImportWizardViewModel.ts:262`），文件数比例无需等于百分比。

用户报告的缓慢作为事实保留。截图无耗时、字节、磁盘或 copy 安装情况，应用二进制与源码身份未核对。下文指出源码成本，实际瓶颈原因未查明。

## 2. 生命周期与证据

### 获取与预览

`preview -> acquire_pinned_preview_snapshot -> TreeRaw/archive -> 内存 snapshot -> preview registry`。

- 固定 commit 获取并保留内容：`src-tauri/src/services/github_import/preview.rs:294`、`:300`、`:307`、`:323`。
- TreeRaw 并发 8、最多 64 个 raw 请求、选中内容最多 8 MiB：`src-tauri/src/services/github_import/tree_import.rs:10`。不能推断 13,016 个文件默认逐文件走 raw HTTP，实际 mode 需要计量。
- 资源预算：archive 文件上限 20,000、展开字节上限 256 MiB、copy 条目上限 20,000（`src-tauri/src/services/resource_budget.rs:7`）。fixture 需为目录条目保留余量。
- 本地 snapshot 持有字节；`snapshot_files_from_local` 对全内容 SHA-256 并排序清单（`src-tauri/src/services/github_import/snapshot.rs:89`）。网络、解包和内存哈希应分开计时。

### 确认导入

`snapshot_import -> binding/完整性校验 -> import_from_snapshot -> import_single_staged_skill -> journaled_central_content_upsert_with_fs -> update_skills_batch -> stage/swap -> DB -> finalize`。

- snapshot lease、成功消费 token、失败释放 lease 以及 cleanup ticket：`src-tauri/src/services/github_import/snapshot_import.rs:42`、`:63`、`:75`、`:119`。
- 确认重新遍历本地 snapshot 字节校验：`src-tauri/src/services/github_import/snapshot.rs:210`、`:215`。
- 每个 Skill 筛选并排序 snapshot 文件：`src-tauri/src/services/github_import/import.rs:95`、`src-tauri/src/services/github_import/progress.rs:19`。
- 路径检查循环逐文件发 `Writing`，增加 completed 文件/字节计数；真正写入调用在循环后：`src-tauri/src/services/github_import/import.rs:526`、`:532`、`:600`。
- emit 和 renderer 每事件 store set：`src-tauri/src/services/github_import/progress.rs:50`、`src/stores/marketplaceStore.githubImportHelpers.ts:39`、`:45`。
- import 逐 Skill 调单元素 update batch：`src-tauri/src/services/github_import/import.rs:130`、`src-tauri/src/services/central_updates/core/content_upsert.rs:57`。多 Skill 会重复顶层锁/selected recovery；单个大 Skill 只有一次服务调用，优化批量 Skill 调度不能代替单 Skill IO 优化。
- upsert 先读取已有目录哈希：`src-tauri/src/services/central_updates/core/content_upsert.rs:45`；收集文件把每个 bytes clone 到 RemoteSkillFile（`src-tauri/src/services/central_updates/fs.rs:120`、`:133`）。

事实：Local 每文件写入进度发生在磁盘写入前。该路径需要纠正完成计数，并可能产生随文件数增长的 IPC/store 成本。事件、WebView、磁盘耗时占比仍为待测假设。

### 导入与更新应用共用成本

- target guard 和 selected recovery：`src-tauri/src/services/central_updates/core/batch.rs:52`、`:98`。
- plan 字节列表 clone 到 write，prepared write clone 到 stage：同文件 `:261`、`:264`、`:154`、`:160`。
- Local stage 再 clone 字节进 blocking closure，按 Skill 顺序执行：`src-tauri/src/services/central_updates/fs/operation.rs:135`、`:137`、`:209`。
- 每文件顺序执行 `create_dir_all(parent)` 和 write，同父目录检查重复：`src-tauri/src/services/central_updates/fs.rs:174`、`:185`、`:191`。
- stage 写后完整读 staging 做哈希：`src-tauri/src/services/central_updates/fs/operation/helpers.rs:28`。
- swap 再读 staging；有旧 target 时再读旧目录，再同级 rename：同文件 `:38`、`:43`、`:53`、`:65`。
- finalize 读取新 target 与存在的旧 backup，再删除 backup/staging/marker：同文件 `:185`、`:201`、`:207`、`:221`。
- manifest 准备时读旧 target、哈希内存新文件：`src-tauri/src/services/central_updates/fs/operation.rs:101`、`:108`。
- local directory hash 递归列目录、逐文件完整读取、排序聚合：`src-tauri/src/services/central_updates/fs.rs:309`、`:505`、`:532`、`:545`。

正常成功路径成本模型（不含 recovery/重试）：

| 操作 | 整树写入 | 新内容整树磁盘哈希 | 旧内容整树磁盘哈希 | 其他成本 |
|---|---|---|---|---|
| 首次导入、target 不存在 | 1 | 3：stage、swap、finalize | 0 | 内存哈希、多次 bytes clone、逐文件进度 |
| 覆盖导入、backup 存在 | 1 | 3 | 4：upsert、manifest、swap、backup finalize | 旧 backup 递归删除 |
| 有变化的普通更新 | 1 | 3 | 3–4：manifest、swap、backup finalize，另有检查时 local hash | 每个 copy 完整复制和旧副本清理 |
| Central 删除真实目录 | 0 | 0 | 2：manifest、backup finalize | rename、全部文件物理删除 |

计数为源码推导，待 probes 核对；缓存、路径缺失、恢复和异常会改变次数。每轮内容校验约为文件数/字节数的线性成本，排序另有 `O(F log F)`；小文件重复读写会重复文件打开和元数据操作。

### 更新检查、无变化更新及 copy

- 已有共享 snapshot cache：10 分钟、最多 8 项/256 MiB、下载并发 4（`src-tauri/src/services/central_updates/snapshots/mod.rs:25`、`:35`）。不提出第二套仓库缓存。
- 已有受条件限制 fresh update-available state local hash 复用：`src-tauri/src/services/central_updates/core/state.rs:76`、`:99`。不能扩大成跨操作无校验缓存。
- 每 Skill 加载 clone 字节、算 remote_hash、算 candidate digest：同文件 `:130`、`:135`、`:137`。candidate digest 先重新哈希整个 snapshot，多 Skill 仓库可能重复全库哈希（`src-tauri/src/services/github_import/snapshot.rs:145`、`:149`）。
- 已有 `remote_hash == local_hash` 时跳过写入：`src-tauri/src/services/central_updates/core.rs:205`；比较前仍加载/哈希 remote bytes。
- Local copy refresh 删除旧目标后完整复制：`src-tauri/src/services/central_updates/fs.rs:279`、`:290`、`:293`；预算限制的递归复制见 `src-tauri/src/services/installation/fs_util.rs:270`、`:283`、`:321`。
- Remote durable write 已按 16 分块、copy 按 32 分块（`src-tauri/src/services/central_updates/fs/batch.rs:29`）。沿用生产 operation hooks；旧 atomic write API 仅用于测试。

### 删除、leftover、收尾与恢复

- manifest physical-path 去重并读取指纹，backup/marker 为同级 operation-owned 路径：`src-tauri/src/services/central_operation/fs.rs:126`、`:145`。
- 已有 rename 暂存：同文件 `:369`、`:393`、`:395`。再次提出 rename 不提供新的优化。
- finalize 确认原路径未重现、marker 和 backup 指纹一致，再物理删除：同文件 `:452`、`:462`、`:464`。
- 指纹 `WalkDir.follow_links(false)` 收集/排序条目，真实文件用 64 KiB buffer 顺序读：同文件 `:537`、`:554`、`:599`。
- Windows 目录 symlink 使用 remove_dir、真实目录 remove_dir_all：同文件 `:474`、`:477`、`:483`。沿用已落地的正确性修复。
- Local leftover 一个 guard 内逐项校验/卸载：`src-tauri/src/services/central_updates/inventory/leftover_cleanup.rs:116`、`:125`、`:148`、`:707`；copy 递归删除、symlink unlink 见 `src-tauri/src/services/installation/native.rs:158`、`:164`。
- Remote leftover 已唯一物理路径去重/256 分块，见现行 `.trellis/spec/backend/central-update-batching.md`。进程数对照使用已优化基线。
- Local preview 为内存 snapshot，不能虚构本地 preview workspace 递归清理瓶颈。Remote preview 删除成功才 ack ticket，失败保留 pending：`src-tauri/src/services/github_import/remote.rs:11`、`:24`、`:42`。
- update backup/staging、delete backup、copy 副本和 remote preview workspace 为不同清理对象。未完成 journal artifacts 无 TTL 清理；禁止扫 `.skillport-*` 后直接删。

## 3. 候选排序

| 顺序 | 候选 | 已知依据/预期 | 条件与风险 |
|---|---|---|---|
| P0 | 分阶段基线、source/runtime 身份 | 定位文件数/字节/IPC/清理占比 | 未测，不预先指定主因 |
| P1 | 真实写入进度、事件合并 | 已确认逐文件预写事件 | 异步侧 emit；blocking 不捕获 AppHandle；错误/终态 flush |
| P1 | bytes 所有权传递/只读共享 | 多处整批 clone | 先 move/复用已有 Arc；不增加全局缓存 |
| P1 | parent mkdir 去重、操作内索引/元数据复用 | 重复 mkdir、清单重建 | 保留逐文件路径检查和恢复时重新校验 |
| P2 | 有界文件 IO 对照 | 单 Skill 文件写入串行 | 比较 1/2/4/8，记录句柄/内存/错误；不按文件 spawn_blocking |
| P2 | 哈希实现优化/复用范围证明 | 多轮全树读 | 保持现有两种 fingerprint/digest framing、排序、SHA-256；锁不阻止外部文件修改 |
| P2 | Local leftover 唯一路径计划 | 按 group/路径循环 | 先测实际重复 FS/DB；保留权限和共享根语义 |
| 后续 | 增量 staging/copy | 少量变化可能少复制字节 | 原子整目录 swap 仍需完整文件集合；不能假定 O(变化文件数)；hardlink 共享可变内容有风险 |
| 后续 | 后台物理清理 | 可能减少结果返回时间 | 改变成功/锁/恢复语义，需独立产品决策与 durable backlog；默认不启用 |

## 4. 不作为收益来源的行为

不改成逐文件 raw 下载，不提高预算掩盖内存，不原地修改 canonical 规避 journal，不按 mtime/size/TTL 信任内容，不跳过 marker/fingerprint，不丢 pending/artifacts，不改 copy/symlink 默认，不关闭安全扫描，不把 UI 提前完成计作收益。

## 5. 验证状态

| 证据 | 状态 |
|---|---|
| 用户截图/缓慢报告 | 已接收，无耗时样本 |
| 源码调用链/成本/事件调查 | `STATIC_ANALYSIS` |
| 耗时、吞吐、内存、线程、句柄、UI 积压 | `NOT_RUN` |
| 真实仓库/网络、截图应用版本 | `NOT_RUN` |
| native WebView、bundle/安装器、SSH/WSL fake/实测 | `NOT_RUN` |
| 产品测试、just ci | `NOT_RUN`；当前仅创建任务文档 |

收益结论等待隔离基线和同条件复测。
