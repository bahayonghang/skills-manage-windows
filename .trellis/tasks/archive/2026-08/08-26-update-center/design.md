# Skills CLI 上游更新检测与更新抽屉 — 技术设计

本文落实 `prd.md` 和父任务 `../08-26-skills-cli-redesign/research/design-contract.md`。
旧设计中“首次检查直接当作无更新”“observed SHA 覆盖 installed SHA”“mtime 近似”“51 条必然限流”
以及“revert 后把 v7 表留给 v6 binary”全部废止。

## 1. Architecture and Ownership

```text
SkillsCliView / GroupHeader / DetailDrawer
        │ actions and visible error handling
        ▼
skillsCliStore ── typed @/lib/ipc + correlated progress listener
        │
        ▼
commands/skills_cli.rs
  Local gate → skills_cli_jobs lease → SecretStore auth/shared client
        │
        ▼
services/skills_cli/updates.rs
  detect (read/network/hash) ── repos/cache
  apply (pinned acquire → mutation guard → journal → supervised CLI → verify)
        │
        ├── github_import shared source parser/client/snapshot/digest helpers
        ├── target mutation guard + blocking FS helper + process supervisor
        └── v7 repositories/state/operation journal
```

- Component/page 不直接 invoke；`skillsCliStore` 是前端唯一 IPC owner。
- Command 层读取 `github_import::github_direct_auth_from_secret_store(&state.db,
  state.secrets.as_ref())` 并构造 `github_import::github_client()`，把 `auth.as_deref()`、client、
  lease ID/cancel flag 注入 service。`repository_sync.rs` 不是 PAT/client factory。
- Check 和 Apply 共用现有 `skills_cli` exclusive family 与
  `cancel_skills_cli_job(jobId)`。Check 不持有 mutation guard；Apply 仅在网络准备完成后持有 Local guard。
- Core check/page integration 只依赖 backend-contract/page-shell。Detail Update 和 batch Escape
  分别在对应任务完成后接线，不制造占位 callback。

## 2. State Model

### 2.1 Persistent identities

每行必须保留三组不可互换的数据：

```rust
struct SkillsCliUpdateStateRow {
    skill_name: String,
    repository_key: Option<String>,
    normalized_source: Option<String>,
    skill_path: Option<String>,

    installed_revision_sha: Option<String>,
    installed_upstream_digest: Option<String>,
    installed_local_digest: Option<String>,
    installed_at: Option<String>,

    observed_revision_sha: Option<String>,
    observed_upstream_digest: Option<String>,
    observed_at: Option<String>,

    pending_revision_sha: Option<String>,
    pending_upstream_digest: Option<String>,
    pending_detected_at: Option<String>,

    status: SkillsCliUpdateStatus,
    last_error_code: Option<String>,
    updated_at: String,
}
```

- **installed baseline**：只在 update-center 的 Verify exact match 或成功 Apply/Reinstall 后写入；普通
  install-wizard 不掌握 pinned upstream identity，不写 baseline。
- **observed upstream**：最近一次成功 pinned check 的 SHA + per-skill digest。
- **pending update**：observed 与 installed identity/digest 不同即写入；重复检查只更新 observed，
  不清除 pending。只有成功 apply 到对应 pinned digest 后清除。
- **installed local digest**：建立基线当时 canonical 的同算法 digest；本地修改只和它比。
- 普通新装/legacy 无 baseline，或 source/repository/path 改变时，保留旧审计数据但使 current baseline 失效，状态进入
  `baseline_required`，直到重新 Verify/Reinstall。

### 2.2 Public status and precedence

持久化状态使用父任务固定枚举：

```text
not_checked | current | update_available | local_modified |
baseline_required | unsupported | rate_limited | failed
```

`checking` 是当前 correlated job 的 renderer/runtime 状态，不伪装成缓存结果。

一次成功检查后的判定顺序：

1. source 非 GitHub或 path 不能安全归一化 → `unsupported`；
2. installed identity/digest 缺失或 source/path 已改变 → `baseline_required`；
3. 当前 local digest != installed local digest → `local_modified`；
4. observed SHA/digest != installed SHA/digest → `update_available` 并持久化 pending；
5. 否则 → `current` 并清除仅与 installed 相等的 stale pending。

若新一轮仓库检查 rate-limited/failed：状态变为 `rate_limited`/`failed`，但 installed、observed、
pending 字段原样保留并标记 cached/stale。卡片提示和组头 count 从 `pending_revision_sha` 派生，
因此不会消失；Apply 在 stale 状态禁用，直到一次成功 Refresh 重新绑定可用 snapshot。

### 2.3 Missing-baseline action

首次读取普通新装或旧安装只创建 `baseline_required` 行，不把当前 GitHub HEAD 写成 installed：

- `Verify current files`：在 blocking FS 中计算 canonical digest；只有它与当前 observed upstream
  digest 完全相等，才用同一 transaction 写 installed SHA/upstream digest/local digest 并转为 `current`。
- digest 不相等：无法区分旧版本和本地修改，保持 `baseline_required`，显示说明与
  `Reinstall current upstream`。Reinstall 进入完整 journaled apply，成功后建立基线。

## 3. Versioned Content Digest

复用/提取 GitHub snapshot 已使用的 domain-separated file-manifest digest framing：

- 输入为 skill 根内所有普通文件的相对 POSIX path、byte length、SHA-256；稳定按 path bytes 排序。
- 排除 operation-owned backup/marker；拒绝越界、symlink escape、不可读项和资源预算超限。
- pinned snapshot 用 `candidate_content_digest_from_snapshot`；Local canonical 用同一 framing 的
  blocking-FS helper。二者算法版本写入 digest prefix，算法升级必须新增版本，不能重解释旧字段。
- `skillFolderHash` 可以继续作为 lock 完整性证据，但由于算法未被仓库证明，不参与
  `local_modified` 或 installed/upstream comparison。

递归扫描使用 `run_blocking_fs_with`，返回纯 digest/summary；AppHandle、DB、events 留在 async side。

## 4. Repository-Grouped Detection

### 4.1 Preparation

从 fresh `skills_cli_list_global` 构造 scope：

1. 使用 shared GitHub source parser 归一化 `owner/repo/branch`，repository key 包含规范化 branch；
2. 对每个 skill path 做仓库相对路径规范化；失败条目直接 `unsupported`，不进入请求集合；
3. 以唯一 repository key 分组；同一仓库的所有技能共享一次 check unit；
4. 对当前 canonical 计算 exact local digest。检查前后重读 lock/source/path 指纹；若期间变化，
   该仓库结果标 `failed`/stale，不把竞态快照发布为 current。

### 4.2 Network unit

每个唯一 repository unit：

1. 读取 repository cache 的 ETag/last identity；有认证且有 ETag 时发 conditional request；
2. 解析 full 40-char commit SHA；
3. 对该 SHA 获取一次 bounded pinned repository snapshot；
4. 从 snapshot 为每个合法 skill path 计算 upstream digest；
5. 可从同一真实 response/snapshot 得到的提交摘要才进入 row；没有可靠 per-skill 证据则返回空列表。

禁止 `commits?path=...` 逐技能循环。可控并发上限沿用现有 GitHub snapshot policy；所有 repository
unit all-settled，单仓库失败不终止其余仓库。

### 4.3 Rate limits and failures

- 读取 `retry-after`、`x-ratelimit-remaining`、`x-ratelimit-reset`、`x-ratelimit-resource` 和 ETag。
- Primary exhaustion、secondary limit、auth/permission、not found、transport、budget、parse/integrity
  使用 typed classifier；UI 只收到 stable code、safe repository identifier 和 reset time。
- `remaining=0` 或 Retry-After 生效后不继续调度新 unit；未启动 unit 为 `rate_limited`/not-checked
  语义，不伪造 failed/current。
- 已完成 unit 与失败 unit 在一个 top-level transaction 替换本轮 cache/state；transaction 失败保留
  完整旧 cache/state。即使 repository row 失败，也不能覆盖 per-skill installed/pending baseline。

## 5. v7 Persistence and Migration

追加 `src-tauri/src/db/migrations/versions/v7.rs` 及 descriptor；不修改 v1–v6 source/checksum。

### 5.1 `skills_cli_update_repositories`

```text
repository_key TEXT PRIMARY KEY
normalized_source TEXT NOT NULL
branch TEXT NOT NULL
observed_revision_sha TEXT NULL
repository_snapshot_digest TEXT NULL
etag TEXT NULL
status TEXT CHECK(status IN ('not_checked','current','rate_limited','failed'))
last_checked_at TEXT NULL
last_attempted_at TEXT NULL
last_error_code TEXT NULL
rate_limit_remaining INTEGER NULL
rate_limit_reset_at TEXT NULL
updated_at TEXT NOT NULL
```

这里的 `current` 只表示 repository observation 成功，不表示每个 skill 已安装最新版。

### 5.2 `skills_cli_update_states`

字段按 §2.1，`status` 有九态中除 runtime-only `checking` 外的八个值；
`repository_key` 可空 FK，常用查询索引覆盖 `(repository_key,status)`、pending 非空和 updated_at。
对同一 `skill_name` replace/upsert 时 installed/pending 字段必须显式保留，禁止 `NULL` 覆盖已有 baseline。

### 5.3 `skills_cli_update_operations`

```text
id TEXT PRIMARY KEY
singleton INTEGER NOT NULL DEFAULT 1 CHECK(singleton = 1)
phase TEXT NOT NULL CHECK(phase IN (...§6 phases...))
manifest_version INTEGER NOT NULL
manifest_json TEXT NOT NULL
last_error_code TEXT NULL
created_at / updated_at / completed_at TEXT
```

以 partial unique index 保证全局最多一个 nonterminal Skills CLI update operation。manifest 只含
operation ID、safe source key、skill names、expected SHA/digest、owned backup/marker relative identity、
lock/placement fingerprints；禁止 token、argv、stdout/stderr 和任意 URL credential。该表不进入
Operation Log export/portable export/telemetry。

### 5.4 Migration tests and rollback

- descriptor 连续 v1–v7、v7 checksum lock、empty DB、v6→v7、v7 reopen、later-step failure restore、
  checksum mismatch、future v8 rejection均需 fixture。
- v7 一旦随发布 binary 打开用户 DB，即视为持久兼容边界。功能撤回采用 forward patch：保留 v7
  descriptor/table/reader，禁用 UI/commands 或停止写入；禁止发布只认识 v6 的回退 binary。

## 6. Apply, Journal, and Recovery

### 6.1 Immutable apply request

```rust
struct SkillsCliApplyUpdateRequest {
    job_id: String,
    repository_key: String,
    selections: Vec<SkillsCliApplySelection>,
}

struct SkillsCliApplySelection {
    skill_name: String,
    skill_path: String,
    expected_installed_revision: Option<String>,
    expected_installed_local_digest: Option<String>,
    expected_pending_revision: String,
    expected_pending_digest: String,
}
```

前端不传 raw argv。后端从 `backend-contract` 的 verified capability plan 生成结构化 argv；同一 plan
序列化为 preview，因此 preview/execution 不漂移。若 `--force` 或 full-SHA source 未被真实研究证明，
不得出现在 plan。remove+add 不能以“同 guard”冒充恢复；只有 §6.2 backup+journal 完整实现后才可选用。

### 6.2 Ordered apply

1. Command 在第一个 await 前获取 `skills_cli` lease，解析 Local request context，加载 SecretStore auth/client。
2. Guard 外重新获取 expected pending full SHA 的 pinned snapshot，校验 repository + per-skill digest；
   同时校验 request/cache/pinned installed/pending/source/path token。若漂移或不匹配，返回
   `skills_cli.update_stale`，不获取 mutation guard、不创建 journal、不 spawn。
3. 获取 Local target mutation guard；在 guard 内 scoped recover 同一 Skills CLI pending operation。
4. 重读 lock/inventory/current digest/placement，逐字段匹配 request；不匹配返回 stale。此时 guard 已获取，
   但不得创建 journal、执行破坏性 FS 写入或 spawn。
5. DB 插入 `prepared`；blocking FS 建 operation-owned backup/markers，阶段转 `backups_staged`。
6. 捕获 lock file 与 owned path fingerprints，转 `cli_started`，经 supervised BulkTransfer runner 执行
   capability plan，并把 lease cancel flag 传入。
7. CLI 成功后转 `cli_succeeded`；重读 lock/canonical/placement，验证 canonical digest 等于 pinned
   upstream digest、lock 仍证明 ownership、managed links 指向 canonical、direct copies 内容一致。
8. 一个 SQLite transaction 写 installed SHA/upstream/local digest、清 pending，并把 journal 转
   `db_committed`。随后删除 backups；失败为 `cleanup_pending`，成功为 `completed`。
9. Async side 发 terminal event、记录脱敏 Operation Log、刷新 global/update inventories；后续刷新失败
   单独提示，不能把已提交 apply 误报失败。

### 6.3 Recovery

- `prepared/backups_staged/cli_started`：在 guard 下比较 old backup fingerprint、current lock/path、expected
  new digest。完整 old → rollback；完整 expected new → roll-forward；混合/碰撞 → `recovery_required`。
- `cli_succeeded`：只有完整 expected new state 才提交 baseline；否则 restore，restore collision 则保留证据。
- `db_committed/cleanup_pending`：验证 DB/current identity 后仅完成 cleanup；不得回退已提交 baseline。
- Cancel 在 journal 前直接结束；journal 后必须执行上述 settle。future drop/process kill 后下一次 Local
  Skills CLI mutation先 recover；缓存加载暴露 pending recovery，用户可调用显式 Retry。
- restore/finalize 前必须重验 operation marker 和 lock/path fingerprint。疑似外部并发 `npx skills`
  改动时不覆盖未知状态，保留 backup/journal 并返回 stable recovery error。

## 7. Placement Consistency

Apply manifest 为每个 selected skill 记录 backend-contract 的 placement topology：

- `managed_link`：记录 link path + normalized canonical target；更新后必须仍解析到 canonical。
- `direct_copy`：记录 copy root + old digest；CLI capability 若不能可靠刷新，则该 selection 在 preview
  阶段不可 apply，并显示 typed blocker，而不是把 copy 当作 link 删除/重建。
- `missing`：保持 missing；Apply 不因更新自动增加平台。
- `conflict`/`unavailable`：绝不触碰；若 capability plan 会覆盖它们，selection fail closed。

成功校验使用 fresh inventory，不使用前端传入 `agentIds` 作为事实。lock entry、canonical、placement
任何不一致都不会前移 baseline。

## 8. IPC and Events

```rust
skills_cli_check_updates(job_id: String)
    -> IpcResult<SkillsCliUpdateInventory>
skills_cli_update_inventory()
    -> IpcResult<SkillsCliUpdateInventory>
skills_cli_verify_update_baseline(job_id: String, skill_names: Vec<String>)
    -> IpcResult<SkillsCliUpdateInventory>
skills_cli_apply_updates(request: SkillsCliApplyUpdateRequest)
    -> IpcResult<SkillsCliApplyResult>
skills_cli_retry_update_recovery(job_id: String, operation_id: String)
    -> IpcResult<SkillsCliApplyRecoveryResult>
cancel_skills_cli_job(job_id: String)
    -> IpcResult<bool> // existing
```

Progress event：`skills-cli://update-progress`。Payload 至少含 `jobId`、`phase`、repository total/completed、
当前 safe repository key、selected total/completed 和 terminal status。前端先 listen，再 invoke，按 jobId
过滤，finally unlisten。事件不得携带 URL、路径、hash、argv 或 error details。

新增 reviewed codes 至少覆盖：

```text
skills_cli.update_stale
skills_cli.update_baseline_required
skills_cli.update_unsupported
skills_cli.update_rate_limited
skills_cli.update_check_failed
skills_cli.update_local_modified
skills_cli.update_topology_conflict
skills_cli.update_recovery_required
skills_cli.update_integrity
skills_cli.update_migration
```

具体 enum 以一失败模式一 variant 为准；public message 位于 IPC registry/i18n，retryability 固定，
动态 source/HTTP/body/path 不进入 envelope。

## 9. Frontend State and UI

`skillsCliStore` 新增独立字段，不能覆盖现有 runtime/inventory/action error tracks：

```ts
updateInventory: SkillsCliUpdateInventory | null;
isLoadingUpdateCache: boolean;
updateJob: { jobId: string | null; phase: "checking" | "verifying" | "applying" | "recovering" | null };
updateError: unknown | null;
updateProgress: SkillsCliUpdateProgress | null;
loadUpdateInventory(): Promise<void>;
checkUpdates(): Promise<SkillsCliUpdateInventory>;
verifyUpdateBaseline(skillNames: string[]): Promise<SkillsCliUpdateInventory>;
applyUpdates(request: UpdateDrawerSelection): Promise<SkillsCliApplyResult>;
retryUpdateRecovery(operationId: string): Promise<SkillsCliApplyRecoveryResult>;
cancelUpdateJob(): Promise<void>;
```

- Page mount 与 normal inventory 并行加载 update cache；cache failure 只影响更新 surface。
- Toolbar 的 Check updates/Refresh 永远可达；checking 显示 progress + Cancel。
- View model 从 `pendingRevisionSha` 派生卡片点/组 count；从 `status` 派生九态文案和动作。
- Update drawer width 使用父契约 460px / content `<720px` full width，不用 `md=768px`。
- `baseline_required` 提供 Verify/Reinstall；`rate_limited` 显示 reset；`failed` 提供 Retry；stale pending
  保留但 Apply disabled。没有 actionable 行时不显示“全部最新”掩盖其它状态。
- 可见 rejection 经 `formatBackendError` 同时显示 inline error + 稳定 toast；retry/close 清 stale inline error。
- 使用父契约稳定 toast id 和 2800ms；Base UI topmost Escape 和焦点恢复由组件拥有。

## 10. Test Matrix

### Rust

- State transitions：legacy、source/path change、repeated check/restart pending、apply clear、failure keeps pending。
- Digest：path order、mtime-only、same-mtime content change、symlink escape、unreadable/budget/join failure。
- Network：unique-repository request counts、pinned SHA/digest、invalid path no request、partial failure、
  403 permission vs 403/429 rate headers、Retry-After、ETag 304、bounded archive/integrity。
- Apply：guard 前 request/cache/pinned stale 零 guard/journal/spawn、guard 内 fresh-state stale 零 journal/
  destructive-write/spawn、argv plan parity、job busy/cancel、guard contention、all placement states。
- Journal/process-kill matrix：每个 phase 的 rollback/roll-forward/recovery_required、external fingerprint collision、
  cleanup retry、Operation Log redaction。
- Migration：v7 descriptor/checksum、新库、v6 upgrade、v7 reopen、failure restore、future/checksum rejection。

### Frontend

- Store：cache vs check 独立错误、listen-before-invoke、jobId stale event/promise、cancel/busy、restart fixtures。
- View model：九态、pending survives failed/rate, group count、selection eligibility、short identity without confusion。
- Components：loading/not_checked/stale/rate/baseline/unsupported/failed、Verify/Reinstall、selection、spinner、
  inline error、focus/Escape、narrow width、card dot/group/detail entry。
- IPC/browser/i18n contracts：typed commands, fail-loud fixtures, generated drift, safe bilingual error rendering。

无测试替身能证明真实 PAT、GitHub 共享配额、Windows junction、native Tauri/WebView2 或 installer 行为；
未执行时一律 `UNVERIFIED`。
