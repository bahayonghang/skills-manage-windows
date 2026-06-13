# skills-manage-windows 优化实施计划

> 输入：`docs/reports/skills-manage-windows-optimization-canvas.md`  
> 输出类型：优化实施计划（本文件只规划，不实施代码改动）  
> 当前核对范围：当前 checkout 为 `dev`（2deba98）；只读抽样核对本地 `main` / `origin/main`（14b61c0）。未切换分支，因为工作树已有 unrelated dirty/untracked 改动。

## 0. 总结结论

审计报告的大方向成立，但需要重排优先级：

- **必须优先处理（P1）**：S-01 path IPC、S-02 CSP/capability、D-01 GitHub import overwrite 原子性、E-01 PR/push CI。
- **中期处理（P2）**：P-01 资源预算/取消、T-01 typed domain model + DB CHECK、DOC-01 安全文档一致性。
- **重述/降级处理**：R-01 release/updater 已有 release workflow 注入与签名校验，应改为 preflight 回归保护；A-01 是可维护性架构债；W-01 是低优先级 Windows path 语义修正。
- **Windows-first 约束**：任何 path、打包、CI、release 改动都必须至少覆盖 Windows 本地/CI 场景；打包链路不能只验证前端构建。

## 1. 逐项深入分析与实施切片

### S-01 — Renderer IPC 任意 path 读取/列目录/打开文件管理器

**结论：成立，P1。**

证据：
- `src-tauri/src/commands/skills.rs:337-354` 仍暴露 `read_file_by_path(path)`, `open_in_file_manager(path)`, `list_directory_tree(path)`。
- `src-tauri/src/services/central_skills/files.rs:54-105` 本地 read/list 直达 `std::fs` 与递归 `read_dir`，未见 canonical allowlist / depth / entries / bytes cap。
- `src/stores/skillDetailStore.ts:255,429,447,484` 仍由前端以 `file_path` 或目录树 path 调 IPC。
- `git show main:<path>` 抽样确认 main 同样存在这些路径。

**目标状态：** renderer 不再提交任意绝对 path；所有 path 操作先过后端 `PathPolicy`，长期迁移为 `skillId + relativePath` 或 opaque handle。

**实施切片：**
1. 新增 `src-tauri/src/security/path_policy.rs`（或 `services/security/path_policy.rs`）：
   - 输入：active target、DB skill row / central root / project root / platform roots、请求 path。
   - 本地：canonicalize 后必须是允许 root 的 child；拒绝 `..`、UNC/extended path 逃逸、symlink 跳出、目录外绝对 path。
   - 远端：仅允许 remote skill/root 下的 normalized POSIX path；禁止相对穿越。
   - 加默认 caps：`max_file_bytes`, `max_tree_depth`, `max_tree_entries`。
2. 保留旧 command 名称一版，但在 command 内改为 policy guarded：
   - `read_file_by_path` 只允许 DB 中当前 detail path 或目录树已签发 path。
   - `list_directory_tree` 只允许 skill dir / central root child。
   - `open_in_file_manager` 只允许本地 allowed root child。
3. 新增 typed IPC：
   - `read_skill_file(skill_id, relative_path)`
   - `list_skill_directory(skill_id, relative_path, max_depth?)`
   - `open_skill_location(skill_id, relative_path?)`
4. 前端迁移 `skillDetailStore` 到 typed IPC；旧 IPC 标为 deprecated，仅 dev/log 观测后删除。

**验收：**
- Rust tests：读取 `~/.ssh/id_rsa`、`C:\Users\...\AppData`、`..` traversal、symlink 跳出必须失败；合法 Central skill `SKILL.md` 和子文件可读。
- 前端 tests：SkillDetail raw source / directory tree 仍可打开合法 skill 文件。
- Gates：`pnpm typecheck && pnpm lint`，`cd src-tauri; cargo test central_skills path_policy`，再跑 `just ci`。

**风险/回滚：**
- 风险：历史 DB 中的 `file_path` 指向旧目录或 remote path；需兼容 Central location migration。
- 回滚：保留 guarded legacy command，不提供 `safePathGuard=off` 给生产；最多允许 dev-only bypass。

---

### S-02 — CSP 关闭 + Tauri capability 过宽

**结论：成立，P1。**

证据：
- `src-tauri/tauri.conf.json:25` 为 `"csp": null`。
- `src-tauri/capabilities/default.json:9-15` 包含 `sql:default`, `fs:default`, `shell:default` 等宽权限。
- main 抽样一致。

**目标状态：** 生产 CSP 非空；capability 按 window / plugin / command 最小授权；renderer compromise 后 blast radius 受限。

**实施切片：**
1. 做 capability inventory：扫描所有 `invoke(...)` 与 Tauri plugin 使用点，生成 `docs/reference/ipc-capability-inventory.md`。
2. 先设置 CSP 基线：
   - `default-src 'self'`
   - `script-src 'self'`
   - `style-src 'self' 'unsafe-inline'`（Tailwind/运行时样式需要先验证）
   - `img-src 'self' asset: https: data:`
   - `connect-src 'self' https://api.github.com https://raw.githubusercontent.com ...`，AI provider endpoints 需按 provider 配置动态评估。
3. 分拆 capability：
   - 移除 `fs:default`、`shell:default`、`sql:default` 的默认授权。
   - 只保留 app 确实需要的 dialog/updater/process/core 权限。
   - 文件/SQL/外部打开通过后端 command 实现，而不是 renderer plugin default。
4. 加 smoke tests / manual checklist：本地 dev、Windows build、GitHub import、Marketplace、Updater check、AI stream。

**验收：**
- `pnpm tauri build --target x86_64-pc-windows-msvc --bundles nsis --ci` 不因 CSP/capability 失败。
- 打包后主要页面无 CSP violation；恶意 inline script 测试不能执行。
- `src-tauri/capabilities/default.json` 不再含 `fs:default` / `shell:default` / `sql:default`。

**风险/回滚：**
- 风险：Markdown 图片、AI stream、GitHub raw、icon/data URI 被 CSP 拦截。
- 回滚：先以最小 CSP + telemetry checklist 分阶段收紧，不一次性限制所有 provider endpoint。

---

### D-01 — GitHub import overwrite 非原子

**结论：成立，P1。**

证据：
- `src-tauri/src/services/github_import/import.rs:85-116` 全量 import overwrite 时先 `remove_dir_all(target_dir)`，写失败无法恢复旧 skill。
- `import.rs:474-572` 的 partial/staged path 已有 staging、backup、rename、restore，可复用。
- DB 写入 `upsert_skill` 与 `assign_github_repository_to_skill` 仍是分步执行，缺少同一 transaction。
- main 抽样一致。

**目标状态：** 所有 GitHub import 都走 staging + backup + atomic rename + DB transaction；失败时旧目录和 DB 状态恢复。

**实施切片：**
1. 将全量 `import_github_repo_skills_with_auth` 的逐 skill 写入替换为 `import_single_staged_skill` 或共同 helper。
2. 把 `db::upsert_skill` + `assign_github_repository_to_skill` 封装到 transaction helper。
3. 将 `restore_or_cleanup_target_dir` 的失败也记录 operation log，避免静默恢复失败。
4. 补 failure injection tests：
   - 写入中途失败：旧目录仍存在。
   - rename 失败：backup restored。
   - DB 第二步失败：FS rollback，DB 不残留半条 repository assignment。
   - Windows 文件锁/跨卷 rename：明确错误与恢复行为。

**验收：**
- `cd src-tauri; cargo test github_import`。
- 手动导入同名 skill overwrite，强制失败后旧 skill 内容与 DB assignment 不变。

**风险/回滚：**
- 风险：Windows rename 在目标存在、杀软扫描、文件锁下更容易失败。
- 回滚：保留旧全量路径 behind dev-only feature flag 一版，但生产默认 staged path。

---

### E-01 — CI 不保护 PR / push

**结论：成立，P1。**

证据：`.github/workflows/ci.yml:1-4` 只在 `release.published` 触发；main 抽样一致。

**目标状态：** PR/push 必跑仓库质量门禁；release workflow 只负责产物发布或依赖已验证 commit。

**实施切片：**
1. 修改 `.github/workflows/ci.yml`：
   - `pull_request`：针对 `main`, `dev`。
   - `push`：针对 `main`, `dev`，可加 paths-ignore 避免 docs-only 成本过高。
   - 保留 concurrency cancel。
2. Job 分层：
   - `just-ci`：PR/push 必跑，Windows 2022。
   - package smoke：可放 `workflow_dispatch` / scheduled / release，或 PR 仅在 packaging files 变化时跑 Windows smoke。
3. 对齐 `just ci` 与 GitHub Actions：使用 `node scripts/run-ci.mjs` 作为同一入口。

**验收：**
- 新建测试 PR 会运行 typecheck/lint/Rust clippy gates。
- release workflow 不再是唯一 CI 入口。
- 本地 `just ci` 与 workflow step 一致。

**风险/回滚：** CI 成本上升；用 paths filter、matrix 分层、concurrency 降成本。

---

### P-01 — Tarball / tree / copy 无统一资源上限与取消

**结论：成立，P2。**

证据：
- `archive.rs:52-80` 一次性读全 archive 与每个 entry。
- `central_skills/files.rs:71-105` 本地目录树递归无 cap。
- `installation/fs_util.rs:203-224` 递归 copy 无 cap。

**目标状态：** 所有 archive/file/tree/copy 操作有统一资源预算、进度、取消；默认值可配置但安全。

**实施切片：**
1. 定义 `ResourceBudget`：
   - archive bytes、file count、per-file bytes、tree depth、tree entries、copy bytes、copy entries。
   - 默认值放 settings 或 constants；UI 可展示因 cap 被截断。
2. GitHub archive：
   - 先检查 `Content-Length`；超限直接失败。
   - streaming 解压，不再把完整 archive + snapshot content 全驻内存。
   - 仅 materialize candidate/path 需要的文件，避免全仓 HashMap。
3. Directory tree：
   - 返回 `truncated: true` / `reason`。
   - 本地与远端使用一致 cap。
4. Copy/import/scan：
   - 接入 cancellation token；UI cancel 通过已有进度事件模型显示。

**验收：**
- 构造超大 tarball、单文件超限、10 万文件目录、深层目录，进程不 OOM，错误信息明确。
- UI 能显示 truncated/cancelled 状态。
- `cargo test github_import resource_budget central_skills directory_tree installation copy`。

**风险/回滚：** 真实大仓库可能被默认 cap 拦截；提供设置项和错误提示，而不是静默失败。

---

### A-01 — IPC handler / DB re-export 架构边界

**结论：部分成立，P2/P3。**

证据：
- `src-tauri/src/lib.rs:300-476` 单个 handler list 约 168 个 command。
- `src-tauri/src/db/mod.rs:9-17,33-51` 顶层 re-export 面大，注释仍保留 legacy split 语义。
- 但当前已经有 commands/services/db/repos/schema 分层，不是“无架构”。

**目标状态：** command registry 可生成/校验，DB repos 是主要边界，顶层 re-export 逐步收窄。

**实施切片：**
1. 先做无行为变更：更新 stale db/module 注释，明确 repos/schema/seed 的真实边界。
2. 引入 command metadata registry：command name、domain、risk level、capability、frontend usage、tests。
3. 由 registry 生成：
   - TypeScript typed IPC client。
   - capability inventory。
   - command test matrix。
4. 分阶段收窄 `db/mod.rs` 顶层 re-export；每个服务改为依赖具体 repo helper。

**验收：**
- `pnpm typecheck` 能发现错误 command 名或参数形状。
- command registry 与 `generate_handler!` 列表一致；CI 可检查 drift。

**风险/回滚：** 迁移面大；必须先生成/检查，再逐域替换，避免大爆炸重构。

---

### T-01 — String 状态与 DB CHECK constraint

**结论：成立，P2。**

证据：
- `db/types.rs` 中 `link_type`, `source_kind`, `status`, `target_kind` 等仍为 String。
- `db/schema/core.rs`, `projects.rs`, `metadata.rs`, `settings.rs` 相关列多为 `TEXT NOT NULL`，未见 CHECK。

**目标状态：** 关键状态字段由 Rust enum + serde/sqlx boundary + SQLite CHECK 约束保护。

**实施切片：**
1. 从低风险字段开始：
   - `LinkType`: `symlink | copy | unknown/legacy`。
   - `SourceKind`: `global | project | plugin | marketplace | ...`（需先从 scanner 现有写入枚举采样）。
   - `SkillUpdateStatus`: `update_available | up_to_date | remote_missing | error | ...`。
2. Rust enum：
   - `TryFrom<String>` / `Display` / serde。
   - 历史未知值进入 `Unknown(String)`，日志提示但不崩溃。
3. DB migration：
   - 先审计历史 DB 值。
   - 清理或映射非法值。
   - 再加 CHECK constraint；SQLite 可能需要重建表，必须有 migration tests。
4. 前端 TS union 类型同步。

**验收：**
- 非法 `link_type` 插入失败或被 repo 层拒绝。
- 旧 DB 可正常升级；未知历史值不会造成启动崩溃。
- `cargo test db enum migration` + `pnpm typecheck`。

**风险/回滚：** SQLite 加 CHECK 可能需要表重建；要先做 read-only audit script，再迁移。

---

### R-01 — Release updater config 与 preflight

**结论：部分成立，重述为 P2/P3 回归保护。**

证据：
- 基础 `tauri.conf.json` 确实有 placeholder pubkey 与 `createUpdaterArtifacts=false`。
- 但 `.github/workflows/release-desktop.yml` 已校验 updater secrets，生成 release-only config 注入真实 pubkey 与 `createUpdaterArtifacts=true`，检查 `.sig`，生成 `latest.json`。
- `docs/reference/release-process.md` 已说明基础 config placeholder 是 intentional。

**目标状态：** release config 注入、签名、`latest.json` 完整性成为可本地/CI 复用的 preflight，而不是只靠 workflow 脚本散落逻辑。

**实施切片：**
1. 抽出 `scripts/release-preflight.mjs`：
   - 检查 release config pubkey 非 placeholder。
   - 检查 NSIS artifact `.sig` 存在。
   - 检查 `latest.json` version/url/signature。
2. `release-desktop.yml` 调用该脚本。
3. `docs/reference/release-process.md` 与 README release 段同步 preflight 命令。

**验收：**
- 缺 secret、placeholder pubkey、缺 `.sig`、缺 `latest.json` 都会 fail fast。
- 不影响普通本地 `pnpm tauri build`。

---

### DOC-01 — README 安全文档过期

**结论：成立，Quick win / P2。**

证据：
- `README.md:135-141` / `README_CN.md:134-141` 仍写 PAT/API key 存 SQLite 且不加密。
- `src-tauri/src/secrets/mod.rs` / `system.rs` 已实现 `SecretStore`，默认 system keyring，Windows DPAPI protected fallback，最终 session fallback。

**目标状态：** 英文/中文 README 与实现一致，用户能理解：系统凭据库优先、Windows DPAPI app-local fallback、session-only fallback、legacy SQLite settings migration。

**实施切片：**
1. 更新 `README.md` 与 `README_CN.md` 隐私安全段。
2. 同步 `docs/reference/release-process.md` 或新增 `docs/reference/security-and-storage.md`，避免 README 过长。
3. 文案明确：
   - 私钥不存储。
   - SSH password / GitHub PAT / AI API keys 的存储边界。
   - fallback 的安全含义与不可持久化时的行为。

**验收：**
- README 中不再出现“PAT/API key 存 SQLite settings 表且不加密”的过期陈述。
- 文档与 `SecretStorageState::{Stored, Protected, Session}` 对齐。

---

### W-01 — Windows home path 优先级

**结论：成立但 P3。**

证据：
- `paths.rs:27-34` 当前 `HOME` 优先，`USERPROFILE` 仅 fallback。
- `paths.rs:282-304` 测试也固定了 HOME 优先。
- main 抽样一致。

**目标状态：** Windows 下默认路径落在 Windows 用户 profile，而不是 Git Bash/MSYS HOME；同时避免已有用户目录突然“搬家”。

**实施切片：**
1. 先增加可观测性：Settings/About 显示 resolved home、app data、Central store、Universal Agents path。
2. Windows 条件化解析：
   - `cfg(windows)`: `USERPROFILE` > `HOMEDRIVE+HOMEPATH` > `HOME` > temp。
   - non-Windows: 保持 `HOME` 优先。
3. 若发现旧 DB 已在 Git Bash home 下，走已有 Central store location preview/apply migration，而不是静默改目录。
4. 更新 tests：Windows/non-Windows 分支分别覆盖。

**验收：**
- Windows 单元测试证明 USERPROFILE 优先。
- 旧路径用户不会在启动时丢失 Central skills；迁移需显式 preview。

---

## 2. 推荐实施顺序

### Phase 0 — Baseline 与保护现状（0.5 天）

1. 记录当前 `git status`，保护 unrelated dirty 文件。
2. 本地跑一次基础 gate：`pnpm typecheck && pnpm lint`，`cd src-tauri; cargo clippy -- -D warnings`。
3. 新增/确认安全回归测试夹具目录，避免后续 P1 只靠手测。

**产出：** baseline gate 结果、失败项列表、后续每个 commit 的测试入口。

### Phase 1 — P1 安全与质量门禁（3–6 天）

推荐拆 4 个提交，彼此可 review：

1. **CI trigger 修正（E-01）**
   - 改 `.github/workflows/ci.yml`。
   - 最小验证：workflow syntax + 本地 `just ci`。

2. **PathPolicy guard（S-01）**
   - 新增 policy 与 Rust tests。
   - 前端先保持旧 command 名，降低 UI 改动面。

3. **GitHub import 原子化（D-01）**
   - 全量 import 复用 staged helper。
   - DB transaction helper + failure-injection tests。

4. **CSP + capability 最小化第一轮（S-02）**
   - 先 capability inventory，再收紧默认权限。
   - Windows bundle smoke 必跑。

**Phase 1 完成标准：** P1 项有测试覆盖；`just ci` 通过；Windows Tauri bundle 可生成；合法 SkillDetail / GitHub import / Marketplace 主流程可 smoke。

### Phase 2 — P2 资源/数据/文档硬化（1–3 周）

1. **README/README_CN 安全文档（DOC-01）**：可最先做，低风险。
2. **ResourceBudget + caps（P-01）**：先 cap，再 streaming/cancel。
3. **Typed domain model（T-01）**：先 Rust enum/TS union，再 DB CHECK migration。
4. **Release preflight（R-01）**：抽脚本，workflow 调用，文档同步。

**Phase 2 完成标准：** 大输入不会 OOM；非法状态无法无声进入 DB；release updater 元数据可脚本化验证；文档与 secret 实现一致。

### Phase 3 — 架构收敛与 Windows path 体验（1–2 个版本）

1. **Command registry / typed IPC client（A-01）**。
2. **DB re-export 收窄（A-01）**。
3. **Windows home path 解析与迁移提示（W-01）**。
4. **长期安全基线**：CodeQL/cargo-audit/pnpm audit、敏感日志脱敏测试、threat model 文档。

**Phase 3 完成标准：** command/capability/test 矩阵可自动检查 drift；Windows 用户能明确看到并安全迁移实际数据路径。

## 3. 测试矩阵

| 改动类型 | 必跑 | 补充 |
|---|---|---|
| 前端 IPC/UI | `pnpm typecheck && pnpm lint` | 相关 Vitest：SkillDetail / Marketplace / Central updates |
| Rust security/path/import | `cd src-tauri; cargo test <module>` + `cargo clippy -- -D warnings` | traversal/symlink/Windows path/failure injection |
| CI/release | workflow syntax + `just ci` | Windows `pnpm tauri build --target x86_64-pc-windows-msvc --bundles nsis --ci` |
| Docs | README/README_CN 双语同步 | docs build / docs-check 若修改 VitePress docs |
| Packaging | `pnpm tauri build` | 确认 NSIS/MSI/signature/latest.json 具体产物 |

## 4. 明确不建议的做法

- 不建议先做 command registry 大重构再修 P1；安全缺陷应先被 guard 和测试锁住。
- 不建议删除旧 path IPC 后立即大改前端所有调用；先兼容 guard，再迁移 typed IPC。
- 不建议为资源预算引入新依赖；优先用现有 Rust/reqwest/tar 能力实现 caps 与 streaming。
- 不建议自动改 Windows home 路径并静默迁移数据；必须 preview，让用户确认。
- 不建议把 release base config 的 placeholder 直接改成生产 key；release-only config 注入是更安全边界。

## 5. 下一步可直接执行的最小计划

若后续进入实施，建议从以下顺序开始：

1. `E-01`：CI trigger 修正，建立后续保护网。
2. `DOC-01`：README 安全文档修正，低风险且立即减少误导。
3. `S-01`：PathPolicy guard + tests。
4. `D-01`：GitHub import full path 复用 staged/backup path。
5. `S-02`：CSP/capability 第一轮收紧 + Windows bundle smoke。

这 5 步完成后，再进入资源预算、typed enums、release preflight 和架构收敛。
