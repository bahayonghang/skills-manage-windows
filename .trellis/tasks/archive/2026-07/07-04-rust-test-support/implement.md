# Implement：Rust test-support harness

> 前置：`prd.md`（需求与 AC）、`design.md`（决策 D1–D7、迁移映射表 §3）。按步序执行，每步一个提交（即回滚点），每步末尾跑该步的验证命令，绿了才进下一步。全程不改产品行为（唯二触碰产品文件的行：lib.rs / obsidian mod.rs 挂 `#[cfg(test)]` 测试模块，属测试基建）。

## Step 1 — 新建 harness module + 自测

- [x] 新建 `src-tauri/src/test_support.rs`：D2 的 9 个公开 fn（mem_pool / mem_pool_single_conn / mem_pool_with_home / file_pool / set_agent_dir / write_skill_md / central_skill_row / seed_central_skill / symlink_dir），模块头 doc 注释声明「测试 fixture 唯一来源；新测试禁止手抄 connect+init」。
- [x] `lib.rs` 挂 `#[cfg(test)] pub mod test_support;`。
- [x] 内联 `mod tests` 写 D6 自测 5 条。
- [x] 验证：`cd src-tauri && cargo test test_support` 全绿；`cargo clippy -- -D warnings` 通过。
- [x] 提交（回滚点 1）：`test(test-support): 新增共享测试 harness`——纯新增，风险为零。

## Step 2 — 第一批迁移（高频域：installation / projects / scanner / commands 全量）

- [x] 按 design §3 映射迁移：commands/{agents/tests,bootstrap,collections,saved_views,settings,tag_groups,central_store_location,central_updates/tests,skill_update_inventory/tests}.rs + services/{installation,projects,scanner}/tests.rs。
- [x] use-alias 现场调用点零改动；installation `setup_db`/`setup_db_with_codex`、remote-home 变体、两份 `make_central_skill` 按薄壳方案处理；agents:71 / scanner:386 内联直调。
- [x] fixture 字面量（"Test skill"/"User skill"/"seed"/"Desc for {id}"、`source: github:owner/repo`）逐字保留在现场薄壳；断言零改动。
- [x] 清理本批文件因迁移孤儿化的 import。
- [x] 验证：`cd src-tauri && cargo test` 全绿（总数不降）；`cargo clippy -- -D warnings` 通过。
- [x] 提交（回滚点 2）：`test(harness): 高频域测试 setup 迁移至 test_support`。

## Step 3 — 第二批迁移（其余 services + db / targets / operation_log）

- [x] services/{ai_provider×3 内联 + tests.rs, ai_tagging, central_skills, github_import, marketplace, portable_state, usage} + db/tests.rs + targets/tests.rs 按映射迁移；operation_log:394/438 内联直调。
- [x] 三处语义豁免（operation_log:382、db/tests:2057、projects/tests:261）各加一行豁免注释（说明「故意无 schema / legacy schema，勿迁 harness」）。
- [x] 清理孤儿 import。
- [x] 验证：`cd src-tauri && cargo test` 全绿；`cargo clippy -- -D warnings` 通过。
- [x] 提交（回滚点 3）：`test(harness): 其余域测试 setup 迁移至 test_support`。

## Step 4 — obsidian 域首批测试（验收演示）

- [x] `services/obsidian/tests.rs`（新建，`#![cfg(test)]`；mod.rs 挂 `#[cfg(test)] mod tests;`）：D5 导入路径 6 条，全走 harness fixture。
- [x] `services/obsidian/query.rs` 尾部内联 `#[cfg(test)] mod tests`：D5 扫描 3 条（`scan_obsidian_vault` 私有 fn 直测，central_dir 参数注入）。
- [x] 错误断言一律 `matches!`（domain-error-enums 约定）。
- [x] 验证：`cd src-tauri && cargo test obsidian` 全绿。
- [x] 提交（回滚点 4）：`test(obsidian): 基于 test_support 的首批 service 测试`。

## Step 5 — 全量门禁 + AC 复核（最后一轮全范围检查）

- [x] `cd src-tauri && cargo test` 全绿，记录总数（AC：≥718+14）；`cargo clippy -- -D warnings` 通过。
- [x] `pnpm typecheck && pnpm lint && pnpm test`（本任务不动前端，跑通即证无联动破坏）。
- [x] grep 复核（AC 证据，逐条记录到任务 notes；用 Grep 工具而非 bash grep 计数）：
  - `connect\(":memory:"\)` → ≤5 且逐处对号 D7 豁免清单；
  - `async fn setup` → 剩余定义均为薄壳（体内无 connect+init 直写）；
  - `create_pool\(` 测试现场 → 仅 harness `file_pool`。
- [x] 对照 `prd.md` AC 四项逐条勾选。
- [x] Trellis Phase 3：spec 更新——新增 `.trellis/spec/backend/test-support.md`（契约：新增 Rust 测试必须用 `crate::test_support` fixture，禁止手抄 connect+init；豁免类别与登记方式），更新 backend/index.md。
- [x] 收尾提交 + `task.py` 归档流程。

## 回滚策略

- 每步独立提交且独立可编译：任一步出问题 `git revert` 该步即可。
- Step 1 纯新增；Step 2/3 只动测试文件（回滚零产品影响）；Step 4 只新增测试。

## 审查门

- **门 1（Step 1 后）**：harness interface 定版（9 fn 面审查，若对 D2 签名有异议此时改最便宜）。
- **门 2（Step 5）**：grep 证据 + 全量门禁齐备后才可宣称完成（verification-before-completion）。
