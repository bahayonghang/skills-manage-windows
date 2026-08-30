# Skills CLI 上游更新检测与更新抽屉

父任务：`08-26-skills-cli-redesign`。

## Goal

为 Local Skills CLI 全局技能提供可解释、可缓存、可取消且可恢复的 GitHub 上游更新流程：
用户能够主动检查更新，区分“当前版本”“有更新”“本地已修改”“缺少安装基线”“不支持”
以及仓库失败/限流；只有在固定上游身份和本地内容均通过验证后才能应用更新。

## User Value

- 更新提示在重复检查和应用重启后仍然稳定，不会因为覆盖了“上次检查 SHA”而消失。
- 普通新装或旧安装无法证明版本时明确显示 `baseline_required`，绝不冒充“已是最新”。
- 更新失败、中断或应用崩溃后保留可恢复证据，不把半更新状态报告成成功。
- GitHub 失败、限流和不支持来源均有可见、可重试且不泄露敏感信息的反馈。

## Confirmed Evidence

- 父任务的任务内权威交互与状态契约是
  `../08-26-skills-cli-redesign/research/design-contract.md`；缺失的原始 Claude Design README
  和静态 HTML 的 no-op 不是实施证据。
- 当前数据库迁移描述符为连续 v1–v6；本任务若新增表必须追加不可变 v7，而不是改写既有迁移。
- GitHub PAT 的真实 command-boundary seam 是
  `github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())`，
  客户端是 `github_import::github_client()`；`repository_sync.rs` 只消费注入的 client/auth，
  不拥有 SecretStore 读取。
- `skills_cli_jobs`、`cancel_skills_cli_job(jobId)`、`ProcessCancellation` 和
  `acquire_target_mutation_guard` 已存在；新增长任务必须扩展这些边界，不另建平行生命周期。
- GitHub 未认证主配额通常为每 IP 每小时 60 次，但当前剩余额度必须以响应头为准；
  不能从技能数量推导“必然限流”。详见 `research/update-center-evidence.md`。

## Dependencies and Delivery Boundary

- 必需前置：`08-26-backend-contract` 已完成并合入，提供稳定的 lock/source/path 字段、
  已验证的 Skills CLI argv 能力和 placement 拓扑类型。
- 必需前置：`08-26-page-shell` 已完成并合入，提供页面工具栏、仓库组头、store/view-model
  所有权和卡片扩展点。
- 集成前置：只有详情抽屉 `Update` 入口依赖 `08-26-detail-drawer`；核心检查和组头入口不得等待它。
- 集成前置：只有跨浮层 Escape 顺序依赖 `08-26-batch-actions`；更新抽屉仍使用 Base UI topmost
  dismissal，不注册第二个无条件全局 Escape handler。
- `08-26-install-wizard` 不是本任务前置。
- 本任务保持 `planning`，在依赖、真实 JSONL 上下文、任务校验和用户复审通过前不得执行
  `task.py start`。

## In Scope

- Local target 的 GitHub 来源归一化、按唯一仓库检查、缓存、进度、取消和部分失败。
- installed baseline、last observed upstream、pending update 与本地内容 hash 的独立状态。
- `not_checked`、`checking`、`current`、`update_available`、`local_modified`、
  `baseline_required`、`unsupported`、`rate_limited`、`failed` 九态 UI。
- 页面 `Check updates` / `Refresh`、卡片更新点、仓库徽标、更新抽屉和详情入口。
- 与检查时固定 SHA/digest 绑定的更新应用、operation journal、恢复、lock/canonical/placement 校验。
- v7 迁移、typed IPC errors、Operation Log 脱敏、IPC/codegen/docs/i18n/测试。

## Out of Scope

- SSH/WSL Skills CLI 更新；非 Local 继续返回 `skills_cli.local_target_only`。
- 非 GitHub 来源的更新实现；它们只进入结构化 `unsupported`。
- 自动把 `direct_copy` 转成 junction，或覆盖/删除 `conflict` placement。
- 在无法取得真实提交/文件证据时生成“变更列表”。
- 修改 Central Update Center 的既有 inventory/state 表或复用其业务状态。
- 把 PAT、完整错误体、命令输出、私有 URL、路径或 journal manifest 写入日志、toast 或导出。

## Requirements

- R1: **依赖与本地边界。** 核心实现只依赖 `backend-contract` 与 `page-shell`；详情 Update
  和跨浮层 Escape 分别在对应子任务落地后接线。所有新 IPC 先解析 request-scoped target，
  非 Local 在触网、读本机 lock/DB 或 spawn 前失败。update capability plan 只消费 backend-contract
  `research/skills-cli-capability-probe.md` 的逐项结论，`UNVERIFIED/unsupported` 能力 fail closed。
- R2: **安装基线与观测状态分离。** 每个技能分别持久化 installed revision SHA、installed
  upstream content digest、installed local content digest、last observed revision SHA/digest、
  pending revision SHA/digest、来源 identity/path 和时间戳。`pending` 只在成功 apply 后清除；
  重复检查和重启不得用 observed 覆盖 installed。
- R3: **无基线记录诚实建基线。** 普通新装或旧记录只要没有 installed revision/digest 就进入
  `baseline_required`，同时允许保存 observed SHA/digest，但不得显示 `current`。用户可执行
  “Verify current files”：精确 digest 相等才建立基线；不相等时保持 unknown，并提供带覆盖警告的
  “Reinstall current upstream”。普通 install 不写 baseline；来源或 skill path 改变同样使旧基线失效。
- R4: **精确本地修改语义。** 使用与上游 snapshot 同一版本化、路径稳定的内容 digest 算法，
  在 blocking-FS 边界重算 canonical 当前内容；只与 installed local content digest 比较。
  禁止 mtime、`updatedAt` 或 observed digest 充当 installed baseline。
- R5: **按唯一仓库检查。** 以规范化 owner/repo/branch 为 repository key；每个唯一来源解析一次
  full commit SHA、获取一次受预算约束的 pinned snapshot，并从同一 snapshot 为其所有 skill path
  计算 upstream digest。禁止逐技能下载仓库；变更摘要只显示真实、可归属的证据，无证据时为空。
- R6: **部分成功、限流和缓存。** 单仓库失败不终止其余仓库；读取 `retry-after`、
  `x-ratelimit-remaining`、`x-ratelimit-reset` 和 `etag`，保存安全的 reset/cache 元数据。
  403/429 必须经 typed classifier 区分限流与权限失败；失败检查保留此前 installed/pending/observed
  记录并把它标记为 stale，而不是发布“无更新”。
- R7: **可到达的检查流程。** 页面工具栏始终提供 `Check updates`；有缓存时显示上次成功时间和
  `Refresh`。缓存加载、联网检查、取消、重试和 stale 结果分别有 store 状态；事件订阅先于 invoke，
  所有进度和 promise settle 以同一 `jobId` 关联，旧事件不得覆盖新作业。
- R8: **九态呈现与入口。** 卡片、组头和更新抽屉使用父契约九态；更新点/更新计数由持久化
  `pending_revision` 派生，即使后续失败/限流也不能丢失。失败或 stale 时可保留提示，但 apply
  必须等待一次成功刷新重新绑定 snapshot。`unsupported`、`baseline_required`、限流和仓库失败
  都有可见原因/动作，不得落入“全部最新”。
- R9: **更新抽屉与固定 apply 输入。** 抽屉按 repository key 展示 selected count、installed →
  observed full identity 的短显示、真实摘要、本地修改/基线/stale 警告和与后端同源的 argv 预览。
  apply request 携带预期 source key、skill path、installed SHA/digest、pending SHA/digest；服务在触网后
  重新获取 pinned snapshot 并核验 full SHA/digest。guard 前的 request/cache/pinned token 不一致直接拒绝且
  不获取 mutation guard；guard 内 fresh lock/inventory/digest/placement 不一致同样拒绝，但已持有 guard，
  两类拒绝都不得创建 journal、执行破坏性 FS 写入或 spawn CLI。
- R10: **可恢复更新应用。** apply 复用 `skills_cli` exclusive lease，准备 snapshot 后按
  lease → Local target mutation guard → scoped recovery → fresh lock/inventory recheck → journal → CLI
  的顺序执行。journal 在任何破坏性步骤前落 `prepared`，记录有界、无凭据的 manifest 和
  `prepared/backups_staged/cli_started/cli_succeeded/db_committed/cleanup_pending/completed/rolled_back/
  recovery_required` 相位。取消只可在破坏前直接返回；破坏后必须同步回滚、滚前或保留可重试 journal。
- R11: **canonical、lock 与 placement 一致性。** journal 保存所选技能的 canonical、lock 条目和
  `managed_link/direct_copy/missing/conflict/unavailable` 拓扑指纹；更新不得把 copy 冒充 link，
  不得覆盖 conflict。成功后 canonical digest 必须等于 pinned upstream digest，lock 仍证明所有权，
  managed junction/symlink 仍指向 canonical，已有 direct copy 内容被 CLI 合法刷新或明确恢复；
  任一校验失败不得提交新 baseline。
- R12: **长任务、错误与隐私。** check/apply 使用现有 `skills_cli_jobs` 和
  `cancel_skills_cli_job(jobId)`；进程经现有 supervised runner，递归 hash/backup/restore 经
  `run_blocking_fs_with`。service 使用 `SkillsCliError` typed variants，command 才映射稳定 IPC code；
  Operation Log 仅记安全 action、计数、phase/code/category 和 job/operation ID，不记录 token、URL、
  argv、stdout/stderr、路径、hash 或 manifest。
- R13: **版本化迁移和可回滚交付。** 追加不可变 v7 descriptor/checksum，建立 repository cache、
  per-skill state 和 recoverable operation journal/索引；新库、v6→v7、v7 reopen、checksum/future-version
  fail-closed fixture 必须覆盖。发布后的功能回滚只能用保留 v7 descriptor/table 的 forward-compatible
  禁用补丁，禁止把已写 v7 的用户数据库交给只认识 v6 的二进制。
- R14: **跨子任务集成。** 详情入口只预选当前技能；组头 `Update all` 只选当前 repository 的
  actionable pending rows。抽屉沿用 Base UI topmost dismissal，关闭后焦点回触发器；batch 任务落地后
  按父契约顺序集成 Escape，不新增全局 handler。
- R15: **生成物、双语与验证。** 新 IPC 进入 Rust-derived command map 和 browser fixture；运行
  `pnpm ipc:codegen`、`pnpm docs:gen` 并提交生成物。所有状态、错误码、reset/retry、基线和覆盖文案
  en/zh 成对；测试覆盖迁移、状态机、请求去重、限流、取消、journal/recovery、拓扑、store 竞态、
  drawer 可访问性和入口。网络、真实 PAT、原生 Windows/junction、installer/WebView2 证据在实测前
  保持 `UNVERIFIED`。

## Acceptance Criteria

- [ ] AC1 (R1): 非 Local 调用在任何 GitHub 请求、本机 lock/DB 读取或进程启动前返回
  `skills_cli.local_target_only`；核心实现的 task start gate 只要求 backend-contract/page-shell。
- [ ] AC2 (R2,R8): 已知 installed=A、observed/pending=B 的记录在再次检查仍为 B 和应用重启后仍保留
  pending；只有成功 apply B 才把 installed 更新为 B 并清除 pending。
- [ ] AC3 (R2,R3): 普通新装、legacy 或来源/path 变化的记录返回 `baseline_required`，即使已观察到 upstream
  也不显示 `current`/“全部最新”。
- [ ] AC4 (R3,R4): “Verify current files” 仅在当前 canonical digest 与 observed upstream digest
  精确相等时建立 installed baseline；不相等时不写 baseline，并显示 Reinstall + 覆盖警告。
- [ ] AC5 (R4): 修改文件内容但保持 mtime、或只改变 mtime 不改变内容的 fixture 分别得到
  `local_modified` 与非 `local_modified`；hash join failure 映射 typed error。
- [ ] AC6 (R5): 同一仓库 51 个技能只进行一次 repository identity resolution 和一次 pinned snapshot
  acquisition；每行 upstream digest 都来自该 snapshot 的真实 skill path，非法/缺失 path 不触发额外网络。
- [ ] AC7 (R5,R6): 混合 GitHub、非 GitHub、解析失败、一个仓库 5xx 和其余成功的检测返回完整九态；
  成功仓库持久化，失败仓库有稳定 code，`unsupported`/`failed` 不被计为 `current`。
- [ ] AC8 (R6): 403/429 fixture 依据响应头区分 `rate_limited`，保留 reset/retry 信息和旧 pending；
  权限失败为 `failed`。测试不依赖“51 > 60”假设，也不在限流时继续无界重试。
- [ ] AC9 (R7,R8): 首次进入先显示缓存 loading；有缓存时显示上次成功时间，stale/失败标记和可达的
  Refresh；无缓存显示 `not_checked` 和 Check updates。检查中显示 `checking`、真实 repository 进度和 Cancel。
- [ ] AC10 (R7,R12): listener 在 invoke 前建立；取消、busy、stale event、较旧 promise 后返回均不能覆盖
  当前 job。取消按钮发送当前 `jobId`，无 job 时不 invoke。
- [ ] AC11 (R8): 卡片更新点和组头 `<k> updates` 等于该组 persisted pending rows 数；后续失败/限流可把
  结果标 stale 并禁用 apply，但不能把更新点或 pending count 清零。
- [ ] AC12 (R8,R9): 更新抽屉展示真实 installed/observed 短 revision、含义说明、选择、真实/空变更摘要、
  本地修改/基线/stale 状态、命令预览；零选择不可提交，焦点与 Escape 符合父契约。
- [ ] AC13 (R9): guard 前任一 request/cache/pinned installed/pending/source/path token 不同即返回
  `skills_cli.update_stale`，不获取 mutation guard、不创建 journal、不 spawn CLI；guard 内重读的
  lock/inventory/current digest/placement 不同也返回 stale，此时允许已持有 guard，但仍不创建 journal、
  不执行破坏性 FS 写入、不 spawn CLI。预览 argv 与实际 argv 来自同一后端 capability plan。
- [ ] AC14 (R9,R10,R11): 成功 apply 后 fresh canonical digest 等于 pinned upstream digest，lock/canonical/
  placement 校验通过，installed baseline 与 journal `db_committed` 在同一 SQLite transaction 提交，
  pending 清除，库存刷新后 drawer 关闭并显示单实例成功 toast。
- [ ] AC15 (R10,R12): cancel/失败注入覆盖 prepared、backups_staged、cli_started、cli_succeeded、
  db_committed 和 cleanup_pending；结果只能是完整旧状态、完整新状态或可见 `recovery_required`，
  不存在无 journal 的半更新成功。
- [ ] AC16 (R10): 重启/下一次 mutation/显式 Retry 对 pending journal 幂等恢复；指纹或路径碰撞时保留
  journal/备份并 fail closed，不覆盖未知外部改动。
- [ ] AC17 (R11): managed junction/symlink、direct copy、missing、conflict 混合 fixture 证明：direct-copy
  refresh 为 `VERIFIED_SUPPORTED` 时成功更新保留原拓扑、刷新 copy 且 conflict 未改；能力为
  `VERIFIED_UNSUPPORTED/UNVERIFIED` 时 preview 阻断含 direct-copy selection 且零 mutation。任一执行后目标
  不一致时 baseline 不前移并进入恢复路径。
- [ ] AC18 (R12): 新失败模式均有稳定 `skills_cli.*` code/retryable，en/zh 经 `formatBackendError` 渲染；
  IPC、Operation Log、runtime log、toast 和导出中不存在 token、完整 URL、路径、argv、输出、hash 或 manifest。
- [ ] AC19 (R13): v7 descriptor/version/checksum 连续且不可变；空库与 v6 fixture 升级、v7 reopen 幂等、
  later-step failure restore、checksum mismatch 和 future v8 均有非零测试并符合 migration contract。
- [ ] AC20 (R13): 回滚演练使用“UI/commands disabled 但 v7 descriptor 保留”的构建打开 v7 DB 成功；
  交付说明明确发布后禁止回退到纯 v6 schema binary。
- [ ] AC21 (R14): 组头 Update all 只预选本仓库 actionable pending；详情 Update 只预选当前技能；
  baseline/unsupported/failed 行不被静默加入 selection。
- [ ] AC22 (R15): IPC codegen/docs 生成后 `ipc:codegen:check` 与 `docs:gen:check` 无漂移，browser fixture
  fail-loud，新增 locale key en/zh 完全对齐。
- [ ] AC23 (R15): focused Rust/Vitest、migration fixtures、`pnpm typecheck`、`pnpm lint`、
  `cargo fmt --all -- --check`、locked Rust tests 与 `just ci` 全部通过；过滤到 0 tests 不计作证据。
- [ ] AC24 (R15): Windows Tauri/junction、真实 PAT/限流、真实仓库和 installer/WebView2 测试若未执行，
  closeout 明确列为 `UNVERIFIED`，不得用 mock/静态检查替代。

## Deferred Evidence, Not Open Product Questions

- `skills@1.5.23` 是否接受 `--force`、固定 full-SHA source、以及是否可靠刷新 direct copy，必须由
  `08-26-backend-contract` 保存真实帮助输出/受控探针后选择 argv；未验证 flag 不进入预览或执行。
- Windows junction 创建、识别、权限失败和安全恢复需要原生 Windows 测试；静态 Rust 测试只能验证逻辑。
- GitHub primary/secondary rate limit、PAT 权限和 private repository 行为必须用受控账号实测；实现仍以响应头
  和 typed denial 为权威。
