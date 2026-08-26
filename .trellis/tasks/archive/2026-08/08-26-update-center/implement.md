# Skills CLI 上游更新检测与更新抽屉 — 执行计划

状态：**planning only**。本文件不授权 `task.py start` 或产品代码修改。

## Dependency Gates

- [ ] `08-26-backend-contract` 完成并合入：稳定 lock/source/path/placement contract 已生成；
      `.trellis/tasks/08-26-backend-contract/research/skills-cli-capability-probe.md` 对
      `skills@1.5.23` 的 force/pinned-source/direct-copy 行为有保存的真实证据，未验证能力不进 argv。
- [ ] `08-26-page-shell` 完成并合入：toolbar/group/card/store/view-model 扩展点稳定。
- [ ] 详情 Update 接线前，`08-26-detail-drawer` 已完成；否则不放 placeholder callback。
- [ ] 跨浮层 Escape 集成前，`08-26-batch-actions` 已完成；更新抽屉自身先遵循 Base UI topmost。
- [ ] 父 `research/design-contract.md`、本任务 `prd.md`/`design.md` 和 JSONL manifests 已复审。
- [ ] 用户在最新规划总结之后明确批准实施。

`08-26-install-wizard` 不是前置。核心 check/cache/drawer 可在 detail/batch 接线之前实现和验证，
但本任务整体完成仍必须满足 PRD AC21。

## Ordered Implementation

### Phase 1 — Freeze capabilities and contracts

- [ ] 读取 `.trellis/tasks/08-26-backend-contract/research/skills-cli-capability-probe.md` 的真实结论，
      形成唯一 `SkillsCliUpdateCapabilityPlan`：
      force、pinned full-SHA source、direct-copy refresh 每项均为 verified/unsupported；无推测 fallback。
- [ ] 为 `SkillsCliError` 设计 update-specific typed variants、stable IPC codes、retryability 和安全 public messages。
- [ ] 固定 versioned local/upstream digest framing、source/path normalization、state transition table、
      journal manifest v1 和 phase graph；先写纯函数/fixture 测试。
- [ ] 明确 operation-log allowlist 和 forbidden fields，保证 token/URL/path/hash/argv/output/manifest 不可进入。

### Phase 2 — v7 migration and repositories

- [ ] 新增 immutable `src-tauri/src/db/migrations/versions/v7.rs`，在 descriptor 数组追加 version 7；
      不修改 v1–v6 source/checksum。
- [ ] 建 `skills_cli_update_repositories`、`skills_cli_update_states`、
      `skills_cli_update_operations`、CHECK/FK/lookup/partial-unique indexes。
- [ ] 新增 repos：transaction-scoped repository cache replace、per-skill state transition、pending-preserving upsert、
      journal insert/expected-phase transition/list pending/finalize。repos 只返回 `sqlx::Error`。
- [ ] 先跑 migration focused tests：empty DB、v6→v7、v7 reopen、descriptor/digest lock、later-step failure restore、
      checksum mismatch、future v8。验证测试数非零。

### Phase 3 — Exact digest and grouped detection

- [ ] 在 `services/skills_cli/updates/` 建 typed domain module；复用 shared GitHub source parser、
      pinned snapshot 和 candidate digest helpers，不复制 HTTP/PAT 层。
- [ ] 提取/复用同 framing 的 Local canonical digest helper；递归 FS 通过 `run_blocking_fs_with`，
      覆盖 mtime-only、same-mtime content change、symlink escape、budget/unreadable/join error。
- [ ] 从 fresh lock/inventory 构造合法 skill scope，按 normalized owner/repo/branch 去重；invalid path
      分类 unsupported 且不触网。
- [ ] 每个 repository 只 resolve full SHA + acquire pinned snapshot 一次；从 snapshot 派生所有 skill digest。
      使用 fake endpoint/client 断言请求数，不访问公网。
- [ ] 读取 ETag/Retry-After/X-RateLimit headers；403 permission、primary/secondary rate limit、429、5xx、
      transport/budget/integrity 分类为稳定 errors。all-settled 后 transactionally publish，失败保留旧 pending。
- [ ] 实现 new-install/legacy/source-change `baseline_required` 与 Verify exact-match action；普通 install 不写
      baseline，不相等绝不自动建基线。

### Phase 4 — Correlated IPC and cache load

- [ ] 扩展 `skills_cli_jobs` family 覆盖 check/verify/apply/recovery；command 在首个 await 前 acquire lease，
      先 Local gate，再经真实 command-boundary SecretStore/client seam 注入 service。
- [ ] 新增 check/cache/verify/retry-recovery/apply commands 与 `skills-cli://update-progress`；
      全 payload camelCase + Specta，event 只含安全计数/phase/key。
- [ ] `skills_cli_update_inventory` 纯读缓存；加载失败不影响 global inventory。Check/Verify 发真实 jobId，
      cancel 复用 `cancel_skills_cli_job`。
- [ ] 新命令加入 IPC registry、generated command map、browser fixtures 和 coverage allowlist ratchet。

### Phase 5 — Journaled apply and recovery

- [ ] Apply request 使用 expected installed/pending/source/path token；guard 前验证 request/cache/pinned
      SHA/digest，stale 零 guard/journal/spawn；guard 内重读 lock/inventory/current digest/placement，stale
      允许已持有 guard，但零 journal/destructive write/spawn。
- [ ] 固定 acquire 顺序：skills_cli lease → network prepare → Local mutation guard → scoped recovery →
      fresh state recheck → prepared journal → backup/marker → supervised CLI。
- [ ] 备份/restore/finalize 作为 coherent blocking-FS units；AppHandle/DB/progress 留在 async side。
- [ ] 每个 phase 以 expected-phase transition 持久化；CLI cancel flag 传 ProcessRequest，破坏开始后 cancel
      必须 settle 或留下 recovery_required。
- [ ] Post-CLI 验证 canonical digest、lock ownership/source/path、managed links、direct copies、missing/conflict；
      不一致不提交 baseline。
- [ ] installed baseline + pending clear + journal db_committed 在同一 transaction；cleanup failure留
      cleanup_pending。显式 Retry/下一次 mutation/启动恢复幂等。
- [ ] 加 phase/collision/process-kill 测试：只允许完整 old、完整 new、recovery_required 三类结果。

### Phase 6 — Store and view model

- [ ] 在 `skillsCliStore` 增加独立 update cache/job/progress/error actions；不覆盖现有 runtime/inventory/action
      error tracks。每次 start 生成 jobId，listen 先于 invoke，finally unlisten。
- [ ] 所有 post-await state write 与 event merge 校验 captured jobId；同 store duplicate start fail busy，
      older promise/event 无权覆盖 current job。
- [ ] view-model 纯函数实现九态、stale/pending、group counts、actionable selection、drawer rows 和
      backend capability argv preview model；preview 不由前端自行拼接 flags。
- [ ] Page mount 并行加载 update cache；toolbar Check updates/Refresh 始终可达；progress/cancel、last checked、
      stale/rate/baseline/recovery states 有独立 UI。

### Phase 7 — Update drawer and integrations

- [ ] 实现 `SkillsCliUpdateDrawer`：repository title、selected count、installed→observed、真实/空摘要、
      local-modified/baseline/stale/recovery warnings、command preview、inline error、Cancel/Update selected。
- [ ] 使用父契约 460px / content `<720px` full-width、Base UI topmost Escape、focus return、稳定 toast id
      `duration=2800ms`；不注册 global keydown。
- [ ] 接 page-shell 卡片 update point、组头 pending count/Update all；失败/限流仍保留 stale pending indicator，
      但 apply disabled until successful refresh。
- [ ] detail-drawer 完成后接“只预选当前技能”；batch-actions 完成后验证多层 Escape 顺序。
- [ ] 所有 visible rejection 使用 `formatBackendError`：inline + toast，retry/close 清 stale error。

### Phase 8 — Generated artifacts, i18n, and docs

- [ ] en/zh 成对增加九态、baseline Verify/Reinstall、rate reset、repository failure/retry、recovery、
      stale/pending、typed backend errors 和 ARIA labels。
- [ ] 运行并提交 `pnpm ipc:codegen`、`pnpm docs:gen` 输出；连续两次 check 确认 deterministic/no drift。
- [ ] 若 update-center 行为形成新的跨任务持久 contract，实施完成后经授权更新
      `.trellis/spec/backend/skills-cli-global.md`；本 planning turn 不改 spec。

## Focused Validation Order

1. Migration and repositories:

   ```powershell
   cargo test --manifest-path src-tauri/Cargo.toml --locked db::migrations
   ```

2. Skills CLI update state/network/digest/journal:

   ```powershell
   cargo test --manifest-path src-tauri/Cargo.toml --locked services::skills_cli::updates
   ```

3. Command/IPC/error/redaction contracts:

   ```powershell
   cargo test --manifest-path src-tauri/Cargo.toml --locked commands::skills_cli
   pnpm ipc:codegen:check
   pnpm docs:gen:check
   ```

4. Frontend ownership paths（测试应放 `src/test/{stores,lib,components/pages,runtime,contracts}`）：

   ```powershell
   pnpm exec vitest run src/test/stores/skillsCliStore.test.ts src/test/lib/skillsCliViewModel.test.ts src/test/components/skillsCli src/test/pages/SkillsCliView.test.tsx src/test/runtime/ipc.test.ts src/test/contracts/i18nLocales.test.ts
   ```

5. Broadened local gate:

   ```powershell
   pnpm typecheck
   pnpm lint
   cargo fmt --all -- --check
   cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
   cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked
   just ci
   ```

任何过滤结果为 0 tests 的命令必须改用完整 module/test name 重跑，不能列为通过证据。

## Manual and External Verification

- [ ] Windows 本机 Tauri：junction/managed symlink/direct copy/missing/conflict 的检查、应用、失败恢复。
- [ ] Windows installer/WebView2：460/full-width 断点、中文、focus/Escape、spinner/progress、toast。
- [ ] 受控 GitHub public/private repositories：PAT/no-PAT、ETag、primary/secondary limit、Retry-After、
      permission/404、一个仓库失败其余成功。
- [ ] 真实 `skills@1.5.23`：capability plan 的 argv、fixed SHA behavior、copy refresh 和 cancel/process tree。

未执行项必须写 `UNVERIFIED`；mock、源码审阅或浏览器静态 fixture 不替代这些证据。

## Rollback and Release Boundary

- PR 合并前若 v7 从未随可运行构建触碰用户 DB，可整体 revert 后重跑 current migration fixtures。
- v7 一旦发布，禁止交付只含 v1–v6 descriptor 的旧 binary。功能撤回必须保留 v7 migration、reader 和
  safe no-op/disabled behavior，并以 forward patch 发布。
- 遇到 nonterminal update journal 时不得卸载/降级功能；先完成显式 recovery 或保留可恢复 artifacts。

## Final Planning Gate

- [ ] 依赖与 backend capability evidence 已满足。
- [ ] `prd.md` 每个 R 都被至少一个 AC 覆盖，design/implement 无 mtime/first-check-no-update/unsafe revert 残留。
- [ ] `implement.jsonl`、`check.jsonl` 只有真实 repo-relative spec/research entries。
- [ ] `task.py validate 08-26-update-center` 与树级 `plan_precheck.py` 无本任务 blocker。
- [ ] 最新规划总结已提交用户审阅；只有后续明确批准才可 start。
