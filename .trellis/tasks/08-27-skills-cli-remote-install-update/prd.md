# Skills CLI 远端安装与更新

父任务：`08-27-skills-cli-remote-target`（源需求 U5）
序位：远端树第 4 个，最后一个

## Goal

让远端 SSH / WSL 目标支持 Skills CLI 的技能安装（含来源预览）与升级检测/应用。
这是远端树中唯一需要远端主机具备外网能力的部分。

## Confirmed Facts

- 安装 argv（spec `skills-cli-global.md:77-80`）：程序是 `node.exe` / `node`，
  `argv[1]` 是 npm `npx-cli.js`，前缀 `--yes --package=skills@1.5.23 -- skills`，
  add 层再加 `-g -y` 与至少一个 `-a`、`-s`。
  **禁止** `Command::new("npx.cmd")` 或 `cmd /c` 字符串拼接；禁止默认 `--all` / `--agent '*'`。
  远端需要构造等价的远端命令，且保持同样的禁止项。
- 来源白名单：拒绝 `&|^%!<>"'`、空格、`-c`（spec `:149`）。远端不得放宽。
- 进程策略（spec `:108-111`）：preview = Standard，add = BulkTransfer；stderr cap 1 MiB；
  stdout / stderr / URL 不进 `IpcError.message` 与未脱敏日志。
  远端对应 `ProcessPolicy` 的 Standard 120s 与 BulkTransfer 15min（`targets/runner.rs:40-77`）。
- 更新（spec `:117-127`）：GitHub SHA / snapshot pinning 用于**检测**，
  复用 SecretStore / `github_client`（在命令边界）；
  产品 argv 绝不含 `--force` / `--keep-links` / 未验证的全 SHA `skills add` 来源；
  pinned 全 SHA add/update 与 direct-copy 刷新是 fail-closed
  （`verified_unsupported` / `unverified`）；
  apply 从 pinned GitHub 快照经 HTTP 刷新 owned canonical 文件，
  再 journal `prepared → cli_started → db_committed`（或 `recovery_required`）；
  顺序为 `skills_cli` lease → 网络准备 → 变更 guard → recheck → journal。
  进度事件 `skills-cli://update-progress`。
- **远端 apply 的关键设计问题**：本机 apply 是「本机 HTTP 拉快照 → 写本机 canonical」。
  远端存在两条路线——由 SkillPort 本机拉取后下发到远端，或让远端主机自己拉取。
  二者对「远端是否需要外网」「凭据是否离开本机」的影响不同，必须在 design.md 中定论。
- `skills_cli_apply_updates` 每请求一个 `repositoryKey`（`generatedCommandMap.ts:989-993`）。
- `install_origin` 标注（spec `:134-135`）目前经 `classify_local_path_origin` 只在 Local 生效；
  远端需要等价语义或明确 fail-closed。
- 远端每次命令一次 SSH 握手；BulkTransfer 类操作需要注意 keepalive 与 15 分钟上限。

## Requirements

- R1：远端安装通过 seam 构造等价的远端 node + npx-cli.js 调用，
  保持 PIN spec 与 `-g -y -a -s` argv 规则；
  禁止 `npx.cmd`、`cmd /c` 拼接、默认 `--all` / `--agent '*'`。
- R2：来源白名单在远端同样生效，不得放宽。
- R3：远端来源预览与安装分别套用 Standard 与 BulkTransfer 等价策略；
  stderr 有上限，且 stdout / stderr / URL 不进 `IpcError.message` 与未脱敏日志。
- R4：远端主机不可访问 npm registry 时返回稳定且可理解的错误码，零写，不留半完成状态。
- R5：远端更新检测复用现有 GitHub 检测路径与 fail-closed 语义
  （`verified_unsupported` / `unverified` / `update_local_modified` / `update_topology_conflict`）。
- R6：远端 apply 按 D1 实现——本机拉取快照，经 SSH 把需刷新的技能子集流式下发到远端 staging。
  GitHub 凭据不得写入远端主机，也不得进入远端命令行参数；token 只存在于本机 HTTP 请求头中。
  **不得**复用 `github_import` 把 token 写进远端 `curl.conf` 的做法。
- R7：远端 apply 遵守 lease → 网络准备 → 远端 target guard → recheck → journal 顺序，
  journal 阶段与本机一致，可 recovery 且收敛。
- R8：`install_origin` 在远端要么有等价语义，要么在能力矩阵中明确标注不支持并 fail-closed。
- R9：进度事件在远端同样发出，前端复用现有 `skills-cli://update-progress` 消费路径。
- R10：远端 stdout / stderr / 路径 / URL 不进入 `IpcError.message` 或未脱敏操作日志。
- R11：新增文案 en/zh 成对；`pnpm docs:gen` 同步。

## Acceptance Criteria

- [ ] AC1 (R1)：远端 argv 表测试断言含 `--yes`、PIN spec、`-g -y -a -s`；
      断言不含 `--all`、`*`、`npx.cmd`、`cmd /c`。
- [ ] AC2 (R2)：`&|^%!` 等来源在远端路径同样被拒绝。
- [ ] AC3 (R3)：远端 add 超时与 stdout 上限经 `ProcessPolicy::for_tests` 等价手段可验证；
      超时返回 `skills_cli.timeout`。
- [ ] AC4 (R4)：模拟远端无外网时返回稳定错误码，远端文件系统零变更。
- [ ] AC5 (R5)：远端更新检测对 direct_copy 与 conflict 拓扑 fail-closed；
      本地已修改 canonical 返回 `update_local_modified`。
- [ ] AC6 (R6)：测试断言 GitHub token 不出现在任何远端命令 argv、远端环境变量或远端落盘内容中。
- [ ] AC7 (R7)：在 `prepared` / `cli_started` / `db_committed` 各注入一次故障，
      断言远端 recovery 可重试并收敛；apply 后 lock 与 placement 一致。
- [ ] AC8 (R8)：能力矩阵中 `install_origin` 的远端行为有明确条目——**远端不支持，fail-closed**，
      返回的 placement 中 `install_origin` 为 `None`（design §2.7）。
      测试断言远端结果不携带猜测出的来源标注。
- [ ] AC9 (R9)：远端 apply 过程发出 `skills-cli://update-progress`，前端渲染进度。
- [ ] AC10 (R10)：植入 stderr 与 URL 哨兵 token，断言其不出现在 IPC message 与操作日志。
- [ ] AC11 (R11)：i18n en/zh parity 与 `pnpm docs:gen:check` 通过。
- [ ] AC12 (Completion Gate)：`just ci` 通过。真实 SSH 主机端到端行为标记 `UNVERIFIED`。
      来源是 `AGENTS.md` 的 Completion Gate 一节，不隶属本任务的任一 R（TPR-09）。

## Out of Scope

- 改变 `SKILLS_CLI_NPM_SPEC` PIN 版本。
- 远端 Node 的自动安装或升级。
- 在远端主机上缓存 npx 包以加速后续安装。
- 持久 SSH 会话池。

## Dependencies

- `08-27-skills-cli-remote-mutate` 必须先合入 `dev`：
  apply 依赖远端 guard 与 journal 顺序已经定型。

## Decisions

- **D1（Q4 已关闭，TPR-01）**：远端 apply 采用**本机拉取 GitHub 快照 + 经 SSH 流式下发**。
  这不是在两个可行方案中择优，而是 R6「凭据不得写入远端主机」只留下这一条路：
  由远端自取必须把 token 送到远端（既有 `github_import/remote.rs:206-212` 正是这么做的，
  把 `Authorization: Bearer` 写进远端 `curl.conf`），违反 R6；
  不带凭据的远端自取则只能支持公开仓库，相对本机 apply 是功能倒退。
  原计划「等 `remote-mutate` 有传输实测数据后再定」不再必要——
  性能只在两条路线都被允许时才是决定因素。详见 `design.md` §2.4。

## Open Questions

无。Q4 已由上方 D1 关闭。
