# Central 删除恢复碰撞支持强制删除

## Goal

用户在 Central Skills 删除对话框里删除一个带有陈旧 `prepared` journal 的技能时，必须看到已评审的失败原因。无 backup / marker 时，用户用一次明确确认即可删掉**当前** Central 副本。用户不必先去 Observability Console 猜 Retry / Reconcile。中央内容相对 journal 指纹已经变化时，强制删除仍然可用。

## Background

2026-08-17 删除 `yao-meta-skill` 失败。`CENTRAL.DELETE` 耗时 45 ms，错误 `delete_restore_collision`。现场 journal `1198b10f-ecf0-4d4a-9ae2-23f0513314ab` 自 2026-08-05 起停在 `prepared`：manifest 含重复平台路径，四个平台 original / backup / marker 均不存在，Central 目录仍在。删除先恢复该行；`restore_delete_local_blocking` 把 `(false, false)` 判成碰撞，新删除没有开始。单删命令把域错误收成字符串，IPC 落到 `internal.unexpected`。Observability Console 的 Retry 会再次碰撞；Reconcile 只做 journal `prepared -> rolled_back`，且指纹漂移时会拒绝。完整证据见 `research/yao-meta-delete-restore-collision.md`。

## Requirements

- R1 删除预览必须声明该技能是否存在非终态 Central journal，以及 `operationKind` / `phase` / 稳定 `errorCode` / 强制删除是否可用。不得展示路径、fingerprint 或 `manifest_json`。
- R2 删除对话框在存在待恢复或本次删除因恢复碰撞失败时，用 `formatBackendError` 展示已评审文案，不得把 `internal.unexpected` 或 `String(err)` 拼进 toast / 内联错误。
- R3 删除对话框在强制删除可用时提供独立确认入口。该入口只在无 backup / marker 残留时可用。
- R4 强制删除必须先把阻塞中的 `prepared` journal 收成 `rolled_back`，再对**当前** owned 路径走既有 journaled delete。不得跳过 `fs_db_operations` 直接 `remove_dir_all` + `DELETE FROM skills`。
- R5 强制删除不得删除当前未勾选的独立 copy 安装，也不得级联 `agent_skill_observations` / project / usage 历史。
- R6 Observability Console Retry / 普通 restore 保持 fail-closed。`(false, false)` 不得被静默当成 already-gone。逃脱口是删除对话框的强制删除，不是放宽 restore。
- R7 若 backup 或 marker 仍在，或 journal 不是 `prepared` 的 `central_delete`，强制删除保持禁用，并指向 Observability Console。不得静默丢弃 backup。
- R8 单删、批量删除、仓库删除共用同一预览字段与 `force` 语义。
- R9 中央内容相对 journal 指纹已漂移、但 backup / marker 都不存在时，强制删除必须仍然可用。现有 Reconcile 的指纹校验保持不变。
- R10 全部新文案进入 `src/i18n/`。删除 IPC 必须带稳定 code，供 `formatBackendError` 与 `backendErrors.*` 使用。
- R11 回归测试覆盖：yao-meta 形态（prepared、重复平台路径、平台 already gone、Central 仍在）、Central 指纹漂移仍可强制删除、有 artifact 时拒绝、强制删除后 Central 行与目录消失且无 pending recovery。

## Out of scope

- 不重建 journal 状态机，不改 `restore_delete_local` 的碰撞表。
- 不在 `fs_staged` / `db_committed` 且仍有 backup 时提供丢弃 backup 的核选项。
- 不把 Observability Console Reconcile 放宽到忽略指纹。
- 不改 SSH/WSL 传输脚本正文；远端只复用同一存在性判定。
- 不提供手工改 SQLite / 手工删目录的产品入口。
- 不在本任务里替本机 `yao-meta` 执行删除。现有 Reconcile 仍是事故临时出口。

## Acceptance Criteria

- [ ] AC1 删除带有 `prepared` 碰撞 journal 的技能时，对话框显示 `backendErrors.central_operation.delete_restore_collision` 对应文案，而不是 “See runtime logs for details.”
- [ ] AC2 无 backup / marker 时，用户确认强制删除后：旧 journal 为 `rolled_back`；随后新删除到达 `completed`；当前 Central 目录与 `skills` 行消失。
- [ ] AC3 中央目录内容相对 journal 指纹已变、且无 backup / marker 时，强制删除仍然成功（R9）。
- [ ] AC4 有 backup 或 marker，或 phase 不是 `prepared` 时，强制删除不可用；journal 与文件证据保持不变。
- [ ] AC5 平台路径 already gone、Central 仍在的重复-path manifest 可被强制删除；缺失平台路径不按 owned missing 拒绝。
- [ ] AC6 批量 / 仓库删除对同一技能给出同一 `error_code` 与同一 `force` 入口。
- [ ] AC7 成功后，安装、扫描、列表不再把该技能标为 pending recovery。
- [ ] AC8 en / zh 文案完整；测试证明 toast / 内联错误不含路径、token 或 manifest。
- [ ] AC9 `just ci` 通过。若命令签名或 schema 有变，包含 `pnpm docs:gen` 产物。
