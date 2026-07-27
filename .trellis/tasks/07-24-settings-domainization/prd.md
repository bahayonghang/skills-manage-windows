# Settings API 域化：allowlist 阻断 target 配置绕写

## Goal

关闭 generic `set_setting` / `set_settings` 跨越配置域的写入旁路，使 target、迁移标记、secret 和其他内部状态只能由专用命令维护；同时在既有 target 配置损坏时安全退回 Local，并向用户持续呈现不含敏感内容的隔离证据。对应审计 P1-09（M-07）。

## User Value

- renderer 或误用调用不能绕过 target 存在性、ID immutable、probe 和凭据边界。
- 一份损坏的 SSH 或 WSL 配置不会让应用启动后悄然失去 target 功能，也不会把另一域的健康配置一并清除。
- 用户能在 Settings 的连接页看到哪个配置域被隔离、何时发生以及可核验摘要，而不会看到或泄漏原始凭据。

## Confirmed Evidence

- `src-tauri/src/commands/settings.rs:17-27,117-142`：generic setter 目前只拒绝 GitHub PAT、AI API key 及 provider-scoped AI secret key，其他任意 key 均可写。
- `src-tauri/src/targets/model.rs:2-4`：`ssh_targets_v1`、`wsl_targets_v1`、`active_target_id_v1` 与普通设置共用 `settings` 表。
- `src-tauri/src/targets/commands.rs:371-395`：专用 active-target 命令会验证 target 存在性；直接写 `active_target_id_v1` 可绕过该校验。
- `src-tauri/src/targets/commands.rs:418-453`：SSH / WSL 配置当前直接反序列化；任一 JSON 解析错误向上传播为 `ParseRemoteTargets` / `ParseWslTargets`。
- `src-tauri/src/targets/registry.rs:332-364`：target 列表先读取 active id，再顺序加载 SSH 与 WSL；任一域解析失败会使整个列表失败。
- `src/components/layout/AppShell.tsx:72-81` 与 `src/stores/targetStore.ts:90-102`：store 记录并重新抛出加载错误，但启动层吞掉 rejection，当前没有持久告警。
- `src-tauri/src/db/repos/settings_repo.rs:81-96`：repository 的 batch setter 已在一个 SQLite transaction 内写入，可作为“先验证全部、再一次提交”的原子边界。
- 当前合法 generic IPC 写入为平台分类可见性、Central 更新检查模式、字体偏好以及非 secret AI 偏好；scan directory 与 target CRUD 已有专用命令。
- `RemoteTargetConfig.password` 使用 `serde(skip)`，但持久结构仍兼容 `protectedPassword`；损坏或被任意写入的原始 JSON 可能包含敏感字段，不能复制到普通 settings、日志、IPC 或状态导出。
- `src/lib/backendError.ts:3-31` 只识别小写 coded-error 前缀；本任务使用 `setting_key_forbidden` 等稳定小写 code。

## Requirements

### R1. Generic write allowlist

1. generic 单写与批写采用显式 allowlist，只允许当前 renderer 实际使用的 UI preference key 族。
2. target keys、secret keys、migration markers、feature gates、quarantine metadata 和未知 key 一律拒绝，返回 `setting_key_forbidden: ...`。
3. 拒绝错误与 operation log 不回显调用方提供的 value；未知 key 也不作为自由文本写入日志。
4. allowlist 只位于 IPC generic setter 边界，不限制 target、secret、migration 等域内部直接调用 repository helper。

### R2. Typed value validation and atomic batches

1. 每个允许的 key 族在写 DB 前执行对应的值校验；非法值返回稳定 coded error。
2. batch 必须先完成全部 key/value 校验，再调用现有 transaction repository；任一项非法时零项落库。
3. generic getter 保持现有只读兼容性，secret getter 的保护不放宽。

### R3. Per-domain target quarantine

1. 启动时验证 SSH 与 WSL 的持久配置；运行期通过统一 loader 再次读取时沿用同一验证与恢复路径。
2. SSH 与 WSL 独立处理：某一域损坏时，该域整体隔离并退回空列表，健康域保持不变；不逐条挽救。
3. active target 不是 Local 且不在剩余健康 target 中时，原子退回 Local。
4. 隔离写入必须与受影响域清空、active-target 修正处于同一 SQLite transaction，避免半恢复状态。
5. 持久隔离证据只包含域、UTC 时间、原始字节数、SHA-256 和稳定 reason code；不得保存或返回原始 JSON、字段值、主机名、用户名、路径、密码或 `protectedPassword`。
6. 隔离流程不得 panic；恢复后 `list_targets` 至少返回 Local。

### R4. Persistent Settings warning

1. backend 提供 typed quarantine-status 读取命令；状态不随单次 toast、页面切换或应用重启消失。
2. target store 在初始加载时同时获取列表和隔离状态。
3. Settings 的“连接与同步”页显示 i18n 告警，明确受影响域、隔离时间、摘要和 Local fallback；不得展示原始内容。
4. 本任务不提供原始配置查看、导出、逐条恢复或自动重建。

### R5. Audit metadata

1. generic setting operation log 只记录稳定 category 集合、key 数量和成功/失败状态，不记录 value 或任意 key 文本。
2. target 隔离日志只记录域、reason code、字节数与摘要，不记录原始配置或解析器自由文本。

## Acceptance Criteria

- [ ] generic 单写和批写 target key、migration marker、secret key、feature gate 或未知 key 均返回 `setting_key_forbidden`，且数据库不变。
- [ ] 当前 renderer 的平台可见性、更新模式、字体和非 secret AI preference 写入全部通过 typed validation；不在清单内的值失败且 batch 原子回滚。
- [ ] 合法 target CRUD / active-target / secret migration 的内部 repository 写入不受 generic IPC allowlist 影响。
- [ ] SSH JSON 损坏只清空 SSH 域，保留健康 WSL；WSL JSON 损坏的对称场景同样成立，不逐条 salvage。
- [ ] 损坏域清空、隔离摘要持久化及无效 active target 回落 Local 在一个 transaction 内完成；应用不 panic，`list_targets` 返回 Local 与健康域。
- [ ] 原始损坏 JSON 及其中潜在 `password` / `protectedPassword` / key path 不进入 quarantine metadata、operation log、runtime log、IPC payload 或前端文案。
- [ ] 重启语义测试证明隔离状态可再次读取，Settings 连接页持续显示中英文告警。
- [ ] operation log 对单写/批写只含 category/count/status，不含 value 或任意调用方 key。
- [ ] `cargo test settings`、target 隔离定向测试、相关 Vitest、`pnpm typecheck`、`pnpm lint` 和 `just ci` 通过。

## Out of Scope

- 完整拆分 `UserPreferencesService` / `TargetConfigService` / `SecurityConfigService`。
- 取消 generic getter 或一次性迁移所有 getter 调用。
- typed IPC 全量迁移；该工作仍由依赖本任务的 `07-24-typed-ipc-migration` 负责。
- 对损坏数组逐条 salvage、原始配置查看/导出/恢复、凭据恢复或 SSH/WSL 自动重建。
- 新数据库表、通用 quarantine framework、跨域配置版本系统或无关 settings UI 重构。

## Resolved Decisions

- 采用用户批准的方案 1：SSH 与 WSL 按域 all-or-nothing 隔离，彼此独立，不逐条 salvage。
- 受影响域回退空列表；无效或缺失 active target 回退 Local。
- 隔离证据持久化并在 Settings 显示持续告警，但基于凭据边界只保留不可逆摘要和结构化元数据，不复制原始 blob。
- M-07 审计报告中的“typed IPC 前置”按渐进落地处理：本任务先用现有 typed command map 与稳定 coded error 关闭写旁路，不等待全量 typed IPC 子任务。

## Blocking Questions

无。
