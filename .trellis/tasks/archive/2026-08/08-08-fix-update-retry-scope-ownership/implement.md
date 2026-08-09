# 执行计划：永久修复 Update Center 仓库归属与重试冲突

> 当前仅完成规划。用户审阅并明确允许实施后，才运行 `task.py start`。实现遵循 test-first；每阶段红灯未消除不得进入下一阶段。

## 阶段 0：固定回归反馈环

- [x] 0.1 在 `inventory/tests.rs` 加入 Skills + Regular 基线、同仓库 stable / gone、Sync repository retry 的最小回归。
- [x] 0.2 断言成功结果、原 scope / mode / inventory id、持久化 reload 和 bucket 唯一性；先运行确认当前代码稳定红在 inventory 唯一键。
- [x] 0.3 运行现有 `retry_with_sync_override_produces_removal_decisions_for_a_regular_inventory` 作为 Repositories-scope 绿灯对照，确认 fixture 没有扩大问题。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  retry_skills_regular_inventory_replaces_repository_slice_without_duplicates -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  retry_with_sync_override_produces_removal_decisions_for_a_regular_inventory -- --nocapture
```

审查关口：红灯必须是已诊断的重复 inventory key，不得通过断言一个宽泛 `is_err()` 固化错误行为。

## 阶段 1：修复新 inventory 的仓库归属生产者

- [x] 1.1 在 inventory 内引入私有 owned remote-missing state，同时携带 `SkillUpdateState` 和 assignment repository id。
- [x] 1.2 `UpdateAvailable` 直接使用 `PreparedSkillUpdate.assignment.repository.id`；不再通过 `repository_id_for_state` 推断。
- [x] 1.3 `reconcile_relocated_remote_skills` 改从 owned state 读取仓库 id；保持索引删除、唯一归位和 pending additions 现有行为。
- [x] 1.4 最终 `RemoteMissingSkill` 从 owned state 直接构建，正常生产路径始终写 `Some(repository_id)`。
- [x] 1.5 补 Skills scope producer 测试：updatable 和 sync remote-missing 都有正确归属。
- [x] 1.6 扩展 Platform fixture：同一被观测仓库含 stable / gone，验证归属、scope 过滤和平台桶不回归。
- [x] 1.7 运行 relocation 与 inventory 全模块测试，确认 regular / sync 两种归位路径不回归。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  central_updates::inventory -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  central_updates::core -- --nocapture
```

审查关口：新生产的仓库型 actionable item 不允许 `repository_id = None`；公开 `Option` 只为旧载荷兼容保留。

## 阶段 2：兼容旧空归属 inventory 的首次重试

- [x] 2.1 在 `retry_failed_repositories_impl` 中用现有 `get_central_skill_ids_by_repository` 汇总目标仓库当前 member skill ids。
- [x] 2.2 新增私有 `RepositoryRetryTargets`，封装“显式 repo id 优先、仅 None 使用 member fallback”的判断。
- [x] 2.3 修改 `merge_inventory_for_repositories`：updatable / remote-missing 使用该判断；added / failed 继续按显式 repo id；unsupported / 平台 / orphan 桶保持基线。
- [x] 2.4 构造旧 payload / entry column 均为 null 的基线，验证首次 retry 自愈并可回读。
- [x] 2.5 补 stale removal：目标技能变为 up-to-date、分片无 replacement 时旧 updatable 消失。
- [x] 2.6 补保守性：非目标 member 的 None 条目和显式属于其它 repo 的条目保持不变。
- [x] 2.7 重新运行阶段 0 精确回归，确认从红转绿。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  retry_ -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  legacy_inventory -- --nocapture
```

审查关口：不得用 URL、branch、source path、skill name 或数组去重推断旧条目归属；不得误删不相关 `None` 条目。

## 阶段 3：建立 strict persistence 不变量与 typed 错误

- [x] 3.1 在 `persist_refresh_inventory` 生成 entries 后校验 `(inventory_id, bucket, entity_key)` 唯一；重复时在 DB transaction 之前返回 `InventoryInvariant`。
- [x] 3.2 在 `CentralUpdatesError` 增加零动态字段的 typed variant，并补 `reviewed_operation_failure`、`diagnostic_category`、`to_ipc_error` 映射。
- [x] 3.3 在 `ipc_error.rs` 注册 `central_updates.inventory_invariant` 固定公开文案；保持 `retryable=false`。
- [x] 3.4 同步中英文 `backendErrors.central_updates.inventory_invariant` 文案，确保 UI 不显示后端英文兜底。
- [x] 3.5 测试 duplicate fixture：精确 domain variant / IPC envelope / operation metadata，且响应中无 SQLite 文本、key、路径、URL 或 secret。
- [x] 3.6 先写合法 run，再尝试重复 inventory；失败后 reload 仍是旧 run，证明前置校验无副作用。
- [x] 3.7 不修改全局 `legacy_plain_message` unique 兼容映射，并用测试证明 inventory 路径不再落入 `resource.conflict`。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  inventory_invariant -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  ipc_error -- --nocapture
```

审查关口：禁止 `INSERT OR REPLACE`、静默 dedup、动态错误正文或把此错误标记为 retryable。

## 阶段 4：固化仓库规范

- [x] 4.1 更新 `.trellis/spec/backend/update-inventory-retry.md`：新增 scope-independent ownership invariant。
- [x] 4.2 把旧的“`repository_id = None` 基线一律保留”改为“仅在无法由当前 membership 证明属于目标仓库时保留”。
- [x] 4.3 记录 strict persistence 和 typed invariant 规则，明确禁止覆盖插入 / 静默去重。
- [x] 4.4 检查 `domain-error-enums.md` 的现有规则足以覆盖该实现；只有发现可复用的新通则时才最小更新，避免重复规范。

审查关口：spec 描述最终契约，不记录临时 workaround 或实现过程。

## 阶段 5：分层验证

- [x] 5.1 运行精确 Skills / Platform / legacy / invariant 用例。
- [x] 5.2 运行完整 Rust 格式、全 targets Clippy 和锁文件测试。
- [x] 5.3 若加入 i18n 文案，运行相关前端错误格式化测试、TypeScript typecheck 和 lint。
- [x] 5.4 运行 `pnpm docs:gen:check` 与 `pnpm ipc:codegen:check`，证明 command / generated docs 未漂移；若实现意外改变 Tauri command 或 codegen 类型，则先按仓库规则重新生成并审查产物。
- [x] 5.5 运行最终仓库门禁 `just ci`；随后运行 `just audit` 做供应链回归检查。
- [x] 5.6 检查最终 diff，只包含本任务代码、测试、i18n、spec 与 Trellis 工件；保留用户已有 `.trellis/workspace/codex/` 和其它工作树内容。

验证命令：

```bash
rtk cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
rtk cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked
rtk pnpm test -- src/test/lib/backendError.test.ts
rtk pnpm typecheck
rtk pnpm lint
rtk pnpm docs:gen:check
rtk pnpm ipc:codegen:check
rtk just ci
rtk just audit
```

若实际错误格式化测试文件名不同，实施时先用 `rg` 定位现有测试，不新建平行测试入口。

## 阶段 6：完成与交付

- [x] 6.1 记录验证结果与精确测试数量，不把未运行项报告为通过。
- [x] 6.2 按 Phase 3 更新必要 spec，使用 `$git-commit` 流程组织原子提交；不创建额外 Trellis commit 任务。
- [x] 6.3 归档任务并写 workspace journal。
- [x] 6.4 向用户说明：代码回归已修复；`archive-planning` 的上游删除仍需用户选择 Keep 或 Delete，不属于数据库损坏。

## 回滚点

- 阶段 1（producer ownership）、阶段 2（legacy merge）和阶段 3（invariant error）分别保持可审查的逻辑边界；任一阶段无法通过对应测试时，停止并回到该阶段修正，不以跳过测试推进。
- 无 schema migration、无自动数据清理、无 Central 文件 mutation；代码回滚不需要数据库逆向迁移。
- 若升级后仍遇到未覆盖的旧 payload，前置 invariant 会保留原 run 并安全失败；完整 Skills + Sync Refresh 仍是非破坏性的恢复路径，不能以此替代根因修复。
