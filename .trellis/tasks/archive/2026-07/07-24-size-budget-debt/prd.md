# 拆分 5 个 >800 行 frozen exception 模块

## Goal

清偿 size budget 的五个 frozen exception，恢复生产源码统一的 800 行规则，同时将这五个历史热点均降到 600 行以下。交付是纯重构：对 Tauri IPC、数据库初始化、Central 更新 Saga、Central Skills 页面和唯一技能卡片入口均不改变用户可观察行为或既有导入 API。

## Confirmed Evidence

- `scripts/check-size-budget.mjs` 当前以 `MAX_LINES = 800` 保护生产源码，并以 `BASELINE_ALLOWLIST` 暂时豁免五个文件；当前实测行数为 `central_updates/core.rs` 860、`commands/collections.rs` 1008、`db/seed.rs` 722、`CentralSkillsView.tsx` 865、`UnifiedSkillCard.tsx` 840。
- 将全局阈值直接改为 600 会使大量非目标生产模块失败（例如 `paths.rs`、`CentralStatePortabilityDialog.tsx`、`settings.rs`、`PlatformView.tsx`），超出本任务的纯重构范围。因此本任务保留通用 800 行规则，并以交付时的逐文件计数证明五个历史 exception 都小于 600 行。
- `central_updates/core.rs` 的 source/assignment/state 构造与主检查/更新流程天然分离；这些 helper 已被 `core/batch.rs`、inventory 和 repository-sync 路径复用，内部可见性与现有 `central_updates::mod` 重导出必须保持。
- `commands/collections.rs` 的生产实现和 Tauri command 壳在第 1-329 行，超限主要来自第 333 行开始的既有单元测试；同目录已经使用 `collections/export_import.rs` 子模块结构。
- `db/seed.rs` 的数据库初始化和 seeder 位于前半部分，内置 agent 路径/工厂/目录清单位于后半部分；`db::builtin_agents*` 和 universal-agent helper 由 migrations、settings、projects、installation 与测试使用。
- `CentralSkillsView.tsx` 已将 store binding、facets、chrome、dialogs 与动作实现拆到相邻模块；页面内仍有一大段把页面 state 绑定到 `useCentralSkillsActions` 的 adapter 和保留滚动位置的选择 handler。
- `UnifiedSkillCard` 已有一个公共入口和判别联合 props。其 props/type 声明与内部 leaf UI 可以拆出，但 `SkillCardModel` 与 `toModel` 必须继续只服务于这个唯一渲染实现，调用方仍从 `UnifiedSkillCard.tsx` 导入公开类型。

## In Scope

1. 将 `central_updates/core.rs` 的 source loading、assignment 到 update state 的映射及其私有辅助函数迁入 `core/state.rs`，由 `core.rs` 保持现有 crate-internal 导出和主流程。
2. 将 collections 现有 test module 原样迁入 `commands/collections/tests.rs`，保留 production implementation、Tauri commands 和 `export_import` 的行为与可见性。
3. 将内置 agent 目录、路径归一化、factory 与 public helper 迁入 `db/seed/agents.rs`，由 `seed.rs` 重导出既有 API 并继续承担初始化与 seed 编排。
4. 将 Central Skills 页面中页面 state 到既有 action hook 的绑定与滚动保持逻辑迁入邻近专用 hook；`CentralSkillsView` 继续是页面入口、继续渲染相同 shell/dialog/palette 组合。
5. 将 `UnifiedSkillCard` 的公开场景 props/type 定义与无状态 leaf UI 拆到 sibling modules，并从原入口重导出 public types；不导出内部 `SkillCardModel`，不创建第二个技能卡片组件。
6. 在五个文件均低于 600 行后，删除 size-checker 的 frozen allowlist 分支，保留全局 800 行检查；同步更新质量规范，声明该规则不再含历史例外。
7. 使用现有 Rust/React 回归用例、typecheck、lint、sizecheck、`just ci` 和逐文件行数检查证明重构未改变行为。

## Out of Scope

- 不把仓库全局 size budget 降到 600，也不触碰其他 600-800 行模块。
- 不新增功能、IPC command、数据库 schema/migration、i18n 文案或视觉行为。
- 不改变 Central 更新批处理/FS+DB Saga、collection 安装语义、builtin agent 数据、卡片场景联合或页面测试 fixture 的合同。
- 不 push、不创建远程 PR、不修改本任务以外的现有未提交 Trellis 工具和工作区文件。

## Acceptance Criteria

- [ ] 原五个 exception 文件均小于 600 行，且 `scripts/check-size-budget.mjs` 不再包含 `BASELINE_ALLOWLIST` 或任何 frozen exception 输出路径。
- [ ] `pnpm sizecheck` 通过，且仍以统一的 800 行生产源码规则扫描全仓库。
- [ ] `cargo test --locked` 中 central updates、collections、database seed 相关测试保持通过；现有 public Rust 导出和 Tauri command 签名不变。
- [ ] `pnpm typecheck`、UnifiedSkillCard 场景正负例和 CentralSkillsView 相关页面测试通过；卡片仍只有 `UnifiedSkillCard` 一个公共渲染入口，调用方导入路径不变。
- [ ] `just ci` 通过，最终 diff 仅包含本子任务的重构、size checker、必要的质量规范与 Trellis 收尾文件。
- [ ] 子任务归档后，父任务复核 16/16 子任务、P3-01 证据及最终 `just ci`，再进行本地 archive/journal；不 push。

## Risks And Mitigations

- Rust module extraction可能改变 private visibility 或测试模块的 `super::*` 路径：保留 root re-export，并优先运行定向 Cargo 测试再跑全量 gate。
- 页面 action adapter 的 dependency/scroll 处理可能引入 stale closure：移动时保留 hook 依赖和现有 CentralSkillsView interaction tests，不改变 action implementation。
- 卡片 type relocation 可能破坏调用方或互斥负例：从原入口 re-export，`pnpm typecheck` 和 variants tests 必须覆盖。
- 600 是本次清债目标而不是新全局门槛：交付时以脚本化逐文件计数验证，常规 CI 继续守住统一 800 行规则。

