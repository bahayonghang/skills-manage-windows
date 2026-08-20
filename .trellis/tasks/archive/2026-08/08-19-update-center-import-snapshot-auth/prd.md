# 永久修复 Update Center 新增导入的快照复用与 GitHub 失败分类

## Goal

永久修复 Update Center 已成功发现远端新增 skill、但 Apply selected 又逐仓库重新访问 GitHub 并把新增项全部误报为 access denied 的问题。刷新与应用之间必须共享同一个不可变仓库身份；缓存失效、应用重启或分支移动时也只能按刷新时确认的 commit 重取并校验，不能静默回退到可移动分支。GitHub 拒绝还必须根据请求是否实际使用了已配置令牌给出准确、脱敏、稳定的错误分类。

## Problem Statement

- Update Center refresh 已按仓库批量获取 snapshot 并从该 snapshot 生成 `remote_added`，但 Apply 的 `import_additions` 分支忽略现有 `snapshots_cache`，再次调用通用 GitHub import；同一仓库会被重新解析、重新下载。
- 该二次获取既放大匿名限流/权限故障，也允许 branch 在刷新与确认之间改变，因而不能保证导入的是用户刚刚看到的内容。
- 当前缓存只有 10 分钟、8 个条目、256 MiB，且仅存在于进程内；仅把 Apply 改为“读缓存”无法覆盖过期、LRU 驱逐、超大 snapshot 或应用重启。
- `GitHubAccessDenial` 已记录 `used_auth`，但转换到 `GithubImportError::AccessDenied(String)` 时丢失该事实。Update Center item failure 随后只能输出通用 `github_import.access_denied`，并可能展示“检查 token”这种与实际请求不一致的建议。
- 当前现场已经在 Integrations & Keys 中配置并成功测试 GitHub token；这证明后续请求具备认证条件，但不能替代上述快照一致性和错误分类修复，也不能反推故障发生时的请求一定使用了 token。

## Requirements

1. Refresh 获取每个 GitHub repository 时必须先解析为完整 commit SHA，再从该 commit 生成 bounded snapshot；`remote_added` 的持久化记录必须绑定 `resolved_commit_sha` 与 repository `snapshot_digest`。
2. Apply 必须按 repository 分组处理新增项，并把刷新阶段的不可变身份作为唯一内容权威；同一仓库一次 Apply 不得为每个 skill 单独解析分支或下载 snapshot。
3. 当匹配的 fresh cache 仍存在时，Local Apply 必须直接使用刷新阶段已获取的 snapshot，且 import 阶段对该仓库产生零次 GitHub acquisition。
4. 当缓存过期、被驱逐、因大小未缓存或应用已重启时，Apply 只允许按持久化的完整 commit SHA 重取一次，并在 Central 文件或 repository membership 变更前校验 repository digest；不得重新解析默认分支、tag 或用户配置的可移动 branch。
5. SSH / WSL Apply 必须在远端创建固定到该完整 commit SHA 的受限 workspace，校验与刷新记录相同的 snapshot digest 后调用 workspace-only importer；不得使用 branch-tip workspace fallback。
6. 同一 repository 的所选 pending additions 若缺失 snapshot identity、identity 不一致、固定 commit 无法取得、候选已消失或 digest 不匹配，必须在该仓库任何 import mutation 前安全失败，并返回稳定的“刷新清单后重试”错误；失败项和 pending additions 保留。
7. 升级前已存在且 provenance 为空的 pending addition 保持可读取；不得猜测其来源字节或自动导入当前 branch，必须要求刷新。不得要求用户手工改 SQLite 或清空整个 Central。
8. `GitHubAccessDenial.used_auth` 必须保留到 domain error、IPC envelope、Update Center item failure 和诊断分类。至少区分：`github_import.rate_limited`、未认证的 `github_import.access_denied`、已配置令牌请求的 `github_import.configured_token_failed`。
9. 用户可见错误必须使用固定、可本地化的安全文案：未认证拒绝可建议配置 token；已认证拒绝应建议检查仓库可见性、token owner 与权限；不得把所有 403 都描述为 token 权限问题。
10. Apply 的 item failure 不得直接把动态 HTTP detail、响应正文、token、repository URL、source path、远端 workspace path 或本地路径暴露到 UI、Operation Log、Runtime Log、telemetry、SQLite 或 portable export。
11. 已配置 token 必须继续只经 `SecretStore`/session fallback 获取，并只发送到受信 GitHub endpoint；本任务不得把 token 写入 snapshot provenance、pending additions 或 target cache。
12. 保留现有 partial-success 语义：不同 repository 可独立成功/失败；成功导入后才删除对应 pending addition，失败 repository 不得撤销其它已完成 repository，也不得误删失败项。
13. overwrite / rename / skip、更新项批处理、远端缺失决策、retry repository slice、relocation、Local / SSH / WSL 目标和 existing per-skill commit/content provenance 不得回归。

## Constraints

- 通过追加的不可变 migration 扩展 pending-addition provenance；不得改写已发布 migration 或用可变 runtime schema 源冒充历史 migration。
- 不把 repository archive/blob 保存到 SQLite，不新增长期 token 存储，不扩大为通用 GitHub import wizard 重构。
- 不新增生产依赖，不改变 updater/installer/release pipeline。
- 修复落在共享 Rust service/repository 边界；Tauri command 只负责取得 active target、client、cache 与 secret。
- 测试使用 fake HTTP/remote runner、内存数据库和临时目录，不访问真实 GitHub、不读取真实 token、不修改用户 Central 或本机应用数据库。

## Acceptance Criteria

- [ ] 精确回归证明：refresh 已发现一个 repository 的多个 additions 后，fresh-cache Local Apply 成功，Apply 阶段没有第二次 commit resolution、tree/raw/archive 请求。
- [ ] 清空/过期 cache 后，Apply 只请求持久化的完整 commit SHA，一仓库至多一次 acquisition；branch 在 refresh 后移动也仍导入 refresh-time bytes。
- [ ] 固定 commit 重取后的 snapshot digest 不匹配、selection 不存在或 repository identity 混杂时，在 Central FS / membership mutation 前失败，返回稳定 refresh-required code，并保留 pending additions。
- [ ] 旧 pending row 的 commit/digest 为 `NULL` 时可正常加载，但 Apply 安全失败并要求 Refresh；新 Refresh 会自愈写入 provenance。
- [ ] SSH 与 WSL fake-runner 测试证明 remote workspace 使用完整 commit SHA、校验相同 digest、没有 branch fallback，并维持 workspace cleanup。
- [ ] 同一 Apply 中一个 repository 成功、另一个 repository 失败时结果保持 partial success；只删除成功导入或明确 skip 的 pending rows。
- [ ] 401/403/429 矩阵覆盖有/无 token：domain、IPC、Update Center item failure、Operation Log category 与前端双语提示均输出精确稳定 code，不依赖英文子串。
- [ ] 错误与日志断言不含测试 token、Authorization header、repository URL、source path、HTTP body、workspace path、SQLite detail 或本地路径。
- [ ] 新 migration 从当前 schema 升级、全新库、checksum/current reopen、旧 `NULL`、新 provenance upsert 与 rollback fixture 全部通过；生成的 schema/architecture 文档已同步。
- [ ] GitHub import、Central update inventory、DB migration、frontend backend-error/i18n 定向测试通过；Rust fmt、全 targets Clippy、locked tests、`pnpm docs:gen:check`、`pnpm ipc:codegen:check` 与 `just ci` 通过。

## Out of Scope

- 修改 GitHub token 的保存方式、权限范围、生命周期或 UI；现场 token 已配置并测试可用。
- 自动扩大 token 权限、绕过私有仓库访问控制、使用公共 mirror 接收 token，或把认证失败自动降级为匿名成功。
- 把 snapshot archive 长期持久化到数据库、portable export 或 target cache。
- 清理用户现有 inventory、修改现有 Central skill 内容、发布 Windows 安装包或变更 updater 元数据。

## Notes

- 根因与代码证据见 `research/root-cause.md`。
- 当前任务保持 `planning`；用户审阅并明确允许实施后才运行 `task.py start`。
