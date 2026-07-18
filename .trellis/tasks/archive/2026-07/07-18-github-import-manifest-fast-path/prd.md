# GitHub 导入清单快路径与性能基线

## Goal

为本地 GitHub import preview/import 增加基于 Git tree manifest 的候选发现和选中子树下载快路径，在受支持仓库中避免无条件下载完整 archive；同时保留现有 archive 作为可靠回退，并用可复现基线证明收益和回归边界。

## Background

- 当前本地 preview 在解析 URL 后立即下载完整 archive（`src-tauri/src/services/github_import/preview.rs:136`）。
- 当前本地 import 再次下载完整 archive（`src-tauri/src/services/github_import/import.rs:32`），preview 与 confirm 之间没有本地 workspace 复用。
- 现有 archive parser 已执行文件数、压缩/展开字节、单文件和安全相对路径检查（`archive.rs:70`、`:105`、`:166`）。
- archive parser 只接收 tar regular file，跳过 symlink、目录及其他非文件条目（`archive.rs:112`）；tree 快路径必须显式保持这一过滤语义。
- preview DTO 已包含 pluginName、完整文件清单与冲突（`types.rs:38`）；文件清单缺根 `SKILL.md` 会阻断（`preview.rs:99`）。
- HTTP 层已经支持 PAT、host rate limit、直接 GitHub/镜像 fallback 和 typed denial（`raw_http.rs:112`）。
- SSH/WSL preview/import 使用远程 workspace，本任务不替换该路径（`commands/github_import.rs:37`、`:74`）。
- SkillKit 的参考实现是 `git/trees?recursive=1` + raw metadata + 选中子树下载，失败回退 tarball（`ref/skillkit/apps/desktop/electron/installer.ts:704`、`:736`）。

## Requirements

### R1. 先建立基线

- 实现前建立可控 mock HTTP fixture，记录 archive preview/import 的请求数、传输字节、解析文件数、总耗时和峰值内存代理值。
- fixture 至少覆盖根 skill、小型嵌套 skill、多 skill 大仓库、plugin manifest、私有/PAT、限流、tree truncated、镜像失败、mode `120000` symlink blob 和 mode `160000` submodule/gitlink。
- 基线与复测数据写入任务 research；网络实测可补充，但不作为 CI 的唯一证据。

### R2. Tree manifest preview

- 优先通过现有 API/mirror HTTP 边界获取 recursive tree，清单条目至少包含安全 path、type 和 byte size；truncated 或无可信 size 时进入 typed fallback。
- 只有 regular blob mode（`100644` / `100755`）进入候选、文件清单与 raw 下载；symlink blob `120000` 和 gitlink/commit `160000` 必须跳过，未知 mode/type 进入 typed archive fallback，不能把 symlink target 文本当普通 skill 文件。
- 候选路径发现必须复用现有 `discover_skill_manifests_from_paths*`、plugin manifest 和 frontmatter 规则，不得在快路径复制一套扫描语义。
- 只读取候选所需的 plugin manifest 与 `SKILL.md` 文本；文本和 JSON 继续受 `ResourceBudget` 限制。
- preview 文件清单由 tree path/size 通过现有 `repo_file_relative_to_source` 生成，输出的 `GitHubRepoPreview`、`GitHubSkillPreview`、pluginName、files、冲突和 error 语义保持兼容。

### R3. 选中子树 import

- import 根据 selection 计算所选 sourcePath 的文件并集；根 sourcePath `.` 表示完整仓库，嵌套 source 只包含精确子树。
- raw bytes 下载采用有界并发、稳定路径排序、单文件/总字节/文件数预算；重复覆盖的文件只下载一次。
- 所有选中文件必须在 Central mutation 开始前下载并校验完成，再转入现有 staging/atomic persistence；快路径不得产生部分 Central 写入。
- 文件数太多、根 skill、API/镜像/PAT 不兼容、网络错误、预算/校验异常时按测量后的阈值选择 archive；不能把逐文件请求放大伪装成优化。

### R4. Archive fallback

- fallback 是一等路径：复用现有 `download_repo_snapshot` 和 snapshot import，不移除 archive tests。
- 只有 acquisition 层可触发 fallback；一旦进入 Central staging/writing 不得重新下载 archive 或切换模式。
- rate limit、access denied、404、truncated、transport、invalid manifest 等行为必须有明确 typed 分类；用户看到的错误不能因尝试快路径而更模糊。

### R5. 缓存与一致性

- 首版先测 preview→confirm→import 的重复 tree 成本。只有未加缓存时无法达到已定义目标，才增加最多 4 项、10 分钟 TTL 的内存 metadata cache。
- cache 只能存 tree/candidate metadata，不存 PAT 或长期文件内容；key 必须包含规范化 owner/repo/branch/sourcePath，切换 PAT/目标或超时后失效。
- 本任务不承诺 pin commit；UI 继续保持现有“预览快照而非落盘证明”语义。

### R6. 兼容与可观测

- 保持 Local/SSH/WSL command、PAT secret store、镜像、`previewWorkspaceId`、selection 和 result DTO 不变。
- 内部测试/日志记录 acquisition mode、fallback reason、request count、transferred bytes 和 elapsed；不把这些字段写入 Central schema 或用户 source metadata。
- 更新 GitHub import backend spec，明确 tree/archive parity 和 fallback matrix。

## Acceptance Criteria

- [ ] 基线报告包含至少 10 类 fixture 的 before/after 请求数、字节与耗时，未把未测网络收益写成通过。
- [ ] 支持的嵌套 skill preview 不请求 tarball，候选/pluginName/files/conflict 与 archive fixture 完全等价。
- [ ] TreeRaw 与 Archive 对 mode `120000` symlink 和 mode `160000` gitlink 产生相同候选/文件清单：两者均不成为 skill 文件，也不触发 raw 下载。
- [ ] 选择 1 个嵌套 skill 时只下载该子树；多选下载并集且去重；根 skill 按阈值走明确策略。
- [ ] fast path 的所有文件在任何 Central mutation 前准备完成；失败不会留下 staging、backup、目录或 DB 部分状态。
- [ ] truncated、PAT denial、rate limit、mirror failure、超预算、无 size、raw 404/5xx 和解析失败均覆盖正确 fallback/error 行为。
- [ ] 现有 plugin manifest、root skill、file manifest、selection payload、partial import、Central update 和 remote workspace tests 无回归。
- [ ] 如加入 cache，覆盖 TTL、LRU、key 隔离和失效；如未加入，research 记录数据驱动的拒绝理由。
- [ ] `cargo test services::github_import`、相关 Vitest、`cargo clippy -- -D warnings`、`git diff --check` 和 `just ci` 通过。

## Out of Scope

- 修改 SSH/WSL 的远程 archive workspace acquisition。
- 固定 commit、离线仓库缓存、持久化 tar/tree cache 或断点续传。
- 改变 GitHub preview UI、selection payload、pluginName/files 持久化边界或数据库 schema。
- 用固定“300 文件”阈值照搬 SkillKit；阈值必须来自 SkillPort 基线和资源预算。
- 为全应用建立通用 telemetry 平台。
