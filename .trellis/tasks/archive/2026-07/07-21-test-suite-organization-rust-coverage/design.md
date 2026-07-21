# 测试目录整理与 Rust 集成测试设计

## 1. Scope And Boundaries

本任务只调整测试资产、测试发现脚本、必要的活文档/spec，以及与重复测试有关的 `#[cfg(test)]` 代码。生产功能、IPC 契约、数据库 schema 和依赖集合保持不变。

前端整理与 Rust 补测保留在同一个任务中：两者属于同一质量门禁迁移，需要共享最终测试清单、fixture 约定和 `just ci` 验收；它们没有独立发布或产品交付价值。

## 2. Frontend Target Layout

`src/test` 使用“主要生产代码归属”作为唯一常规分类规则：

| Target | Ownership rule |
| --- | --- |
| `components/<domain>/` | 主要测试 `src/components/<domain>/` 下的组件 |
| `pages/` | 主要测试页面、页面级工作流或 page-owned harness |
| `stores/` | 主要测试 Zustand store |
| `lib/` | 主要测试纯函数、view model 或通用库 |
| `hooks/` | 主要测试 React hook |
| `runtime/` | 主要测试 IPC/runtime adapter、日志或环境边界 |
| `fixtures/` | 主要测试 `src/fixtures` 的行为 |
| `app/` | 主要测试根应用组合与启动 shell |
| `contracts/` | CI、字体、排版、主题、i18n 等仓库级静态契约 |
| `scripts/` | 主要测试仓库脚本 |
| `support/` | 全局 setup、IPC mock、平台替身和共享 fixture |

归类优先级：仓库静态合约或脚本测试先进入特殊目录；测试支持文件进入 `support/` 或唯一消费方所在目录；其余文件按主要被测模块归属。禁止仅凭文件名前缀另建并行产品分类体系。

`centralSkillsViewTestSupport.tsx` 与 `marketplaceViewTestSupport.tsx` 分别跟随其页面测试放入 `pages/`；`setup.ts`、`ipcMock.ts`、`testPlatform.ts` 和通用 JSON fixture 放入 `support/`。组件测试在 `components/` 下继续镜像现有组件域，避免形成新的大平面目录。

## 3. Import Migration

- 移动后的 TypeScript 测试优先用现有 `@/*` alias 引用 `src` 内生产代码，降低目录深度耦合。
- 同组 harness 与 helper 保持短相对导入。
- `vi.mock()` 的模块标识与正常 import 同步更新，确保指向同一解析结果。
- 仓库根脚本、workflow 和 `src-tauri` 资源使用明确的仓库根解析或正确相对路径，不增加新 alias。
- 只更新当前活文档、配置和源码注释中的测试路径；归档任务与历史计划不做批量改写。

## 4. Test Discovery Contract

Vitest 继续使用 `src/test/**/*.test.*` / `src/test/**/*.spec.*`，只把 setup 路径改为移动后的 `src/test/support/setup.ts`。

`scripts/run-vitest-sequential.mjs` 提取并导出无副作用的递归收集函数：

1. 从 `src/test` 深度遍历普通文件。
2. 只选择 `.(test|spec).(js|jsx|ts|tsx)`。
3. 输出仓库相对、正斜杠规范化路径。
4. 全局稳定排序。
5. 仅在脚本作为 CLI 直接执行时运行现有逐文件逻辑；被测试导入时不得启动 Vitest。
6. 显式传入测试文件的现有行为保持不变。

新增 `src/test/scripts/runVitestSequential.test.ts`，使用临时嵌套目录验证递归发现、过滤和排序。目录迁移后的 Vitest 文件基线应为 128：原有 127 个，加 1 个发现器回归测试。

## 5. Rust Integration Boundary

新增 `src-tauri/tests/cli_api_e2e.rs`。测试 crate 只能通过 `skillport_lib::*` 公共接口访问生产代码，证明 CLI library/binary 边界可用；不为测试扩大 `pub(crate)` 可见性。

覆盖场景：

1. list/show/dry-run sync 在公共 API 上保持同一 `uid` / `id` 身份并生成确定计划。
2. 同一 skill 的 `uid` 与 `id` 同时输入时，dry-run 计划去重。
3. 同名 skills 通过名称查询时返回 `CliApiError::Ambiguous`，并保持 `code()` / `exit_code()` 机器契约。
4. `--all` 与显式引用冲突、空选择和非法安装方式返回 `InvalidInput`，不产生文件系统写入。

这些用例不调用 marketplace 搜索或远程安装，因此不访问网络。`CliContext` 使用 `cli_api_e2e.rs` 内的 no-op `SecretStore`，不触碰系统 keyring。

## 6. Shared Rust Fixtures

新增 `src-tauri/tests/common/mod.rs`，集中提供：

- `fresh_db()`：内存数据库加完整 schema/seed；保留集成 crate 无法访问 `#[cfg(test)] test_support` 的结构性豁免注释。
- `write_skill_md()` 与可指定 id/name 的 `seed_central_skill()`。

只由 CLI 集成测试使用的 `NoopSecretStore` 留在 `cli_api_e2e.rs`，避免 `projects_e2e` 编译未使用的测试辅助代码。

`projects_e2e.rs` 改用 common fixture，避免第二个集成 crate 出现复制。`cli_api` 模块内与新外部 happy-path 完全重复的测试移除，保留解析和来源导入等内部白盒测试。

同步更新 `.trellis/spec/backend/test-support.md`：结构性豁免落点从 `projects_e2e.rs` 的本地 helper 改为 `tests/common/mod.rs`，并记录它由多个 integration crates 共享。

## 7. Compatibility And Failure Handling

- Windows 路径在发现器边界统一成 `/`，内部文件访问仍使用 `node:path`。
- symlink 集成测试的现有 Windows 跳过语义不变。
- 目录移动不改变测试内容或断言；若移动暴露出路径依赖，只修正路径，不顺带重构测试逻辑。
- 任一定向测试失败时先停在对应迁移批次；不继续扩大移动范围。

## 8. Rollback

前端按目录批次移动，每批均可独立反向移动并还原 import。发现器改动先于大规模移动落地，使后续失败不会造成串行测试静默漏跑。

Rust common fixture 提取与新 `cli_api_e2e` 同一批完成；若共享 fixture 改变 `projects_e2e` 行为，先还原 fixture 提取，新测试暂保留本地 fixture，不能以删除既有断言通过验收。
