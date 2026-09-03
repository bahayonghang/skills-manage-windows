# 风险导向测试覆盖补强（第二轮）

## Goal

在第一轮已关闭的安装补偿、目标删除回滚、仓库同步事务、便携导入终态和 AI 凭据部分失败之外，继续按业务风险补齐 SkillPort 剩余跨边界变更的回归测试，使 Central 迁根、adopt-into-Central、SSH/WSL 创建更新、分类/安装刷新和凭据保存在失败后仍可恢复、状态一致，且明文 secret 不进入 renderer。

## Background

- 本任务是归档任务 `08-30-risk-driven-test-coverage` 的后续，不重复第一轮已落地的测试。风险证据见 `research/backend-risk-coverage-r2.md` 和 `research/frontend-risk-coverage-r2.md`。
- 仓库没有可复用的覆盖率采集命令、阈值或 CI 门禁。本任务不新增覆盖率依赖，不编造覆盖率百分比。
- 应用没有用户/角色 RBAC。权限相关风险由 Tauri capability 静态契约、目标作用域、路径边界和凭据存储边界承担。
- 第一轮已建立前端 “mutation 已提交 / reload required” 模式（`targetStore.requiresTargetReload`）。本轮同类刷新失败沿用该契约，而不是把后续 refresh 失败写成“变更从未发生”。

## Requirements

### R1. Central 迁根的文件系统与数据库一致性

- 在 `UPDATE agents SET global_skills_dir` 被 trigger 拒绝后，调用失败，源技能字节不变，目标侧仅有的技能字节不变，agents / scan_directories / skills / skill_installations 的路径列与调用前快照一致。
- 允许 agents 更新、拒绝后续 `skills` 路径 `REPLACE` 时，四张表必须一起回滚，不得出现 “agent 已迁、skills 仍指向旧根”。
- SQL 字符串 `REPLACE` 不得改写仅共享旧根前缀的兄弟路径（例如旧根 `…/store` 不得改写 `…/store-extra/...`）。
- 去掉 trigger 后同一请求可重试成功：新根生效、目标侧独有技能保留、源内容仍在。
- 迁根过程中新建的目标技能目录在数据库失败后必须补偿删除；本轮不要求恢复“同名覆盖前”的目标内容（需独立 backup/journal 任务）。

### R2. `ensure_centralized` 的 copy/upsert/retry

- 对非 Central 技能注入 `skills` upsert 失败：调用报错，源目录字节不变，`is_central` 仍为 false，本次复制出的 canonical 目录被补偿删除。
- 若生产代码曾把 copy 留在 Central，去掉 trigger 后的重试必须完成 upsert（`is_central` true 且 `canonical_path` 已写），不得因 `SKILL.md` 已存在而短路跳过数据库修复。
- 同一失败通过 Local `prepare_target_local` / `install_skill` 公共入口时，不得报告成功却留下非 Central 行。

### R3. SSH/WSL 创建与更新的凭据、settings 与 cache 一致性

- 不连接真实 SSH/WSL 主机。在 probe 成功之后的 persist/credential/cache 边界上注入失败。
- 创建：persist `ssh_targets_v1` 失败时，SSH 列表为空、无凭据键、无 remote pool；去掉 trigger 后重试成功且密码只存一次。
- 创建：persist 成功而 `remote_db` 失败时，列表、凭据和 pool 一并回滚（创建视为一次 mutation）。
- 更新：改密码后 persist 失败时，旧密码仍是唯一存储 secret，host/user/id JSON 不变。
- 更新：password→key 且 persist 成功后凭据删除失败时，settings 与凭据收敛为同一种认证状态，不得出现 JSON 已是 key-auth 却残留密码。
- WSL 创建/更新在 persist 成功、`remote_db` 失败时采用与 SSH 相同的回滚契约。

### R4. Central 元数据与 AI review 的空输入、首命令失败与刷新失败

- `createRepository` / `createTag` / `acceptAiTagReview` / `skipAiTagReview` 覆盖成功与首命令拒绝：loading 清除、error 写入、rethrow、刷新命令集合正确。
- `unassignSkillTags(skillId, [])` 零 IPC、零状态变化；非空成功与失败各一条。
- `bulkSuggestSkillTags([])` 零 IPC、零 job/loading 变化。
- accept/skip（以及 create repository/tag）在 mutation 成功、refresh 失败时进入明确的 reload-required/error，不得表现为“分类从未改变”。
- `loadAiTagReviews` 拒绝时写入确定性 error 并结束 loading，不得留下未处理 rejection。
- 本轮不为这些短 mutation 新增 generation-gating；不写 target-switch stale-write 测试。

### R5. GitHub PAT 保存/清除/测试的失败与明文隔离

- `set_github_pat` / `clear_github_pat` / `test_github_pat` 拒绝时清除对应 loading、写入 error、向调用者 rethrow。
- 成功与失败路径均递归证明 Zustand state、error 字符串和普通诊断不含 PAT sentinel；`githubPatState` 不含已键入明文。

### R6. Central 安装/删除/切换的提交后刷新失败

- 对 `installSkill`、`batchInstallSkills`、`deleteCentralSkill`、`togglePlatformLink`：mutation 成功、`get_central_skills` 拒绝时，loading 清除、error 保存、rethrow，并进入明确 “mutation committed / reload required”，不得把列表不变解释成安装/删除未发生。

### R7. 持久化 target ID 与 cache-path 校验对齐

- 对 SSH/WSL ID 表测 `../escape`、`a/b`、`a\\b`、空白/控制字符和合法 `ssh-demo_1`。
- 非法 ID 在 `remote_cache_db_path` / 建目录之前被隔离或拒绝，且不在 target-cache 根之外创建目录。
- `validate_target_ids` 与 `sanitize_target_id` 对同一矩阵接受/拒绝结果一致。

### R8. 发布元数据生产者在生成时 fail-closed

- 用自有临时目录覆盖 `generate-latest-json.mjs` 的参数解析、`v` 前缀剥离、双 Windows updater key、URL/tag/repo 规范化与 `latest.json` 写入位置。
- 缺失 `.sig`、空 `.sig`、零个或多个 NSIS 候选在生成时失败，不写出错误的 `latest.json`。重复候选不得再按字典序取最后一个。
- `prepare-release-body.mjs` 覆盖精确版本 notes、系列 `major.minor.md` 与 fallback，以及 `--output` 路径。

### R9. Update Center apply 后 inventory 刷新失败

- `apply_skill_update_decisions` 成功而 `get_skill_update_inventory` 拒绝时，`isApplying` 清除、error 保存、rethrow，并进入明确 “decisions already applied / reload required”。

### R10. 缺陷处理与验证顺序

- 测试先行。若回归证明当前行为违反事务、补偿、reload-required 或凭据隔离契约，只允许在同一模块内做使该不变量成立的最小生产修复。
- 不为测试方便扩大全局可见性、不新增依赖、不做无关重构、不测试纯样板或第三方库行为。
- 每完成一个模块立即运行其聚焦测试，并确认过滤结果非零。
- 后端模块完成后运行完整 locked Rust tests；前端模块完成后运行完整 Vitest。
- 最后运行 `just ci`，分别报告通过、失败、跳过和缺失证据。

## Acceptance Criteria

- [x] AC1: R1 的迁根 trigger/前缀/重试测试通过；失败后路径表与源/target-only 字节收敛。
- [x] AC2: R2 的 upsert 失败补偿与短路修复测试通过；retry 能完成 Central 行更新。
- [x] AC3: R3 的 SSH/WSL 创建更新回滚测试通过；无真实主机，明文密码不进入 settings JSON。
- [x] AC4: R4 的元数据/review 空输入、首命令失败和 refresh 失败测试通过。
- [x] AC5: R5 的 PAT 失败与 sentinel 缺席测试通过。
- [x] AC6: R6 的安装/删除/切换 refresh 失败测试通过，并带 reload-required。
- [x] AC7: R7 的 target ID 矩阵与 cache-path 对齐测试通过。
- [x] AC8: R8 的 `latest.json` / release body 生成测试通过；空签名与重复 NSIS 在生成时失败。
- [x] AC9: R9 的 apply 后 inventory 失败测试通过。
- [x] AC10: 每个模块都有非零聚焦测试证据；没有为覆盖率数字添加低价值测试。
- [x] AC11: `cargo test --manifest-path src-tauri/Cargo.toml --locked`、`pnpm test` 和最终 `just ci` 通过；任何跳过或环境限制均单独列明。

## Out of Scope

- 引入 `cargo-llvm-cov`、Vitest coverage provider、覆盖率阈值或 CI coverage 门禁。
- 为不存在的用户/角色 RBAC 模型编造权限测试。
- 重复第一轮已覆盖路径：项目安装卸载补偿、目标删除/Local/空/未知 ID、仓库同步预校验、便携导入终态、AI key 部分失败、target store 首命令/list_targets/密码 sentinel。
- 重复已有 GitHub import、IPC/runtime redaction、target quarantine 隔离、Marketplace snapshot、local archive zip-slip、startup recovery 的 dense 测试。
- SSH/WSL connection-test 以及 local-remote sync `Ok` 载荷中的 `error.to_string()` 脱敏：仍需独立 redaction spec，本轮不固化泄漏、不改公开成功载荷。
- AI settings 并发 flush 的 latest-edit-wins：仍需产品语义决策。
- 短 mutation 的 generation-gating / target-switch stale-write。
- 迁根覆盖前目标内容的 backup/journal 恢复。
- Collection JSON 导入的全有或全无事务、local-remote sync FakeRunner apply。
- 远端 SSH 主机、真实凭据提供方、Tauri 原生 GUI 和发布环境验证。

## Technical Notes

- 生产修复必须由新增失败回归驱动，并保持在对应模块的最小范围内。
- 测试优先复用 `fresh_db` / `mem_pool`、SQLite trigger、`MemoryCredentialBackend`、owned temp directory、现有 IPC mock。
- SSH/WSL 创建更新通过提取 probe 之后的 persist helper 测试，避免 live transport。
- `src-tauri/src/targets/commands.rs` 当前约 709 行；若回滚逻辑会超过 800 行生产源码预算，将 persist/compensation 抽到 sibling module。
- Windows 链接权限、真实 SSH/provider 行为和发布环境结论保持 `UNVERIFIED`。
