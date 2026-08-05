# 技术设计

## 1. 边界与原则

本修复不改变共享 `reqwest::Client` 的 redirect policy。`Policy::none()` 仍是默认且唯一的通用行为；合法 GitHub archive redirect 由 `github_import::archive` 在收到响应后显式解析、验证并最多执行一次。

最小数据流：

```text
structured GitHubRepoRef
  -> built-in API/mirror endpoint request
  -> 200 archive -----------------------> bounded archive read
  -> 302 + Location
       -> production codeload validator
       -> unauthenticated second GET
       -> 200 --------------------------> bounded archive read
       -> any 3xx/error ----------------> typed failure
  -> snapshot extraction
  -> Update Center inventory/state persistence
```

renderer payload、`normalizedUrl`、响应正文和任意 redirect authority 均不能绕过该边界。

## 2. Archive Redirect Contract

### 2.1 第一跳

- 继续通过既有 built-in endpoint 构造 `/repos/{owner}/{repo}/tarball/{branch}`。
- direct GitHub 请求保留 Bearer；mirror 请求继续无 Bearer。
- 200、404、限流、拒绝和 mirror retry 维持现有分类。
- direct GitHub 的 `302 Found` 进入 codeload 分支，`301 Moved Permanently` 仅进入 numeric repository canonicalization 分支；mirror 只允许 302 codeload。其它非成功 3xx 仍失败，不扩大协议面。

### 2.2 Production Validator

使用结构化 URL parser 验证 `Location`，不做字符串前缀判断。codeload 目标必须满足：

- absolute URL，scheme 为 `https`；
- host 的 ASCII 形式精确等于 `codeload.github.com`；
- 端口为空或解析为 443；
- username、password、fragment、query 均为空；
- 普通 branch 的 path segments 精确为 `{owner}/{repo}/legacy.tar.gz/refs/heads/{branch}`；当结构化 ref 是 40 位十六进制 commit SHA 时，path segments 精确为 `{owner}/{repo}/legacy.tar.gz/{sha}`；两种形状不得交叉接受；
- 没有 numeric canonicalization 时，owner/repo 与结构化输入逐段 ASCII case-insensitive 等价；有 direct numeric 证明链时，canonical owner/repo 分别通过现有安全 component 校验；ref 在两种情况下都与结构化输入精确相等；
- 不接受额外 segment、dot segment、percent-encoded separator 或路径尾缀。

direct numeric API 目标另行验证为绝对 `https://api.github.com:443/repositories/{positive_ascii_decimal_id}/tarball/{same_ref}`，无 userinfo/query/fragment/额外 segment；只有 initial response 的 request URL 属于 trusted direct API policy 时才授权。验证失败统一返回零动态字段的 `GithubImportError::ArchiveRedirectRejected`，具体 URL、numeric ID 和响应头永不进入错误值或日志。

### 2.3 第二跳

- direct 302 使用同一个 no-redirect client 对已验证 codeload URL 发起新的 GET；构建新 request、不复制 headers、不调用 `bearer_auth`。
- direct 301 对已验证的同 authority numeric API URL 发起新的 GET，可重新附加原 Bearer；只接受其 302 codeload 响应，再构建不带 Bearer 的最终 GET。
- codeload 只接受 2xx；numeric API 非 302、codeload 后任何 3xx 都返回 `ArchiveRedirectRejected`，防止未授权 chain/loop。
- 404 映射到现有 archive unavailable 语义；其它状态/transport 使用既有安全分类或固定 archive failure code。
- 2xx body 进入现有 `read_response_bytes_bounded`，解包继续走既有 archive/expanded/file budgets。

## 3. Test Seam

严格 production policy 不能直接把 `api.github.com:443` 或 `codeload.github.com:443` 指向当前明文 `TcpListener` fixture。实现只增加一个私有/test-only seam：

- production wrapper 固定使用不可配置的 codeload policy；
- pure validator tests 对真实 production policy 跑完整 hostile matrix；
- test helper 可注入本地 API/codeload endpoint policy，复用完全相同的 302 与 301->302 状态机、header 构造、跳数限制和 bounded body 路径；
- test-only 配置不得进入 public API、Tauri command、持久化配置或 release build 的可变输入。

测试组合必须同时覆盖“production policy 正确”和“redirect transport/data path 正确”，不把本地 HTTP fixture 冒充为生产 TLS 验证。

## 4. Error and Observability Flow

```text
GithubImportError::ArchiveRedirectRejected
  -> CentralUpdatesError::GithubImport (transparent)
  -> UpdateCommandError safe mapping
     -> IPC coded envelope -> IpcError { code, message, retryable }
     -> Operation Log details { errorCode, phase }
  -> frontend IpcInvokeError
  -> backendErrorStateValue (preserve code, drop details)
  -> formatBackendError + en/zh backendErrors key
  -> runtime failure recorder keeps public code/message only
```

设计约束：

- `GithubImportError::ipc_code()` 增加稳定 archive redirect code；`to_ipc_error()` 只输出固定 summary。
- `CentralUpdatesError` 仅透明转发经审查的 GitHub import code；其它动态错误保持 `internal.unexpected`。
- `UpdateCommandError::Display` 继续不展示内部错误。Operation Log failure details 从错误对象提取静态 `errorCode`/`phase`，不保存 `Display` 的动态 source。
- command 返回前保留 coded envelope，使 `ipc_error.rs` 的白名单映射生成固定 `IpcError`；Rust/TypeScript canonical message 与 en/zh i18n key 同步。
- `updateCenterStore` 使用 `backendErrorStateValue` 保存错误状态，避免 `String(IpcInvokeError)` 丢 code；UI 仍通过 `formatBackendError` 渲染。
- Runtime Log 只依赖现有 IPC failure recorder 记录结构化 public code/message。不得新增包含 URL、token、仓库路径或响应正文的 `tracing` 字段。

## 5. Compatibility Matrix

| 场景 | 修复后行为 |
| --- | --- |
| GitHub public archive 302 到匹配 codeload | 第二跳无 Bearer，下载并构建 snapshot |
| owner/repo 仅大小写 canonicalization | 视为同一 GitHub identity，codeload 无 Bearer并成功 |
| direct API 301 到 numeric repository endpoint | 验证同 authority/same ref，API hop 可带 Bearer；canonical codeload 无 Bearer |
| mirror 301 到 numeric repository endpoint | fail closed；mirror 响应不能授权 repository identity 变化 |
| GitHub import pinned 40 位 commit SHA | 仅接受 `legacy.tar.gz/{same-sha}` 并保持 immutable preview fallback |
| built-in mirror 直接返回 200 archive | 保持现有成功路径 |
| direct GitHub rate limit 且无 PAT | 保持既有 public mirror fallback |
| direct/mirror 401/403/404/transport | 保持既有 denial/not-found/retry 分类 |
| 302 到 lookalike/私网/错误 repo/ref | fail closed，不发第二请求，不 fallback 掩盖 |
| codeload 第二跳再次 3xx | fail closed，不继续链式跳转 |
| private repo 需要跨 host Bearer | 不转发 PAT；安全失败，后续单独设计 |
| raw/API/preview Markdown 普通请求 | 仍然不跟随任何 redirect |

## 6. Rollback

- 产品代码回滚点是 archive 专用 redirect state machine、typed error mapping 与前端 code 保留三个独立提交片段；不需要数据库迁移或数据回填。
- 若合法 codeload 路径格式与假设不兼容，回滚 handler 即恢复当前 fail-closed 行为，不得临时开启全局 redirects。
- 若 observability 改动产生兼容问题，可保留 transport fix 并回滚新增 code 映射；不得回滚 redaction 或把动态错误直接暴露给 IPC。

## 7. Spec Updates

- `backend/github-import-preview-contract.md`：把“任何 3xx 都不能选第二 URL”改为“共享 client 不跟随；archive 仅允许显式验证的一跳 codeload”。
- `backend/domain-error-enums.md`：记录 archive redirect 语义变体与跨域 typed IPC 映射。
- `backend/redaction-policy.md`：记录 Operation/Runtime 只保留静态 code/phase，禁止 Location/URL。
- `backend/central-update-inventory-progress.md`：记录合法 redirect 完成与拒绝失败的进度结算。
- `quality/test-suite-layout.md`：登记 validator/transport fixture、Update Center persistence 与 frontend i18n 回归位置。

## 8. 全技能分类与持久化

### 8.1 两个统计维度

`SkillRefreshScope` 先解析为 scope 内技能集合；repository progress 只对其中可查询的 GitHub assignment 按 repository identity 去重。两者必须同时保留：

```text
scope skills (141)
  -> classify every skill
     -> queryable GitHub assignments (7 skills / 1 deduplicated repository)
     -> unsupported assignments (134 skills)
  -> inventory buckets; installed update baseline remains untouched
```

repository progress 继续是网络工作的真实分母，但 payload/UI 文案必须说明它不是技能筛选数量。

### 8.2 Unsupported DTO

在 `SkillUpdateInventory` 增加 `unsupported: Vec<UnsupportedSkill>`，并为反序列化提供默认空集合以兼容旧 payload/旧 inventory。`UnsupportedSkill` 只携带 UI 所需的稳定字段：`skill_id` 与枚举 `reason_code`（unknown source / unsupported source type / missing source path / generic unsupported）；不传递文件路径、动态 SQL/HTTP 错误、source type 文本、历史 repository 候选或 token。前端按 code 映射 en/zh i18n。

refresh 现有 assignment/state 分类仍是事实来源，但 DTO reason 必须由结构化 assignment 条件生成，不从 `state.error` 字符串反向嗅探。聚合阶段不再丢弃 `SkillUpdateStatus::Unsupported`；成功查询且 up-to-date 的技能仍不进入 actionable inventory，既有 updatable/added/removed/failed/platform bucket 与 apply 语义保持不变。

### 8.3 原子提交

继续复用 `replace_skill_update_inventory` 的现有 SQL transaction，在同一事务中：

1. 替换 inventory run；
2. 替换该 run 的 entries；

任何步骤失败都 rollback。refresh 不调用 `upsert_skill_update_state`，也不扩展 helper 去写 baseline；unsupported 只持久化为 inventory payload/entry，不填充虚构的远端 hash/branch。trigger-based regression 在 entry 写入阶段故意失败，断言 run/entries 均未出现本轮部分数据，且既有 `skill_update_states` 逐字段不变。

### 8.4 Reload 与 UI

`persist_refresh_inventory` 把 unsupported 写成 inventory entry；`load_skill_update_inventory` 按 bucket 还原。前端 tab union、计数与 renderer 增加 `unsupported`，只提供查看，不进入 apply selection。

`preferredUpdateCenterTab` 在 actionable/error/platform bucket 都为空且 unsupported 非空时返回 unsupported。空状态逻辑把 unsupported 视为有结果，不能显示“无条目”或“全部最新”。中英文文案使用“无法检查/Unsupported”，并说明缺少受支持的远端来源。

## 9. Provenance 边界

历史 snapshot 只能证明部分技能过去属于某 repository，不能证明当前仍应绑定。当前代码中的 detach、重新导入、stale-skill cascade 均可合法改变 membership；跨 snapshot 还存在 identity 冲突。因此本次不做自动 backfill，也不根据 skill 名称或 `skills.source` 推断 repository。后续若要恢复来源，必须单独设计可审计、可预览、按技能确认的数据修复流程。

## 10. 新增回滚点

- DTO/UI 回滚可以移除 unsupported tab，但不得恢复静默“全部最新”文案。
- 原子持久化不得拆开现有 run/entries transaction，也不得把 `skill_update_states` 加入 refresh transaction。
- 不需要 schema migration；bucket 使用现有字符串列，旧 run 无 unsupported entry 时自然加载为空。

## 11. Scanner 权威覆盖保护

`ScanPersistenceBatch` 记录 `central_root_scanned`，它只在配置中的权威 Central 路径真实存在且对应目录扫描成功后置为 `true`。stale parent 删除继续在现有事务内执行，但 SQL 增加覆盖守卫：非 Central 行按既有 keep set 清理；Central 行只有在 `central_root_scanned = true` 时才可清理。

```text
Central root missing / not scanned
  -> is_detected = false
  -> do not touch Central installation/observation keep scope
  -> preserve missing Central parents and owned relations

Central root exists and scans successfully, even when empty
  -> mark central_root_scanned
  -> reconcile missing Central parents
  -> FK cascade and repository pruning retain existing intended behavior
```

本设计不把“keep set 为空”等同于“权威目录为空”。覆盖证据与扫描结果必须同时存在，才能执行 destructive reconciliation。local 与 SSH remote scanner 共用同一持久化标志；任一上游扫描错误仍在持久化前返回，不发布部分批次。

## 12. Migration Checksum Compatibility

`descriptor_checksum(version)` 继续产生唯一 canonical checksum，供新 migration metadata 写入和锁定测试使用。preflight 比较改为版本绑定的匹配函数：先接受 canonical，再接受该版本显式发布过的 legacy alias。当前只登记 migration 1 的 Windows CRLF-era checksum；migration 2-4 不增加 alias。

```text
stored checksum
  -> exact canonical match -----------------> accept
  -> exact alias for the same version ------> accept without rewrite
  -> alias belonging to another version ----> reject
  -> any other value ------------------------> reject before backup/write
```

alias 是发布兼容数据，不是从当前 source 重算出的第二套 hash。这样既恢复 7 月 27 日数据库的可打开性，也不把 checksum 验证退化为“多个算法任选其一”。

## 13. Startup Rebuild Authorization

`attempt_startup` 仍用 typed `DatabaseOpenStage` 分类 open/initialize failure，但 `can_rebuild` 不再仅由文件存在决定。只有 integrity diagnostic 明确为 `Corrupt` 时设置为 true；`Healthy` 说明数据页完整但 schema/版本不兼容，`Unavailable` 没有足够证据，二者都 fail closed 为 retry/exit。

该规则使已知 legacy checksum 通过正常 open 修复，未知 checksum/future schema 留在原地等待兼容版本，而不会通过空库重建掩盖数据兼容 bug。

## 14. Provenance Recovery Preview

恢复来源固定为用户明确选择的 `startup-recovery-*` 数据库，不扫描任意备份目录猜测“最新”。preview 对两库只读打开并检查 integrity/FK，然后以 `skills.id` 为唯一 join key：

```text
backup membership
  -> current parent missing ----------------> missing-parent
  -> current membership absent -------------> addable
  -> current membership same repo/path -----> already-same
  -> current membership differs ------------> conflict

current skill without backup membership ----> unresolved
```

UID、name、`skills.source`、目录后缀和历史 delete log 都不是 join key。preview 输出计数和固定 repository identity，不携带 DB 路径、Central 路径、内容或凭据。真实 apply 是后续显式审批步骤：应用关闭、新备份成功、preview snapshot 未漂移、0 未处理 conflict 后，才在一个事务中插入 repository/membership 与审计记录。

## 15. Canonical Archive Redirect State Machine

GitHub repository owner/name 在路由层大小写不敏感，因此 direct 或 mirror 的正常 codeload 302 可把这两个 segment 与输入做 ASCII case-insensitive 比较；ref 仍精确匹配。已重命名或迁移的 repository 由 GitHub direct API 返回 numeric canonicalization：

```text
built-in direct api.github.com /repos/{old_owner}/{old_repo}/tarball/{ref}
  -> 302 codeload with case-only owner/repo canonicalization
       -> unauthenticated bounded archive read
  -> 301 api.github.com /repositories/{positive_decimal_id}/tarball/{same_ref}
       -> authenticated request to the same trusted API authority
       -> 302 codeload with validated canonical owner/repo components + same ref
       -> unauthenticated bounded archive read

built-in mirror initial response
  -> 302 codeload only when owner/repo are case-insensitively equal to input
  -> 301 numeric canonicalization rejected
```

状态机最多处理 direct API 301 与 codeload 302 各一次。numeric Location 只接受绝对 HTTPS、`api.github.com:443`、无 userinfo/query/fragment、精确五段 path 与正十进制 repository ID；它不能由 mirror 响应授权。canonical codeload owner/repo 仍逐 segment 通过 repository component validation，不接收编码分隔符、dot segment、额外 suffix 或变更 ref。Bearer 只在两个 trusted API 请求中使用，永不复制到 codeload。

实现应显式携带 initial response 是否来自 trusted direct API，不能仅凭 Location host 推断授权；否则 mirror 可把任意 numeric repository 伪装成 GitHub canonicalization。普通 API/raw request acceptance 不变。
