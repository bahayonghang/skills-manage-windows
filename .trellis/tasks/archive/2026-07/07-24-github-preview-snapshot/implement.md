# 实施计划：GitHub immutable preview snapshot

## 1. 激活与实现前规范

- [x] 用户批准最终 planning summary 后运行 `python ./.trellis/scripts/task.py start 07-24-github-preview-snapshot`。
- [x] 加载 `trellis-before-dev`，阅读 manifest 中的 backend/frontend/quality spec 与 live research。
- [x] 复核 `git status`，仅把本子任务产品代码、spec 和 task artifacts 纳入后续提交；保护父任务、兄弟任务、Trellis runtime、`.gitattributes` 与审计报告。

## 2. Digest 与统一 registry

- [x] 新增稳定 digest v1 helper：safe normalized path、UTF-8 byte 排序、u64 big-endian length framing、per-file SHA-256、repository/candidate domain separator。
- [x] 将现有 remote workspace registry 演进为 Local/Remote enum-backed snapshot registry，保存 target/repo/source/commit/manifest/expiry/storage。
- [x] 实现 lookup/prune/discard 与单 import lease；失败释放、成功消费、lease 中 discard 延迟清理。
- [x] 为 digest 顺序、framing、tamper、TTL、binding、busy/consume/retry 状态机补纯测试。

## 3. Preview acquisition 与 DTO

- [x] Local tree/raw 与 archive acquisition 返回 resolved commit SHA 和 retained bounded snapshot；只从该 snapshot 生成 manifest/digests/candidates。
- [x] SSH/WSL supervised protocol 返回 resolved `HEAD` 与 path/length/SHA-256 manifest；校验通过后才注册，失败清理 workspace。
      （commit 由 `/repos/{owner}/{repo}/commits/{ref}` 统一解析，remote 用该 pinned commit 拉 tarball，两种 transport 的 commit 语义一致。）
- [x] 扩展 Rust/TypeScript `GitHubRepoPreview` 与 file DTO：必填 `previewId`、`resolvedCommitSha`、`snapshotDigest`、`expiresAt` 和 file `sha256`。
- [x] 保持 plugin grouping、root/nested source mapping、resource budgets 与 tree/archive parity，不把 raw token/path 序列化到非 discard/import/read payload。

## 4. Snapshot-only reads、import 与 provenance

- [x] `fetch_github_skill_markdown` 改为必填 `previewId`，校验 target/repo/source/candidate 后只读 registered storage，并复核文件 digest。
- [x] `import_github_repo_skills` 改为必填 `previewId`，删除 Local re-download 和 Remote missing/expired fallback，mutation 前完成 lease、binding、selection 与 digest 校验。
- [x] 复用现有 staging/mutation guard/rollback/partial import；失败保留 snapshot 供重试，成功原子消费并清理；并发 import fail closed。
- [x] 新增 immutable migration v4 与 checksum，给 `skill_repository_members` 追加 nullable commit/digest；不修改 v1-v3 history。
- [x] 扩展共享 repository transaction，在 skill upsert/repository assignment 同一 transaction 写 provenance；覆盖 overwrite/rename/skip 与 Local/SSH/WSL parity。

## 5. IPC、store、UI 与兼容调用方

- [x] 更新 command map、store/action types、fixtures 与 mocks，统一 `previewId`，删除 `previewWorkspaceId` optional fallback。
- [x] reset、新 preview 替换、target change 和 wizard close 都显式 discard token；成功 import 清空已消费 preview，失败保留供重试。
- [x] wizard 显示短 commit SHA 与本地化 expiry；稳定 backend code 映射为中英文重新预览提示，不展示 token/path/content。
- [x] 迁移 Marketplace official install、deep-link/import intent 与 Central repository sync/update 调用方；所有 preview-import 调用必须取得真实 token，禁止传 null 或伪造值。
      （Central sync/update 与 portable state/CLI 改为走各自已验证的 inventory + 自建 workspace 的 `import_github_repo_skills_remote_with_auth`/`..._with_auth`，`CentralRepositoryAddedSkillSelection.previewWorkspaceId` 已从 Rust/TS DTO 删除，不再伪造 token。）
- [x] 用 `rg` 审计 `previewWorkspaceId`、optional import payload、Local markdown HTTP fallback、Remote fresh-workspace fallback 和 raw error/log 泄漏为零。

## 6. 分层测试

- [x] Rust digest/registry/error 单元测试：稳定排序、length framing、missing/expired/mismatch/tamper/busy、failed retry、success consume。
- [x] Local acquisition/import 测试：preview 后 branch bytes 改变仍导入旧 snapshot，Markdown 同 digest，import 无二次 download。
      （`preview_import_module_cannot_acquire_repository_content` 结构性锁定 import 侧不存在任何 acquisition 入口。）
- [x] SSH/WSL FakeRunner parity：resolved commit、manifest hash、workspace-only read/import、expiry/discard/reset cleanup、无 fallback；保留现有 remote protocol 安全断言。
      （FakeRunner 覆盖 remote inventory 单次 `run_script` 与脚本内容；`remote_inventory_digest_matches_the_local_snapshot_digest` 证明 remote/local digest 同值。`connect_remote_target` 没有测试注入缝，真实远端读写仍由既有 helper 级测试覆盖。）
- [x] migration/DB 定向测试：v3 -> v4、NULL unknown、checksum/current reopen、overwrite/rename provenance、skip preservation、transaction rollback。
- [x] frontend store/wizard/official preview/Central sync 测试：required token、SHA/expiry、双语 re-preview、retry/consume/reset 和 IPC contract coverage。

## 7. 验证门禁

- [x] `cd src-tauri; cargo test github_import --locked`
- [x] `cd src-tauri; cargo test db::migrations --locked`
- [x] `cd src-tauri; cargo test db:: --locked`
- [x] 运行受影响 marketplace/store Vitest 文件，并连续复跑异步 wizard 组确认无 flake。
- [x] `pnpm typecheck`
- [x] `pnpm lint`
- [x] `cd src-tauri; cargo fmt --all -- --check`
- [x] `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`
- [x] `cd src-tauri; cargo test --locked`
- [x] `just ci`

## 8. 独立检查、spec 与收尾

- [x] 运行 `trellis-check`，逐项核对 PRD acceptance、跨层 payload、无 branch fallback、migration immutability、日志脱敏与 dirty-tree scope。
      （自修 3 处：confirm 步骤 toast 漏出 raw coded 信封、`formatGitHubImportError` 的 normalize/parse 顺序、`formatGitHubImportToast` 只翻译 `preview*` 信封；补 provenance COALESCE 与 rename/skip 测试。）
- [x] 运行 `trellis-update-spec`，更新 GitHub preview contract、database migration contract，以及实际形成的 frontend IPC/error contract；只记录已实现且有测试的约定。
      （新增 `backend/github-import-preview-contract.md` 的 Immutable Preview Snapshot Lifecycle 场景并修正已过期的 Markdown Fetch Boundary；`backend/database-migrations.md` 补 migration 4 与 4 行 contiguous 断言；新增 `frontend/github-preview-snapshot-token.md`；`guides/cross-layer-thinking-guide.md` 补 optional-token-fallback 反例。）
- [x] `git diff --check` 并检查 scoped diff；用符合仓库历史的中文 emoji 提交本子任务代码/spec/task artifacts，不混入排除文件。
      （提交 `8394e8c7`，63 files；`.trellis/scripts/*`、`workflow.md`、`.gitattributes`、兄弟/父任务目录与审计报告均未纳入。）
- [ ] `task.py archive 07-24-github-preview-snapshot` 后记录 journal；核对父任务变为 `11/16`，不 push，不完成父 goal。

## 9. 回滚点

- [x] registry/IPC/UI 可作为一个产品行为单元回滚，但不得恢复 silent branch re-fetch；需要回退时 fail closed 并要求重新 preview。
- [x] migration v4 一旦应用只做向前兼容保留 nullable 列，不执行 destructive down migration。
- [x] 若 Central sync/update 无法在当前子任务内取得真实 token，回到 Phase 1 修正规划，不以 nullable token 或旁路 import 绕过 gate。
