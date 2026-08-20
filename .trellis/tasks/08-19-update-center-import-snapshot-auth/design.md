# 技术设计：Update Center 不可变新增快照与鉴权错误语义

> `prd.md` 定义结果与验收，本文定义内容权威、数据流、错误边界、兼容和回滚。

## 0. 设计结论

| 决策点 | 方案 |
| --- | --- |
| Refresh 内容权威 | 每个 repository 解析一次完整 commit SHA，并从该 commit 生成 snapshot 与 digest |
| Apply 快路径 | cache 中 commit + digest 同时匹配时直接复用 snapshot，零次 GitHub acquisition |
| Cache miss | 只按已持久化的完整 commit SHA 重取一次并校验 digest；禁止重新解析 branch |
| 跨重启一致性 | pending addition 保存 nullable commit SHA + snapshot digest；cache 只是优化，不是权威 |
| 远端目标 | 在 SSH / WSL 上按固定 commit 创建 workspace，校验 digest 后走 workspace-only importer |
| 旧 pending 数据 | nullable provenance 继续可读；Apply fail closed 并要求 Refresh，不猜测当前 branch |
| 鉴权分类 | `AccessDenied` 保留 `used_auth`，由同一个 typed error 表驱动 IPC 与 diagnostics |
| 用户文案 | `rate_limited`、匿名 `access_denied`、`configured_token_failed` 使用固定双语文案 |
| 安全边界 | token 仍只来自 SecretStore/session；provenance 只含 commit/digest，不含 secret/blob/path |

## 1. 当前数据流与断点

```text
Refresh
  load repositories
  -> prepare_snapshots_for_repo_refs(..., cache)
  -> inspect cached snapshot
  -> persist pending additions (repo/path/name only)

Apply selected
  -> load repository URL
  -> import_github_repo_skills_*_with_auth
  -> resolve branch + acquire repository again       <-- 断点
  -> GithubImportError::AccessDenied(String)          <-- used_auth 丢失
  -> SkillUpdateApplyFailure                         <-- 泛化 code / 动态文本
```

更新项路径已经把 `snapshots_cache` 传给 `update_central_skills_impl`；只有新增导入路径完全绕开 cache。当前 cache 又是短期、进程内且有容量上限，所以“Apply 先查 cache，miss 后照旧按 branch 导入”只能缓解请求量，不能建立确认内容的不变量。

## 2. Pinned Central snapshot

### 2.1 内部值对象

在 `services::central_updates::snapshots` 内引入私有的 repository snapshot authority（最终名称按现有风格确定）：

```rust
struct PinnedCentralUpdateSnapshot {
    display_repo: GitHubRepoRef,
    resolved_commit_sha: String,
    snapshot_digest: String,
    snapshot: Arc<GitHubRepoSnapshot>,
}
```

- `display_repo.branch` 保留用户配置的 branch，用于 UI、repository identity 和后续元数据。
- acquisition 只使用由现有 GitHub snapshot 模块校验过的 40 位 commit SHA。
- `snapshot_digest` 复用 immutable preview 已发布的 repository digest framing，不创建第二种 hash 格式。
- cache entry 同时保存 immutable identity 和 bytes；`get_fresh` 仍受 TTL/LRU/byte budget 管理。
- 为避免扩大公开接口，Central updates 只通过小的 `snapshot() / identity()` 读取面使用该类型；普通 GitHub import DTO 不暴露它。

### 2.2 Refresh 获取

`prepare_snapshots_for_repo_refs*` 对去重后的每个 display repo：

1. 若 `UseFresh` cache 命中，返回同一 pinned entry。
2. 否则通过现有 commit resolver 把 branch/tag/default branch 解析为完整 SHA。
3. 以该 SHA 构造 pinned ref，并使用现有 bounded tree/archive acquisition。
4. 对 retained files 计算 repository digest，连同 SHA 和 bytes 写入 cache；超出 cache budget 时仍把该 entry 返回当前 Refresh 使用。
5. Repository progress、并发上限、retryable transport retry 和 partial failure 语义保持不变。

所有 remote-added discovery、relocation 和 update comparison 继续读取 `entry.snapshot`，因此一次 refresh 中同仓库只有一个内容权威。

## 3. Pending-addition provenance 与 migration

### 3.1 Schema

追加下一版 immutable migration，为 `skill_repository_pending_additions` 增加：

```sql
resolved_commit_sha TEXT NULL,
snapshot_digest     TEXT NULL
```

并同步 runtime metadata/schema generation source。不得修改 migration 1-4 的 source/checksum。旧 row 保持 `NULL`，语义是“旧版本未记录可证明的 refresh snapshot”。

`SkillRepositoryPendingAddition`、SELECT/UPSERT 和测试 fixture 同步增加 nullable 字段。新 refresh 写入两者；同 `(repository_id, source_path)` 的 upsert 必须一起替换 commit/digest，避免 path 被新 refresh 更新但 identity 仍指向旧内容。

### 3.2 为什么不存 blob

SQLite 只保存 40 位 commit SHA 和固定格式 digest。repository bytes 继续受内存/remote workspace budgets 管理，不进入数据库、日志、portable export 或 target cache。跨重启通过 immutable commit 重取即可恢复，不需要长期保存 archive。

### 3.3 Legacy 行

读取保持向后兼容。只要某个要导入的 row 缺任一 provenance 字段，或同仓库选中行的 `(commit, digest)` 不一致，该仓库返回 `central_updates.inventory_refresh_required`，不解析当前 branch。用户 Refresh 后 row 自愈；无需手工清 DB，也不影响其它 repository 的 partial success。

## 4. Apply repository authority

### 4.1 Repository 分组与前置校验

把现有逐 `CentralRepositoryAddedSkillSelection` 的逻辑收进 repository-level helper：

1. 规范化 selections，处理明确的 `Skip`；其余选择按 repository 合并。
2. 从 pending additions 加载每个 selected source path 的 provenance，确认路径均存在且 identity 唯一。
3. 加载 repository 配置并核对 owner/repo/display branch 与 persisted repository id；不从 URL 文本猜测另一 repository。
4. 在任何 Central 文件、membership 或成功 pending-row 删除前取得 verified authority。
5. 从该 authority 重建 candidates，确认每个 selection 仍存在，再进入既有 staged import/mutation lock。

失败只生成一个受控 repository item failure；失败 selections 保留。不同 repository 仍按既有顺序独立结算。

### 4.2 Local

- 首先按 display repository cache key 读取 fresh entry。
- 仅当 `resolved_commit_sha` 与 `snapshot_digest` 都和 pending identity 相等时命中；否则按 cache miss 处理，不能使用“最近但不同”的 entry。
- miss 时以完整 commit SHA 直接调用 pinned snapshot acquisition，不调用 branch resolver；计算 digest 并 constant-time/普通等值比较固定非秘密 digest。
- digest 匹配后调用现有 snapshot-only importer，并使用 commit/candidate content digest 写入既有 `skill_repository_members` provenance。

### 4.3 SSH / WSL

远端不能把 Local cache bytes 当成本地目录直接写入。新增一个窄的 shared service helper：

1. 连接 active target；
2. 使用 display repo + 持久化 full SHA 创建受限临时 workspace；
3. 从 remote manifest 计算同一 repository digest并与 pending identity 比较；
4. 校验 selections 后调用现有 `import_github_repo_skills_remote_from_workspace`；
5. 无论成功或失败都走既有 supervised cleanup。

该 helper 不解析 branch，不接受 renderer 提供 digest，不把 workspace path返回 IPC。Local/remote 都以服务端 DB provenance 为信任输入。

## 5. 鉴权与错误模型

### 5.1 保留 `used_auth`

`GitHubAccessDenial` 已正确根据请求 header 设置 `used_auth`。把 `GithubImportError::AccessDenied(String)` 改为保留该布尔事实的 typed 形状，或等价的两个明确 variant；不得在后续再解析 Display 文本。

唯一分类表：

| 条件 | Stable code | Retryable | 安全提示 |
| --- | --- | --- | --- |
| 429 或已证明的 rate-limit 403 | `github_import.rate_limited` | true | 稍后重试；可配置认证提高额度 |
| 401/403，`used_auth=false` | `github_import.access_denied` | false | GitHub 拒绝匿名访问；私有仓库可配置 token |
| 401/403，`used_auth=true` | `github_import.configured_token_failed` | false | 已使用 token；检查 owner 可见性与 token 权限 |
| provenance 缺失/混杂 | `central_updates.inventory_refresh_required` | false | Refresh 后重新确认 |
| pinned bytes digest 不匹配 | `central_updates.snapshot_changed` | false | 内容验证失败；Refresh 后重试 |

`RateLimited` 保持独立 typed variant。`RepoNotFound`、transport、archive budget 等现有码保持不变。

### 5.2 IPC 与 Update Center item failure

- `GithubImportError::ipc_error_code()` 是 GitHub import code 的单一来源。
- `SkillUpdateApplyFailure::from_github_import` 不再把 `error.to_string()` 作为 public `error`；它用 stable code 查固定英文 public message，并把同一 code 用作 category。
- 前端 `backendErrors` 和 Update Center toast 按 code 翻译；删除/替换依赖英文子串判断 configured-token 的 fallback，仅为旧版本 envelope 保留明确兼容测试。
- Operation Log 只记录 operation、phase、repository logical id 与静态 category；HTTP detail 只允许在不含 secret/URL/path/body 的受控内部诊断中存在。

## 6. Token 边界

Tauri apply command继续通过 `github_direct_auth_from_secret_store()` 取得 app-wide token，并把借用值传入 snapshot acquisition。当前现场的 token 测试成功是验收前提，不触发以下变化：

- 不修改 SecretStore/DPAPI/session fallback；
- 不持久化 token 到 DB；
- 不把 token 传给 public mirror/proxy；
- 不在失败后自动降级为匿名再尝试私有内容；
- 测试只用 sentinel token 和受控 endpoint 断言 header routing。

## 7. Transaction、partial success 与并发

- Existing Central mutation lock、staging/atomic swap、membership transaction、overwrite/rename/skip 语义保持。
- Snapshot identity/digest/candidate 校验在每个 repository 的 mutation 前完成。
- 一个 repository 成功后才删除其成功 imported source paths；另一个 repository 的失败不回滚已成功 repository。
- 同 repository 的并发 apply 继续由 Central mutation/import边界串行化；若实现发现 pending provenance 可能在 authority 校验后被 Refresh 改写，应在 mutation 前二次读取 identity 或在现有锁内完成读取，避免 TOCTOU。
- Clear inventory 仍可删除 pending rows；in-flight Apply 发现 row 缺失时安全失败，不以 selections payload 自行重建来源。

## 8. 文件改动面

| 区域 | 预期文件/责任 |
| --- | --- |
| Snapshot acquisition/cache | `central_updates/snapshots/*`：pinned identity、cache、commit/digest |
| Inventory refresh/apply | `central_updates/inventory/mod.rs`、`repositories.rs`、可选新私有 helper |
| Pending persistence | `db/types.rs`、`db/repos/pending_additions_repo.rs`、schema metadata、下一版 migration |
| GitHub import seam | `github_import/error.rs`、snapshot/remote workspace helper、必要 re-export |
| IPC/public errors | `central_updates/inventory/types.rs`、`ipc_error.rs` |
| Frontend i18n/tests | `src/i18n/locales/en.json`、`zh.json`、backend error / Update Center tests |
| Generated docs | `pnpm docs:gen` 产生的 schema/architecture artifacts，仅在真实生成差异时纳入 |
| Specs | snapshot、inventory retry/progress、redaction、domain errors、migration contract 的已实现规则 |

不预期改变 Tauri command 名称/参数、installer/updater、生产依赖或 repository URL schema。

## 9. 测试策略

### 9.1 精确红灯

- Fake GitHub server 记录 commit resolution、tree/raw/archive 调用次数。
- Refresh 发现同仓库两个 additions；Apply 前不改 server。
- 当前代码红在 Apply 再次 acquisition；修复后 fresh-cache Apply acquisition count 为零。
- 第二用例清 cache 并把 branch tip 移到另一 commit；修复后只访问旧 full SHA 并导入旧 bytes。

### 9.2 安全失败

- cache bytes identity 不匹配时视为 miss，不误用。
- pinned reacquire digest mismatch、selection unavailable、legacy `NULL`、同仓库混合 identity 均在 mutation 前失败。
- 失败后 Central 目录、membership provenance 和 pending rows保持；error envelope 不含动态 detail。

### 9.3 Target/auth matrix

- Local、SSH、WSL 使用同一 commit/digest fixture；remote fake runner 断言脚本只含安全的 full SHA ref，并执行 cleanup。
- 401/403/429 × auth absent/present 的 classifier、IPC、item failure、i18n 全链路测试。
- Sentinel token 只到 `github.com` / `api.github.com` / `raw.githubusercontent.com` / trusted codeload endpoint，不到 mirror。

### 9.4 Migration与回归

- current schema -> next migration、全新库、future/gap/checksum、current reopen、旧 NULL、新 upsert 替换 identity。
- 既有 update batch、repository retry、relocation、preview snapshot、provenance transaction、partial import tests 保持绿。

## 10. 兼容、发布与回滚

- 新列 nullable，旧 DB 与旧 pending rows可读；新版本不执行 destructive data rewrite。
- 新 binary 回滚时新增列可安全保留；已发布 migration source/checksum 不删除不改写。
- 代码回滚不得恢复“cache miss 后按 branch 静默导入”；若必须临时禁用 pinned import，应 fail closed 要求 Refresh。
- 无发布/打包配置改动，验收以定向测试和 `just ci` 为强制门禁；Windows installer 实际发布仍走既有 release gate。

## 11. 被排除方案

- **只依赖 10 分钟 cache**：无法覆盖重启、驱逐和 oversized snapshot。
- **cache miss 后按 branch 重下**：降低不了 TOCTOU 风险，导入内容可能与清单不同。
- **把 archive/blob 存入 SQLite**：扩大数据库、备份、portable 和敏感数据面，收益不必要。
- **只改 token 提示或要求配置 token**：不能消除二次 acquisition，也不能保证 immutable bytes。
- **所有 403 统一成 token failure**：无 token 请求会得到错误建议，且继续丢失真实 auth context。
