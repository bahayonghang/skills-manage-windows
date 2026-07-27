# Rust 测试 fixture 约定（test_support）

> 状态：生效（2026-07-04，任务 07-04-rust-test-support）
> 适用：`src-tauri` 全部 Rust 单元测试

## 契约

1. **新增测试禁止手抄池构造**。`SqlitePool::connect(":memory:")` + `db::init_database` 一律改用 `crate::test_support` 的池 fixture：
   - `mem_pool()` —— 内存池 + 全 schema + 内置种子（最常用）
   - `mem_pool_single_conn()` —— `max_connections(1)` 单连接语义（并发任务共享同一内存库时用）
   - `mem_pool_with_home(home: &str)` —— remote-home 语义（内置 agent 目录指向 POSIX home）
   - `file_pool() -> (DbPool, TempDir)` —— 文件库池（需要跨连接可见性/真实落盘时用；TempDir 存活期须覆盖池生命周期）
2. **常用 fixture 同样从 harness 取**：`set_agent_dir`（agent/central 目录重定向）、`write_skill_md`（SKILL.md 目录 fixture）、`central_skill_row`（中性 Skill 骨架，域特有字段用 struct-update 补）、`seed_central_skill`（写文件 + upsert 行）、`symlink_dir`（跨平台目录符号链接）。
3. **域内 setup 只允许薄壳**：本地 `setup_*` 允许保留（护住域内签名与 fixture 字面量），但体内必须委托 harness 原语，不得出现 connect+init 直写。
4. **harness 语义冻结**：`mem_pool` 与历史手抄逐字节同款（默认池选项）。不要「顺手」给它加 `max_connections(1)` 或其它池参数——那会改变事务/并发语义，需要不同语义就新增显式命名的变体。
5. 所有 harness pool 复用生产 `after_connect`，每条连接必须回读
   `PRAGMA foreign_keys = 1`。`file_pool` 走 `open_database`，覆盖版本迁移、
   validation 与 seed；不得再组合 raw `create_pool + init_database`。

## 豁免（仅两类，现场必须带豁免注释）

- **语义性**：测试需要**未 init 的裸池**（无 schema 容错、手工搭 legacy schema 验证迁移）。在案现场：`operation_log.rs`（无表容错）、`db/tests.rs`、`db/migrations/tests.rs`（五个 tag-pinned SQL fixture、preflight 与故障恢复）、`services/projects/tests.rs`、`services/marketplace/tests.rs`（legacy schema 迁移）。Tag fixture 必须提交可读 SQL 与 tag/commit/source/fixture checksum manifest，禁止用当前 schema 冒充旧版本。
- **结构性**：`tests/` 下的集成测试 crate（`#[cfg(test)] pub mod test_support` 对其不可见）。在案现场：`tests/common/mod.rs` 集中提供 `fresh_db`/`write_skill_md`/`seed_central_skill`，供 `projects_e2e.rs` 与 `cli_api_e2e.rs` 复用；只服务单个 integration crate 的 helper 留在该测试文件内。

新增豁免现场时，在 connect 调用上方注释「豁免 test_support::…：<原因>」，并更新本清单。

## Crash / Remote Recovery Harness

- FS+DB Saga crash tests use a real subprocess helper with explicit phase markers; the parent kills the child, reopens the file DB and filesystem, then asserts convergence and artifact retention/cleanup. In-process early returns do not count as crash evidence.
- SSH/WSL recovery protocol tests use the shared `FakeRunner` seam and assert complete script/argv/stdin plus stable output rows. They must not expose credentials, full command output, or host paths in expected diagnostics.
- Real Windows WSL rename/restore smoke is ignored unless `SKILLPORT_TEST_WSL_DISTRO` names an available disposable distro. FakeRunner parity is required even when the live smoke cannot run.
- Tests that exercise production Central mutation entrypoints share a test-only in-process mutex for the real default file-lock path. Do not replace the production timeout or OS lock with this mutex; low-level isolated-path tests must still prove contention, timeout, crash release, and different-target independence.

## 巡检命令

```
Grep: connect\(":memory:"\)   # 命中应仅：test_support.rs + 上述语义豁免 + tests/common/mod.rs
Grep: async fn setup          # 剩余定义体内不得出现 connect+init 直写
```

## 背景

2026-07-04 架构评审：26 份手抄 setup 散布 23 文件（4 种返回类型、域域重造 seed），obsidian 域因首条测试成本畸高长期零测试。收敛后基线：`connect(":memory:")` 31 → harness 文件外只保留在案语义豁免与 integration crate 的共享结构性豁免；obsidian 0 → 10 条。详见 `.trellis/tasks/archive/2026-07/07-04-rust-test-support/design.md`。
