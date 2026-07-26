# 实施计划：Request-scoped TargetContext

## 1. 激活与规范加载

- [ ] `python ./.trellis/scripts/task.py start 07-24-target-context-snapshot`
- [ ] 加载 `trellis-before-dev`，阅读 backend index、domain errors、transport seam、test support。
- [ ] 记录并保护现有 Trellis runtime/tooling 与其他子任务规划改动。

## 2. Context 基础设施

- [ ] 在 `targets` 层新增拥有型 `TargetContext` 与只读访问器、Clone/Debug 实现。
- [ ] 新增 `TargetRegistry::db_for_target`，只根据显式 `ActiveTarget` 选择 pool。
- [ ] 新增 `TargetRegistry::resolve_active_context`，active ID 只读取一次。
- [ ] 新增 `AppState::resolve_target_context`；旧 helper 在 spec 中标记为迁移期 API，旧 `active_db` 委托 resolver；Rust `#[deprecated]` 属性延后到兼容调用清零。
- [ ] 为 local/SSH/WSL resolver、pool identity、切换后 context 稳定性补单元测试。

## 3. P1 Command 迁移

- [ ] 迁移 GitHub preview/import/discard workspace 路径；service helper 接受显式 target/context 数据。
- [ ] 迁移 Central Update 与 skill update inventory 的 DB/FS/log identity。
- [ ] 迁移 portable state export/preview/import。
- [ ] 迁移 scanner 与 skills delete/update/link flows。
- [ ] 迁移 agents/settings 中同时需要 target 与 DB 的路径。
- [ ] 删除 ad-hoc SSH/WSL operation-log ID helper，统一真实 target ID/label。

## 4. 静态与竞态回归

- [ ] 增加 architecture/grep test：同一 production command 不得组合调用 `active_target()` 与 `active_db()`。
- [ ] 增加 local↔SSH、SSH-A↔SSH-B、SSH↔WSL barrier matrix。
- [ ] 断言 context A 的 target、cache DB marker、log identity、模拟 event payload 在切换 B 后不变；新 context 为 B。
- [ ] 断言两个 SSH target 的 operation log ID/label 可区分且 kind 仍兼容。

## 5. 分层验证

- [ ] `cd src-tauri; cargo test targets --locked`
- [ ] `cd src-tauri; cargo test operation_log --locked`
- [ ] 分模块运行 github_import、central_updates、portable_state、scanner、skills tests。
- [ ] `cd src-tauri; cargo fmt --all -- --check`
- [ ] `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`
- [ ] `cd src-tauri; cargo test --locked`
- [ ] `just ci`

## 6. 文档、检查与回滚

- [ ] 新增 backend target-context spec，同步 architecture docs 与 domain-error helper 描述。
- [ ] 用 `rg` 审计全仓旧 helper 组合与 ad-hoc target ID。
- [ ] 运行 `trellis-check`，检查 diff 未混入其他子任务产品改动。
- [ ] 若某模块迁移回归，可回滚该模块到单次 `resolve_target_context` 之前的签名；不得恢复双重 active ID 解析。
- [ ] 提交工作改动，归档本子任务，并在父任务中登记完成与后续依赖解锁。
