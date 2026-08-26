# Skills CLI 数据、Placement 与安全 IPC 契约

父任务：`08-26-skills-cli-redesign`。

## Goal

建立 Skills CLI 重设计唯一可信的后端边界：库存返回 lock 元数据和逐平台 placement，读取
`SKILL.md` 有明确预算，link/unlink/reveal/export/remove 都只能作用于经 lock 和路径策略证明的对象；
最近源通过受约束的 settings key 持久化。任何普通目录、错误目标的 reparse point 或 renderer
传入的任意路径都不能被覆盖或删除。

## Dependencies and Scope

- 本任务是 `page-shell`、`batch-actions`、`install-wizard`、`detail-drawer`、`update-center` 的显式前置；
  消费者在本任务完成并合入前保持 `planning`，不得以 placeholder 完成 AC。
- 本任务唯一拥有 Rust inventory/placement 类型、bounded doc、link/unlink/reveal/export writer、
  `skills_cli_preview_remove_global`/`skills_cli_remove_global` 安全语义、recent-source settings policy、IPC registry/codegen 与
  `.trellis/spec/backend/skills-cli-global.md`。
- 本任务不实现页面布局、弹窗、抽屉、批量循环或更新中心网络检查。

## Confirmed Facts

- 当前 `InventoryPlatform` 只含 display name/path（`src-tauri/src/services/skills_cli/inventory.rs:14`），
  并在普通 `is_dir` 命中时计入 `agents`（`src-tauri/src/services/skills_cli/inventory.rs:46`）；它不能区分
  junction、symlink、direct copy 与冲突。
- Windows 当前 link helper 位于 `src-tauri/src/services/installation/fs_util.rs:179`，使用 directory symlink；
  `method=auto` 在 `src-tauri/src/services/installation/native.rs:81` 失败后 fallback 为 copy，因此不能作为
  本任务的 managed-link 创建原语。
- generic settings 写入由 `src-tauri/src/commands/settings_policy.rs:43` 的 exact allowlist 和
  `src-tauri/src/commands/settings_policy.rs:58` 的 value validator 保护；未知
  `skills_cli.recent_sources` 当前会返回 `setting_key_forbidden`。
- `src-tauri/src/services/bounded_ingestion.rs:109` 的 `read_file_text_bounded` 通过
  `src-tauri/src/services/bounded_ingestion.rs:126` 的 `limit + 1` opened-handle 读取处理增长；
  `.trellis/spec/backend/external-text-ingestion.md:28` 把 `SKILL.md` 预算固定为 1 MiB。
- 原始 `--force` / `--keep-links` 帮助输出尚未持久化，不能进入产品 argv 或卸载设计。

## Requirements

- R1: `SkillsCliGlobalSkill` 增加 `canonicalPath`、`folderHash`、`installedAt`、`updatedAt` 和
  `placements`；lock 缺少可选字段时返回 `null`，既有字段语义保持兼容，`agents` 只作为兼容派生字段保留。
- R2: 每个 placement 使用 `managed_link | direct_copy | missing | conflict | unavailable` 五态；
  `managed_link` 另带 `windows_junction | symlink` link kind，且必须解析到当前 lock-owned canonical。
  普通目录只能是 `direct_copy`，错误/broken link、文件或无法证明所有权的对象是 `conflict`。
- R3: `skills_cli_read_skill_md(skillName)` 只从 lock-owned canonical 解析 `SKILL.md`，使用共享
  bounded ingestion 的 1 MiB 限额；oversize、增长越界、非法 UTF-8、缺失和归属失败分别映射稳定错误码。
- R4: `skills_cli_link_platform(jobId, skillName, skillportAgentId)` 只接受 Local、enabled、mapped
  target 的 `missing` placement。Windows 必须以无 shell 的 reparse-point API 创建 junction；其它受支持平台
  创建 symlink。创建后再次解析目标，失败时清理本次创建的 entry，不 fallback 为 copy。
- R5: `skills_cli_unlink_platform(jobId, skillName, skillportAgentId)` 只接受重新校验后仍为
  `managed_link` 的 placement，只移除 link/reparse entry；`missing` 幂等成功，`direct_copy`、`conflict`、
  `unavailable` 零写入拒绝，任何分支都禁止 `remove_dir_all` 平台路径。
- R6: `skills_cli_reveal_skill_folder(skillName)` 不接收 renderer path；后端从 lock 解析 canonical，
  canonicalize root/candidate 并执行 component-aware containment，确认目录存在后才调用系统文件管理器。
- R7: `skills_cli_export_inventory(path, json)` 只接受 1 MiB 内、UTF-8、有效且符合
  `SkillsCliInventoryExportV1` envelope 的 `.json`，使用同目录临时文件、flush/sync 和 atomic persist；
  payload/log 不包含 PAT、命令环境、SKILL.md 正文或错误 details。
- R8: `skills_cli_preview_remove_global(skillName)` 从 fresh lock/placement 返回不含路径或 argv 的结构化
  `SkillsCliRemovePlan`；`skills_cli_remove_global` 在 mutation guard 内重新读取 lock 并重算同一语义。
  任何 `conflict` 在零写入时阻断。成功只移除 owned canonical、精确 lock row 和再次验证的 managed links，
  返回 retained `direct_copy` 摘要，绝不 spawn `skills remove`、传 `--keep-links`、重建悬空 link 或删除普通目录。
- R9: remove 使用操作级恢复清单和 staged sibling rename：破坏性步骤前持久化 `prepared`，lock row
  以比较前后 fingerprint 的原子替换提交；提交前失败同步回滚，提交后遗留 backup 由下一次 Skills CLI
  mutation/recovery 幂等完成。清单只可包含恢复必需的 operation-owned 绝对路径，不含文件内容、凭据或
  命令输出；manifest、operation id 与 backup/recovery 路径不得进入 IPC、日志、telemetry 或 portable export，
  正常 inventory/export 路径字段必须独立由 R1/R2 ownership classifier 生成。
- R10: link/unlink/remove 的顺序固定为 Skills CLI exclusive job lease → Local target mutation guard →
  under-guard ownership/placement recheck → FS/lock mutation；busy/cancel/typed failure 不得绕过 guard。
- R11: exact generic settings key `skills_cli.recent_sources` 归入独立 `skills_cli` audit category；值必须是
  0–8 个去重 source 的 JSON 数组，每项非空、最多 2048 bytes、无控制字符和 URL credentials/query secrets，
  并通过与 Skills CLI source 相同的纯验证规则。整个序列化值上限 16 KiB，任一非法成员导致零写入。
- R12: 新增六条 command（read/link/unlink/reveal/export/preview-remove），并修订既有 remove 返回类型；所有 fallible
  command 返回 `IpcResult<T>`，service 返回 `SkillsCliError`。`ipc_registry.rs` 是 runtime/generated registry
  唯一登记点，生成 TypeScript 和架构文档不得手改。
- R13: 在 `research/skills-cli-capability-probe.md` 持久化 `skills@1.5.23` add/remove `--help` 原始输出，
  并以隔离临时 HOME/probe 验证 pinned full-SHA source 和 direct-copy refresh；每项记录采集命令、UTC 时间、
  版本、退出状态、原始 stdout/stderr 或安全失败结论。缺少直接证据的能力保持 `UNVERIFIED/unsupported`，
  `--force` / `--keep-links` 及其它猜测行为不得进入产品 argv。
- R14: `.trellis/spec/backend/skills-cli-global.md` 同步 placement 状态表、所有权、命令签名、错误矩阵、
  recent-source policy、安全 remove/recovery 和 Windows junction 测试边界。

## Constraints

- 所有 `skills_cli_*` command 都是 Local target only；非 Local 在任何 lock read、文件检查或进程启动前返回
  `skills_cli.local_target_only`。recent sources 是仅由 Local-only install surface 调用的本机 renderer preference，
  通过 exact settings allowlist/policy 校验，不宣称 generic settings command 自带 target gate。
- service 层只返回 typed `SkillsCliError`；公开 error message/log 不能包含绝对路径、lock bytes、文件内容、
  source credential、CLI stdout/stderr 或命令行。成功 DTO 只可包含 R1/R2/R7 明定的 inventory/export 路径字段。
- link/unlink 的平台路径只能由数据库 agent `global_skills_dir + sanitized lock name` 构造，不能接受
  renderer path。canonical 只能来自固定 Universal root 与 lock-owned name。
- Windows junction 实现复用现有 `windows-sys` 依赖并按需扩展 feature，不引 shell、不调用 `mklink`，
  不新增 copy fallback。Windows 原生创建/识别/权限失败/安全删除在直接执行前保持 `UNVERIFIED`。
- export 只负责安全写入；v1 serializer、scope 和默认文件名由 `batch-actions` 唯一拥有。

## Out of Scope

- 把 `direct_copy` 自动转换为 junction/symlink，或删除/覆盖它。
- 对 `conflict` 做自动修复或强制删除。
- SSH/WSL placement、远程文件管理器、CSV/Markdown 导出或快照导入。
- 根据未持久化的 CLI help 猜测 flag 或行为。

## Acceptance Criteria

- [ ] AC1 (R1): lock camel/snake 字段齐全、缺失和空串 fixtures 均得到确定的 nullable metadata；既有字段和 legacy fixture 可回读。
- [ ] AC2 (R1,R2): 每个库存技能按稳定 agent 顺序返回 placement；`agents` 只由 `managed_link` 与 `direct_copy` display name 派生，不使用路径字符串二次猜测。
- [ ] AC3 (R2): Windows junction、Windows symlink、Unix symlink 解析到 canonical 时为 `managed_link` 且 link kind 正确；普通目录为 `direct_copy`，wrong/broken link、文件和特殊对象为 `conflict`。
- [ ] AC4 (R2): canonical 缺失或 target disabled/未检测/不支持时，缺失平台 entry 为带稳定 `reasonCode` 的 `unavailable`；只有可安全创建 link 的 absent slot 为 `missing`。
- [ ] AC5 (R3): owned 且不超过 1 MiB 的 UTF-8 `SKILL.md` 返回正文和真实 byte size；lock 外名字与 canonical escape 在读取前被拒绝。
- [ ] AC6 (R3): exactly-limit 成功，metadata oversized、opened file growth 到 `limit + 1`、非法 UTF-8 和缺失文件分别返回 review 过的 code，serialized IPC 不泄漏 path/content。
- [ ] AC7 (R4,R10): link 在 lease/guard 后重校验 `missing`，Windows 创建并验证真实 junction，Unix 创建并验证 symlink；已是本 canonical 的 managed link 幂等，创建失败不留下 partial entry。
- [ ] AC8 (R4,R5): `direct_copy`/`conflict`/`unavailable` link 或 unlink 均返回稳定 typed code 且字节级保持原对象；错误 target link 不被删除。
- [ ] AC9 (R5,R10): unlink 仅删除再次验证仍指向 canonical 的 link entry，canonical 内容保持；missing 幂等成功，非 Local 与 busy/cancel 均发生零 FS 写。
- [ ] AC10 (R6): reveal 只打开 lock-owned canonical；lock 外名字、缺失/非目录、相似前缀、final/intermediate symlink escape 均拒绝，command 不接收 path 参数。
- [ ] AC11 (R7): export 拒绝非 `.json`、oversize、invalid UTF-8/JSON、错误 schema/version；成功 atomic replace，persist 失败保留旧文件并清理 temp。
- [ ] AC12 (R8): `skills_cli_preview_remove_global`/remove result 分别给出 owned canonical、managed link、retained direct-copy 和 blocking conflict；preview 只返回逻辑 ID/显示名/布尔值/reason code 而无路径或 argv，任何 conflict 时 canonical、lock、links、copies 均不变。
- [ ] AC13 (R8,R9,R10): 无 conflict 的 remove 成功后 canonical/精确 lock row/managed links 消失，direct copies 字节级保留；不 spawn `skills remove`，不使用 `--keep-links`。
- [ ] AC14 (R9): 注入 prepared、staged、lock replace、cleanup 与进程中断后，恢复收敛到完整旧状态或已提交新状态；未知 fingerprint/collision 保留恢复证据并 fail closed。
- [ ] AC15 (R11): settings policy 接受 0–8 个合法去重源并可重启读回；第 9 项、重复、空值、控制字符、超长、credential/query secret、未知键或混合非法 batch 均零写入且错误/日志不含 caller value。
- [ ] AC16 (R12): runtime registry 仅新增 read/link/unlink/reveal/export/preview-remove 六条命令，remove result 类型同步；`pnpm ipc:codegen`/`pnpm docs:gen` 后二次 check 无 drift，fixtures 与 `src/types/index.ts` re-export 同步。
- [ ] AC17 (R13): `research/skills-cli-capability-probe.md` 对 help、pinned full-SHA source、direct-copy refresh 三类能力记录命令/版本/UTC 时间/退出状态和原始输出或安全失败；每项只有 `VERIFIED_SUPPORTED`、`VERIFIED_UNSUPPORTED`、`UNVERIFIED` 三种结论，未验证能力不进入产品 argv。
- [ ] AC18 (R14): backend spec 的签名、placement/ownership、错误矩阵、recovery、settings 与测试段均更新，明确 ordinary directory 永不由 link/unlink/remove 删除。
- [ ] AC19 (R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,R13,R14): focused Rust/settings/IPC tests、locked all-target Clippy/tests、`pnpm typecheck`、codegen/docs checks 与最终 `just ci` 通过；Windows native junction 与 installer 行为没有直接证据时报告 `UNVERIFIED`。
