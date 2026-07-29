# GitHub preview 与 import 的 immutable snapshot 绑定

## Goal

让用户在 GitHub import preview 中查看、确认的文件树和 Markdown 与实际导入内容严格一致。preview 必须产出绑定 resolved commit、content digests、目标和过期时间的 opaque token；Local、SSH、WSL import 都只消费该 snapshot，不得按 branch 静默重新解析或下载。对应审计 P1-10，并为 L-02 provenance 提供首个可持久化基线。

## User Value

- preview 与 import 之间即使 branch 被 force-push 或产生新 commit，用户导入的仍是已确认内容。
- snapshot 失效、目标切换或校验失败时，界面明确要求重新 preview，而不是悄悄换成新内容。
- 已安装技能保留 commit SHA 与 content digest，后续更新检查能区分 provenance unknown 与已知基线。

## Confirmed Evidence (2026-07-27 live `dev`)

- `src-tauri/src/services/github_import/types.rs:4-9,55-60`：`GitHubRepoRef` 仍只有 owner/repo/branch/normalized URL；`GitHubRepoPreview` 只有可选 `previewWorkspaceId`，没有 commit SHA、snapshot digest、manifest digest 或 `expiresAt`。
- `src-tauri/src/commands/github_import.rs:32-47,66-88`：Local preview/import 不传 workspace；Local import 按 URL 重新下载。SSH/WSL 才把可选 workspace ID 传入 remote import。
- `src-tauri/src/services/github_import/preview.rs:358-406`：Local preview 构造 `GitHubRepoSnapshot` 后只返回 candidate/file tree，函数退出即丢弃 snapshot。
- `src-tauri/src/services/github_import/remote.rs:376-405`：remote token 缺失、过期或 registry 中不存在时会创建新 workspace 并继续 import；这会把 preview-time 内容静默替换为当前 branch 内容。
- `src-tauri/src/services/github_import/types.rs:161,228-255` 与 `preview_workspace.rs:18-51`：remote workspace 已有 30 分钟 TTL、target/repo/source 绑定、registry lookup/take/prune，可作为统一 snapshot registry 的兼容起点。
- `src-tauri/src/services/github_import/remote.rs:364-367`：remote workspace 仅在成功 import 后消费和删除；失败时保留，现有 UI 具备 retry 语义。
- `src/stores/marketplaceStore.githubImportSlice.ts:63-98,156-202,229-292,455-471`：preview、markdown、import 和 target-reset 已在同一 volatile store 生命周期内；应用重启后没有可恢复 preview UI，因此 token 不需要跨进程持久化。
- `src-tauri/src/db/schema/metadata.rs:16-28` 的 repo 元数据是多技能共享的；同一 repo 的技能可能来自不同 preview。provenance 不能只放 repo 级 row，应使用 `skill_repository_members` 的 nullable per-skill 字段和正式版本化 migration。
- `src-tauri/src/db/migrations.rs:113-179` 与 `migrations/versions/mod.rs`：正式 migration runner 已落地到 v3，本任务新增 nullable provenance 列应作为 v4，不再使用旧 `ensure_column` 演进方式。
- `.trellis/spec/backend/github-import-preview-contract.md`、`local-archive-import.md`：既有契约要求 bounded snapshot、typed error、无副作用校验与 fingerprint mismatch fail-closed；本任务扩展该模式，不另建通用 provenance framework。

## Requirements

### R1. Unified immutable preview token

1. Local、SSH、WSL preview 都返回必填 opaque `previewId`、`resolvedCommitSha`、repository snapshot digest、candidate file manifest（含 path、byte length、SHA-256）和 `expiresAt`。
2. snapshot registry 绑定 target ID/kind、normalized repo/source、resolved commit、manifest 与 snapshot storage；token 不包含路径、凭据或可伪造状态。
3. TTL 沿用现有 remote workspace 的 30 分钟；registry 为应用会话级。应用重启、token 不存在或过期时要求重新 preview。
4. snapshot digest 与 manifest 按稳定 path 排序和长度分帧计算，不能依赖 `HashMap` 遍历顺序或字符串拼接歧义。

### R2. Snapshot-only reads and import

1. `fetch_github_skill_markdown` 和 `import_github_repo_skills` 均要求 `previewId`，只读取 registry 指向的 Local snapshot 或 remote workspace。
2. import 不再接受 repo URL 作为内容定位依据；repo URL/branch 只用于显示、token binding 和后续 update tracking。
3. token 缺失、过期、target/repo/source mismatch、manifest/digest mismatch 均 fail closed，返回稳定 coded error，不自动下载当前 branch。
4. import 失败时保留未过期 snapshot，允许在同一 preview 上重试；显式关闭、target reset、过期和最终消费后清理内存或远端 workspace。

### R3. Provenance persistence

1. 通过 migration v4 给 `skill_repository_members` 增加 nullable `resolved_commit_sha` 与 `content_digest`；旧 row 保持 NULL，解释为 provenance unknown。
2. Local 与 remote import 在技能 DB upsert/repository assignment 的同一 transaction 中写入 preview 的 commit SHA 与该 candidate content digest。
3. overwrite/rename 使用最终 skill ID 写 provenance；skip 不改动现有 row。
4. 本任务不把 Marketplace/archive 等其他来源统一迁入 content-addressed provenance。

### R4. Typed UI failure and compatibility

1. Rust/TypeScript `GitHubRepoPreview`、command map 和 store action 使用统一字段；移除 optional workspace fallback。
2. snapshot expired/missing/mismatch/target-changed 使用可由 `parseBackendError` 识别的稳定 code，并提供中英文“重新预览”提示。
3. preview 页面显示 resolved commit 的短 SHA 与过期时间；不显示本地/远端 workspace 路径或 token 内部信息。
4. Central repository sync/update flows 构造的 preview payload 必须显式适配新 contract，不得伪造 preview token。

### R5. Security and resource bounds

1. snapshot storage 继续受现有 GitHub archive/file/expanded-size budgets 限制；Local snapshot 不复制超预算 blob。
2. logs、operation logs、DB provenance 和 UI 不记录 PAT、workspace path、raw token 或完整文件内容。
3. digest mismatch 在任何 Central 文件或业务 DB mutation 前返回；不得 fallback 到 branch fetch。

## Acceptance Criteria

- [ ] 测试模拟 preview 后 branch 指向变化；Local、SSH、WSL import 均导入 preview-time bytes，且没有第二次 branch download。
- [ ] 同一未过期 preview token 的 file-tree/Markdown reads 返回相同 digest/content；HashMap 插入顺序变化不影响 digest。
- [ ] token 缺失、过期、target/repo/source mismatch 与 tampered manifest 都在 mutation 前返回稳定错误，UI 中英文提示重新 preview。
- [ ] import 失败后可用同一 token 重试；成功后 token 立即消费，再次读取或导入稳定失败并要求重新 preview。
- [ ] migration v4 对旧 DB 保留 NULL provenance，新 import 的 overwrite/rename row 写入正确 commit SHA 与 content digest，skip 不覆盖旧 provenance。
- [ ] Local/SSH/WSL 的重复 Markdown 查看不消费 snapshot；成功 import、显式 discard、target reset 与 expiry prune 清理对应 storage，且日志不含 token/path/content/PAT。
- [ ] `cargo test github_import --locked`、migration/DB 定向测试、相关 marketplace Vitest、`pnpm typecheck`、`pnpm lint` 和 `just ci` 通过。

## Out of Scope

- Marketplace、local archive、WebDAV 等全部来源的统一 content-addressed provenance。
- snapshot 跨应用重启恢复、SQLite 持久化 preview blobs、用户查看/导出内部 token 或 workspace。
- branch protection、签名 commit 验证、GitHub release/tag provenance 或 update center 的完整 content-addressed redesign。
- 改变现有 GitHub archive resource budgets、Central import conflict 产品语义或 AI summary 功能。

## Resolved Decisions

- 沿用 30 分钟 session-scoped TTL；应用重启后重新 preview。
- Local、SSH、WSL 使用同一个必填 `previewId` contract；不保留 optional fallback。
- preview reads 和失败重试在 token 未过期时可重复；过期、target reset 和显式 discard 必须清理。
- 成功 import 后立即消费 token；同一 token 同时只允许一个 import lease，失败释放 lease 供重试，成功原子失效，避免重复 mutation。
- provenance 以 nullable per-skill repository membership 字段通过 migration v4 落库，旧记录标 unknown。
