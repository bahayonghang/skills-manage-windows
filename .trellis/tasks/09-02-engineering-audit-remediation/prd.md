# 工程级审计整改总纲

## Goal

承接 2026-09-02 对 `dev@7c2134ce` 的 deep whole-project 工程审计，将已确认的正确性、安全、发布、架构与质量证据问题拆成可独立实施和验证的子任务。本父任务不直接修改产品代码，只维护 finding ledger、子任务映射、顺序约束和最终集成复核。

## Audit Scope And Evidence Boundary

- 已审：`src/`、`src-tauri/src/`、SQLite schema/repos、`.github/`、`scripts/`、`.agents/`、`.codex/`、`.claude/`、`.trellis/scripts/`、根构建与质量配置。
- 排除：`ref/` 第三方参考、生成/缓存/构建目录、历史 `.trellis/tasks/archive/**` 的逐项审查、真实 provider/SSH/WSL/安装器/生产发布执行。
- 当前证据：离线 Rust Clippy 通过；Rust tests `1521 passed / 7 ignored`；版本、生成物字节检查、Node 脚本语法、Rust fmt、sizecheck、entrypointcheck 与离线 Cargo metadata 通过。
- `missing evidence`：当前 `node_modules` 不完整，TypeScript/Vitest/ESLint/Tauri CLI 未验证；未联网刷新 npm/Cargo advisory；未执行真实 Windows bundle、签名 provider、NSIS/MSI、WebView2、SSH/WSL、GitHub/AI provider。

## Finding Ledger

按严重度降序、同严重度内按预估修改成本 `S → M → L` 排序。

| ID | 严重度 | 成本 | 证据锚点 | 结论 | 子任务 |
|---|---|---:|---|---|---|
| BE-CORR-001 | Critical | M | `src-tauri/src/services/github_import/remote.rs:426,484,544` | **fixed**：远程/本地覆盖走 journaled apply，DB 失败可恢复 backup | `github-import-fs-db-atomicity` |
| SEC-002 | High | S | `.trellis/scripts/common/task_store.py:296-347,419-437` | **fixed**：`resolve_contained_path` 拒绝穿越 `.trellis/tasks` | `trellis-path-security` |
| REL-002 | High | S/M | `.github/workflows/release-desktop.yml:97-168` | **wontfix（contract-evidenced）**：R1 未过，用户拒绝范围内全部 workflow 实现；updater 密钥/OIDC 未收窄；残留风险仍在。见 `research/rel-001-002-wontfix-contract-2026-09-03.md`。**不是 fixed** | `windows-release-signing` |
| FE-CORR-001 | High | S | `src/components/layout/GlobalSearchDialog.tsx:142-159`; `src/App.tsx:119-123` | **fixed**：搜索命中导航 `/collections` | `collection-search-correctness` |
| BE-CORR-004 | High | S | `src-tauri/src/services/usage/fs_backend.rs:240`; `services/usage/mod.rs:301`; `db/repos/usage_repo.rs:131` | **fixed**：`exists` 返回 `Result`；target-fatal 先于空替换 | `usage-refresh-failure-integrity` |
| SEC-001 | High | M | `.trellis/scripts/common/task_context.py:77-85`; `.codex/hooks/inject-subagent-context.py:242-248,428-443,605-622` | **fixed**：manifest 路径 containment；gitignore 钩子 fail-closed | `trellis-path-security` |
| REL-001 | High | M | `.github/workflows/release-desktop.yml:156-246,316-336` | **wontfix（contract-evidenced）**：CLI 2.11.4 无法证明 bundler 消费 Authenticode 前任 digest；用户拒绝包内替换/自制 bundler；残留风险仍在。见 `research/rel-001-002-wontfix-contract-2026-09-03.md`。**不是 fixed** | `windows-release-signing` |
| FE-CORR-002 | High | M | `src/stores/collectionStore.ts:177-203`; `src/pages/CollectionsListView.tsx:181-186` | **fixed**：详情 latest-wins / 单调选择 | `collection-search-correctness` |
| BE-CORR-002 | High | M | `src-tauri/src/services/github_import/import.rs:604,666-684` | **fixed**：journaled upsert；恢复失败可观察 | `github-import-fs-db-atomicity` |
| BE-CONC-001 | High | M | `src-tauri/src/services/github_import/remote.rs:321,396` | **fixed**：远程导入持有共享 target mutation guard | `github-import-fs-db-atomicity` |
| FE-ARCH-001 | Medium | S | `src/stores/updateCenterStore.ts:20-21`; `src/lib/updateCenterRefreshScope.ts:7`; `src/pages/centralUpdateCheckMode.ts:1-3` | **fixed**：`UpdateCheckMode` 迁入 `src/lib/updateCheckMode.ts`，lib/store 不再 import pages | `frontend-boundary-cleanup` |
| FE-ARCH-003 | Medium | S | `src/lib/explanationStream.ts:2`; `src/stores/projectsStore.ts:2` 等 | **fixed**：生产 `@tauri-apps/api/event` 仅 `invoke.ts`；`UnlistenFn` 从 `@/lib/ipc` 导出 | `frontend-boundary-cleanup` |
| ARCH-001 | Medium | S | `src-tauri/src/commands/mod.rs:31` 及 6 个 `services/*` 调用点 | **fixed**：`APP_USER_AGENT` 在 `http_identity`；生产 services→commands 为 0 | `backend-boundary-ratchets` |
| TOOL-001 | Medium | S | `.codex/hooks/inject-subagent-context.py:276-299,428-447` | **fixed**：UTF-8 ledger 与 JSONL 行上限截断 | `subagent-runtime-resilience` |
| TOOL-003 | Medium | S | `.trellis/scripts/common/git.py:41-55`; `task_store.py:351-367` | **fixed**：远端探测有 timeout；失败不阻塞 create | `subagent-runtime-resilience` |
| QUAL-003 | Medium | S | `scripts/check/check-dependency-audit.mjs:91-167,204-206` | **fixed**：非阻断 advisory 作为 evidence；blocker 语义不变 | `dependency-audit-observability` |
| FE-CORR-003 | Medium | M | `src/components/layout/GlobalSearchDialog.tsx:66-120`; `src/components/layout/AppShell.tsx:76-85` | **fixed**：搜索前确保 collections/Central `hasLoaded` | `collection-search-correctness` |
| FE-ARCH-002 | Medium | M | `src/pages/CollectionView.tsx:1` 等 4 个模块 | **fixed**：4 个不可达模块与 2 个孤立测试已删除，无 wrapper；`/collections` 仍走 `CollectionsListView` | `frontend-boundary-cleanup` |
| TOOL-002 | Medium | M | `.trellis/scripts/common/task_utils.py:242-280`; `hooks/linear_sync.py:90-104` | **fixed**：hook/subprocess 走 `run_bounded_process` | `subagent-runtime-resilience` |
| QUAL-001 | Medium | M | `scripts/docs/build-schema-table.mjs:20-103` | **fixed**：生成器归并 ALTER/UNIQUE INDEX 最终状态 | `generated-schema-evidence` |
| QUAL-002 | Medium | M | `.github/workflows/release-desktop.yml:263,316-336` | **fixed（contract + fixture）**：NSIS/MSI smoke matrix 与 `windows-installer-smoke.ps1` 已接线；真实 windows-2022 install/launch/uninstall 仍 UNVERIFIED；不证明 REL-001 | `windows-installer-verification` |
| ARCH-002 | Medium | L | `src/lib/ipc/invoke.ts:45-53`; `src/lib/ipc/commandMap.ts:389-442` | **fixed**：剩余 47 个命令已进入 generated map；生产 string overload 已删除 | `typed-ipc-remainder` |
| ARCH-003 | Medium | L | `src-tauri/src/db/mod.rs:7-58`; 87 个非测试直接 import | **fixed（渐进）**：三领域宽函数调用 174→0；历史债务 388 no-growth；未全库清零 | `backend-boundary-ratchets` |
| FE-I18N-001 | Low | S | `src/stores/settingsStore.aiSlice.ts:467-472`; `src/pages/MarketplaceView.tsx:167-177` | **fixed**：AI/Marketplace browser fixture 文案走 `settings.aiTestBrowserUnavailable` 与 `marketplace.previewUnavailable*` | `frontend-boundary-cleanup` |
| TOOL-004 | Low | S | `.agents/skills/trellis-spec-bootstarp/SKILL.md:2-8` 与正确拼写副本 | **fixed**：旧拼写副本移除/不可发现 | `subagent-runtime-resilience` |
| QUAL-SIZE-001 | Medium | S | `src-tauri/src/services/central_updates/core/batch.rs` | **fixed**：从 807 行拆出 `batch/commit_fault.rs`；当前 `batch.rs` 775 行（预算 800） | `github-import-fs-db-atomicity` |
| REL-003 | Low | S | `.github/workflows/release-desktop.yml:46,288,326` | **fixed**：`release-context` `timeout-minutes: 15`；`windows-install-smoke` `timeout-minutes: 20`；resolver/`run()` 与 smoke helper 均有 deadline | `windows-installer-verification` |
| REL-004 | Low | S | `.github/workflows/release-desktop.yml:46-88`; `scripts/release/release-context.mjs:63` | **fixed**：Node 26 setup+assert 先于第一次 `release-context.mjs`；Rust 1.98.0 setup+assert 先于完整 resolver | `windows-installer-verification` |
| ARCH-004 | Low | M | `src/types/index.ts:1`；生产 importer 基线待按 child 约定命令重测 | **fixed（no-growth）**：基线 199，复跑 191；未全量拆 barrel | `frontend-boundary-cleanup` |

## Requirements

- R1：每个 finding 只由一个主责子任务闭环；child 必须用自身 R/AC 声明所承接 ID，跨子任务依赖写入 child PRD/implement。
- R2：Critical/High 子任务先于 Medium/Low；`windows-installer-verification` 的最终资产验收依赖 `windows-release-signing` 的签名链稳定。
- R3：不以宽松 validator、legacy fallback 或忽略错误方式消除失败；边界必须 fail closed，恢复失败必须可观察。
- R4：业务代码、任务激活、提交、归档、PR、merge 与 push 均不属于本轮授权；所有任务保持 `planning`。
- R5：外部 provider、真实远程、正式签名与安装器结果必须保留 `UNVERIFIED`，直到对应直接证据实际取得。

## Acceptance Criteria

- [x] AC1（R1）：12 个 child 均有可测试 PRD、design、implement 与真实 implement/check context；ledger 每个 ID 可追溯到一个 child R 和一个 child AC。
- [x] AC2（R2, R3）：产品修复的 Critical/High 均有失败回归。REL-001/REL-002 **不是产品修复**：以 R1 FAIL + 用户拒绝选项 2/3 作为 contract-evidenced `wontfix`（`research/rel-001-002-wontfix-contract-2026-09-03.md`）；未改签名顺序、未放宽 validator。
- [x] AC3（R1, R4）：Medium/Low finding 均由对应 child 闭环；QUAL-002 的真实 runner 生命周期仍 UNVERIFIED，不把 REL-001 标成 fixed。
- [x] AC4（R2）：12 个 child 已归档后，父任务运行 `just ci`、`just audit`、`pnpm docs:gen:check`；扫描复跑与 REL 收口见 `research/`。独立 trellis-check 在 REL contract 后为 PASS。
- [x] AC5（R5）：`research/integration-2026-09-03.md` 与 `research/rel-001-002-wontfix-contract-2026-09-03.md` 列出 passed / wontfix / failed / skipped / missing evidence；真实 Windows/provider/remote 不得由 fixture 推断；不得把 REL-001 标成 fixed。

## Out Of Scope

- 本父任务直接实现任何修复或一次性同时启动多个 child。
- 安装新依赖、联网刷新 advisory、访问真实凭据、发布、push 或生产变更。
- 无证据的全仓重写；DB facade、types barrel 与 Typed IPC 均采用有基线的渐进 ratchet。
