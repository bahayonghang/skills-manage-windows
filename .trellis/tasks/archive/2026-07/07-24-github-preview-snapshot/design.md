# 设计：GitHub immutable preview snapshot

## 1. 系统不变量与边界

用户确认 import 时，后端只能读取该次 preview 已注册的字节。branch、tag 或默认分支在 preview 后变化，不得改变 Markdown、文件清单或最终导入内容；token 缺失、过期、binding 不一致或完整性校验失败时必须在 Central FS 和业务 DB mutation 前失败。

本任务统一 Local、SSH、WSL 的 preview authority，并给 GitHub import 写入首个 per-skill provenance 基线。它不把 Marketplace、archive、WebDAV 或 Central update 全部改造成通用 content-addressed 系统，也不持久化 preview blob 跨应用重启恢复。

## 2. Snapshot 数据模型

在 `services::github_import` 内把现有 remote workspace registry 演进为统一 registry。注册项拥有：

- opaque `preview_id`，仅作为随机 lookup key，不编码路径、target、repo 或凭据；
- `TargetBinding`：target ID 与 kind，来自同一个 request-scoped `TargetContext`；
- normalized `GitHubRepoRef` 与解析后的 source root；
- `resolved_commit_sha`、repository `snapshot_digest`、按 path 排序的 repository manifest 与 candidate manifest；
- `created_at`、`expires_at`，TTL 固定为 30 分钟；
- `SnapshotStorage::Local(Arc<GitHubRepoSnapshot>)` 或 `SnapshotStorage::Remote(RemotePreviewWorkspace)`；
- lifecycle 状态：`Ready`、单一 `Importing` lease，以及需要在 lease 结束后执行的 discard 标记。

`GitHubRepoPreview` 返回必填 `previewId`、`resolvedCommitSha`、`snapshotDigest`、`expiresAt`。每个 `GitHubSkillPreviewFile` 在既有 `path`、`byteLen` 外增加 `sha256`；candidate `content_digest` 由其映射后的完整文件 manifest 计算并只在后端 registry/provenance 中使用，不把 digest 交给 renderer 作为信任输入。

## 3. Digest v1 契约

所有 path 先经过既有 safe repository-relative normalization，再按 UTF-8 byte lexicographic order 排序。每个文件先计算原始 bytes 的 SHA-256。聚合 digest 使用 domain-separated、无歧义的二进制 framing：

```text
domain_len:u64be | domain_bytes |
record_count:u64be |
repeat(path_len:u64be | path_bytes | byte_len:u64be | sha256_raw[32])
```

repository 使用 domain `skillport.github.repository-snapshot.v1`；candidate 使用 domain `skillport.github.skill-content.v1`。对外和 DB 保存 `sha256-v1:<lowercase hex>`。固定宽度长度字段、domain separator 和稳定排序共同避免字符串拼接歧义与 `HashMap` 顺序漂移。

Remote inventory 必须返回每个 regular file 的 path、byte length 和 SHA-256，并在 workspace 注册前完成解析、预算与 duplicate/path collision 校验。Local 直接从已获得的 bounded snapshot bytes 生成同构 manifest，不再下载第二份内容。

## 4. Preview 与 registry 生命周期

### Local

Local preview 继续复用 tree/raw fast path 或 archive fallback，但 acquisition 结果必须包含 resolved commit SHA 和完整 `GitHubRepoSnapshot`。preview DTO 构造完成后把 snapshot、manifest 和 binding 注册到 session registry，而不是让 snapshot 随函数退出丢失。

### SSH / WSL

Remote preview 创建现有 bounded workspace，在同一个 supervised remote protocol 中得到 resolved `HEAD`、完整 manifest/digests 和 candidates；只有全部校验成功才注册 token。注册失败或 malformed/budget/integrity 失败必须删除未注册 workspace。

### 清理

- 每次 lookup 先 prune 过期项；Local 释放内存，Remote 通过既有 target-bound cleanup 删除 workspace。
- 新 preview 替换旧 preview、显式关闭、`resetGitHubImport` 和 `resetForTargetChange` 都调用统一 discard command。
- 应用重启时 registry 为空，旧 token 自然失效；不扫描或恢复上次会话 workspace。
- import lease 活跃时的 discard 标记为 pending，lease 结束后清理，避免删除正在读取的 remote workspace。

## 5. Snapshot-only read 与 import

`fetch_github_skill_markdown` 接收必填 `preview_id`、repo 和 source path。registry 先校验 TTL、target、repo/source binding 与 candidate membership，再从 Local snapshot 或 remote workspace 读取 `SKILL.md`；读取 bytes 后重新核对 manifest byte length/SHA-256。该 command 不再走 local raw HTTP fallback。

`import_github_repo_skills` 接收必填 `preview_id`、repo URL 和 selections。repo URL 只用于解析后与 token binding 比较及保留后续 update metadata，不再决定 acquisition。registry 原子获取单一 import lease；并发第二次 import 返回稳定 busy error。

导入前按 registry manifest 重算所选 candidate digest，并验证 selections 只引用已 preview 的 candidates。之后复用现有 staging、Central mutation guard、atomic swap 和 partial import 语义：

- transport/validation/FS/DB 任何失败均释放 lease；未过期且未 discard 的 snapshot 回到 `Ready`，允许同内容重试；
- 完整或 partial import command 成功返回后原子消费 token并清理 storage；即使 selections 全部为 `skip`，这仍是一次成功确认并消费 token；
- token 成功消费后，后续 Markdown read/import 均返回 missing/consumed coded error；
- mutation 已开始后的既有 rollback 语义保持不变，不因 token lifecycle 吞掉原始 domain failure。

## 6. Migration v4 与 provenance transaction

新增独立 immutable migration v4，为 `skill_repository_members` 追加 nullable：

```sql
resolved_commit_sha TEXT NULL
content_digest     TEXT NULL
```

不得修改 v1-v3 source、checksum 或已发布 descriptor。v4 descriptor 使用专属 SQL/source 与锁定 checksum，保持版本连续；新库也按 v1 -> v2 -> v3 -> v4 运行。旧 row 的 NULL 明确表示 provenance unknown。

扩展 repository assignment/upsert API，使 skill row、repository row、membership `source_path`、`resolved_commit_sha` 和 candidate `content_digest` 在同一个 SQLite transaction 中提交。overwrite 使用原 skill ID，rename 使用最终 skill ID；skip 不触碰现有 membership/provenance。Local 与 remote import 调用相同 repository 层 API，避免只修 Tauri command 或单一 transport。

读取模型只在实际需要 provenance 的 repository assignment/domain DTO 增加 nullable 字段；不把这些值写入 operation log、runtime log、portable export 或通用 skill card。

## 7. IPC、前端与兼容调用方

Rust/TypeScript DTO、`IPC_COMMANDS` 与 store action 统一使用 `previewId`，移除 `previewWorkspaceId?:` 和 optional payload fallback。discard command 改为 token 语义；兼容 helper 名称可在实现中一次性迁移，但 renderer 不再判断 Local/remote 才决定是否传 token。

wizard 在 preview header 显示 resolved commit 短 SHA 与本地化过期时间，不显示 raw token、Local snapshot path 或 remote workspace path。稳定错误 code 由 `parseBackendError` 分类为“重新预览”状态，中英文资源覆盖 missing/expired/mismatch/integrity/consumed；wizard 保持当前 surface、清除无效 preview 并允许重新 preview。

Central repository sync/update 的 remote-added flow 不允许继续构造 `previewWorkspaceId: null` 调同一 import contract。它必须在用户确认的 repository snapshot 上取得真实 backend preview token再调用 import，或继续走自身已验证的 Central update snapshot pipeline；不得伪造 token，也不得恢复 branch fallback。具体迁移以 live 调用图为准，并用 contract test 保证所有 `import_github_repo_skills` 调用都提供真实 preview ID。

## 8. 错误、安全与资源约束

新增 typed `GithubImportError` variants，并在 IPC 边界输出稳定 envelope，例如 `github_import.preview_expired:<safe summary>`。至少区分 missing/consumed、expired、binding mismatch、integrity mismatch 和 import busy；服务层不新增 `Result<T, String>`，前端不靠英文子串判断。

token、workspace path、PAT、完整 manifest/content 和 raw remote stderr 不进入错误、日志、operation log、telemetry 或 DB provenance。既有 archive/file/entry/expanded-size budget 对两种 storage 继续生效；digest 计算以已受限 bytes 为输入，不为 hash 再复制超预算 blob。

## 9. 测试、发布与回滚

- 纯测试锁定 digest v1 framing、path order independence、duplicate/path collision 与 tamper detection。
- Local/SSH/WSL 测试模拟 preview 后 branch 变化，证明 reads/import 使用 preview-time bytes，且 import 阶段没有第二次 branch acquisition。
- lifecycle 矩阵覆盖重复 reads、并发 import lease、失败重试、成功消费、expiry、discard、target reset 与 app-session registry reset。
- migration fixtures 覆盖 v3 -> v4、旧 NULL、新 provenance、checksum drift 与 current reopen；DB 测试覆盖 overwrite/rename/skip transaction。
- frontend store/wizard/central sync 测试覆盖必填 token、短 SHA/expiry、双语 re-preview、reset cleanup 和无 optional fallback。

这是内部 contract/schema 追加，不改 release workflow 或 Tauri bundle 配置，因此本子任务验收以定向测试和 `just ci` 为强制门禁，不额外要求 Windows bundle。若实现中发现必须修改打包配置，则回到规划补充 `pnpm tauri build` 与真实产物验收后再继续。

回滚代码时可移除新 registry/IPC/UI 路径，但已应用 migration v4 的 nullable 列保留，不做 destructive down migration；旧 rows/new rows 都可安全解释为 nullable provenance。任何回滚不得恢复 import 按 branch 静默重新 acquisition。
