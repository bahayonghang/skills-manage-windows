# Skills CLI 数据、Placement 与安全 IPC 契约 — 技术设计

共享产品契约见 `../08-26-skills-cli-redesign/research/design-contract.md`。本文定义后端唯一落点。

## 1. IPC 数据类型

`src-tauri/src/services/skills_cli/mod.rs` 增加由 Rust/Serde/Specta 同源生成的类型：

```rust
#[serde(rename_all = "snake_case")]
pub enum SkillsCliPlacementState {
    ManagedLink,
    DirectCopy,
    Missing,
    Conflict,
    Unavailable,
}

#[serde(rename_all = "snake_case")]
pub enum SkillsCliManagedLinkKind {
    WindowsJunction,
    Symlink,
}

#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacement {
    pub agent_id: String,
    pub display_name: String,
    pub target_path: String,
    pub state: SkillsCliPlacementState,
    pub managed_link_kind: Option<SkillsCliManagedLinkKind>,
    pub reason_code: Option<String>,
}
```

`SkillsCliGlobalSkill` 追加：

```rust
pub canonical_path: Option<String>,
pub folder_hash: Option<String>,
pub installed_at: Option<String>,
pub updated_at: Option<String>,
pub placements: Vec<SkillsCliPlacement>,
```

现有 `agents` 保留兼容，但只能由 placement state 为 `managed_link` 或 `direct_copy` 的 display name
派生。删除原规划的 `agentIds` / `linkTargets` 平行数组，避免索引漂移；新 UI 只消费 `placements`。

`CliLockEntry` 增加 nullable `skill_path`、`skill_folder_hash`、`installed_at`、`updated_at`，
`lock_entry_from_value` 同时接受 lock v3 的 camelCase 与 legacy snake_case，空串归一为 `None`。

## 2. Placement classifier

新增纯分类模块 `services/skills_cli/placement.rs`。库存对当前 Local 的 mapped builtin agent 按稳定
registry 顺序构造 candidate；所有路径都由数据库 agent dir 和 sanitized lock name 拼接。

分类顺序：

1. 用 `symlink_metadata`/Windows reparse metadata 观察 entry 本身，不能先跟随 link 调用 `is_dir()`。
2. entry 存在且为 junction/symlink：解析最终 target；只有 component-aware 等价于当前 canonical 才是
   `managed_link`，并记录 `windows_junction` 或 `symlink`；broken/wrong target 是 `conflict`。
3. entry 为普通目录：`direct_copy`。不以目录内容相同推断它可由 link/unlink/remove 管理。
4. entry 是文件、其它 reparse kind、特殊对象或 metadata 无法形成所有权证明：`conflict`。
5. entry 不存在、canonical 为 owned directory且 target enabled/detected/支持本地 placement：`missing`。
6. entry 不存在但 canonical 缺失、target disabled/未检测或平台不支持：`unavailable`，使用稳定
   `reasonCode`（例如 `canonical_missing`、`platform_disabled`、`platform_not_detected`）。

`SkillsCliInstallKind` 继续描述 canonical/copy/missing 的全局展示 fallback；placement 只描述平台 slot。
`Unlinked only`、header count、batch preview、detail row 全部使用 placement，不再从 `agents` 推断。

## 3. Managed directory-link primitive

在 `services/installation/fs_util.rs` 抽出共享的 typed directory-link 原语，但不改变既有
`method=auto` 安装 fallback：

```rust
pub(crate) enum ManagedDirectoryLinkKind { WindowsJunction, Symlink }
pub(crate) fn inspect_managed_directory_link(link: &Path, expected: &Path)
    -> Result<Option<ManagedDirectoryLinkKind>, InstallationError>;
pub(crate) fn create_skills_cli_directory_link(target: &Path, link: &Path)
    -> Result<ManagedDirectoryLinkKind, InstallationError>;
pub(crate) fn remove_verified_directory_link(link: &Path, expected: &Path)
    -> Result<(), InstallationError>;
```

- Windows 新建路径使用现有 `windows-sys`，补
  `Win32_Storage_FileSystem`/`Win32_System_IO`/`Win32_System_Ioctl`/`Win32_System_SystemServices` features：
  先创建 operation-owned 空目录，再以 `CreateFileW(FILE_FLAG_OPEN_REPARSE_POINT |
  FILE_FLAG_BACKUP_SEMANTICS)` 打开 entry，构造 bounds-checked `IO_REPARSE_TAG_MOUNT_POINT` buffer（substitute
  name 为 canonical absolute target 的 NT `\\??\\` form），通过 `DeviceIoControl(FSCTL_SET_REPARSE_POINT)`
  写入。任一步失败只清理本次创建且仍为空的 entry；不调用 `cmd.exe`/`mklink`，不要求 symlink privilege，
  不 fallback 为 copy。创建后用 `FSCTL_GET_REPARSE_POINT` 重新读取 tag/substitute target；验证失败清理本次 entry。
- Windows classifier 同时识别 junction 与真实 directory symlink；删除前以 reparse tag + resolved target
  再验证，只调用 entry-safe `remove_dir`，绝不 `remove_dir_all`。
- Unix 使用 `std::os::unix::fs::symlink`，删除前 `read_link` + `paths_equivalent`，只调用 `remove_file`。
- 其它平台返回 typed unavailable。普通目录与未知 reparse tag 永不进入删除 helper。

Windows 单元测试必须使用真实 tempdir junction/symlink；权限或文件系统不支持时不能把 skipped 当 PASS，
应保留 `UNVERIFIED` 并在 Windows native gate 单独报告。

## 4. Bounded SKILL.md 与 Reveal

新增 `services/skills_cli/files.rs`：

```rust
pub(crate) async fn read_skill_md(skill_name: &str) -> Result<SkillsCliSkillDoc, SkillsCliError>;
pub(crate) fn reveal_skill_folder(skill_name: &str) -> Result<(), SkillsCliError>;
```

两条入口都先加载 lock ownership，以 sanitized name 构造 `canonical_root/name`，canonicalize root 与
candidate 并做 component-aware containment。读取再拼 `SKILL.md`，通过
`run_blocking_fs_with(... read_file_text_bounded(ReadLimit::new("Skills CLI SKILL.md", 1_048_576)))`；
将 `LimitExceeded`、`InvalidUtf8`、not-found、join/IO 映射为独立 `SkillsCliError`，公开错误不含路径/内容。

Reveal command 不接受 path；在 containment 后确认 candidate 是 directory，再复用抽出的本地文件管理器
launcher。launcher 只接收 `Path` 作为单独 argv，禁止 shell/string concat；spawn 失败映射稳定安全错误。

## 5. Link / Unlink command boundary

签名：

```rust
skills_cli_link_platform(job_id, skill_name, skillport_agent_id) -> IpcResult<SkillsCliPlacement>
skills_cli_unlink_platform(job_id, skill_name, skillport_agent_id) -> IpcResult<SkillsCliPlacement>
```

命令层先于第一个 await 获取 `skills_cli_jobs` lease，再冻结 TargetContext 并完成 Local gate。service 获取
Local target mutation guard 后重新加载 lock、canonical、agent 和 placement，避免 preview/action TOCTOU。

- Link：`missing` → create+verify → 返回 `managed_link`；已是 owned `managed_link` 幂等返回；其它状态拒绝。
- Unlink：owned `managed_link` → remove verified entry → 返回 `missing` 或 `unavailable`；`missing` 幂等；
  `direct_copy`/`conflict`/`unavailable` 拒绝。
- cancel flag 在取得 guard 前以及实际 FS mutation 前检查；一旦 entry 创建/删除开始，函数同步完成验证或
  cleanup，不能返回未收敛的 cancellation。
- 操作日志只记录 action、skill logical id、agent logical id、status 和 stable code，不记录路径/target。

`exclusive-job-lifecycle.md` 和 `skills-cli-global.md` 把 link/unlink 加入 Skills CLI family start commands；
锁顺序固定为 lease → target guard → recheck → mutation。

## 6. Safe remove and recovery

新增只读预览 command，返回逻辑影响而不返回 filesystem path 或 CLI argv：

```rust
pub struct SkillsCliRemovePlan {
    pub skill_name: String,
    pub owned_canonical: bool,
    pub managed_placements: Vec<SkillsCliRemovePlacementSummary>,
    pub retained_direct_copies: Vec<SkillsCliRemovePlacementSummary>,
    pub conflicts: Vec<SkillsCliPlacementConflict>,
    pub confirmable: bool,
}

pub struct SkillsCliRemovePlacementSummary {
    pub agent_id: String,
    pub display_name: String,
}

skills_cli_preview_remove_global(skill_name: String)
    -> IpcResult<SkillsCliRemovePlan>
```

`SkillsCliRemovePlacementSummary` 与 `SkillsCliPlacementConflict` 只含 agent logical ID、display name，后者
额外含 reviewed reason code；两者都不含路径、target、
文件内容或 raw details。preview 从 fresh lock/placement 计算；`confirmable` 当且仅当 lock owns canonical 且
`conflicts` 为空。renderer 展示此结构化影响摘要，不展示或拼接“remove command”。

既有 `skills_cli_remove_global` 改为返回：

```rust
pub struct SkillsCliRemoveResult {
    pub removed_canonical: bool,
    pub removed_managed_agent_ids: Vec<String>,
    pub retained_direct_copy_agent_ids: Vec<String>,
}
```

它不再 spawn 未证明能保留 copy 的 `skills remove`。在 lease + Local guard 内：

1. 恢复同一技能遗留的 domain-local remove manifest，然后重新读取 lock/placement。
2. lock 不拥有名字 → `SkillNotOwned`；任一 `conflict` → 零写入 `PlacementConflict`。
3. 写入不含内容/凭据的 version-1 `prepared` manifest，记录 operation id、skill logical id、lock fingerprint、
   canonical/managed-link operation-owned path 与 phase；manifest 位于 `paths.rs` 定义的 app-data recovery root。
4. 将 owned canonical 原子 rename 到同父目录 operation-scoped backup；逐个移除再次验证的 managed link，
   更新 manifest 为 `staged`。`direct_copy` 完全不进入 mutation path list。
5. 对 lock v3 删除精确 row：持有 SkillPort adjacent lock-file lease，验证 opened bytes fingerprint 未漂移，
   同目录 temp + flush/sync + atomic replace，然后把 manifest 标记 `metadata_committed`。
6. 删除 canonical backup 与 manifest。cleanup 失败保留 manifest 并返回 recovery-required，不伪报 terminal success。

提交前任何失败都按 manifest 恢复 canonical 和 managed links；恢复碰到占用或 fingerprint drift 时 fail closed
并保留证据。提交后恢复只 finalize backup，不恢复已删除 lock row。每次 Skills CLI mutation 在同一 Local guard
内先恢复相关 pending manifest；显式 focused recovery tests 覆盖重复执行和中断点。普通目录不写入 manifest，
因此恢复也没有删除 copy 的能力。

## 7. Export writer

`skills_cli_export_inventory(path, json)` 接收 `batch-actions` 生成的 v1 serialized snapshot。后端在 blocking
closure 中检查：`.json`、UTF-8/1 MiB、JSON object、`schemaVersion == 1`、scope、`skillCount == skills.len()`；
每个 skill 必须且只能含 `name/source/sourceType/sourceUrl/installKind/canonicalPath/folderHash/installedAt/updatedAt/placements`，
每个 placement 必须且只能含 `agentId/displayName/state`，state 限于父设计冻结的五态。不接受缺字段、
等价字段别名或任何未知字段。写入复用 portable-state file adapter 的 same-directory temp + flush + sync + atomic
persist 模式，但使用 `SkillsCliError`，不复用 portable manifest parser。

save dialog 只返回 path，renderer 不获得 filesystem write authority。取消发生在调用 IPC 前。

## 8. Recent-source settings policy

继续使用 generic `get_setting`/`set_setting`，但在 `commands/settings_policy.rs` 明确登记唯一 key
`skills_cli.recent_sources` 与 `SettingCategory::SkillsCli`。validation：

- serde `Vec<String>`，deny 非数组；0–8 项；serialized <= 16 KiB；每项 1–2048 bytes；无 control char。
- 去除首尾空白后的值必须等于原值，使用 BTreeSet 检查 exact duplicate。
- 复用/抽出 Skills CLI source 纯验证；URL 不得含 username/password/query/fragment，避免 credential/token 落库。
- batch 仍先验证全部再单 transaction 写；audit details 只含 category/keyCount/valueStored。

最近源在成功安装后才写。写入失败是安装后的独立 follow-up failure，不能把已成功安装误报为失败。

## 9. Error and IPC matrix

`SkillsCliError` 增加语义变体与固定 code，包括：

| Failure | Code | retryable |
| --- | --- | --- |
| lock 不拥有名字 | `skills_cli.skill_not_owned` | false |
| canonical 缺失/非目录 | `skills_cli.canonical_missing` | false |
| SKILL.md 缺失/过大/非法 UTF-8 | `skills_cli.skill_doc_missing` / `skills_cli.skill_doc_too_large` / `skills_cli.skill_doc_invalid_utf8` | false |
| direct copy 不可 toggle | `skills_cli.direct_copy_not_toggleable` | false |
| conflict / unavailable | `skills_cli.placement_conflict` / `skills_cli.placement_unavailable` | false |
| export schema/write | `skills_cli.export_invalid` / `skills_cli.export_failed` | false |
| reveal 失败 | `skills_cli.reveal_failed` | false |
| remove 有待恢复状态 | `skills_cli.recovery_required` | true |
| guard/同 family busy | `skills_cli.busy` | true |

未知 IO/Windows API details 只到受控日志，IPC 固定为 `internal.unexpected` 或上表 reviewed message。

新增 command 恰为六条：read、link、unlink、reveal、export、preview-remove。只编辑 `ipc_registry.rs` 的 runtime/generated
registry 声明；运行 `pnpm ipc:codegen` 和 `pnpm docs:gen`，不手改生成物。`src/types/index.ts` re-export
placement/doc/remove DTO，browser fixture 与 IPC coverage 同步。

## 10. Test and rollback boundaries

Rust tests覆盖 lock parsing、placement table、bounded read、containment/reveal、settings policy、link/unlink
锁序/typed error、export atomic replace、safe remove recovery/fingerprint collision。Windows native tests单独证明
junction create/inspect/remove；没有直接运行证据时标记 `UNVERIFIED`。

回滚不得简单恢复旧 remove 行为。若新恢复 manifest 存在，先由当前版本收敛/清理，再回滚 code；新增 IPC
和字段为 additive，但 settings key、generated artifact、spec 与 fixtures 必须随同回滚。
