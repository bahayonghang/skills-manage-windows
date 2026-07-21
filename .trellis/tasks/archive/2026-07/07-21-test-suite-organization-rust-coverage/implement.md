# 实施计划

## 1. Preflight

- [x] 重新确认任务状态、工作树和相关 spec；加载 `trellis-before-dev`。
- [x] 保存 `pnpm exec vitest list --filesOnly` 的 127 文件基线，并确认 `src-tauri/tests/projects_e2e.rs` 当前 5 个集成测试。
- [x] 检查实施开始后出现的无关用户改动并保持隔离。

## 2. Make Nested Discovery Safe

- [x] 先为 `scripts/run-vitest-sequential.mjs` 添加嵌套目录发现的失败测试。
- [x] 将串行发现器改为可导入、无副作用、递归且确定排序的实现，保持显式文件参数行为。
- [x] 定向运行发现器测试，并用一个嵌套测试路径执行 `pnpm test:serial -- <path>`。

## 3. Reorganize `src/test`

- [x] 先移动 `support/`、`contracts/`、`scripts/`，更新 `vite.config.ts` setup 路径与活文档引用。
- [x] 按 `components/<domain>/` 批量移动组件测试，使用 `@/*` 修复生产模块 import 与 mock。
- [x] 依次移动 `pages/`、`stores/`、`lib/`、`hooks/`、`runtime/`、`fixtures/`、`app/`，每批运行对应 Vitest 文件。
- [x] 搜索残留的旧 `src/test/<file>` 活引用；历史计划/归档任务保持不动。
- [x] 运行 `pnpm exec vitest list --filesOnly`，确认发现 128 个测试文件且原 127 个测试一一存在。
- [x] 运行 `pnpm typecheck`、`pnpm lint`、`pnpm test`。

## 4. Add Rust External-Crate Coverage

- [x] 新建 `src-tauri/tests/common/mod.rs`，提取两个集成测试共用的数据库与 skill fixture；在 `cli_api_e2e.rs` 内实现 no-op secret store。
- [x] 先新增 `cli_api_e2e.rs` 的公共契约测试：身份/计划、重复引用、歧义错误、无效输入。
- [x] 删除 `cli_api` 模块内与外部 happy-path 完全重复的测试，保留非重复白盒覆盖。
- [x] 更新 `.trellis/spec/backend/test-support.md` 的 integration harness 与结构性豁免说明。
- [x] 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test cli_api_e2e --locked`。
- [x] 运行 `cargo test --manifest-path src-tauri/Cargo.toml --test projects_e2e --locked`。
- [x] 运行 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`。
- [x] 运行 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`。

## 5. Full Gate And Review

- [x] 运行最终 `just ci`，确认 web/Rust 两条链全部通过。
- [x] 比较最终 Vitest/Rust 测试清单与基线，区分移动、迁移和真实新增，确认没有静默减少。
- [x] 检查 `git diff --check`、`git status --short` 和完整 diff，确认没有产品行为、依赖或无关格式变化。
- [x] 按 Trellis Phase 3 更新必要 spec、提交并收尾；没有当前证据的检查不得写成通过。

## Risky Files And Rollback Points

- `scripts/run-vitest-sequential.mjs`：先用聚焦测试锁住递归/排序，再移动任何测试。
- `vite.config.ts`：setup 路径错误会使全套测试失去全局 mock，移动 support 后立即定向验证。
- 大量测试 import/`vi.mock()`：按目标目录分批，任何批次失败即在该批修正或回滚。
- `src-tauri/tests/common/mod.rs`：保持 integration crate 结构性豁免，不引入 Cargo feature 或扩大生产 API。
- `src-tauri/src/cli_api/mod.rs`：只删除与外部测试完全重复的 `#[cfg(test)]` 用例，不改生产实现。
