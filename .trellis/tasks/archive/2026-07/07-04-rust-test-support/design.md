# Design：Rust test-support harness

> 前置：`prd.md`。本文档裁决 PRD 留白的三件事：harness 可见性方案、interface 面、迁移批次与 AC 数值定版。
> 证据基线（2026-07-04 本任务实测，比评审数字略增）：`async fn setup*` 定义 **26 个**（PRD 记 24，另含 2 个组合变体）；`connect(":memory:")` **31 处**（PRD 记 29）。差异不影响结论，AC 以本文基线为准。

## 1. 现状形态归类（26 个 setup + 5 处无 setup 的内联 connect）

| 形态 | 定义 | 现场 |
| --- | --- | --- |
| **A. 裸内存池** | `connect(":memory:")` + `init_database` | 19 个定义：commands/{agents,bootstrap,collections,saved_views,settings,tag_groups,central_store_location,central_updates,skill_update_inventory}、services/{ai_provider×3(claude/config/secret),central_skills,portable_state,projects,scanner,usage}、db/tests、targets/tests(memory_db) |
| **B. remote-home 内存池** | `connect(":memory:")` + `init_database_for_remote_home(home)` | 2 个定义：central_updates `setup_remote_test_db`、skill_update_inventory `setup_test_db_with_home`；另 agents/tests.rs:71 内联 1 处 |
| **C. 文件库池** | `tempdir` + `create_pool` + `init_database` | 3 个定义：ai_provider/tests、marketplace/tests（返回 `(DbPool, TempDir)`）、github_import/tests（`std::mem::forget(dir)` 后只返回 pool） |
| **D. 内存池 + agent 目录重定向** | A + `UPDATE agents SET global_skills_dir WHERE id=...` | 2 个定义：installation `setup_db(central,agent)` / `setup_db_with_codex`；central_skills/central_store_location 另有独立的 `set_*_central_root` 重定向 helper |
| **E. 单连接内存池** | `SqlitePoolOptions::max_connections(1)` + init | 1 个定义：ai_tagging/tests |
| **豁免（语义性，不迁移）** | 故意**不建 schema** 或建 legacy schema | operation_log.rs:382（best-effort 无表容错）、db/tests.rs:2057 与 projects/tests.rs:261（迁移测试手工建旧 schema） |
| **豁免（结构性）** | 集成测试独立 crate | tests/projects_e2e.rs（`#[cfg(test)]` 模块对其不可见，见 D1） |

关键事实：`DbPool = SqlitePool` 纯类型别名（db/types.rs:14），4 种返回类型差异是表皮；`tempfile` 已是主依赖（Cargo.toml），harness 无需动依赖面。

## 2. 决策

### D1 放置与可见性：`#[cfg(test)] pub mod test_support` 单文件，不做 feature gate

- 落点 `src-tauri/src/test_support.rs`（预计 ~150 行 + 自测），`lib.rs` 挂 `#[cfg(test)] pub mod test_support;`。单元测试（全部 26 个现场所在）经 `crate::test_support::*` 使用。
- **否决**备选「dev-dependency 自引用 + `test-support` feature」：能惠及的只有 tests/projects_e2e.rs 一个文件（~30 行本地 helper），却要引入 Cargo feature 面与 Tauri 三 crate-type 构建变量，收益/复杂度不成比例。projects_e2e.rs 保留本地 `fresh_db`/`write_skill_md`/`seed_central_skill`，本文档记录在案为**结构性豁免**。
- **否决**备选「tests/common/ 目录」：那只服务集成测试；存量 26 个现场全在 lib 单元测试里。

### D2 interface 面：9 个公开 fn，一屏内

```rust
// ── 池 fixture ──
pub async fn mem_pool() -> DbPool                         // 形态 A：connect(":memory:") + init_database
pub async fn mem_pool_single_conn() -> DbPool             // 形态 E：max_connections(1) 变体
pub async fn mem_pool_with_home(home: &str) -> DbPool     // 形态 B：init_database_for_remote_home
pub async fn file_pool() -> (DbPool, TempDir)             // 形态 C：tempdir + create_pool + init_database
// ── agent fixture（形态 D 的原语化）──
pub async fn set_agent_dir(pool: &DbPool, agent_id: &str, dir: &Path)  // 含 id='central' 的中央目录重定向
// ── skill fixture ──
pub fn write_skill_md(dir: &Path, name: &str, description: Option<&str>) -> PathBuf
pub fn central_skill_row(id: &str, canonical_dir: &Path) -> Skill      // 中性默认 struct builder
pub async fn seed_central_skill(pool: &DbPool, canonical_dir: &Path, id: &str, description: &str)
// ── FS fixture ──
pub fn symlink_dir(target: &Path, link: &Path)            // cfg(unix)/cfg(windows) 分派
```

- `write_skill_md` 语义取 projects/tests.rs 版本（exact-dir、`description: Option`、frontmatter 逐字 `---\nname: {n}\ndescription: {d}\n---\n\n# {n}\n`，None 省略 description 行），返回 `dir.to_path_buf()`；installation 的 `create_central_skill(central_dir, id)`（join id、固定 "Test skill" 描述）改为本地薄壳适配，**描述字符串逐字保留**以护住既有断言。
- `central_skill_row` 只填公共骨架：`is_central: true`、`canonical_path: Some(dir)`、`file_path: dir/SKILL.md`、`name: id`、`scanned_at: now`，其余 `None`。central_updates / skill_update_inventory 两份手抄 `make_central_skill`（真重复对）改薄壳：`Skill { description: Some(...), source: Some("github:owner/repo".into()), ..central_skill_row(id, dir) }`——差异字段留在现场、逐字保留。
- `seed_central_skill` = `write_skill_md` + `upsert_skill(central_skill_row + description)`；projects 薄壳传 `"seed"` 保断言。

### D3 迁移策略：use-alias 优先，组合逻辑留薄壳

1. **形态 A（19 处）**：删除本地 fn，改 `use crate::test_support::mem_pool as setup_test_db;`（usage 别名 `setup_pool`、central_store_location 别名 `setup`、targets 别名 `memory_db`）——**调用点零改动**，diff 每文件 ±5 行。
2. **形态 B/C/D/E 及带组合逻辑者**：保留本地薄壳，体内改调 harness 原语（installation `setup_db` = `mem_pool` + 2×`set_agent_dir`；github_import 薄壳保留 `std::mem::forget`；skill_update_inventory `setup_test_db_with_home(&Path)` 薄壳内做 `to_string_lossy`）。
3. **内联 connect（operation_log:394/438、agents:71、scanner:386）**：调用点直接改 `test_support::mem_pool()` / `mem_pool_with_home`；scanner:386 现场的 `DELETE FROM agents` 等测试特有语句留在现场。
4. **豁免现场**加一行注释声明豁免原因（防后续任务误收敛）。
5. 每文件顺手清掉**因本迁移而孤儿化**的 `use sqlx::SqlitePool` 等 import；不动其它无关 import。

### D4 语义保真规则（PRD 约束「不重写断言」的落点）

- `mem_pool` 与现状**逐字节同款**：`SqlitePool::connect(":memory:")` 默认池选项。**不**统一到 `max_connections(1)`——那会改变池并发语义（事务持有 + 并发 acquire 场景可能死锁），ai_tagging 独享 `mem_pool_single_conn` 保持其原语义。
- `unwrap()` vs `expect("msg")` 的 panic 文案差异视为非语义（只影响失败输出，不影响断言），统一为 harness 内 `expect`。
- 所有被迁移文件的断言零改动；fixture 产出的字节（SKILL.md 内容、DB 字段值）逐字保留。

### D5 obsidian 首批测试（验收演示，≥5 条）

**测什么**：源模式导入（`import.rs` 两个 pub 入口）+ vault 扫描核心（`query.rs::scan_obsidian_vault`）。`get_obsidian_vaults_impl` 依赖真实 home 注册表扫描，非 hermetic，不进首批。

**挂载点**（均为仓内既有先例，不改产品行为）：
- `services/obsidian/tests.rs`（sibling `#![cfg(test)]`，mod.rs 挂 `#[cfg(test)] mod tests;`）→ 导入路径 6 条：
  1. platform 导入 symlink：创建目录符号链接 + skills 行 `is_central=false, source="symlink"` + installation 行 `link_type="symlink", symlink_target=Some(源)`
  2. platform 导入 copy：目录被复制 + `link_type="copy", symlink_target=None`
  3. 目标已存在 → `matches!(SkillExistsInAgent{..})`
  4. agent 不存在 → `matches!(AgentNotFound(..))`
  5. method 非法 → `matches!(UnsupportedInstallMethod(..))`
  6. method 缺省（None）按 symlink 处理（`"auto"`/None 分支）
  fixture 组合即 harness 展示面：`mem_pool()` + `set_agent_dir(pool, "claude-code", tmp)` + `write_skill_md(vault_skill_dir, ..)`。
- `query.rs` 尾部内联 `#[cfg(test)] mod tests`（访问私有 `scan_obsidian_vault`，先例：secret.rs/claude.rs/config.rs/usage 等）→ 扫描 3 条：
  7. 三源优先级：同 id 同现 `.skills` 与 `.claude/skills` → 取 `.skills`，且不重复
  8. `is_already_central`：central_dir 存在同名目录时为 true（central_dir 由参数注入，hermetic）
  9. 无 `.obsidian` 标记的目录 → 返回空
- 错误断言遵守 `.trellis/spec/backend/domain-error-enums.md`：`matches!` 分支，禁止字符串嗅探。
- Windows 可行性：installation/tests.rs 既有 `symlink_dir` 测试全绿证明本环境（开发者模式）symlink 可用；obsidian symlink 断言用 `fs::symlink_metadata().file_type().is_symlink()`。

### D6 harness 自测（护住 fixture 本身，~5 条）

`test_support.rs` 内联 `mod tests`：`mem_pool` 种子生效（agents 表含 `central`/`claude-code`）；`mem_pool_with_home("/home/alice")` 后 claude-code 目录为 `/home/alice/.claude/skills`；`set_agent_dir` 回读一致；`write_skill_md` 产物可被 `parse_skill_md` 解析出 name/description；`file_pool` 建表成功且 TempDir 存活期内文件存在。

### D7 AC 数值定版（PRD 留白的目标值）

| 指标 | 基线 | 目标 | 实测（完成时） |
| --- | --- | --- | --- |
| 手写池体（connect+init 直写在 setup/测试体内）定义数 | 26 | **src/ 内 0**；全仓 1（projects_e2e，结构性豁免） | ✅ 0 / 1 |
| `connect(":memory:")` 出现次数（harness 文件外） | 31 | **≤5**：语义豁免 + projects_e2e，各带豁免注释 | ✅ 4（op_log 1 + legacy 迁移 2）+ e2e 1；实施中新发现 marketplace legacy 迁移测试为第 4 处语义豁免（`create_pool` 裸文件池同理），已注释在案 |
| 测试现场 setup 形态的 `create_pool(` | 3 | 0（收敛进 `file_pool`） | ✅ 0（marketplace 迁移测试 1 处豁免） |
| obsidian service 层测试 | 0 | ≥9（D5：导入 6 + 扫描 3） | ✅ 10（导入 7 + 扫描 3） |
| cargo test 总数 | 718 | ≥ 718 + 14（obsidian 9 + harness 5） | ✅ 733 |

## 3. 迁移映射表（23 文件 → 处置）

| 文件 | 处置 |
| --- | --- |
| commands/{agents/tests,bootstrap,collections,saved_views,settings,tag_groups}.rs、services/{central_skills,portable_state,projects,scanner}/tests.rs、services/ai_provider/{claude,config,secret}.rs、db/tests.rs、targets/tests.rs、services/usage/mod.rs、commands/central_updates/tests.rs(:6)、commands/skill_update_inventory/tests.rs(:35)、commands/central_store_location.rs | use-alias 替换（D3.1） |
| services/installation/tests.rs | `setup_db`/`setup_db_with_codex` 薄壳化；`create_central_skill`/`create_user_skill` 改调 `write_skill_md`（保留描述字面量）；本地 `create_symlink_for_test` 删除改用 harness `symlink_dir` |
| services/projects/tests.rs | `setup_test_db` alias；本地 `write_skill_md`/`seed_central_skill` 薄壳化或直调 harness（描述 `"seed"` 逐字保留） |
| commands/central_updates/tests.rs(:12)、commands/skill_update_inventory/tests.rs(:43) | remote-home 薄壳 → `mem_pool_with_home`；两份 `make_central_skill` → `..central_skill_row()` struct-update 薄壳 |
| services/{ai_provider,marketplace,github_import}/tests.rs | → `file_pool()`（github_import 薄壳保留 `mem::forget`） |
| services/ai_tagging/tests.rs | → `mem_pool_single_conn` alias |
| operation_log.rs(:394,:438)、commands/agents/tests.rs(:71)、services/scanner/tests.rs(:386) | 内联直调 harness |
| operation_log.rs(:382)、db/tests.rs(:2057)、services/projects/tests.rs(:261)、tests/projects_e2e.rs | 豁免 + 注释 |

## 4. 风险与对策

1. **sqlx `:memory:` 池语义漂移** → D4：mem_pool 逐字节复刻现状，不「顺手改进」。
2. **installation 断言依赖 fixture 字面量**（"Test skill"/"User skill"/"seed"） → 薄壳保留字面量，harness 只提供参数化原语。
3. **init_database 的 scan_directories 种子先于 set_agent_dir**（重定向后 scan_directories 仍指旧路径） → 与现状 `setup_db` 行为完全一致，非新增风险，不修。
4. **孤儿 import 触发 clippy -D warnings** → 每文件迁移时同步清理自己造成的 unused import；Step 门禁含 clippy。
5. **Windows symlink 权限**（CI/无开发者模式环境） → 与既有 symlink 测试同一约束面，不新增；若环境不可用属既有测试面问题，不在本任务扩大。

## 5. 提交与回滚形状

4 个实现提交（Step 1 harness / Step 2 第一批迁移 / Step 3 第二批迁移 / Step 4 obsidian 测试），每步独立编译独立绿，`git revert` 单步可回滚；Step 2/3 仅动测试代码，回滚零产品影响。详见 `implement.md`。
