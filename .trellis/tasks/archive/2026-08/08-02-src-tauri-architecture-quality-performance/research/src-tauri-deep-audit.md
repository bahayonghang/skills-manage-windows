# src-tauri 深度架构、质量与性能审计

## 1. 审计基线与方法

- 基线：`dev@b242ed92`。
- 范围：`src-tauri/src/**/*.rs`、相关 `.trellis/spec/backend|quality`、`docs/architecture`，以及 2026-07-24 已归档审计任务，用于排除已闭环问题。
- 规模：278 个 Rust 文件，约 88,088 行；`services/` 54,603 行、`db/` 11,885 行、`commands/` 9,667 行、`targets/` 5,738 行。
- 测试画像：生产树内约 1,056 个 `#[test]` / `#[tokio::test]` 标记。结论不是“缺少测试”，而是若干高风险边界缺少针对失败原子性、容量和恶意输入的测试。
- 方法：按 IPC/CLI -> service use case -> repository -> filesystem/network/target 的数据流追踪；检查路径来源、分配前预算、锁与事务边界、缓存所有权、分页执行位置、错误和测试契约。
- 规划基线门禁：`just check` 与完整 `just ci` 均通过。`just ci` 覆盖 common 与 Windows Rust lanes，主 Rust suite 为 1,031 passed / 6 ignored，且 Clippy、fmt、frontend、IPC codegen 和 docs build 全绿。环境仍提示当前 Node `25.9.0`，仓库固定 Node 22 LTS；这是环境漂移，不记作 Node 22 下的验证证据。

## 2. 当前架构画像

```text
React / skillport-cli
        |
        v
commands/* (Tauri IPC adapter)    cli_api/* (CLI adapter)
        |                              |
        +------------+-----------------+
                     v
services/* (use cases, domain errors, orchestration)
        |             |                 |
        v             v                 v
db/repos/*       targets/*        filesystem/network
SQLite           Local/SSH/WSL    locks, journal, budgets
```

整体方向是健康的：`services/mod.rs:1-5` 明确 service owns orchestration，`db/mod.rs:1-11` 把 repo 定义为 SQL 所有者，并把 broad re-export 标记为迁移期兼容层。`cli_api` 也有活动 spec 禁止调用 `commands::*`。

成熟且应复用的能力：

- `TargetContext` 将 target、DB 与请求生命周期绑定，避免 target switch race。
- `SecretStore`、DPAPI/keyring fallback 与统一 redaction 保住凭据边界。
- Central mutation lock 与 FS+DB operation journal 已覆盖 Central delete/update/recovery。
- GitHub import 已有 immutable preview snapshot、import lease、完整目录导入、provenance 与资源预算。
- SSH/WSL process supervision、remote canonical containment 和 path policy 已有跨 transport 契约。
- `ExclusiveJob` 使用 RAII lease，取消和异常退出不会永久占用 job。
- 生产代码未发现可泛化为新任务的 `unsafe` 或 `unwrap` 热点；Windows foreground 与 DPAPI FFI 范围明确。

主要问题不是“没有分层”，而是少数旧路径绕过了成熟层，另有若干缓存、输入和批量写入没有完成资源/事务闭环。

## 3. Findings

### F1 - P0 - Marketplace 安装可越出 Central 根并绕过完整安装契约

证据：

- `services/marketplace/mod.rs:475-476` 直接执行 `central_dir.join(skill_name)`。
- `services/marketplace/mod.rs:486-523` 从缓存行读取 `download_url`，用独立、无 timeout/redirect policy/body cap 的 client 下载完整响应。
- `services/marketplace/mod.rs:525-551` 用 `skill.name` 构造 Local/SSH/WSL 路径，直接创建目录并只写一个 `SKILL.md`。
- `services/marketplace/mod.rs:555-559` 文件写完后只更新 `marketplace_skills.is_installed=1`，没有 skill upsert、repository provenance、Central mutation lock 或 operation journal。
- `services/github_import/source.rs:247-278` 表明 Marketplace name 来自远端 `SKILL.md` frontmatter；`services/scanner/mod.rs:96-111` 的共享解析只要求 name 存在，不保证它是单一路径组件。
- `paths.rs:428-437` 的 `remote_join` 只拼 POSIX 路径，不消除 `..`。Local Windows `Path::join` 还会接受 drive/root 语义。
- `docs/architecture/marketplace-pipeline.md:38-53` 声称下载 folder peers、upsert skills 且复用 `ensure_centralized`，与实现不一致。
- 对照实现 `services/marketplace/skills_sh.rs:243-309` 已经通过 GitHub snapshot/import use case 安装完整目录。

影响：恶意或误配置 registry 的 frontmatter name（例如 `../outside` 或 Windows 绝对路径）可让用户触发 Central 根外写入；普通名称也只得到孤立文件，数据库/来源/安装标记可能互相矛盾。失败恢复、跨进程互斥和 CLI 共享状态均被绕过。

修复方向：移除该直接下载/写文件路径。以 registry + 当前同步候选的稳定 identity 重新获取受控 GitHub snapshot，解析后匹配唯一候选，并调用现有 Local/SSH/WSL GitHub import use case。`download_url` 只能作为展示/兼容字段，不能继续作为请求 authority；安装成功后才更新 Marketplace 状态。

任务：`08-03-marketplace-install-central-contract`。

### F2 - P1 - GitHub snapshot 状态有 TTL 但没有容量边界，远端清理会丢失所有权

证据：

- `services/central_updates/snapshots.rs:59-107` 使用无上限 `HashMap<String, CachedGitHubSnapshot>`；过期只导致 miss，不删除 entry。
- `services/central_updates/snapshots.rs:71-84` 每次命中 clone `GitHubRepoSnapshot`；该类型在 `github_import/types.rs:170-173` 内含 `HashMap<String, Vec<u8>>`，所以是完整 bytes 深拷贝。
- `services/central_updates/snapshots.rs:281-284` 下载后又 clone 一次，分别放入 cache 与本次结果。
- `services/github_import/snapshot_registry.rs:20-61` 的 process-global registry 没有 entry/byte cap；lookup `:65-80` 拒绝 expired token 但保留 entry。
- `services/github_import/remote.rs:4-16` 会全局 prune 所有 target 的 expired entry，却只对当前 connection 同 target 的 workspace 调 `remove_tree`。其他 target 的 registry ownership 已删除，远端 workspace 却无法再被找到和清理。
- 活动 spec 已说明 prune-on-lookup 因并行测试耦合被撤回，且 import lease 不能中途回收；优化必须保留这些正确约束。

影响：长时间运行、反复 Central refresh 或异常 renderer 流程会造成内存增长和大快照复制；跨 target 预览会形成远端临时目录泄漏。单个 archive expanded budget 可达 256 MiB，缓存没有总量上限时进程级风险仍然存在。

修复方向：快照 bytes 改为共享 `Arc`；分别定义可测试的 entry/aggregate-byte limits 和确定性淘汰策略；过期/淘汰必须返回 ownership 给能连接 owning target 的清理协调器。活跃 lease 不淘汰，清理失败要保留可重试 ownership，不能静默丢引用。

任务：`08-03-bounded-github-snapshot-lifecycle`。

### F3 - P1 - 外部响应和文本在限额前完整分配，并存在 UTF-8 byte slicing panic

证据：

- AI SSE：`ai_provider/stream.rs:133-138` 只有 connect timeout；`:255-340` 的 `sse_buffer` 与 `full_text` 无累计上限或 idle/total deadline；`:358-363` 完成事件又 clone 全文。
- AI 错误 body：`ai_provider/stream.rs:208-236`、`ai_provider/claude.rs:170-199,279-304` 先 `.text()` 完整读取。AI tagging client 在 `ai_tagging/mod.rs:231-242` 无 timeout，响应在 `ai_tagging/prompt.rs:114-141` 完整读取。
- Marketplace：`marketplace/mod.rs:502-523` 无 deadline、redirect policy 或 body cap；P0 子任务完成后该下载路径应删除，而不是重复加固。
- Git tree：`github_import/tree_manifest.rs:320-331` 只预检可选 `Content-Length`，chunked 响应会先 `.text()` 完整分配，再由 parser 检查 16 MiB budget。
- 主 skill 读取：`central_skills/files.rs:49-85` 的 Local/remote `SKILL.md` 无 1 MiB budget；任意 remote 文件在 `:394-400` 才在 `read_file` 完整分配后检查。
- scanner/AI：`scanner/mod.rs:91-93` 和 `ai_tagging/prompt.rs:19-28` 都先完整读文件；后者只在 prompt 构造的 `:62` 截 4,000 chars。
- `ai_provider/prompt.rs:85-90` 文档称截 8,000 chars，实际按 byte length 判断并执行 `&content[..8000]`。中文等多字节内容在 8,000 不是 char boundary 时会 panic。
- `ai_provider/claude.rs:199,304` 对 provider body 的 500-byte 摘要有同类风险。

影响：慢流可长期占用任务；chunked/恶意响应与大文件可在拒绝前消耗不可控内存；常见中文 SKILL.md 可确定性触发后端 panic。现有 `ResourceBudget` 已有 1 MiB file 和 16 MiB tree cap，但调用位置不完整。

修复方向：提供共享 bounded response/file reader，在每个 chunk 上 checked-add 并尽早取消；AI streaming 单独设置 idle、total、wire bytes 与 decoded text 上限；Local/remote 文件 API 接受 budget，在 transport 分配前截断/拒绝；所有摘要/提示截断使用 UTF-8 安全 helper。

任务：`08-03-bounded-external-text-ingestion`。

### F4 - P2 - Central 分页 API 仍为 O(N) 全量加载、富化和同步文件 stat

证据：

- `central_skills/query.rs:191-214` 先读取所有 Central skills、agents，并以全部 skill IDs 查询 installations/repositories/tags。
- `central_skills/query.rs:215-228` 对每一行富化；`skill_time.rs:40-64` 在缓存时间缺失时同步 `std::fs::metadata`。
- `central_skills/query.rs:281-293` 到最后才在内存 filter、sort、skip/take。
- 三个 batch repo helper 都按全部 ID 生成动态 `IN (?,...)`：`installations_repo.rs:102-110`、`repositories_repo.rs:643-651`、`tags_repo.rs:468-495`。
- 当前分页测试 `central_skills/tests.rs:508-537` 只有两条 Central rows，没有大库、特殊 filter 组合、查询计划或 enrichment 数量回归。

影响：page size 即使为 1，也会读取和富化全库；搜索、换页和排序成本随 Central 总量线性增长，缺失时间缓存时还会在 async 路径同步 stat 全库。大库动态 binds 最终也会接近 SQLite variable limit。

修复方向：repository 层使用绑定参数完成 filter/count/order/limit/offset，使用 `instr(lower(...), ?)` 保持 `%`/`_` 的 literal contains 语义；只对本页最多 500 个 ID 批量富化。分页列表以持久化 `fs_*` + `scanned_at` fallback 为排序/展示 authority，避免 hot path stat；用旧 evaluator 作为测试 oracle，覆盖 tags/source/install-state 特殊语义。

任务：`08-03-sql-central-pagination`。

### F5 - P2 - 多步 metadata/cache mutation 缺少事务，失败会留下部分状态

证据：

- `repositories_repo.rs:240-250` detach 依次删 update state、repository member、prune repo；`:571-605` 对多个 skill 逐条 assignment，均无 transaction。
- `tags_repo.rs:127-167` 对 skill x tag 逐条提交；`:193-215` 先删全部 AI links 再逐条加；`:218-269` 先删 pending reviews，再在循环中验证/写入。后续失败会丢旧状态或保留部分新状态。
- `collections_repo.rs:80-90` 显式删 membership 后再删 collection；collection relation 没有 collection FK cascade。
- `projects_repo.rs:102-115` 在 pool 已对每条连接启用 FK 的情况下仍分两步删；第二步失败会留下 project 空壳。
- `marketplace/mod.rs:238-266` registry 两步删除无 transaction；`:363-403` sync 逐条 upsert 后才写 success，失败可留下混合 cache，而且没有删除远端已消失的旧 row。
- 仓库已有正确事务模板，例如 `skills_repo.rs`、`repositories_repo.rs:425-439`、`tags_repo.rs:340-435`，不需要引入新框架。

影响：批量命令可能返回 error 但数据库已经部分改变；AI 建议刷新失败会破坏旧建议；Marketplace “成功”缓存不是远端 snapshot，已删除技能会永久残留。

修复方向：在 repository/use-case 顶层开启 transaction，内部 helper 接受 transaction executor；先验证再 mutate，并用 trigger 故障注入证明 rollback。Marketplace 成功 fetch 后在单个 transaction 中原子替换该 registry cache 并更新 success metadata，失败保留旧 cache、记录 error attempt。

任务：`08-03-transactional-metadata-mutations`。

### F6 - P3 - command/service/repository 边界仍有迁移债，但不适合本轮全仓重构

证据：

- `commands/` 仍有 38 个公开 `*_impl`；主要集中在 agents、collections、saved_views、settings、tag_groups。
- `commands/bootstrap.rs` 仍直接聚合多个 repository 并包含 raw SQL；commands 下约 154 个 `db::`/`sqlx::query` 命中（含测试）。
- `db/mod.rs` 继续 glob re-export 全部 repos，文档已将其标记为兼容层。
- 与此同时，`shared-local-cli.md` 明确规定 `cli_api` 只能调用 services/repositories，不能调用 commands；`domain-error-enums.md` 也允许 command internal helper 暂留 String。因此命中数量本身不是正确性 bug。

判断：不创建“搬空 commands”式任务。F1 和 F5 会在真实 use case 上建立更深边界；完成后再按可观察收益审计 bootstrap/settings/metadata，而不是先做目录重排。

## 4. 测试与质量结论

现有测试密度高，并且以下高风险能力已有有效回归：immutable preview/import lease、GitHub archive/tree budget、Central operation rollback、orphan repair transaction、remote path containment、process cancellation/output limit、secret migration/redaction、target context 和 typed IPC。

真正缺少的是针对本轮发现的测试形状：

1. Marketplace frontmatter name 为 `../x`、absolute/drive/UNC、分隔符和正常 display name；Local/SSH/WSL 全路径。
2. 快照 cache 达 entry/byte limit、TTL 后 reclaim、active lease、跨 target cleanup failure/retry。
3. chunked oversized response、slow/no-progress stream、UTF-8 boundary、remote pre-allocation limit。
4. 5k+ Central fixture 的 reference-equivalence、page-only enrichment 和 `EXPLAIN QUERY PLAN`。
5. transaction 中间步骤 trigger failure、混合 valid/invalid IDs、Marketplace stale row replacement。

## 5. 性能优化验收原则

- Cache：记录 entry count、retained bytes、eviction/reclaim outcomes；常驻状态必须有硬上限。
- Ingestion：wire bytes 与 decoded/output bytes分开计量；限额在每个 chunk/read 发生时检查。
- Pagination：同一 deterministic fixture 对比旧 evaluator 和新 SQL；记录返回相同 ID/total，并证明 enrichment 输入不超过 page limit。
- Transactions：优先减少逐行 round trips，但不能用超长动态 `IN`/VALUES 取代正确性；需要时按 SQLite bind 上限分块，并由一个 transaction 包住全部 chunk。
- Wall clock：在 release build、固定 fixture 和多轮 warm-up 后记录 before/after p50/p95，作为证据而非跨机器硬阈值。

## 6. 已排除与避免重复立项

- 2026-07-24 的 SSRF、renderer capability、TargetContext、process supervisor、remote canonical path、DB FK/migration、FS+DB journal、exclusive job、typed IPC、startup resilience 等子任务均已归档。
- 旧 net-boundary 任务明确未覆盖 Marketplace renderer/AI endpoint；本轮发现的是 backend Marketplace install 和 remaining ingestion delta。
- `unsafe` 主要位于 Windows foreground/DPAPI FFI；未发现新的广泛内存安全问题。
- scanner 只扫描配置根的直接子项，目录树已有 depth/entry budget；不把它们误报为无界递归。
- 日志和 operation log 已统一 redaction；没有证据表明 PAT/API key 被明文持久化或导出。

## 7. 建议顺序

1. 先完成 Marketplace P0，消除任意路径写和错误安装状态。
2. Snapshot lifecycle 与 bounded ingestion 可分别规划；ingestion 中 Marketplace 代码以 P0 完成后的终态为准。
3. Transactional metadata 可与 snapshot 工作并行，但 Marketplace install marker 归 P0 task，sync/remove 归 transaction task。
4. Central SQL pagination 独立实施，先锁行为 oracle 和性能基线。
5. 五个 child 全部验收后，父任务做一次跨域锁顺序、helper 复用、资源常量、generated docs 和 `just ci` 集成复核。
