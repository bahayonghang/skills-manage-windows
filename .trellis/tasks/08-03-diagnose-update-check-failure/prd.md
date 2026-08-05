# 修复检查更新的 GitHub archive 与全技能结果缺失

## Goal

恢复 Central Skills“检查更新”对 GitHub 仓库快照的正常获取，同时保留既有 SSRF、凭据隔离和资源预算边界；当仓库快照仍然失败时，让 Update Center、Operation Log 与 Runtime Log 都能提供稳定、可本地化且不泄露敏感信息的错误信号。检查全部技能时，任何无法查询远端的范围内技能都必须得到可持久化、可见的分类，不能因为缺少可查询的远端仓库而从结果中消失。数据库启动必须继续接受已发布二进制写入的等价迁移 checksum，且不得把完整、健康但 schema 不兼容的数据库引导到有损重建。

## User Value

- 用户更新并保存有效 GitHub PAT 后，可以正常检查仓库来源技能的更新，不再把 GitHub 官方 `302 -> codeload.github.com` 流程误判为失败。
- 用户遇到真正的 archive 获取异常时，可以区分“受信任跳转被拒绝”与通用内部错误，而不需要暴露 token、URL、路径或任意底层文本。
- 用户选择“检查全部”时，Update Center 能说明哪些技能已查询远端、哪些技能因缺少受支持来源而无法查询，不再把 141 个技能显示成只有 1 个仓库且结果全空。
- 已由旧版本正常写入的健康数据库不会因为换行归一化导致的迁移 checksum 变化而被拒绝；未知 schema/checksum 仍 fail closed，但 UI 不再向健康数据库提供“一键重建”这种有损出口。
- 已经发生重建时，可先从保留的 `startup-recovery-*` 数据库生成只读、可审计的 repository provenance 恢复预览；真实数据库恢复必须另经用户确认。
- 安全加固不回退：普通 GitHub API/raw 请求仍然不能自动跟随任意重定向，PAT 也不会跨主机转发。

## Confirmed Facts

- 同一 DPAPI 凭据执行只读 `/rate_limit` 请求返回 HTTP 200 和认证配额 5000，因此本次失败不是 PAT 未保存、不可读取或认证无效；完整运行时证据见 `research/diagnosis.md`。
- GitHub archive API 对 `repos/{owner}/{repo}/tarball/{ref}` 正常返回 HTTP 302：普通 branch 的 `Location` 形状为 `https://codeload.github.com/{owner}/{repo}/legacy.tar.gz/refs/heads/{branch}`；GitHub import 的 pinned 40 位 commit SHA 则返回 `https://codeload.github.com/{owner}/{repo}/legacy.tar.gz/{sha}`。
- 共享 client 在 `src-tauri/src/services/github_import/pat.rs:41-47` 使用 `redirect::Policy::none()`；archive 获取在 `src-tauri/src/services/github_import/archive.rs:37-53` 调用共享请求 helper。
- `src-tauri/src/services/github_import/raw_http.rs:421-456` 将该 302 归类为 `GithubImportError::Http`，因此合法 archive 响应在读取和解包前失败。该行为由提交 `35e0c086` 引入。
- `src-tauri/src/commands/skill_update_inventory.rs:60-63` 将 Operation Log 的错误固定为 `Update Center action failed`；同文件 `:131-135` 与 `src-tauri/src/ipc_error.rs:38-49` 又把底层原因收敛为 `internal.unexpected`。
- `src/stores/updateCenterStore.ts:264-266` 通过 `String(err)` 保存 refresh 错误，导致结构化 IPC code 即使存在也会在 Update Center 状态中丢失。
- inventory 表为空证明失败发生在最终持久化之前；进度报告必须在失败时结算 repository 状态并终止 refresh，不能留下悬挂状态。
- 当前只读数据库证据为：Central 技能 141 个，其中 7 个绑定到同一个 GitHub 仓库，134 个无 repository membership；现有 refresh 确实加载 141 个技能并为无绑定技能计算 `unsupported`，但最终只把 actionable bucket 写入 inventory，因此结果 run 为 1、entry 为 0。`skill_update_states` 为 0 符合其“成功 apply/update 后的安装 baseline”语义，不是本缺陷证据。
- 进度 `0 / 1 repositories` 统计的是本次可查询且去重后的 GitHub 仓库，不是技能筛选数量；该计数本身不证明只选择了一个技能，但现有文案没有解释两个维度。
- 只读历史快照可为当前 134 个无绑定技能中的 111 个提供确定的旧 membership，但 23 个没有同一快照来源；跨快照还有 30 个技能出现 repo identity 冲突。自动回填会重建可能已被用户主动 detach/overwrite 的来源，本任务不修改用户数据库或 Central 文件系统。
- `PRAGMA quick_check` 对当前库、迁移前备份和 `startup-recovery-*` 数据库均返回 `ok`。本次不是 SQLite 物理损坏。
- `startup-recovery-20260729T035522.330Z-*` 中的数据库仍有 134 个技能、23 个 GitHub repository 和 111 条 membership；当前库同一秒重新创建，随后扫描得到 141 个技能，但只保留后来导入的 1 个 GitHub repository / 7 条 membership。
- 该恢复数据库的 migration 1 checksum 为 `aabde4...`，当前 binary 期望 `173296...`；其余 migration 2-4 checksum 完全一致。提交 `a47c7cd9` 为换行归一化修改 checksum 并更新锁值，却没有保留已发布 Windows checksum 别名，日志因此记录 `startup.schema_initialization_failed` 且 diagnostic 为 `Healthy`。
- 当前缺失的 111 条 membership 全部能按稳定 `skills.id` 精确匹配现库，0 个缺失 parent、0 个与现有 membership 冲突；另有 23 个重建前就无 membership 的技能，不能混入无歧义恢复集合。
- 用户已明确批准恢复这 111 条无冲突 membership 及其引用的 23 个 GitHub repository；批准范围要求保留当前 7 条 membership 与 23 个 unresolved 技能，不恢复 projects、settings、tags、update baselines 或其它备份数据。
- 恢复后对 24 个真实 GitHub repository 运行禁用自动跳转的 archive 探针：22 个首跳为 302，2 个已重命名/迁移的旧地址首跳为 `301 -> https://api.github.com/repositories/{numeric_id}/tarball/{same_branch}`，随后再 302 到 canonical codeload 地址；另有 4 个 302 仅规范化 owner/repo 的 ASCII 大小写。
- 当前 archive policy 只接受首跳 302，并把 codeload owner/repo 与持久化旧值做大小写敏感比较，因此上述 6 个合法 GitHub 行为会让“检查全部”稳定以 `github_import.archive_redirect_rejected` 失败；这不是 PAT、数据库完整性、检查范围或下载完成后的持久化错误。

## Requirements

### R1. Archive 专用受信任重定向

- 共享 GitHub client 必须继续使用 `redirect::Policy::none()`。
- 只允许 archive 获取路径处理显式有限状态机：direct GitHub 可以直接 `302 -> codeload`，或先 `301 -> api.github.com/repositories/{numeric_id}/tarball/{same_ref}` 再 `302 -> codeload`；mirror 只能直接 302 到与输入 identity 大小写等价的 codeload。不得为 API/raw/preview Markdown 或调用方提供通用 redirect 开关。
- 所有 `Location` 都必须是可解析的绝对 HTTPS URL、默认 443、无 userinfo、fragment 或 query。numeric API path、普通 branch 的 `refs/heads/{branch}` 与 40 位 commit SHA 的 `/{sha}` 形状分别精确验证；未经 direct numeric canonicalization 时 owner/repo 与输入做 ASCII case-insensitive 等价，经该证明链后 canonical owner/repo 必须分别通过安全 component 校验。
- 缺失、重复、相对、格式错误或不匹配的 `Location` 必须 fail closed；numeric API 后非 302、最终 codeload 后再次 3xx 或任何额外跳转都必须拒绝。

### R2. 凭据、镜像与资源边界

- Bearer token 只允许发送到原有 direct GitHub API endpoint以及由其受限 301 证明的同 authority numeric API endpoint；最终 codeload 请求不得包含 `Authorization`，mirror 也不得获得 Bearer。
- 现有 mirror fallback、direct/mirror auth 隔离、超时、限流分类和 404 语义必须保持不变；不受信任 redirect 不得通过 mirror 重试来掩盖。
- 第二跳响应必须继续使用现有 archive bytes、expanded bytes、file count、single-file 和 retained snapshot budgets。

### R3. 稳定错误与日志可观测性

- 为 archive redirect 拒绝增加语义化 `GithubImportError` 变体，不使用错误字符串嗅探。
- 该变体必须映射为稳定 `github_import.*` IPC code、固定 public message 和明确 `retryable`，未知或动态底层文本仍然 fail closed 为 `internal.unexpected`。
- GitHub 网络族失败（传输失败、限流、拒绝访问、仓库不存在、archive 不可用、响应不可解析、请求地址不合法、资源预算超限、凭据不可读）必须各自映射稳定 `github_import.*` code、固定 public message 与明确 `retryable`，并有 en/zh i18n。同一 code 表是 IPC envelope、Operation Log 与 Runtime Log 的唯一来源。
- Operation Log 至少记录安全的 error code 与 phase；未被分类的失败也必须记录静态 `errorCategory`，不得只留固定摘要。Runtime Log/failure recorder 必须保留同一稳定 code。两类日志都不得包含 token、完整 URL、仓库路径、文件路径或任意响应正文。
- Update Center 命令失败必须在 Rust 边界写入 Runtime Log（action、error code、error category、phase、duration），使「See runtime logs for details」的提示确实指向可读记录。
- Update Center store 必须保留结构化错误 code，并通过现有 `formatBackendError` 与中英文 i18n 显示；不得直接显示 Rust 动态错误。

### R4. Update Center 行为完整性

- 合法 direct 302、case-only canonical 302 与 direct `301 numeric API -> 302 codeload` archive 流程都必须构建 snapshot，使一个 repository-backed skill 的 refresh 能完成 inventory/state 持久化。
- 合法第二跳完成时进度必须报告 `repository_completed`；拒绝或下载失败时必须报告 `repository_failed`。
- 单个仓库快照获取失败必须结算为该仓库的 `failed_repositories` 条目，refresh 继续计算其余仓库并持久化 inventory；不得因为一个仓库不可达而丢弃整轮结果。同一 repository 在 `failed_repositories` 中只保留一条记录，快照获取失败优先于它派生出的下游原因。
- `failed_repositories` 条目对已分类失败必须携带稳定 `error_code` 与经审阅的固定文案；不得写入域错误的 Display 文本、URL、token 或路径。旧持久化条目没有该字段时按 `None` 读取。
- 不改变 `SkillRefreshScope`、缓存策略、现有 actionable bucket 的 apply 行为或检查模式选择；允许新增只读诊断用的 `unsupported` bucket。

### R5. 规范与验证

- 同步修订 GitHub import redirect、域错误、redaction、Update Center progress 和测试布局规范，消除“全局禁止 3xx”与“必须支持 GitHub archive canonicalization”的冲突。
- 覆盖生产 policy 的 hostile URL/path 矩阵、case-only canonicalization、受限 numeric API 状态机、API/codeload Bearer 隔离、额外跳转拒绝、资源预算、稳定 IPC/日志 code、前端 code 保留与中英文 i18n。
- 修改 Tauri command 后运行 IPC 文档生成与只读检查；最终 `just ci` 必须通过。

### R6. 全技能结果分类

- refresh 必须对 scope 解析出的每个 Central skill 完成分类；没有 repository membership、来源类型不受支持或缺少可解析远端路径的技能进入 `unsupported`，不得静默丢弃。成功查询且 up-to-date 的技能继续不进入 actionable inventory。
- `SkillUpdateInventory` 增加向后兼容的 `unsupported` 集合，条目至少包含稳定的 `skill_id` 与可枚举的 `reason_code`；旧持久化 inventory 没有该 bucket 时按空集合读取。
- `unsupported` 必须写入并可从 `skill_update_inventory_entries` 重新加载；当其它 bucket 为空时，Update Center 默认显示 `unsupported`，并显示条目与计数。前端只把 `reason_code` 映射到 en/zh i18n，不显示 Rust 的动态 `state.error`、source type、URL 或路径。
- 进度 UI 必须明确 repository 计数是“可查询的去重远端仓库”，同时保留当前 scope 的技能数量；不得把 `1 repository` 表述成只检查了一个技能。
- 只要存在 `unsupported`，空状态不得宣称所有技能都是最新状态或所有范围内技能均已成功检查。

### R7. Inventory 原子性与 baseline 隔离

- refresh 的 inventory run 与全部 entries 必须在同一数据库事务中提交；任意 entry 写入失败时全部回滚，不得留下部分 run/entries。
- refresh 继续不得创建、更新或删除 `skill_update_states`；该表只保存成功 apply/update 后的安装 baseline。`unsupported` 只存在于当前 inventory，不得覆盖已有 baseline，也不得伪造远端 hash、branch 或 repository identity。
- 不根据 skill 名称、`source` 字符串或冲突历史快照推断 repository membership；本任务不得写入用户真实数据库、恢复历史 assignment 或修改 Central 文件。

### R8. Scanner 权威覆盖与 Central 数据完整性

- scanner 只有在权威 Central 根目录真实存在且成功完成扫描时，才可以把未出现在本轮 keep set 的 `is_central = 1` skill 判定为 stale 并删除。Central 根目录缺失、不可见或未成功扫描时必须 fail closed，保留既有 Central skill 父行。
- Central 根目录缺失时，同一批持久化不得清空 Central agent 的 installation/observation keep set，也不得通过 FK cascade 删除 UID、repository membership、update baseline、collection、tag、AI review、explanation 或其它 owned relation。
- 已存在且成功扫描为空的 Central 根目录仍是权威空快照，必须继续清理真正已从磁盘删除的 stale Central skill；非 Central skill 的既有 reconciliation 语义保持不变。
- 修复只保护未来扫描，不回填历史 membership、不推断 repository、不修改用户真实数据库或 Central 文件。

### R9. 已发布迁移 checksum 兼容

- migration preflight 必须接受当前 canonical checksum，以及代码内显式列举、带版本绑定的已发布 legacy checksum；不得接受任意 checksum、前缀、大小写变体或从数据库内容动态学习别名。
- migration 1 的 Windows legacy checksum `aabde4fd51822355cbe2a7982ac895073f6e49e9f34882a50086d145462a736d` 仅作为版本 1 的兼容别名；新建/新迁移数据库继续写 canonical LF-normalized checksum `173296a19419edf197e3baa3b22de1f33184a1d8631141549751fbf1cfc24f7f`。
- legacy alias 只放宽 metadata 等价性，不跳过 descriptor 连续性、future version、foreign key、migration 或 seed 验证，也不改写已有 metadata 行。

### R10. Healthy 数据库的启动重建保护

- `schema_initialization_failed` 且 integrity diagnostic 为 `healthy` 或 `unavailable` 时，启动状态必须 `canRebuild=false`；用户只能 retry/exit，不能从 UI 触发把完整数据库移入恢复目录后创建空库。
- 只有 integrity diagnostic 明确为 `corrupt` 时才允许 rebuild。备份集合、回滚和 clean initialization 的既有原子性保持不变。
- startup status 和日志继续只暴露稳定枚举/code，不返回 checksum、SQL、文件路径或内部错误。

### R11. 现有 provenance 的恢复预览与审批门

- 只读恢复预览必须固定读取明确选择的 `startup-recovery-*` 数据库和当前数据库，运行 integrity/FK 检查，并按稳定 `skills.id` 分类为 addable、already-same、conflict、missing-parent 和 unresolved。
- 本轮证据中只有 111 条 `addable` membership 可作为无歧义候选；23 个在该恢复数据库中就无 membership 的技能保持 unresolved，不从历史删除日志、技能名、目录名或网络搜索自动推断。
- 预览必须同时报告 repository 数、membership 数、冲突数和当前数据库快照标识。任何真实 apply 必须在应用关闭、创建新的可验证备份、预览未漂移且用户明确批准后，以单事务执行并写入脱敏审计记录。
- 经本轮用户批准后，真实 apply 只能插入预览确定的 111 条 membership 与其缺失 repository 行；必须保留现有 7 条 membership，保持 23 个 unresolved 技能不变，并在任意摘要、计数、repository metadata 或 FK 健康条件漂移时整笔回滚。

### R12. GitHub Canonical Archive Redirect 状态机

- archive codeload validator 对 owner/repo 使用 ASCII case-insensitive 等价性，因为 GitHub repository identity 大小写不敏感；branch/ref、路径形状、host、scheme、port、userinfo、query、fragment、dot segment 和编码分隔符仍按现有严格规则验证。
- 只有首个响应确实来自内建 direct `api.github.com` archive 请求时，才允许一次 `301 Moved Permanently`，且 Location 必须精确为 `https://api.github.com/repositories/{positive_ascii_decimal_id}/tarball/{same_ref}`；mirror、raw/API 普通请求、任意其它 host/path/status 均不得进入该分支。
- numeric canonicalization 请求可继续携带 Bearer，因为目标仍是精确的 `api.github.com:443`；其响应必须是一次合法 codeload 302。最终 codeload 请求必须重新构建且不得携带 Bearer，后续任意 3xx 均拒绝。
- 经 direct GitHub numeric canonicalization 后，最终 codeload owner/repo 可使用 GitHub 返回的 canonical/renamed identity，但两个 segment 都必须满足现有 repository component 安全约束，且 ref 必须与结构化输入完全一致；没有 301 证明链时仍只接受与输入 owner/repo ASCII case-insensitive 等价的 codeload identity。
- 不自动改写 repository ID、owner/repo、membership 或 Central 文件；旧来源以后可重复走受限 canonicalization 链路。失败继续映射现有稳定错误码且不记录 Location、numeric ID、repository identity 或 token。

## Acceptance Criteria

- [x] AC1: 本地 HTTP fixture 返回 `302 -> 200 tar.gz` 时，archive 下载并构建 snapshot 成功；生产 validator 独立证明普通 branch 只接受严格的 `.../legacy.tar.gz/refs/heads/{branch}`，40 位 pinned commit SHA 只接受严格的 `.../legacy.tar.gz/{sha}`。
- [x] AC2: HTTP、userinfo、fragment、query、非 443 端口、lookalike/private/loopback/link-local host、缺失或相对 Location、owner/repo/branch/path 不匹配均在第二请求前被拒绝；第二跳 3xx 同样被拒绝。
- [x] AC3: 捕获的第一跳 direct GitHub 请求包含测试 Bearer，第二跳请求不包含该 Bearer；既有 mirror auth 隔离测试继续通过。
- [x] AC4: archive 响应与解包继续受所有既有资源预算约束，cap-plus-one 与危险 archive entry 测试继续通过。
- [x] AC5: repository-backed refresh 使用由 redirect fixture 构建的 snapshot 后可持久化 inventory/state；成功/失败 progress 序列分别正确结算。
- [x] AC6: redirect 拒绝通过固定 `github_import.*` code 到达 IPC、Operation Log、Runtime failure record 和 Update Center 状态；英文/中文 UI 显示本地化 public message，且 adversarial secret/URL/path seed 均不出现。
- [x] AC7: 共享 client 仍不自动跟随 redirect，普通 API/raw 请求和现有 GitHub import/Update Center 测试无回归。
- [x] AC8: `pnpm docs:gen` 后生成物无未解释漂移，定向 Rust/前端检查、完整 Rust 门禁与最终 `just ci` 全部通过。
- [x] AC9: 两技能 fixture 中，一个 GitHub-backed skill 与一个 unassigned skill 执行“检查全部”后，结果同时包含已查询状态与 `unsupported`；刷新后从数据库重新加载仍保留 `unsupported`。
- [x] AC10: 同一 fixture 的全部结果只写入/读取 inventory；refresh 前后 `skill_update_states` 保持逐字段不变，空 baseline 也保持为空。
- [x] AC11: trigger 注入 entry 写入失败时，inventory run 与全部 entries 回滚，且 `skill_update_states` 不受影响。
- [x] AC12: Update Center 显示 `Unsupported`/“无法检查”tab、计数、skill ID 与固定原因；当其它 bucket 为空时默认选中该 tab，且 en/zh 文案一致。
- [x] AC13: 模式弹窗同时显示 scope 技能总数与“可查询的去重仓库”进度语义；141 skills / 1 repository 不再被误解为只筛选一个 repo。
- [x] AC14: 定向 Rust/Vitest、IPC/docs 生成检查、完整 Rust 门禁和最终 `just ci` 重新通过，最终 diff 不包含真实数据库、备份、token、Central 内容或 provenance 回填。
- [x] AC15: Central skill 已有固定 UID、repository membership、update baseline 与全部 owned relations 时，移走 Central 根目录再扫描不会删除或改写这些记录，空 repository 也不会被误 prune。
- [x] AC16: Central 根目录存在且成功扫描为空时，原目录中已删除的 Central skill 仍被 stale reconciliation 清理。
- [x] AC17: scanner 定向测试、完整 Rust 门禁与最终 `just ci` 通过；最终 diff 不含真实数据库、备份或历史 membership 回填。
- [x] AC18: 使用带 migration 1 legacy Windows checksum、其余版本 canonical checksum 的文件数据库时，生产 open/preflight 成功且不创建新备份、不改写 schema migration metadata；随机 checksum 仍在任何写入前失败。
- [x] AC19: `schema_initialization_failed + healthy/unavailable` 序列化为 `canRebuild=false`，`corrupt` 仍允许 rebuild；前端不为不可重建状态显示重建按钮。
- [x] AC20: 只读恢复预览对真实恢复库报告 111 addable、0 conflict、0 missing-parent、23 unresolved，并证明当前库/恢复库 `quick_check=ok`；预览不写任何数据库文件。
- [x] AC21: 新增定向 migration/startup 前后端测试、完整 Rust/前端门禁与最终 `just ci` 重新通过；最终 diff 和测试产物不包含真实数据库、备份、日志、路径或 membership 数据。
- [x] AC22: 用户批准后的真实 provenance apply 从新验证备份与未漂移 preview 出发，在一个事务内恢复恰好 111 条 membership 和 23 个 repository，保留原 7 条 membership 与 23 个 unresolved 技能，并写入不含路径、repository identity 或凭据的 Operation Log 审计行；提交后两库继续 `quick_check=ok` 且当前库 FK violation 为 0。
- [x] AC24: 已安装二进制与源码的时间线证据表明 2026-08-04 的失败来自 2026-08-03 21:03 编译的 release 版本，该版本不含 R12；实机复测须在重新构建并覆盖安装后进行。
- [x] AC25: 24 个真实仓库的禁跳探针显示 18 个同名 302、4 个仅大小写规范化 302、2 个 `301 numeric API -> 302 canonical codeload`，全部落在当前源码策略的接受集合内。
- [x] AC26: 单个仓库快照失败结算为 `failed_repositories` 条目并保留其余仓库结果；inventory run 与 failed entry 持久化，`skill_update_states` 保持为空；同一 repository 只保留一条失败记录。
- [x] AC27: GitHub 网络族 code 在 Operation Log details（`errorCode` + `errorCategory` + `phase`）、IPC envelope 与 Runtime Log 中一致；未分类失败仍记录 `errorCategory`，且 adversarial token/URL/path seed 均不出现。
- [x] AC28: 新增 GitHub 网络族 code 在 en/zh 下均有本地化文案，`formatBackendError` 不回落到 code 字面量，也不泄露 token 或 codeload URL。
- [x] AC23: captured GitHub fixtures 覆盖 owner/repo case canonicalization 与 `301 numeric API -> 302 canonical codeload -> 200 archive`；三请求链只在 direct API 生效，前两跳 Bearer 仅发往 `api.github.com` policy endpoint、codeload 无 Bearer，mirror numeric 301、非数字 ID、ref/path/query/host 变化和额外重定向全部 fail closed。修复后真实 24-repository 禁跳探针不再发现 policy mismatch，定向 Rust、完整门禁和 `just ci` 通过。

## Acceptance Evidence: Archive Redirect Defect

- Archive policy and transport: 7 focused Rust tests passed, including branch/SHA shapes, hostile and malformed `Location`, invalid structured refs, one-hop auth isolation, and second-hop rejection.
- Inventory behavior: redirect-produced snapshot persistence passed; snapshot failure emitted `repository_failed`, skipped `finalizing`, and left inventory run/entry plus skill update state tables empty.
- Error flow: focused Rust command/IPC tests and 81 frontend tests passed for stable code, Operation Log phase, runtime recorder redaction, store preservation, dialog localization, and en/zh parity.
- Regression gates: locked full Rust suite passed with 1118 tests plus all binary/integration/doc tests; final `just ci` passed with 1126 Rust tests in its Rust lane and the complete common lane.
- Environment: local CI used Node 24.14.0 and pnpm 11.9.0 while the repository declares Node 22.x and pnpm 10.12.3; the engine drift was warning-only and every required command exited successfully.

## Acceptance Evidence: Canonical Archive Redirect

- 定向 `cargo test --manifest-path src-tauri/Cargo.toml --locked archive_redirect -- --nocapture` 通过 11/11，覆盖大小写规范化、direct numeric API 证明链、Bearer 隔离、mirror 拒绝、非 302 和额外重定向拒绝。
- 真实 24-repository 禁跳探针记录 18 个同名 302、4 个仅大小写规范化 302、2 个 `301 numeric API -> 302 canonical codeload`，全部落在受信任状态机的接受集合内。
- 最新 `just ci` exit 0：common 与 rust-platform lanes 全部通过；前端 147 个 test files 为 1619 passed / 1 skipped，Rust 主测试为 1136 passed / 7 ignored，Clippy、fmt、IPC/docs、typecheck、lint、build 和 docs build 均通过。
- 本地门禁使用 Node 25.9.0 / pnpm 10.12.3；Node 版本相对仓库声明的 22.x 仅产生 engine warning，未跳过任何检查，敏感路径检查未发现真实数据库、凭据或 Central 内容。

## Acceptance Evidence: All-Skills Inventory And Scanner Integrity

- Inventory refresh 定向组通过 22/22，覆盖 queryable + unassigned、invalid
  `source_path` 无网络请求、unsupported persistence/reload、transaction rollback 和 baseline
  隔离。
- Scanner 使用三个完整测试名分别通过缺失 Central 根保留、`CentralRootRead` 失败前终止、
  成功扫描空根清理；SSH batch protocol 通过 8/8。未把过滤后 0 tests 计入证据。
- 前端完整 Vitest 通过 147 个 test files：1617 passed，1 skipped；`pnpm typecheck`、
  `pnpm lint`、IPC codegen、docs generation/check/build 均通过。
- all-target Clippy 在 `-D warnings` 下无问题；完整 locked Rust 通过 1140 tests，7 ignored；
  最终 `just ci` exit 0，common 与 rust-platform lanes 全部通过。
- 最终路径与敏感检查没有发现真实数据库、备份、日志、archive、安装包、token、Central
  内容或 provenance backfill。命中的 PAT/Authorization 文本均为既有/新增测试 fixture 与固定
  redaction 断言。
- 环境偏差：直接 pnpm 命令使用 Node 25.9.0 / pnpm 10.12.3；`just ci` 子进程报告
  Node 24.14.0 / pnpm 11.9.0。仓库声明 Node 22.x / pnpm 10.12.3；两者仅产生 engine
  warning，没有跳过检查。

## Acceptance Evidence: Migration Compatibility And Recovery Safety

- Red evidence: legacy Windows v1 checksum 在修复前稳定失败为 `Migration checksum mismatch for version 1`；Healthy schema preflight 在修复前错误返回 `can_rebuild=true`。
- Focused green: migration `preflight_` 3/3、startup module 9/9、canonical checksum lock 1/1；随机、大小写、前缀和跨版本 alias 均拒绝。
- Startup authorization: SQLite primary result code 11/26 映射为 `Corrupt` 并可 rebuild；Healthy、Unavailable 与 NotRun 均 fail closed。前端完整 Vitest 包含新增无 rebuild 按钮用例。
- Recovery preview: Python regression 2/2，覆盖稳定 skill ID 分类、严格只读、固定 read transaction 和 WAL-only drift digest。真实只读预览为 111 addable、0 already-same、0 conflict、0 missing-parent、23 unresolved，两库 quick check 均为 `ok`、FK violation 均为 0。
- Final `just ci` exit 0：147 frontend files，1618 passed / 1 skipped；Rust lib 1128 passed / 7 ignored，binary/integration/doc tests 全部通过；all-target Clippy、fmt、IPC/docs、typecheck、lint、build 均通过。
- 最终 diff/sensitive artifact 检查未发现真实 DB、WAL/SHM、恢复目录、token、Central 内容或 provenance apply。`just ci` 使用 Node 24.14.0 / pnpm 11.9.0，声明版本仍为 Node 22.x / pnpm 10.12.3；仅有 engine warning。

## Acceptance Evidence: Approved Provenance Apply

- 用户明确批准恢复 111 条无冲突 membership；应用关闭后重新预览仍为 111 addable、0 already-same、0 conflict、0 missing-parent、23 repositories-to-insert、0 repository metadata conflict、23 unresolved。
- 新 DB/WAL/SHM 备份通过只读 health/FK、141 Central skills、7 memberships、1 populated GitHub repository 与语义摘要比对；扩展摘要同时覆盖 repository `last_synced_at`。
- 一次性工具回归 5/5：成功事务、当前摘要漂移零写入、repository metadata 冲突零写入、只读 preview 和 WAL snapshot digest 稳定性全部通过。
- `BEGIN IMMEDIATE` 事务恰好插入 23 个 repository、111 条 membership 和 1 条脱敏 Operation Log；提交后为 118 memberships、24 populated GitHub repositories、23 unresolved，当前/备份/恢复源均 `quick_check=ok` 且 FK violation 为 0。
- 独立只读比对证明原 7 条 membership 和原 2 个 repository 逐字段不变，111 条恢复关系与恢复源逐字段一致；除 repository、membership、Operation Log 外的 29 张表全部与备份一致。

## Out of Scope

- 修改、迁移、清除或重新验证用户的 GitHub PAT。
- 恢复全局自动重定向，或允许 renderer/远端数据选择任意请求 authority。
- 改造 Central Skills“常规检查/增量和删减”模式的业务语义；仅修正文案，使技能 scope 与 repository 进度维度清晰。
- 修改 Update Center apply、force update、仓库增删决策或 Central 文件 mutation 语义。
- 为私有仓库向 `codeload.github.com` 转发 PAT；若无凭据第二跳无法访问，必须安全失败并作为后续兼容性问题单独处理。
- 从备份、技能名、`source` 字符串或目录结构自动恢复 repository membership；来源恢复需要单独的数据修复契约和用户授权。
- 恢复本轮明确批准的 111 条 provenance 之外的任意数据库内容，包括 unresolved 技能来源、projects、settings、tags、update baselines 或其它历史快照数据。
- 发布安装包、推送远端分支、创建 PR 或变更 GitHub 配置。

## Blocking Open Questions

无。用户目标、修复范围、安全边界和验收行为均已由运行时证据与项目规范确定。
