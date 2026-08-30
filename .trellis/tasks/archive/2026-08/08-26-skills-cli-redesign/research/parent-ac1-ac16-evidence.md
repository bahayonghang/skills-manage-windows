# Parent AC1–AC16 integration evidence

Date: 2026-08-27 (local). Inspector HEAD: `1516be77673f1e31149441ef9d52e8cc2351ac77`
(`1516be77 test(skills-cli): 对齐归档路径与 IPC 数量冻结`).

Parent task is integration acceptance only. This file records requirement-by-requirement
status against `.trellis/tasks/08-26-skills-cli-redesign/prd.md`. Product source was not
modified.

Status values are exactly `pass`, `fail`, or `UNVERIFIED`.

## How this matrix was built

Inspected current HEAD/working tree (not conversation memory).

Child product commits immediately precede their archive commits:

| Child | Product commit | Archive commit |
| --- | --- | --- |
| `08-26-backend-contract` | `58d8ac7e` | `cfcee6a7` |
| `08-26-page-shell` | `c184913d` | `d532740f` |
| `08-26-install-wizard` | `0eb5b60c` | `9d5870a3` |
| `08-26-batch-actions` | `ed77b753` | `73e85a5f` |
| `08-26-detail-drawer` | `6822062e` | `186d098d` |
| `08-26-update-center` | `591cf6c4` | `229361d0` |

Follow-up product commits on the same line: `6dd91497`, `f400192b`, `1516be77`.

Commands executed for this check:

| Command | Exit |
| --- | --- |
| Session `just ci` log `9d94c359-e42b-4983-b4cd-300c822acce9.txt` (`[ci] All checks passed.`, `JUST_CI_EXIT=0`; rust lib tests `1378 passed; 0 failed; 7 ignored`) | 0 |
| `pnpm vitest run src/test/pages/SkillsCliView.test.tsx src/test/lib/skillsCliViewModel.test.ts src/test/components/skillsCli src/test/stores/skillsCliStore.test.ts src/test/pages/DashboardView.test.tsx src/test/components/skill/UnifiedSkillCard.test.tsx src/test/contracts/skillsCliPageShell.test.ts` | 0 (19 files, 184 tests) |
| `cargo test --manifest-path src-tauri/Cargo.toml skills_cli` | 0 (76 passed; 0 failed; 0 ignored in filter) |
| `pnpm ipc:codegen:check` | 0 |
| `pnpm docs:gen:check` | 0 |
| Node key-set compare of `en.json` / `zh.json` `skillsCli` and `backendErrors.skills_cli` | 0 (226/226 and 34/34, no extras) |

Surfaces that this working tree did **not** execute, and that must not be marked `pass`:

- Windows installer / WebView2 screenshot, focus, Escape, i18n/theme matrix
- Real GitHub network, live PAT, live rate-limit
- Real user HOME / real Skills CLI lock library

The unrelated flake `targets::runner::tests::timeout_is_bounded_without_blocking_the_runtime`
did not appear in the session `just ci` log or in the focused `skills_cli` filter.

## 16-row matrix

| AC | Status | Authoritative evidence |
| --- | --- | --- |
| AC1 | pass | Vitest `SkillsCliView` / `DashboardView` / header / view-model tests below |
| AC2 | pass | Store + page tests for doctor/stale/first-load inventory errors |
| AC3 | pass | `skillsCliViewModel.test.ts` filter/group/empty-bucket tests + page/toolbar/group-header tests |
| AC4 | pass | Dense-card 76px class + exact 719/720/899/900/1179/1180 band tests. Native clip/scroll: see AC16 |
| AC5 | pass | Rust placement/link tests on this Windows host, plus UI derivation tests |
| AC6 | pass | Batch preview/uninstall/toast tests + Rust remove tests |
| AC7 | pass | Install dialog/surface tests (preview await, refresh-split) |
| AC8 | pass | Detail drawer + store reveal/link tests. Native WebView2 focus: see AC16 |
| AC9 | fail | Several required update states have no status-asserting test (see row) |
| AC10 | fail | Apply success is tested; later journal FS/DB fault phases are not |
| AC11 | pass | Export v1 serializer + save-cancel tests |
| AC12 | UNVERIFIED | Component Escape tests exist; Windows WebView2 stacked Escape/focus was not executed |
| AC13 | pass | en/zh key sets equal; toast contract tests. No fake-timer file (duration asserted as argument) |
| AC14 | pass | Bounded SKILL.md / reveal / export / redaction tests |
| AC15 | pass | This-session codegen/docs checks + migration tests from `just ci` |
| AC16 | UNVERIFIED | `just ci` passed; installer/WebView2/GitHub/HOME remain UNVERIFIED |

## Per-AC detail

### AC1 (R1,R3,R15)

Quote: Local 页面首次加载显示页头计数、工具栏和 Repository 分组网格，不再渲染 `InventoryCensus`；Dashboard 渲染同一组件，四个计数都由 inventory 派生。

Status: **pass**

Evidence:

- `src/test/pages/SkillsCliView.test.tsx` `lists Skills CLI global skills with doctor status and paths` — asserts census KPI (`库存统计 KPI`) is absent, header install/export present, inventory rendered. Re-run this session: included in 184 passed.
- `src/test/components/skillsCli/SkillsCliHeader.test.tsx` `renders four counts and a successful runtime status`.
- `src/test/lib/skillsCliViewModel.test.ts` `counts installed, managed_link linked, enabled-missing unlinked, and distinct sources`.
- `src/test/pages/DashboardView.test.tsx` `renders the Skills CLI census on Local without changing central summary` (`dashboard-skills-cli-census`).
- Source: `SkillsCliView.tsx` does not import `InventoryCensus`; `DashboardView.tsx` does.

### AC2 (R1,R12)

Quote: doctor 失败时 stale inventory 仍可浏览，运行时 pill 显示本地化错误并禁用 Install；inventory 首次失败与 stale refresh 失败不会被空态吞掉。

Status: **pass**

Evidence:

- `src/test/stores/skillsCliStore.test.ts` `keeps the inventory when doctor rejects cli_unavailable`.
- `src/test/stores/skillsCliStore.test.ts` `keeps stale skills and surfaces inventoryError when list fails on refresh`.
- `src/test/stores/skillsCliStore.test.ts` `reports a failed first load as inventoryError without fabricating skills`.
- `src/test/pages/SkillsCliView.test.tsx` `keeps the inventory rendered once when doctor reports cli_unavailable` (Install disabled).
- `src/test/pages/SkillsCliView.test.tsx` `keeps the stale list visible when a refresh fails`.
- `src/test/pages/SkillsCliView.test.tsx` `shows the inventory error instead of the empty state on first-load failure`.
- `src/test/components/skillsCli/SkillsCliHeader.test.tsx` `shows a safe runtime error, disables Install, and keeps counts`.

### AC3 (R2,R3)

Quote: 搜索、四种分组、平台单选、Unlinked only、折叠、Select all 与空结果在组合使用时结果正确且无空桶。

Status: **pass**

Evidence:

- `src/test/lib/skillsCliViewModel.test.ts` `matches name, source label, and canonical path case-insensitively`.
- `src/test/lib/skillsCliViewModel.test.ts` `matches a platform chip only on managed_link or direct_copy for that target`.
- `src/test/lib/skillsCliViewModel.test.ts` `keeps Unlinked only on enabled missing and excludes copy, conflict, and unavailable`.
- `src/test/lib/skillsCliViewModel.test.ts` `stacks query, platform chip, and Unlinked only`.
- `src/test/lib/skillsCliViewModel.test.ts` grouping tests for repo / platform / status / none, including `uses a single stable all bucket for none grouping and drops empty buckets`.
- `src/test/components/skillsCli/SkillsCliToolbar.test.tsx` `exposes search, group, chip, unlinked, and select controls`.
- `src/test/components/skillsCli/SkillsCliGroupHeader.test.tsx` `is sticky, exposes expanded/controls, and remembers the stable bucket id`.
- `src/test/pages/SkillsCliView.test.tsx` `shows the search query in the filtered empty state`; `toggles selection from the card, merges Select all, and shows the batch bar`.

### AC4 (R3,R14,R15)

Quote: `UnifiedSkillCard` 的 Skills CLI dense 场景在默认字号达到任务契约的紧凑三行布局；内容容器在 1180/900 精确切换 4/3/2 列，抽屉在 720 精确切换全宽，无裁切或水平页面滚动。

Status: **pass**

Evidence (jsdom/CSS contract, not WebView2 pixels):

- `src/test/components/skill/UnifiedSkillCard.test.tsx` `uses the 76px dense-row target and rejects the 168px compact branch` (`min-h-[76px]`).
- `src/test/lib/skillsCliViewModel.test.ts` `uses shared 719/720 drawer and 899/900 plus 1179/1180 grid boundaries`.
- `src/test/lib/skillsCliViewModel.test.ts` `exports the named container grid contract classes` (forbids viewport `md/lg` grid).
- `src/test/contracts/skillsCliPageShell.test.ts` `locks named container query classes and forbids viewport grid breakpoints`.
- `src/test/pages/SkillsCliView.test.tsx` `locks the named content container and 2/3/4-column grid classes`.
- `src/test/components/skillsCli/skillsCliDetailModel.test.ts` `uses content width 719 as full and 720 as 460px`.
- `src/test/components/skillsCli/SkillsCliUpdateDrawer.test.tsx` `uses full width below the 720px content band and shows empty summary`.

Native “no clip / no horizontal page scroll” under installer/WebView2: **UNVERIFIED** (AC16).

### AC5 (R4)

Quote: Rust inventory tests 覆盖 Windows junction、symlink、direct-copy、missing、conflict 和 unavailable；UI 图标、linked/unlinked 计数与允许动作严格按 placement 状态表派生。

Status: **pass**

Evidence executed in this Windows tree (`cargo test … skills_cli`, exit 0):

- `services::skills_cli::placement::tests::windows_junction_is_managed_link`
- `services::skills_cli::placement::tests::stable_order_and_compatible_agents_ignore_missing_conflict` (direct_copy + missing)
- `services::skills_cli::placement::tests::file_slot_is_conflict`
- `services::skills_cli::placement::tests::canonical_missing_absent_slot_is_unavailable`
- `services::skills_cli::placement::tests::disabled_and_undetected_reason_codes`
- `services::skills_cli::link::tests::ordinary_directory_is_zero_write`
- `services::skills_cli::link::tests::missing_to_managed_link_and_idempotent_unlink`
- UI: `skillsCliViewModel.test.ts` count/filter tests; `skillsCliDetailModel.test.ts` `maps the five placements…`; `skillsCliStore.test.ts` `skips link/unlink IPC for direct_copy, conflict, and unavailable`.

Notes (not a fail):

- `unix_symlink_is_managed_link` is `#[cfg(unix)]` and was not compiled/executed on this Windows host. Symlink-kind classification exists in `placement.rs` (`ManagedDirectoryLinkKind::Symlink`).
- No dedicated inventory fixture creates a Windows `symlink_dir` reparse point (product create path is junction). That native privilege path is **UNVERIFIED**.
- `just ci` also ran `directory_link::tests::windows_junction_create_inspect_remove` (not in the `skills_cli` name filter).

### AC6 (R5,R11,R12)

Quote: 批量 preview 分别显示 managed links、retained copies、blocking conflicts；确认只删除 owned 对象，direct-copy 保留且 conflict 时不能提交，结果同步列表/选择并使用 2800ms 单实例危险 toast。

Status: **pass**

Evidence:

- `src/test/lib/skillsCliViewModel.test.ts` `counts owned canonicals and managed links, retains direct copies, and blocks on conflict`.
- `src/test/components/skillsCli/SkillsCliUninstallDialog.test.tsx` `renders owned, managed, retained, and conflict buckets from the backend plan` (asserts no `--keep-links` / `--force` / `skills remove`).
- `src/test/components/skillsCli/SkillsCliUninstallDialog.test.tsx` `keeps failed names and uses a destructive toast on partial remove`.
- `src/test/components/skillsCli/SkillsCliBatchBar.test.tsx` zero-selection hide; busy disable.
- Rust: `remove::tests::preview_has_no_paths_or_argv_and_conflict_blocks`; `remove_preserves_direct_copy_bytes_and_drops_canonical_and_link`; `conflict_is_zero_write`.
- Toast: `skillsCliActionToast.test.tsx` `uses a stable id, 2800ms duration, and replaces by that id`; `maps the four reviewed icon and tone pairs`.

### AC7 (R6)

Quote: 安装弹窗可前后导航；最近源必须 await 真实 preview，失败留在 Source 并显示安全错误；Skills/Platforms 默认选择和 argv preview 与当前 inventory/targets 一致；mutation success 后 refresh reject 仍报告安装成功并单独提示刷新失败。

Status: **pass**

Evidence:

- `src/test/components/skillsCli/SkillsCliInstallDialog.test.tsx` `opens on source with a three-state stepper and resets after close`.
- `…` `stays on source while preview is pending and advances only after success`.
- `…` `starts the same preview from a recent pill without changing step first`.
- `…` `keeps source editable and shows inline error plus toast when preview fails`.
- `…` `defaults to uninstalled skills and supports select all, clear, and empty continue`.
- `…` `uses the shared platform grid, defaultSelected targets, and mapped cliAgents` (asserts no `--force` / `--keep-links` in preview).
- `src/test/lib/skillsCliInstallViewModel.test.ts` `renders npx tokens in build_add_global_argv order with repeated flags`.
- `src/test/components/skillsCli/SkillsCliInstallSurface.test.tsx` `reports refresh failure without reopening, recasting install, or resubmitting`.
- `src/test/stores/skillsCliStore.test.ts` `keeps a successful mutation outcome when trailing refresh throws`.

### AC8 (R7,R13)

Quote: 详情普通入口 focus 为 null，Manage Links 入口只聚焦 links；placement action 同步抽屉与卡片并在失败时回滚；Reveal 只能打开 owned canonical；关闭后 focus 和焦点正确复位。

Status: **pass** (component/jsdom). Native WebView2 focus: **UNVERIFIED** (AC16).

Evidence:

- `src/test/lib/skillsCliViewModel.test.ts` `opens detail with null focus by default and links focus only when requested`.
- `src/test/pages/SkillsCliView.test.tsx` `opens detail with null focus, links focus, and uninstall payload, then resets`.
- `src/test/components/skillsCli/SkillsCliDetailDrawer.test.tsx` `scrolls links once for Manage Links focus…`; `restores focus to the return target on close`; `links only missing rows from Link all…`; `toasts a formatted link failure without raw path details`.
- `src/test/pages/SkillsCliView.test.tsx` `reveals the owned folder by skill name…`; `links only missing placements from the drawer and never mutates a direct copy`.
- `src/test/stores/skillsCliStore.test.ts` `reveals by skillName only and keeps typed errors`; `links only missing placements serially, rolls back a failed item…`.
- Rust: `files::tests::reveal_rejects_non_directory_and_symlink_escape`; `rejects_missing_and_unowned_and_escape`.

### AC9 (R8,R9)

Quote: fresh cache、已有 baseline、new-install/legacy no-baseline、上游变化、重复检查未 apply、重启、local modification、unsupported、rate limit 和 repository failure 都有稳定状态测试；普通 install 不写 baseline，任何未知/失败都不显示成 current。

Status: **fail**

Present status tests (`cargo test … skills_cli`, exit 0):

| Required state | Evidence | Result |
| --- | --- | --- |
| new-install / no baseline | `updates::tests::classify_new_install_is_baseline_required`; `grouped_check_calls_github_once_per_repo` (`BaselineRequired`) | covered |
| existing baseline → current | `verify_writes_baseline_only_on_exact_match` | covered |
| upstream change | `apply_refreshes_canonical_without_forbidden_flags` (`UpdateAvailable`) | covered |
| repeat check without apply | `failed_check_keeps_pending_and_never_reports_current` (pending SHA kept, not `Current`) | covered |
| rate limit | `rate_limit_skips_remaining_repos` | covered |
| repository failure | `failed_check_keeps_pending_and_never_reports_current` (`Failed`, `assert_ne!(…Current)`) | covered |
| nine UI labels | `skillsCliViewModel.test.ts` `exposes the nine update statuses` | enum list only |
| ordinary install does not write baseline | `add_global` does not call update repo; install surface does not invoke `skills_cli_verify_update_baseline` on the success path | covered in source + install tests |

Missing (blocking):

- **local modification**: `classify_successful_check` implements `LocalModified`, but no Rust/Vitest test asserts `SkillsCliUpdateStatus::LocalModified`.
- **unsupported**: `unsupported_skill_row` exists; no test asserts `SkillsCliUpdateStatus::Unsupported` (all update fixtures use `sourceType: github`).
- **restart**: update tests use in-memory `mem_pool()`; no file-DB close/reopen proving pending update rows survive process restart.
- **fresh cache `not_checked`**: no test asserts `NotChecked` from an empty update cache (store test `loads update cache independently…` covers cache-load failure, not the `not_checked` status).

Frontend `checking` overlay is covered by `visibleUpdateStatus(… current … inFlight) → checking`. That does not repair the missing backend status tests.

### AC10 (R8,R9,R16)

Quote: Apply update 成功后 canonical、lock、installed baseline、last observed 和 placement 一致；注入 remove/add/DB/FS 中断后 recovery 可重试或回滚，不留下被 UI 当作成功的半状态。

Status: **fail**

Present:

- `updates::tests::apply_refreshes_canonical_without_forbidden_flags` — canonical body becomes `new`, status `Current`, installed revision `SHA_B`, pending cleared; GitHub fake never sees `--force`/`--keep-links`. Does **not** assert lock JSON or placement rows after apply.
- `updates::tests::apply_stale_is_zero_write` — stale token is zero-write.
- `updates::tests::apply_fault_after_prepared_is_recoverable` — `ApplyFault::Prepared` → `UpdateRecoveryRequired` → `retry_update_recovery_at` → `phase == "rolled_back"`, canonical still `old`.

Missing (blocking):

- `ApplyFault::{Backups, CliStarted, CliSucceeded, DbCommitted}` exist in `updates/apply.rs` and are never injected in tests. Parent text requires interrupt of remove/add/DB/FS style phases; only the `prepared` phase is proven.
- No apply test asserts lock file + placement snapshot consistency after success.

Uninstall journal coverage (`remove::tests::injected_phase_faults_converge_or_fail_closed`) is **not** apply-update evidence.

### AC11 (R10)

Quote: Export all 与 Export selected 生成同一版本化 schema，分别覆盖全量和选择集；save dialog 取消无错误，不可写路径显示本地化错误，序列化快照测试稳定。

Status: **pass**

Evidence:

- `src/test/lib/skillsCliViewModel.test.ts` `emits the exact v1 whitelist, target order, trailing newline, and scoped filenames`.
- `src/test/lib/skillsCliViewModel.test.ts` `cancels silently when the save dialog returns null and otherwise writes the v1 JSON`.
- `src/test/pages/SkillsCliView.test.tsx` `exports the unfiltered inventory and stays silent when the save dialog is cancelled`.
- `src/test/pages/SkillsCliView.test.tsx` `exports the current selection in store order`.
- `src/test/stores/skillsCliStore.test.ts` `writes export JSON through IPC and surfaces a coded failure`.
- Rust: `export::tests::rejects_unknown_skill_field`; `atomically_replaces_existing_file`; `persist_failure_keeps_old_target_and_cleans_temp`.

### AC12 (R13)

Quote: 多层真实组件测试与 Windows 手工检查证明一次 Escape 只关闭 topmost 层并按规定顺序退栈；页面没有重复全局 handler，关闭后焦点返回触发器。

Status: **UNVERIFIED**

Executed component evidence (not sufficient to mark native Windows pass):

- `src/test/lib/skillsCliViewModel.test.ts` `does not add window Escape listeners…` (static source scan of page/batch/uninstall/export/update).
- `src/pages/skillsCliPageHandlers.ts` `handlePageKeyDown` returns if `activeSurface !== null` or `linkMenuOpen` or `defaultPrevented`.
- `src/test/pages/SkillsCliView.test.tsx` `clears selection on bubbling Escape only when no surface is open…`; `does not clear selection when Escape originates from a text input`.
- `src/test/components/skillsCli/SkillsCliBatchBar.test.tsx` `closes the open menu on Escape without invoking link`.
- `src/test/components/skillsCli/SkillsCliInstallDialog.test.tsx` Escape close / pending-refuse tests.
- `src/test/components/skillsCli/SkillsCliDetailDrawer.test.tsx` `restores focus to the return target on close`.
- Grep: no `window.addEventListener` keydown in `SkillsCliView.tsx` or `src/components/skillsCli/`.

Gaps:

- `activeSurface` is a single exclusive union, so uninstall/install/update/detail cannot be open together. There is no one test that presses Escape through the PRD order 卸载确认 → 安装弹窗 → 更新抽屉 → 详情抽屉 → link menu → 清除选择.
- Windows installer/WebView2 topmost Escape and focus restore: **UNVERIFIED** (no screenshot/manual log in this working tree).

### AC13 (R12,R15)

Quote: 新增 en/zh key 集合一致，无硬编码可见字面量；toast 时长/稳定 id/替换/危险图标与内联失败反馈有 fake-timer 和组件测试。

Status: **pass**

Evidence:

- This-session Node compare: `skillsCli` 226/226 keys; `backendErrors.skills_cli` 34/34 keys; no extras.
- `src/test/lib/skillsCliViewModel.test.ts` `keeps en/zh keys aligned for batch, export, uninstall impact, and new backend codes`.
- `src/test/components/skillsCli/skillsCliDetailModel.test.ts` `keeps en/zh detail keys and new backend codes aligned`.
- `src/test/contracts/skillsCliPageShell.test.ts` `forbids prototype hex, remote fonts/CDN, and hardcoded display fonts`.
- Toast: `skillsCliActionToast.test.tsx` stable id `skills-cli-action`, duration `2800`, replacement by id, four semantic icon/tone pairs. No `vi.useFakeTimers` in Skills CLI tests; duration is asserted as the helper argument (sonner owns the timer).
- Inline failure: install dialog preview-fail test; uninstall preview-error test; header runtime error test.

### AC14 (R16)

Quote: SKILL.md 读取限制 1 MiB，拒绝越界、增长竞争和非法 UTF-8；reveal、link/unlink、export 使用 path policy，错误不泄漏绝对路径或密钥。

Status: **pass**

Evidence (`cargo test … skills_cli`, exit 0):

- `files::tests::reads_exact_limit_utf8_document`
- `files::tests::rejects_metadata_oversize`
- `files::tests::opened_handle_growth_maps_to_too_large_without_path_or_size`
- `files::tests::rejects_invalid_utf8_without_leaking_bytes`
- `files::tests::rejects_missing_and_unowned_and_escape`
- `files::tests::reveal_rejects_non_directory_and_symlink_escape`
- `files::tests::prefix_trap_is_not_contained`
- `link::tests::ordinary_directory_is_zero_write`
- `export::tests::rejects_non_json_extension` / `rejects_unknown_skill_field` / persist-failure keeps old target
- `commands::skills_cli::tests::mutation_log_details_omit_paths_and_argv`
- `ipc_error::redaction_contract_tests::skills_cli_contract_codes_keep_reviewed_public_messages`
- `ipc_error::redaction_contract_tests::arbitrary_diagnostics_always_use_the_fixed_fallback` (includes `ghp_super_secret`)
- `services::skills_cli::tests::ac14_ipc_message_never_contains_stderr`
- `services::skills_cli::tests::ac15_missing_npx_js_public_message_omits_candidate_paths`

### AC15 (R16)

Quote: IPC 变更后 `pnpm ipc:codegen` 与 `pnpm docs:gen` 已运行，check 命令无漂移；数据库新库、旧库升级、checksum、future-version 和 rollback fixture 通过。

Status: **pass**

Evidence:

- This session: `pnpm ipc:codegen:check` exit 0 (`[ipc-codegen] checked … generatedCommandMap.ts`).
- This session: `pnpm docs:gen:check` exit 0 (ipc-commands.md and data-model.md up to date).
- Session `just ci` also ran `docs:gen:check` (up to date) as part of the common lane.
- `cargo test … skills_cli` included `db::migrations::tests::migration_seven_creates_skills_cli_update_tables_and_reopens` (ok).
- Full migration matrix from session `just ci` (1378 passed), including in `src-tauri/src/db/migrations/tests.rs`:
  - `migration_sources_are_checksum_locked`
  - `selected_release_fixtures_upgrade_with_backup_and_cascades` (v1–v7, v7 Skills CLI tables)
  - `preflight_rejects_checksum_gap_and_future_versions_without_backup` (inserts version 8 → `newer than supported`)
  - `failed_fk_migration_restores_the_original_database` (rollback/restore)

### AC16 (all R)

Quote: `just ci` 通过；Windows installer/WebView2 的 junction、响应式、焦点、Escape、i18n/theme 截图矩阵有人工证据，未执行项明确为 `UNVERIFIED`。

Status: **UNVERIFIED**

Split:

- `just ci`: **pass** — log `9d94c359-e42b-4983-b4cd-300c822acce9.txt`, `[ci] All checks passed.`, `JUST_CI_EXIT=0`. Not re-run in this check (focused suites above were re-run).
- Windows installer / WebView2 junction UX, responsive, focus, Escape, i18n/theme screenshot matrix: **UNVERIFIED** (no executed evidence in this working tree).
- Real GitHub network / live PAT / real user HOME: **UNVERIFIED**.
- TempDir NTFS junction unit tests are not installer/WebView2 evidence.

## Capability probe (`--force` / `--keep-links`)

Do **not** treat `--force` or `--keep-links` as product argv.

Ledger: archived `.trellis/tasks/archive/2026-08/08-26-backend-contract/research/skills-cli-capability-probe.md` plus `research/probe-evidence/probe-raw.json`.

| Probe | Ledger status |
| --- | --- |
| P1 add/remove help (`skills@1.5.23`, isolated temp HOME, win32, rc=0) | executed |
| P1 `--force` in add/remove help | `VERIFIED_UNSUPPORTED` (absent from stdout) |
| P1 `--keep-links` in add/remove help | `VERIFIED_UNSUPPORTED` (absent from stdout) |
| P2 pinned full-SHA source | `UNVERIFIED` |
| P3 direct-copy refresh | `UNVERIFIED` |

Production argv inspection (this HEAD):

- `src-tauri/src/services/skills_cli/argv.rs` builders (`build_preview_argv`, `build_add_global_argv`, `build_list_global_argv`, `build_probe_argv`) contain no `--force` / `--keep-links`.
- `src-tauri/src/services/skills_cli/updates/capability.rs` `update_capability_plan()`: `force_flag` / `keep_links_flag` = `VerifiedUnsupported`; `apply_argv_preview` tokens are `refresh owned-canonical from-pinned-github-snapshot` (not CLI flags).
- `src-tauri/src/services/skills_cli/remove.rs` does not spawn `skills remove`.
- `build_remove_global_argv` exists as a helper and is used in argv unit tests; uninstall execution path does not spawn it.
- Tests that assert absence: `updates::tests::capability_plan_is_fail_closed`; `ac4_argv_prefix_and_forbidden_tokens`; install/update/uninstall frontend tests.

## SecretStore / PAT in Skills CLI error paths

Search of `src-tauri/src/services/skills_cli/` and `src-tauri/src/commands/skills_cli.rs` found **no** `github_pat`, `GITHUB_PAT`, `ghp_`, or `Authorization:` literals.

Auth wiring: `commands/skills_cli.rs` `github_from_state` reads `github_direct_auth_from_secret_store` and on failure maps to `SkillsCliError::UpdateCheckFailed` (typed public message, original secret error discarded). `updates/github.rs` `map_github_error` maps to `UpdateRateLimited` / `UpdateCheckFailed` only.

Operation log for check/apply records `skillCount` / `jobId` / `skillCount` — not token, URL, path, or argv (`mutation_log_details_omit_paths_and_argv`).

`IpcError::new` only accepts reviewed `&'static str` messages. `arbitrary_diagnostics_always_use_the_fixed_fallback` plants `ghp_super_secret` and asserts it does not serialize.

Conclusion: no SecretStore PAT leak found on Skills CLI error/log paths in this tree.

## Blocking fail items

1. **AC9 fail** — missing stable tests for `local_modified`, `unsupported`, process-restart persistence, and empty-cache `not_checked`.
2. **AC10 fail** — apply journal fault injection only covers `prepared`; `backups_staged` / `cli_started` / `cli_succeeded` / `db_committed` are unimplemented as tests; success path does not assert lock + placement.

Non-blocking UNVERIFIED (must stay unlabeled as pass): AC12 native Escape/focus, AC16 installer/WebView2/GitHub/HOME.

No archive. No git commit. Product source was not edited.
