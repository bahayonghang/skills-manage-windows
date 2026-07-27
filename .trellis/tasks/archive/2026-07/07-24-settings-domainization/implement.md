# Implementation Plan

## 1. Backend generic settings policy

- [x] 新增 settings policy 模块，定义允许 key、category、typed value validator 与稳定 coded errors。
- [x] 改造 `set_setting_impl` / `set_settings_impl`：全部验证先于写入，batch 只调用一次现有 transaction repository。
- [x] 将 generic settings operation log 改为 category/count/status，移除 raw key/value。
- [x] 补 Rust tests：当前合法 key 族、未知/target/secret/migration/feature key 拒绝、非法值、batch 零写入、错误与日志不泄漏输入。

## 2. Target configuration quarantine

- [x] 在 `targets` 建立 versioned snapshot / quarantine types 与统一 loader。
- [x] 实现 SSH、WSL 独立 schema validation；保留旧 `credentialKey` / `protectedPassword` 兼容。
- [x] 对损坏域生成仅含 stable reason、UTC、bytes、SHA-256 的 metadata；原始 blob 不持久化或记录。
- [x] 用一个 `db::set_settings` transaction 原子写入受影响域 `[]`、quarantine metadata 和必要的 Local fallback。
- [x] 将 startup 与 registry 读取路径接到统一 loader，保证失败不 panic 且 `list_targets` 至少返回 Local。
- [x] 补 Rust tests：SSH/WSL 对称隔离、健康域保留、active fallback、重复 digest、transaction failure、潜在 credential 字段不泄漏、重启后状态读取。

## 3. Read-only status IPC

- [x] 新增 `get_target_config_quarantine_status` command 并注册 handler。
- [x] 在 `commandMap.ts` 增加 typed request/result；同步 Rust/TS camelCase shape。
- [x] 补 handler/command coverage，确认 status 不包含 raw/error text。

## 4. Frontend persistent warning

- [x] 扩展 target store state，在 `loadTargets` 中同时读取 targets 与 quarantine status，并保持 Local fallback。
- [x] 经 `settingsViewBindings` / `settingsPageSections` 将 status 传入连接 section。
- [x] 在 `RemoteTargetsSettingsSection` 顶部增加符合现有 Settings 结构的 persistent warning，使用 lucide icon、可访问 status 语义及中英文 i18n。
- [x] 补 store、AppShell startup、SettingsView/section tests：重启式 reload 后仍显示、健康状态不显示、SSH/WSL 文案正确且无 raw 内容。

## 5. Verification

- [x] `cargo fmt --all -- --check`。
- [x] `cargo test settings --locked` 以及新增 target quarantine 定向测试。
- [x] 相关 Vitest：target store、AppShell、SettingsView/RemoteTargets section、IPC coverage。
- [x] `pnpm typecheck`。
- [x] `pnpm lint`。
- [x] `just ci`。
- [x] 检查最终 diff 只含本任务代码、spec 和 task artifacts；不包含 sibling task、Trellis runtime/config、`.gitattributes` 或审计报告。

## 6. Trellis finish gates

- [x] `trellis-check` 做全量 spec / PRD / design / implementation 复核并修正发现。
- [x] `trellis-update-spec` 判断并记录 settings domain / quarantine 契约；无新增知识也要留下结论。
- [ ] 以符合历史的中文 emoji message 仅提交本任务工作文件；不 push。
- [ ] archive `07-24-settings-domainization`，journal 只记录工作 commit SHA。

## Rollback Points

- policy 影响合法 UI 写入时，回到步骤 1 修正精确 allowlist/validator，不改为 denylist。
- quarantine 发现兼容性误判时，回到规划/设计校正 schema validation；不得逐条 salvage 或保存 raw blob 绕过凭据边界。
- status UI 需要改变用户行为或新增 restore/export 时停止并回到 Phase 1，不在本任务内扩大。
