# Implementation Plan: 风险导向测试覆盖补强

## 1. Baseline And Guardrails

- [ ] 读取相关 backend/frontend/quality specs 与两份 research 报告。
- [ ] 记录初始 `git status`，保留所有无关改动。
- [ ] 运行现有聚焦测试，确认过滤表达式实际发现非零测试。

## 2. Critical Module A — Portable Import Terminal State

- [ ] 在 `centralSkillsStore.test.ts` 增加 post-import refresh rejection。
- [ ] 增加 reset/target generation change 后旧 refresh completion/error 隔离。
- [ ] 若回归失败，实施最小 terminal-state/correlation 修复。
- [ ] 运行：`pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "portable state"`，确认非零。

## 3. Critical Module B — AI Settings Partial Failure

- [ ] 增加 secret save success + settings save failure。
- [ ] 增加 pre-switch flush rejection。
- [ ] 递归断言 store/error/ordinary settings payload 不含 secret sentinel。
- [ ] 若回归失败，实施最小 secret-clearing/terminal-state 修复；不增加尚未定义的并发保存语义。
- [ ] 运行：`pnpm exec vitest run src/test/stores/settingsStore.test.ts`。

## 4. High Module C — Project Install/Uninstall Consistency

- [ ] 在 `services::projects::tests` 增加 install DB-write failure 与 uninstall DB-delete failure。
- [ ] 断言 FS/DB/canonical source 的完整前后状态。
- [ ] 若回归失败，实施最小 validation/ordering/compensation 修复。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked services::projects::tests`。

## 5. High Module D — Target Mutation And Credential State

- [ ] 后端增加 Local/空/未知 target ID 零写入边界，以及 target deletion/active-target persistence failure rollback 测试。
- [ ] 前端 table-drive create/update/test/password/delete/switch 首命令 rejection，断言 loading 清理、existing list/active target 保持、error 与 rethrow。
- [ ] 前端增加 create/delete/switch mutation 成功后 `list_targets` refresh 失败，断言明确 error/reload-required 语义并避免把 mutation 标为未发生。
- [ ] 对密码型成功与失败路径递归断言 Zustand state、错误和普通诊断不含 secret sentinel。
- [ ] 若回归失败，实施最小 transaction/compensation 或 store terminal-state 修复；不改变 connection-test 成功载荷契约。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked targets::tests` 与 `pnpm exec vitest run src/test/stores/targetStore.test.ts`，确认均非零。

## 6. High Module E — Repository Sync Validation And Transaction

- [ ] 增加 heterogeneous decisions 中的 late invalid path 测试。
- [ ] 增加第二条 skip/unskip write trigger failure 与 retry 测试。
- [ ] 断言 membership、update state、skip rows 零 partial write。
- [ ] 若回归失败，实施最小 prevalidation/transaction 修复。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates::repository_sync`，确认非零。

## 7. Frontend Gate

- [ ] 运行：`pnpm typecheck`。
- [ ] 运行：`pnpm lint`。
- [ ] 运行：`pnpm test`。

## 8. Backend Gate

- [ ] 在 `src-tauri` 运行：`cargo fmt --all -- --check`。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked`。

## 9. Independent Check And Completion Gate

- [ ] 派发 `trellis-check` 独立检查 spec compliance、回归质量、测试盲区与无意义覆盖。
- [ ] 根据同范围反馈修正并重跑受影响聚焦测试。
- [ ] 运行最终 `just ci`。
- [ ] 检查 `git diff --check` 与最终 `git status`。
- [ ] 分别记录通过、失败、跳过、零测试过滤和外部/原生环境 `UNVERIFIED` 证据。

## Risky Files / Rollback Points

- `src/stores/centralSkillsStore.updateSlice.ts`：已提交 import 后的 terminal state 与 stale writes。
- `src/stores/settingsStore.aiSlice.ts`：secret/ordinary settings 两阶段写。
- `src-tauri/src/services/projects/crud.rs`：FS 已变而 DB 失败的补偿边界。
- `src-tauri/src/targets/commands.rs` 与 `src/stores/targetStore.ts`：credential、settings 与 mutation 后 refresh 结果。
- `src-tauri/src/services/central_updates/repository_sync.rs`：跨 decision 批次的 validation/transaction 边界。

任何修复若需要 schema migration、新依赖、全局权限模型、尚未定义的公开载荷/并发语义或跨模块重构，停止并回到规划，而不是扩大本任务。

## Validation Evidence

- Frontend focused: portable state 8 passed / 49 skipped; settings store 35 passed; target store 23 passed; SettingsView 103 passed.
- Rust focused: projects 33 passed; targets 59 passed; repository sync 2 passed; every filter discovered nonzero tests.
- Full frontend: 177 files, 1981 passed / 1 skipped.
- Full Rust locked suite: 1506 passed / 7 ignored.
- Final repository gate: `just ci` passed after fixing one TargetState fixture drift and moving target settings rollback into `targets/config.rs` to satisfy the 800-line production-source budget.
- `task.py validate` and `git diff --check` passed; no dependency, schema, generated-doc, capability, or entrypoint drift was introduced.
- UNVERIFIED: real system keyring, SSH/WSL hosts, Tauri native GUI, symlink failure injection under restricted Windows privileges, provider behavior, and process-crash recovery windows.
