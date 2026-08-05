# 实施计划

## 0. 启动门禁

- [x] 用户批准最新 Goal / Scope / Acceptance / Design 摘要。
- [x] 运行 `python ./.trellis/scripts/task.py start .trellis/tasks/08-03-diagnose-update-check-failure`，确认状态为 `in_progress`。
- [x] 加载 `trellis-before-dev`，读取本任务涉及的 backend/frontend/quality 规范。
- [x] 从 `dev` 创建短生命周期 task 分支并将任务 `base_branch` 保持为 `dev`；不推送远端。

## 1. Red: Archive Redirect Regression Tests

- [x] 在 `src-tauri/src/services/github_import/tests.rs` 添加 production validator 正例：真实 GitHub branch 形状的 `.../legacy.tar.gz/refs/heads/{branch}` 与 pinned 40 位 commit SHA 形状的 `.../legacy.tar.gz/{sha}` 分别被接受且不可混用。
- [x] 添加 hostile matrix：scheme、userinfo、fragment、query、port、lookalike/private/loopback/link-local host、缺失/相对 Location、owner/repo/ref/path mismatch、额外 segment、编码分隔符均被拒绝且不发送第二请求。
- [x] 使用 test-only local endpoint/policy fixture 添加 `302 -> 200 tar.gz -> snapshot` 回归。
- [x] 捕获两跳 request，断言第一跳 direct request 有测试 Bearer，第二跳无 `Authorization`。
- [x] 添加第二跳 3xx 拒绝、缺失 Location、预算 cap-plus-one 与危险 archive entry 保持失败的断言。
- [x] 运行最小失败集，确认测试在实现前因当前 302 行为失败。

## 2. Green: Archive-Specific Redirect Boundary

- [x] 在 `github_import/archive.rs` 增加结构化 codeload validator 与一次性第二跳 handler；共享 client `Policy::none()` 不变。
- [x] 只在 archive acquisition 接受 302；不为 `raw_http` 的普通 surface 增加通用 allow-redirect 参数。
- [x] 第二跳从全新 request builder 创建，不转发 Bearer 或第一跳动态 header。
- [x] 复用现有 bounded response reader 和 archive extraction budgets。
- [x] 保持 direct/mirror、rate limit、404 和 transport 分类；不受信任 redirect 直接 fail closed。
- [x] 运行 archive redirect、SSRF、auth isolation 和 budget 定向 Rust 测试。

## 3. Typed Error and Safe Observability

- [x] 在 `github_import/error.rs` 添加零动态字段的 archive redirect 语义变体与稳定 `ipc_code`。
- [x] 在 `central_updates/error.rs` 透明保留经审查的 GitHub import coded error，不对其它字符串错误做 sniffing。
- [x] 调整 `skill_update_inventory.rs` 的 refresh error mapping：IPC 保留 coded envelope；Operation Log details 只增加静态 `errorCode`/`phase`；通用 Display 继续隐藏内部文本。
- [x] 在 `ipc_error.rs` 加入固定 code/message/retryable 白名单和 adversarial seed 测试。
- [x] 确认 Runtime failure recorder 获得结构化 public code/message；不新增动态 Rust error/URL tracing。
- [x] 补 command/operation-log 回归：safe code 可见，token、URL、路径、响应正文均不可见。

## 4. Frontend Code Preservation and i18n

- [x] `updateCenterStore` 用 `backendErrorStateValue` 保存 refresh rejection，保留 code、丢弃 details。
- [x] 同步 `src/lib/ipc/errors.ts` canonical code，以及 `src/i18n/locales/en.json` / `zh.json` 的 `backendErrors.github_import.*` 文案。
- [x] 补 `updateCenterStore`、`backendError`、runtime IPC、模式弹窗和 i18n parity 测试，证明 Update Center 显示本地化消息而非 `internal.unexpected` 或动态 Rust 文本。
- [x] 运行定向 Vitest、`pnpm typecheck` 与 `pnpm lint`。

## 5. Update Center Persistence and Progress

- [x] 用 redirect fixture 产出的 snapshot 覆盖 repository-backed refresh 的 inventory/state 持久化；如需注入，仅增加 private/test-only seam，不增加 runtime 配置面。
- [x] 断言成功序列含 `repository_completed`；archive typed rejection 与通用 snapshot failure 出口组合证明失败序列含 `repository_failed` 且命令失败。
- [x] 断言失败前不写入不完整 inventory run/state，既有 fail-fast 行为保持。

## 6. Specs and Generated Contracts

- [x] 更新 `github-import-preview-contract.md`、`domain-error-enums.md`、`redaction-policy.md`、`central-update-inventory-progress.md`、`async-error-feedback.md` 与 `test-suite-layout.md`。
- [x] 运行 `pnpm docs:gen`；检查并仅保留由 command/IPC 合同实际产生的生成物变化。
- [x] 运行 `pnpm docs:gen:check` 与 `pnpm docs:build`。

## 7. Validation Gates

- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked archive_redirect -- --nocapture`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked github_client_does_not_follow_redirects -- --nocapture`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked authenticated_api_fallback_does_not_forward_bearer_auth_to_mirror -- --nocapture`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked refresh_skill_update_inventory -- --nocapture`
- [x] `pnpm test -- src/test/stores/updateCenterStore.test.ts src/test/lib/backendError.test.ts src/test/runtime/ipc.test.ts src/test/contracts/i18nLocales.test.ts`
- [x] `pnpm typecheck`
- [x] `pnpm lint`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [x] `just ci`
- [x] 检查最终 diff，仅包含本任务产品代码、测试、生成文档、规范和 Trellis 资产；不包含 token、运行日志、数据库或用户 Central 数据。

## 8. Review and Rollback Points

- [x] 安全复核：production host/path policy 不可配置；无跨 host Authorization；只允许一跳；无全局 redirect 回退。
- [x] 正确性复核：合法 GitHub 302 成功、hostile redirect 拒绝、progress 结算、inventory 持久化。
- [x] 可观测性复核：IPC/Operation/Runtime/UI code 一致，所有文本经固定 message/i18n，动态数据被排除。
- [x] 若任何安全断言失败，先回滚 archive handler 到 fail closed，不使用自动 redirect 作为临时修复。

## 9. Red: 全技能结果缺失回归

- [x] 添加两技能 backend fixture：一个 GitHub-backed skill 使用缓存 snapshot，一个 unassigned skill；执行 all/skills scope 的常规检查。
- [x] 断言实现前 unassigned skill 未出现在 inventory 且 reload 丢失该技能；同时锁定 refresh 不修改 `skill_update_states`；保留一次明确 red 输出。
- [x] 添加 trigger-based rollback 测试，证明 inventory run/entries 原子提交且 baseline 不受影响。
- [x] 添加 frontend fixture，断言 unsupported tab/count/entries、preferred tab 与进度文案；保留实现前 red 输出。

## 10. Green: 全技能分类与原子持久化

- [x] 增加向后兼容的 `UnsupportedSkill` / `SkillUpdateInventory.unsupported` DTO。
- [x] refresh 聚合 computed results，把 unsupported 放入 inventory，并保持所有模式都不修改安装 baseline。
- [x] 复用 `replace_skill_update_inventory` 的现有 run/entries 事务；不扩展其写入 `skill_update_states`。
- [x] 持久化并 reload unsupported entry，不修改现有 actionable/apply 语义。
- [x] 前端增加 Unsupported tab、计数、只读条目和 preferred-tab 逻辑；同步 en/zh 文案。
- [x] 进度弹窗明确 scope 技能数与可查询去重 repository 数的不同语义。

## 11. 新缺陷验证与审查

- [x] 运行定向 inventory Rust tests，并确认 red tests 转绿。
- [x] 运行 Update Center store/page/controller Vitest、`pnpm typecheck` 与 `pnpm lint`。
- [x] DTO 变化后运行 `pnpm docs:gen`、`pnpm docs:gen:check` 与 `pnpm docs:build`，仅保留预期生成物。
- [x] 执行 `trellis-check` 独立质量复核，修复 spec、正确性、原子性、测试覆盖和兼容性发现。
- [x] 更新 central update inventory、transaction、scanner deletion、frontend async/UI test 与 test layout 规范。
- [x] 运行完整 Rust gates 与 `just ci`，检查最终 diff 不含用户 DB/Central 数据或 provenance backfill。

## 12. Red/Green: Scanner 不完整覆盖保护

- [x] 添加 Central 根目录缺失回归：首次权威扫描后写入固定 identity、repository membership、baseline 与 owned relations；移走根目录再扫描，现实现稳定删除父行并使测试失败。
- [x] 在 `ScanPersistenceBatch` 增加 `central_root_scanned` 权威覆盖标志；local/remote 仅在 Central 根真实存在且扫描成功后置位。
- [x] stale parent SQL 只在覆盖标志为真时删除缺失 Central 行；非 Central reconciliation 保持不变。
- [x] Central 根缺失时不触碰 Central agent 的 installation/observation keep scope，避免独立 cleanup 先删除关联。
- [x] 添加反向回归：存在且成功扫描为空的 Central 根仍删除真正 stale 的 Central skill。

## 13. Scanner 验证与完整门禁

- [x] 使用三个完整测试名分别验证 Central 根缺失、不可读与成功空扫描；SSH batch protocol 8/8。
- [x] 完整 `cargo test --manifest-path src-tauri/Cargo.toml --locked`（1140 passed，7 ignored）与 all-target clippy。
- [x] 最终 `just ci` exit 0；diff/sensitive-artifact 检查不含真实 DB、备份、token、Central 内容或 provenance 回填。

## 第一缺陷完成证据

- 定向回归：archive 7/7、redirect snapshot persistence 1/1、snapshot failure/no-partial-persistence 1/1、frontend 81/81。
- 完整 Rust：1118 unit tests passed，另有 release verifier、CLI E2E、projects E2E 与 doc tests 全部通过。
- 最终门禁：`just ci` exit 0，Rust lane 1126 tests，common lane 的生成物、typecheck、lint、capability、size、entrypoint、IPC、Vitest、build 与 docs 全部通过。
- 环境提示：门禁使用 Node 24.14.0 / pnpm 11.9.0，仓库声明 Node 22.x / pnpm 10.12.3；仅产生 engine warning，未跳过任何检查。

## 全技能与 Scanner 缺陷完成证据

- Inventory refresh 定向组 22/22；其中 all-scope fixture 同时证明 2 skills / 1 queryable
  repository、unsupported persistence/reload、invalid source path 不发网络请求，以及 refresh
  不修改 baseline。
- Scanner 三个完整 Central test names 各 1/1，SSH batch protocol 8/8；过滤后 0 tests
  明确不计入验收。
- Frontend 完整 Vitest：147 files，1617 passed，1 skipped；typecheck/lint 通过。
- 完整 Rust：1140 passed，7 ignored；all-target Clippy `-D warnings` 无问题。
- IPC/docs codegen/check/build 通过；最终 `just ci` exit 0，common/rust-platform lanes 全部通过。
- 当前只进入 Phase 3.4 提交审批门；未提交、未归档、未推送。

## 14. Red: Migration Checksum 与 Startup Rebuild 回归

- [x] 从 7 月 29 日恢复库提取只含 schema migration metadata 的最小 fixture：v1 legacy Windows checksum，v2-v4 canonical checksum；测试必须走生产 file open/preflight。
- [x] 断言实现前 legacy fixture 因 v1 mismatch 失败，随机 checksum 同样失败且二者都不产生备份/写入。
- [x] 添加 startup failure 回归：healthy schema initialization failure 不得提供 rebuild；corrupt database 仍可重建。
- [x] 添加前端 StartupRecoveryView 回归：`canRebuild=false` 时不渲染重建按钮。

## 15. Green: 显式兼容与有损操作保护

- [x] 为 migration descriptor 增加版本绑定的 legacy checksum alias 匹配；新 metadata 继续只写 canonical checksum。
- [x] `attempt_startup` 仅在 integrity diagnostic 为 corrupt 时设置 `can_rebuild=true`；保持 typed status 与 redaction。
- [x] 更新 migration/startup specs 与 break-loop retrospective，记录已发布 checksum 不可替换、健康数据库不得用重建绕过兼容错误。

## 16. 只读 Provenance 恢复预览

- [x] 对当前库与指定 startup recovery 库执行 `quick_check`、FK 和稳定 ID join；记录 addable/already-same/conflict/missing-parent/unresolved 计数。
- [x] 本次预览验收固定为 111 addable、0 already-same、0 conflict、0 missing-parent、23 unresolved；7 个重建后新导入且已有 membership 的技能不进入 unresolved。
- [x] 不写真实数据库、WAL/SHM、恢复目录或 Central 文件；真实 apply 保留为用户明确审批后的独立门。

## 17. 重新验证

- [x] 运行 focused migration/startup Rust tests 与 StartupGate/StartupRecoveryView Vitest。
- [x] 运行 `cargo fmt --all -- --check`、all-target locked Clippy、locked Rust tests、完整 frontend tests/typecheck/lint。
- [x] 运行 IPC/docs 只读检查和最终 `just ci`；复核 diff/sensitive paths 后再回到提交审批门。

## 18. 已批准 Provenance 恢复

- [x] 记录用户明确批准范围：仅恢复 111 条无冲突 membership 与其引用的 23 个 GitHub repository，保留当前 7 条 membership 和 23 个 unresolved 技能。
- [x] 确认 SkillPort 未运行、上次失败目标不存在；重新运行只读 preview，确认 111 addable / 0 already-same / 0 conflict / 0 missing-parent / 23 unresolved，且两侧语义摘要与前次证据一致。
- [x] 创建新的 DB/WAL/SHM 完整备份并以只读 preview 验证为 141 Central skills / 7 memberships / 1 populated GitHub repository、`quick_check=ok`、FK violation 0、当前库摘要一致。
- [x] 实现并通过一次性 apply 工具回归：摘要漂移、repository metadata 冲突、备份不匹配或计数不精确时不得写入；成功路径必须在一个 `BEGIN IMMEDIATE` 事务中插入 repository、membership 与脱敏审计行。
- [x] 应用事务并确认恰好新增 111 条 membership、23 个 repository，原 7 条 membership 逐字段不变；不写 projects/settings/tags/baselines/Central 文件。
- [x] 提交后验证 118 total memberships、24 populated GitHub repositories、23 unresolved，当前库 `quick_check=ok`、FK violation 0，恢复源仍 `quick_check=ok` 且摘要不变。
- [x] 更新 AC22 与验收证据，运行恢复工具测试、Trellis validate 和 `git diff --check`，保持未提交、未归档、未推送。

## 19. Red/Green: GitHub Canonical Redirect

- [x] 建立真实 24-repository 禁跳反馈环并稳定复现：4 个 case-only codeload mismatch、2 个 direct API 301 numeric canonicalization；最小化第二类为 `301 /repositories/{id}/tarball/{ref} -> 302 canonical codeload`。
- [x] 添加 production validator red tests：owner/repo case-only canonicalization 接受；numeric API target 只接受 direct `api.github.com:443`、正十进制 ID、same ref、无 userinfo/query/fragment/extra segment，mirror 不能授权。
- [x] 添加三请求 transport red test：initial direct 和 numeric API 请求含测试 Bearer，最终 codeload 无 Bearer并产出 snapshot；canonical owner/repo 可与旧 identity 不同。
- [x] 实现显式 redirect state machine，不改变共享 `Policy::none()`、普通 API/raw、mirror fallback、资源预算、typed error/redaction 或 repository persistence。
- [x] 添加 hostile chain 回归：numeric 后非 302、codeload 后 3xx、非 trusted initial 301、重复/缺失 Location 均以 `ArchiveRedirectRejected` 终止且不继续请求。
- [x] 运行 archive redirect 定向 Rust tests、真实 24-repository feedback loop、相关 inventory 回归、fmt、locked all-target Clippy/tests 与最终 `just ci`；更新 R12/AC23/spec 和完成证据。
