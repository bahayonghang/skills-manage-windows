# Skills CLI 远端只读列举

父任务：`08-27-skills-cli-remote-target`（源需求 U5）
序位：远端树第 2 个

## Goal

让用户切到 SSH / WSL 目标后，`/skills-cli` 能列出该远端主机上的 Skills CLI 全局技能、
placement 状态与 canonical / lock 路径。本任务只交付读路径。

## Confirmed Facts

- 本机 `skills_cli_list_global` 是 lock v3 + 文件系统读，**不 spawn CLI**
  （spec `skills-cli-global.md:81-94`）。远端应保持同一性质。
- 成员资格只看 lock 名字；`path` / `installKind` 优先 `universal_skills_dir/<name>`，
  否则取 mapped∩detected agent 下的 copy 目录（`canonical` | `copy` | `missing`）。
- 权威平台状态是 `placements` 五态（`managed_link` | `direct_copy` | `missing` | `conflict` | `unavailable`），
  由 `placement.rs:27-110` 的 `classify_placements` / `classify_absent` 产生。
  不得新增并行的 `agentIds` / `linkTargets` 数组。
- 缺失或空 lock 返回空 `skills` 数组但仍带 `canonicalRoot` 与 `lockPath`，**不是错误**。
  列举 IO 失败映射 `internal.unexpected`，绝不用 `skills_cli.cli_unavailable`。
- `classify_one`（`placement.rs:40-71`）对每个技能 × 每个平台做一次 `observe_directory_slot`，
  本机是廉价 syscall。**远端逐次做会变成 N×M 次 SSH 握手**（每次握手连接超时 10s），
  必须改为常数级往返。
- `canonical_is_owned_directory`（`placement.rs:112-117`）依赖 `symlink_metadata` 且要求
  「是目录且非 reparse/symlink」。远端需要等价语义的远端探测。
- 前端闸门：`SkillsCliView.tsx:210-217`（非 Local 渲染占位）、`:111-116`（非 Local 不 `loadAll()`）、
  `Sidebar.tsx:113-114`（非 Local 隐藏入口）。
- 切换 target 时 `AppShell.tsx:91-127` 会重置 skills-cli store 并触发全局重扫，已有机制可复用。

## Requirements

- R1：远端列举通过 seam 提供的传输执行，读 lock v3 + 远端文件系统，不 spawn CLI。
  返回结构与本机 `SkillsCliGlobalSnapshot` 完全一致，前端不区分来源。
- R2：远端列举的 SSH 往返次数与技能数、平台数**无关**（常数级）。
  实现方式为少数几次批量远端命令输出结构化清单，而非逐条 stat。
- R3：远端 placement 分类产出与本机相同的五态与相同的 `reason_code` 集合
  （`canonical_missing` / `platform_unsupported` / `platform_not_detected` / `platform_disabled`），
  包括「目录 vs 符号链接 vs 普通文件」的区分语义。
- R4：远端平台探测（detected / enabled）在远端主机文件系统上进行，
  不得复用本机平台探测结果。
- R5：远端缺失或空 lock 返回空 `skills` 且带 `canonicalRoot` / `lockPath`，不报错。
  远端 IO 失败映射 `internal.unexpected`。
- R6：解除前端读路径闸门：非 Local 目标时侧边栏显示入口、页面加载远端库存。
  写操作按 seam 能力矩阵，未支持的显示本地化原因而非静默 disabled。
- R7：远端连接失败 / 认证失败 / 超时有区分明确的稳定错误码与重试语义，
  库存保留 stale-while-revalidate 行为（沿用现有 `inventoryError` 分轨）。
- R8：远端 stdout / stderr / 路径不进入 `IpcError.message` 或未脱敏操作日志。
- R9：新增文案 en/zh 成对。

## Acceptance Criteria

- [ ] AC1 (R2)：fake runner 断言远端列举的远端命令调用次数在
      「3 技能 × 4 平台」与「30 技能 × 4 平台」两种输入下**相同**。
- [ ] AC2 (R1)：远端列举路径上没有任何 CLI spawn（断言 argv 构造器未被调用）。
- [ ] AC3 (R3)：远端分类测试覆盖五态各一例，并覆盖四个 `reason_code`；
      与本机同输入的分类结果逐字段一致。
- [ ] AC4 (R3)：远端「managed link」判定在 Unix symlink 与远端 Windows junction 两种情形下都正确，
      且普通目录不被误判为 managed link。
- [ ] AC4b (R4)：**前置条件是本机与远端平台布局故意不同**（TPR-08 之前 R4 无任何 AC 覆盖）。
      构造：本机检测到平台 A、B 且两者 global skills dir 均存在；远端只存在平台 A，
      平台 B 的目录在远端不存在。断言远端列举结果中平台 B 为 `platform_not_detected`
      而非沿用本机的「已检测」结论；平台 A 的 enabled 状态也取远端值。
      同一测试反向构造一次（远端多于本机），断言远端独有的平台出现在结果中。
- [ ] AC4c (R4)：远端列举路径上不发生对本机平台目录的探测——
      以 fake 本机 FS 或路径断言证明远端流程未读取本机 `resolve_home_dir()` 派生路径。
- [ ] AC5 (R5)：远端 lock 缺失与 lock 为空两种情形都返回空 `skills` 且携带路径，不产生错误。
- [ ] AC6 (R6)：切到 SSH target 后侧边栏出现 `/skills-cli` 入口，页面渲染远端库存；
      未支持的写操作显示本地化原因。
- [ ] AC7 (R7)：远端连接失败时展示可重试的库存错误，已有列表不被清空。
- [ ] AC7b (R7)：连接/认证失败与超时返回**可区分**的稳定错误码——
      前者为新增的 `skills_cli.remote_unavailable`，后者沿用 `skills_cli.timeout`。
      新增码有 en/zh 公开句且经 `pnpm ipc:codegen` 落入 reviewed codes；
      公开句不含主机名、用户名、路径或 stderr（design §2.5、§4）。
- [ ] AC8 (R8)：植入 stderr 哨兵 token，断言其不出现在 IPC message 与操作日志。
- [ ] AC9 (R9)：i18n en/zh parity 通过。
- [ ] AC10 (Completion Gate)：`just ci` 通过。真实 SSH 主机端到端行为标记 `UNVERIFIED`。
      来源是 `AGENTS.md` 的 Completion Gate 一节，不隶属本任务的任一 R（TPR-09）。

## Out of Scope

- 远端 link / unlink / 卸载 / 安装 / 更新（属后续两个子任务）。
- 远端 leftover 扫描。
- 持久 SSH 会话池。

## Dependencies

- `08-27-skills-cli-remote-seam` 必须先合入 `dev`。
