# Rust test-support harness：收敛 24 份手抄测试 setup

## Goal

建一个 `test_support` harness module（内存库 + 迁移 + 临时目录 + 常用 fixture 的一个小 interface），把全仓约 24 份手抄 `setup_test_db` 收敛为薄调用方，并以 obsidian 域首批测试作为 harness 的验收演示。

## 背景与证据（2026-07-04 架构评审 · 测试面走查，718 条 Rust 测试）

- **24 个** `setup_test_db / setup_db / setup_pool / setup()` 手写定义散布 **23 个文件**；**29 处** `SqlitePool::connect(":memory:")` 散布 21 文件。主体近乎相同（connect + `db::init_database`），返回类型却有 4 种（`DbPool` / `SqlitePool` / `(DbPool, TempDir)` / `crate::db::DbPool`）。
- 每域再私造 seed helper：`installation/tests.rs:41`（`setup_db(central_dir, agent_dir)` 手写 `UPDATE agents SET global_skills_dir` 两次）、`:60`（`setup_db_with_codex`）、`:88`（`create_central_skill`）、`projects/tests.rs:468`（`seed_central_skill`）——同一 fixture 概念域域重造。
- 全仓无 `test_support`/`common` 模块；`db::init_database` 是唯一共享原语。
- 后果实例：**obsidian 域是全仓唯一 service + command 双层 0 测试的域**（其余域 7–66 条）；跨域集成测试仅 1 个（`tests/projects_e2e.rs`）。

## Requirements

1. 落一个共享 test-support module，interface 覆盖高频组合：内存池 + 迁移、临时 central/agent 目录、常用 seed（agent、central skill）。可见性方案（`#[cfg(test)]` gate 或 dev-dependency crate）由 design 裁决。
2. 存量 24 份 setup 迁移到 harness；允许分批（design 定批次），但 installation / projects / scanner / commands 等高频域必须进第一批。
3. 用 harness 为 obsidian 域写首批 service 测试（vault 扫描 + 源模式导入的核心路径），作为「新域首条测试成本骤降」的验收演示。
4. 不改任何产品代码——本任务只动测试与测试基建。

## Constraints

- fixture 语义与现有测试断言保持兼容：迁移是替换 setup 来源，不重写断言。
- Windows-first：harness 的临时目录/符号链接 fixture 必须在 Windows 测试二进制下可用（CONTEXT.md 构建约束）。

## Acceptance Criteria

- [ ] grep 验证：`setup_test_db` 类手写定义数从 24 降至个位数（目标值由 design 定）；`connect(":memory:")` 调用点收敛进 harness。
- [ ] obsidian 域获得首批 service 层测试（≥5 条，覆盖扫描与导入主路径）。
- [ ] `cd src-tauri && cargo test` 全绿，测试总数不低于基线 718。
- [ ] `cargo clippy -- -D warnings` 通过。

## Notes

- 复杂度：complex（涉及 23 个文件，但改动机械）→ 需 `design.md` + `implement.md`。
- 排序：建议紧随 redaction 任务之后、先于所有 Rust 重构类任务（2/5/6 号候选直接受益）。
- 评审附带记录（本任务不处理，供后续参考）：commands 层 `State<AppState>` 不可构造导致 48 个 `_impl` 影子函数、IPC 边界零测试——若未来要解决，另立任务。
