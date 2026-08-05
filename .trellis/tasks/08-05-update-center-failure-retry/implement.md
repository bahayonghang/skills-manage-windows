# 执行计划：更新中心失败项重试与移除决策自动化

按后端 → IPC → 前端顺序推进。每个阶段结束跑该阶段的验证命令，最后一轮跑全量门禁。

## 阶段 1：失败项分类（后端）

- [x] 1.1 `inventory/types.rs` 增加 `FailedRepositoryRetry` 枚举与 `FailedRepository.retry` 字段（`#[serde(default)]`，Default = `Unknown`）。
- [x] 1.2 `central_updates/error.rs` + `ipc_error.rs` 注册 `central_updates.skill_source_missing`、`central_updates.relocation_failed` 两个错误码及其公开英文文案。
- [x] 1.3 现有 4 处 `FailedRepository` 构造点补分类（`mod.rs:191-207`、`mod.rs:247-262`、`mod.rs:354-361`、`relocation.rs:101-110`）。
- [x] 1.4 删除 `mod.rs:247-262` 的 "Switch to incremental and removal mode..." 硬编码文案。

验证：`cd src-tauri && cargo test central_updates::`

## 阶段 2：常规模式自动归位

- [x] 2.1 `relocation.rs` 抽出 `unique_relocation_target` 与 `apply_relocation`；`reconcile_relocated_remote_skills` 改为调用二者，保证增量模式行为不变（现有测试必须全绿，不改断言）。
- [x] 2.2 `mod.rs` 步骤 3 循环改为收集 `pending_regular_relocations`，不在循环内产出常规模式的 remote-missing 失败项。
- [x] 2.3 新增按 `repo_cache_key` 缓存的整仓候选构建（`build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)`），同仓库只扫一次。
- [x] 2.4 循环后统一处理待归位集合：唯一命中 → `apply_relocation` → `UpdateAvailable` 进 `updatable`；未命中/多命中/新路径被他人占用 → `DecisionRequired` 失败项；归位执行失败 → `Retryable` 失败项。
- [x] 2.5 测试（`inventory/tests.rs`）：
  - 移动路径 + 内容有变更 → 无失败项，进 `updatable`，`skill_repository_members.source_path` 已更新；
  - 移动路径 + 内容一致 → 无失败项，不进任何桶；
  - 0 候选 → `DecisionRequired`，不写库；
  - 2 个同 `skill_id` 候选 → `DecisionRequired`，不写库；
  - 新路径已被其它技能占用 → `DecisionRequired`，不写库；
  - 增量模式既有归位用例保持通过。

验证：`cd src-tauri && cargo test central_updates::inventory`

## 阶段 3：分片刷新与合并

- [x] 3.1 `mod.rs` 拆出 `compute_skill_update_inventory`，`refresh_skill_update_inventory_impl` = compute + persist，签名不变。
- [x] 3.2 新增 `inventory/retry.rs`：`merge_inventory_for_repositories` + `retry_failed_repositories_impl`，在 `inventory/mod.rs` 挂模块与导出。
- [x] 3.3 测试：
  - 单仓库重试成功 → 该仓库条目刷新，其它仓库 updatable/added/missing 条目逐条不变；
  - 单仓库重试仍失败 → 该仓库失败项被新错误替换，其它桶不变；
  - `mode_override = Sync` + base 为 Regular → 分片产出的 `remote_missing` 并入结果，持久化 `inventory_id` 与 `run.mode` 仍为 base 值；
  - `repository_ids` 为空 → 返回基线，不触网、不写库；
  - `unsupported` 与平台桶在分片重试后保持基线值。

验证：`cd src-tauri && cargo test central_updates`

## 阶段 4：IPC 壳层

- [x] 4.1 `commands/skill_update_inventory.rs` 新增 `retry_failed_update_repositories`，复用现有进度事件与 `with_operation_log` 结构，details 增加 `repositoryCount` / `modeOverride`。
- [x] 4.2 `ipc_registry.rs` 注册命令。
- [x] 4.3 `pnpm ipc:codegen` 生成产物；连续跑两次 `pnpm ipc:codegen:check` 验证确定性。
- [x] 4.4 更新 ipc 契约测试中的命令计数与 `docs/architecture/_generated/ipc-commands.md`。

验证：`pnpm ipc:codegen:check`、`cd src-tauri && cargo test ipc`

## 阶段 5：前端 store

- [x] 5.1 `types/skillUpdateInventory.ts` 同步 `FailedRepository.retry` 类型。
- [x] 5.2 `updateCenterStore.ts` 新增 `retryingRepositoryIds` 与 `retryRepositories`；非 Tauri 运行时返回当前 inventory。
- [x] 5.3 测试（`src/test/stores/updateCenterStore.test.ts`）：进行中状态进出、结果整体替换、成功/失败判定、异常路径清理 `retryingRepositoryIds`。

验证：`pnpm test -- src/test/stores/updateCenterStore.test.ts`

## 阶段 6：Failed 面板与文案

- [x] 6.1 `UpdateCenterTabHandlers` 增加 `retryRepositories`；`UpdateCenterDialog.tsx` 接线（含全局互斥禁用）。
- [x] 6.2 `FailedRepositoriesPanel` 增加批量按钮、按分类的行内动作、行内 loading。
- [x] 6.3 `zh.json` / `en.json` 新增按钮、提示、`backendErrors.central_updates.skill_source_missing`、`backendErrors.central_updates.relocation_failed`。
- [x] 6.4 测试：可重试项渲染重试按钮、`decision_required` 渲染增量重查按钮、`unknown` 无按钮、批量按钮计数与禁用、批量部分失败提示文案。

验证：`pnpm test -- src/test/components/central`

## 阶段 7：全量门禁

- [x] 7.1 `pnpm typecheck`
- [x] 7.2 `pnpm lint`
- [x] 7.3 `pnpm test`
- [x] 7.4 `cd src-tauri && cargo clippy -- -D warnings`
- [x] 7.5 `cd src-tauri && cargo test`
- [x] 7.6 `just ci`

## 审查关口

- 阶段 2 结束：确认增量模式既有归位测试未被改写断言，常规模式没有产出任何 `remote_missing` 条目、没有删除路径。
- 阶段 3 结束：确认合并后持久化仍只写 base `inventory_id` 一行 run，未产生分裂记录。
- 阶段 6 结束：确认无英文硬编码文案残留，错误文案不含 token / URL / 本地路径。

## 回滚点

- 阶段 2、3、4 各自可独立回退：阶段 2 回退到"常规模式产出失败项"，阶段 3 回退到"仅整轮刷新"，阶段 4 之后的前端改动依赖阶段 3、4，需一并回退。
- 无 DB 迁移，回滚不需要数据修复。

## 执行记录

- 全部阶段完成，`just ci` 通过（Rust 1143 测试 / 前端 1633 测试 / clippy / fmt / sizecheck / ipc codegen 双跑一致）。
- 计划外调整：新的 `skill_repository_members` 查询无法放进 `repositories_repo.rs`（该文件正好在 800 行预算上限），改为新建 `db/repos/repository_members_repo.rs`。
- 计划外调整：重试分片继承基线 cache policy，而非固定 Bypass，使重试与再刷一次该面板看到的一致；已写入 spec。
- 既有测试 `refresh_regular_mode_returns_only_content_buckets` 的断言从「文案含 incremental and removal mode」改为「error_code = central_updates.skill_source_missing 且 retry = decision_required」，对应硬编码文案的移除。
- 提交：`b4715bf0 feat(central-updates): [AI] ✨ 更新中心失败项重试与常规模式自动归位`
