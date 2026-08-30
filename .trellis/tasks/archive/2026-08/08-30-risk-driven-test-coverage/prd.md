# 风险导向测试覆盖补强

## Goal

以业务风险而非行覆盖率数字为导向，补齐 SkillPort 核心跨边界变更在异常、回滚、并发、参数校验和凭据保护方面的回归测试，使失败结果可恢复、状态一致且不会泄露敏感信息。

## Background

- 当前 Vitest 能发现 177 个前端测试文件；Rust 源码和集成测试中已有 1,516 个 `#[test]` / `#[tokio::test]` 属性，说明测试面较广，但不能代表分支覆盖率或已执行证据。
- 仓库没有配置可直接复用的前端或 Rust 覆盖率采集命令、阈值或 CI 门禁，本机也没有 `cargo-llvm-cov`。本任务不新增覆盖率依赖，不编造覆盖率百分比。
- 现有 GitHub 导入、IPC 错误规范化/脱敏、runtime logger、target config quarantine 和 Tauri capability drift 已有较强测试；不为提高数字重复 happy path。
- 应用没有用户/角色 RBAC 业务模型。权限相关风险主要由 Tauri capability 静态契约、目标作用域、路径边界和凭据存储边界承担。
- 风险证据见 `research/backend-risk-coverage.md` 和 `research/frontend-risk-coverage.md`。

## Requirements

### R1. 项目安装/卸载的文件系统与数据库一致性

- 对项目技能 copy 安装注入 `project_skill_installations` 写失败，证明失败后没有目标残留、没有安装行，Central 源内容不变。
- 对已安装技能的卸载注入安装行删除失败，证明不会留下“数据库仍声明安装但文件已消失”的分裂状态。
- 使用现有临时目录和 SQLite trigger seam；只有在当前环境能可靠创建链接时才补链接变体，并明确报告跳过原因。

### R2. Central 仓库同步的校验前置与批次回滚

- 混合有效与非法相对路径（如 traversal）时，在任何 authoritative write 前拒绝整个输入。
- 第二条 skip/unskip 写入失败时，第一条不得残留；移除故障注入后同一请求可重试成功。
- 覆盖 typed error、成员关系、update state 和 skip rows，而不是只断言返回字符串。

### R3. 目标变更与凭据状态边界

- 覆盖 Local/空/未知 target ID 的零写入或 fail-closed 边界。
- 对目标删除或活动目标更新的中途持久化失败，证明 target lists、active target 和 credential 不会部分丢失。
- 前端 target mutation 首命令失败时必须清理 loading、保留既有列表/active target、保存错误并继续向调用者抛出。
- 后端 mutation 成功而 `list_targets` 刷新失败时，前端必须进入明确 reload-required/error 状态，避免把已完成变更误报为“从未发生”并诱发重复 mutation。
- 密码型 create/update/test 的成功与失败路径均递归证明明文 secret 不进入 Zustand state、普通诊断或错误。

### R4. 便携状态导入的终态与过期写隔离

- backend import 已成功但任一 post-import refresh 失败时，作业必须进入明确 terminal failure，清除 running/busy 状态、保留 job/correlation ID、保存已脱敏错误并向调用者抛出。
- target generation/job 在 await 期间变化时，旧 refresh 的完成或失败不得覆盖新 target 状态。

### R5. AI 凭据设置的部分失败

- `set_ai_api_key` 成功而 `set_settings` 失败时，store 呈现明确失败/重试语义，普通设置和诊断不得包含明文 secret。
- provider switch 前置 flush 失败时，不发起新 provider 的后续读写，不遗留 loading/save 状态。

### R6. 缺陷处理边界

- 测试先行。若新增回归证明当前行为违反已存在的 transaction、redaction、job correlation 或 renderer authority spec，只允许在同一模块内做使该安全不变量成立的最小生产修复。
- 不为测试方便扩大全局可见性、不新增依赖、不做无关重构、不测试纯样板或第三方库行为。

### R7. 验证顺序

- 每完成一个模块立即运行其聚焦测试，并确认过滤结果非零。
- 后端模块完成后运行完整 locked Rust tests；前端模块完成后运行完整 Vitest。
- 最后运行仓库完成门禁 `just ci`，分别报告通过、失败、跳过和缺失证据。

## Acceptance Criteria

- [x] AC1: R1 的 install/uninstall 故障注入测试通过，FS 与 DB 在错误后保持收敛。
- [x] AC2: R2 的非法路径预校验和第二写失败回滚测试通过，失败请求没有部分持久化。
- [x] AC3: R3 的 backend target CRUD 回滚、frontend mutation/refresh failure 和 secret-retention 测试通过；没有凭据进入 renderer state 或普通诊断。
- [x] AC4: R4 的 refresh failure 与 stale completion/error 测试通过，portable import 不遗留 running job。
- [x] AC5: R5 的 partial failure 与 pre-switch failure 测试通过，明文 API key 不进入普通设置、状态诊断或错误。
- [x] AC6: 每个模块都有非零聚焦测试证据；没有为覆盖率数字添加低价值测试。
- [x] AC7: `cargo test --manifest-path src-tauri/Cargo.toml --locked`、`pnpm test` 和最终 `just ci` 通过；任何跳过或环境限制均单独列明。

## Out of Scope

- 引入 `cargo-llvm-cov`、Vitest coverage provider、覆盖率阈值或 CI coverage 门禁。
- 为不存在的用户/角色 RBAC 模型编造权限测试。
- 重复已有 IPC/runtime/GitHub import happy path，或测试 getter、样板、第三方库内部实现。
- 本轮不处理较低风险的 Central metadata/review store 全面补测和 release metadata producer 重构；若高风险五类完成后仍有余量，仅记录为后续候选。
- SSH/WSL connection-test 的 `Ok(Result { ok: false })` 成功载荷脱敏语义尚未由现有 redaction spec 定义；本轮不擅自改变公开结果契约。
- AI settings 并发 flush 的 authoritative-backend latest-edit-wins 需要排队、互斥、版本检查或 coalescing 的产品语义决策；本轮不凭空选定机制。
- persisted target ID 与 remote cache path validator parity 属于中风险补强，低于本轮纳入的 frontend target mutation 高风险失败分支，延期处理。
- 远端 SSH 主机、真实凭据提供方、Tauri 原生 GUI 和发布环境验证。

## Technical Notes

- 生产修复必须由新增失败回归驱动，并保持在对应模块的最小范围内。
- 测试优先复用 `fresh_db`、SQLite trigger、`MemoryCredentialBackend`、owned temp directory、现有 IPC mock/deferred promise seam。
- Windows 链接权限、真实 SSH/provider 行为和发布环境结论保持 `UNVERIFIED`。
