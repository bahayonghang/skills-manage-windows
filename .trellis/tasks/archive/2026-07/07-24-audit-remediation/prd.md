# 极限审计整改总纲：系统级不变量收口

## Goal

承接 `skills-manage-windows-extreme-review-2026-07-24.md`（GPT Pro 审计，基准 main@35a3174）的整改父任务：管理 24 项发现（P1×10 / P2×11 / P3×3）的任务映射、跨子任务验收与集成复核。本父任务不承担直接实现，只负责需求源、子任务地图与最终集成复核。

## 逐条核对结论（2026-07-24，dev 分支上复核）

24 项发现全部在当前 dev 分支代码上核实，无一失效。逐条证据：

| 审计 ID | 核对结论 | 当前代码证据 |
|---|---|---|
| P1-01 | ✅ 成立 | `src-tauri/src/lib.rs:73-85` 的 `active_db`/`active_target` 两次独立解析；`targets/registry.rs:359,405` |
| P1-02 | ✅ 成立 | `capabilities/default.json` 仍含 `shell:default`、`$HOME/**`、`fs:allow-read/write-text-file`；`reveal_ai_api_key`/`reveal_github_pat` 命令存在 |
| P1-03 | ✅ 成立 | `services/github_import/raw_http.rs:61-65` 非 raw URL 原样请求；`pat.rs:38-48` client 仅设 user-agent；`raw_http.rs:86-94` 仅 content_length 预检后 `.bytes()` 全量缓冲 |
| P1-04 | ✅ 成立 | `services/central_skills/files.rs:463-519` 远端仅词法包含 + 终点 symlink 检测；`targets/exec.rs` 无 realpath（本地版 `files.rs:428-461` 有 canonicalize，远端无对等语义） |
| P1-05 | ✅ 成立 | `db/repos/skills_repo.rs:619-698` 两分支均遗漏 `collection_skills`/`skill_ai_tag_reviews`/`skill_explanations`；`delete_skill`(574-609) 多语句直连 pool 无事务；FK 仅 `schema/projects.rs:44`、`schema/marketplace.rs:48` 两处 |
| P1-06 | ✅ 成立 | `services/central_skills/delete.rs`：FS 删除（29/34/69/93）在 `db::delete_skill`（359/579）之前，无补偿协议 |
| P1-07 | ✅ 成立 | `targets/runner.rs:9,42,65` async 门面下同步 `std::process` `.output()`/`wait_with_output()` |
| P1-08 | ✅ 成立 | `.github/workflows/release-desktop.yml` 触发条件为 `release: types: [published]` |
| P1-09 | ✅ 成立 | `commands/settings.rs:17` `PROTECTED_SETTINGS_KEYS` 仅 2 个 secret key；`targets/model.rs:2-4` target 配置为普通 settings key |
| P1-10 | ✅ 成立 | `services/github_import/raw_http.rs:131-136` `GitHubRepoRef` 仅 owner/repo/branch/normalized_url，无 commit SHA/digest |
| P2-01 | ✅ 成立 | `lib.rs:57,64` 两个共享 `AtomicBool`（注释明说 reset false on entry）；`lib.rs:90-119` `AiTagJobRegistry` 已有 per-job 模式可参照 |
| P2-02 | ✅ 成立（范围扩大） | `CentralStatePortabilityDialog.tsx:189-211` 直接 read/writeTextFile；**新发现** `MarketplaceView.tsx:212-215` 也直接 writeTextFile 到 HOME |
| P2-03 | ✅ 成立 | `db/schema/` 40 处 `CREATE IF NOT EXISTS`/`PRAGMA table_info`，无 `schema_migrations` 表 |
| P2-04 | ✅ 成立 | `ci.yml` PR/push 完整链仅 `windows-2022`（:25,:70）；macos-14/matrix 仅 release 段 |
| P2-05 | ✅ 成立（微调） | `src/lib/ipc/commandMap.ts:254` `UNTYPED_IPC_COMMANDS` 现为 **104** 项（审计时 105） |
| P2-06 | ✅ 成立 | workflows 全部使用 `@v4/@v5/@v2` 可移动 tag；无 cargo audit/deny、pnpm/OSV audit、CodeQL |
| P2-07 | ✅ 成立 | `lib.rs:259-276` 启动 `expect` ×3 |
| P2-08 | ✅ 成立 | `commands/skill_update_inventory.rs:341-342` 记录字面 `"ssh"`/`"wsl"` |
| P2-09 | ✅ 成立 | `docs/reference/ipc-capability-inventory.md:19` 称 `shell:default` removed，`capabilities/default.json:11` 仍存在 |
| P2-10 | ✅ 成立 | `central_migration.rs:99-131` 直接 `std::fs`；由 `lib.rs:322-324` `async_runtime::spawn` 启动，未走 `run_blocking_fs_with`，未纳入 mutation 协调 |
| P2-11 | ✅ 成立（风险接受项） | `targets/exec.rs:186` `StrictHostKeyChecking=accept-new` |
| P3-01 | ✅ 成立 | `scripts/check-size-budget.mjs:10-16` 五个例外 861/1033/810/865/840，与审计完全一致 |
| P3-02 | ✅ 成立 | `targets/exec.rs` SSH/WSL run/read/write/copy 等重复实现 |
| P3-03 | ⚠️ 成立但需决策 | `Result<T, String>` 是 `.trellis/spec/backend/domain-error-enums.md` 明文约定的 commands 层契约；审计建议的结构化 `IpcError` 与现有 spec 冲突，须先做架构决策再动 |

## 子任务地图（16 个）

| 子任务 | 覆盖审计项 | 优先级 | 依赖 |
|---|---|---|---|
| 07-24-net-boundary-ssrf | P1-03 (QW-01) | P1 | 无（可立即做） |
| 07-24-renderer-capability-min | P1-02, P2-02, P2-09 (QW-02/06) | P1 | 无 |
| 07-24-target-context-snapshot | P1-01, P2-08 (M-01, QW-07) | P1 | 无（多数中期项的前置） |
| 07-24-remote-process-supervisor | P1-07 (M-02)；关联 P3-02、P2-11 | P1 | 建议后于 target-context-snapshot |
| 07-24-remote-path-canonical | P1-04 (M-03) | P1 | 可与 supervisor 并行 |
| 07-24-db-stale-cleanup-fix | P1-05 (QW-03) | P1 | 无（schema-versioning 的前置） |
| 07-24-db-schema-versioning-fk | P2-03 (M-04) | P2 | db-stale-cleanup-fix |
| 07-24-fs-db-operation-journal | P1-06 (M-05) | P1 | target-context-snapshot、db-schema-versioning-fk |
| 07-24-job-concurrency-lease | P2-01, P2-10 (QW-04) | P2 | 无 |
| 07-24-release-pipeline-gate | P1-08 (QW-05) | P1 | 无 |
| 07-24-ci-supply-chain | P2-04, P2-06 (QW-08) | P2 | 无 |
| 07-24-settings-domainization | P1-09 (M-07) | P1 | 无 |
| 07-24-github-preview-snapshot | P1-10 (L-02 初步) | P1 | 无 |
| 07-24-startup-resilience | P2-07 | P2 | 无 |
| 07-24-typed-ipc-migration | P2-05 (M-06)；P3-03 决策点 | P2 | 建议后于 settings-domainization |
| 07-24-size-budget-debt | P3-01 (L-05) | P3 | 无 |

## 需要用户/架构决策的事项（不自动执行）

1. **P2-11 host key TOFU**：`accept-new` 是明确的产品取舍。是否引入首连 fingerprint 确认 UX，由用户决策后再排期（暂记录在 remote-process-supervisor 的非目标里）。
2. **P3-03 IpcError 结构化**：与现有 `domain-error-enums.md` spec 冲突。若采纳，需先改 spec 再动代码；决策点挂在 typed-ipc-migration。
3. **P2-02 扩大范围**：MarketplaceView 的 plugin-fs 直写是审计未列出的同类问题，已并入 renderer-capability-min 范围。

## Requirements

- 每个子任务在 `task.py start` 前完成自身 prd（复杂子任务另需 design.md + implement.md）。
- 子任务实现必须遵守现有 spec：`domain-error-enums.md`（分层错误契约）、`spawn-blocking-io.md`（重 IO 包装）；与审计建议冲突处以 spec 决策优先。
- 审计报告原文件保留为需求源，不随子任务修改。

## Acceptance Criteria

- [x] 10 项 P1 全部由对应子任务闭环（审计 §11 将 P1-01/02/03/04/05/07/08 列为下一正式 release 阻断项）
- [x] P1-06/P1-10 若未完成完整 Saga/content-addressing，至少落地审计 §11 列出的降级要求（operation exclusivity、durable backup、immutable preview token、partial-failure UI）
- [x] P2 项全部闭环或建立带 owner 的显式风险接受记录
- [x] 集成复核：跨子任务回归（target race 测试、SSRF 矩阵、remote path 矩阵、DB integrity、release 演练）全部通过
- [x] 相关 spec 文档（capability inventory、错误契约、IO 契约）与最终实现无漂移

## Notes

- 审计 §4 列出的现有良好实践（archive 安全矩阵、deep link parser、scanner transaction、central mutation lock）是子任务实现时应复用的既有模式，不要另起炉灶。
