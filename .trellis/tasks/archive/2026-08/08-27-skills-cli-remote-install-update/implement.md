# 执行计划 — Skills CLI 远端安装与更新

依据 `prd.md` 与 `design.md`。按段执行，每段结束跑该段验证命令再进入下一段。

**前置**：`08-27-skills-cli-remote-mutate` 已合入 `dev`（apply 依赖远端 guard 与 journal 定型）。

## 段 1 — 远端 launcher 与 argv（回滚单元 A）

- [ ] 1.1 `SkillsCliTransport` 增加远端 launcher 解析：找到远端 node 后，
      按与本机 `npx_js_candidates`（`argv.rs:233`）相同的候选顺序在远端探测 `npx-cli.js`。
      **一次远端往返**，与段 2 的预检合并。
- [ ] 1.2 解析失败返回 `SkillsCliError::CliUnavailable`（既有码，语义已由
      `doctor-gate` 收敛为「环境无法执行」）。
- [ ] 1.3 远端 argv 构造：形状与本机逐字一致
      （`--yes --package=skills@<PIN> -- skills -g -y -a … -s …`），
      每个元素经 `shell_quote`（`exec.rs:703`）转义后拼接。
      **禁止**出现 `npx.cmd`、`cmd /c`、默认 `--all` 或 `--agent '*'`。
- [ ] 1.4 来源白名单校验（拒绝 `&|^%!<>"'`、空格、`-c`）在**发往远端之前**执行，
      复用本机既有校验函数，不另写一份、不放宽。

验证：`cargo check -p skillport`

## 段 2 — 远端预览与安装（回滚单元 B）

- [ ] 2.1 远端预览走 `run_command`（`ProcessPolicy::standard()`，120s）。
- [ ] 2.2 远端 add 走 `run_script_cancellable`（`bulk_transfer()`，15min）。
      **必须用 cancellable 变体**——既为 15min 上限，也为让 lease 的取消旗标生效。
- [ ] 2.3 非零退出 → `SkillsCliError::CliFailed` → `skills_cli.cli_failed`。
      失败路径不写 journal、不改 lock、不建链接（R4 的零写面）。
- [ ] 2.4 失败时的结构化 warn 沿用 `doctor-gate` design §2.5 的字段白名单，
      追加 `target_kind`（`local` / `ssh` / `wsl`，静态字面量）。
      **不得**记录远端主机名、用户名、路径、stderr、URL。

验证：`cargo test -p skillport skills_cli`

## 段 3 — 远端更新检测（回滚单元 C）

- [ ] 3.1 GitHub SHA / snapshot pinning 检测**保持在本机**，不加远端往返（design §2.6）。
- [ ] 3.2 远端 canonical 摘要计算：一次远端脚本，参照
      `central_updates/fs.rs:384` 的 `REMOTE_HASH_SCRIPT`（多根一次哈希）。
      比对在本机做。
- [ ] 3.3 `update_local_modified` 由 3.2 的摘要与 baseline 比对产生。
- [ ] 3.4 `update_topology_conflict` 判据取 `remote-inventory` 的分类结果：
      direct_copy 与 conflict 拓扑 fail-closed。
- [ ] 3.5 `verified_unsupported` / `unverified` 的 fail-closed 语义在远端不放宽。

验证：`cargo test -p skillport skills_cli`

## 段 4 — 远端 apply：快照下发与 journal（回滚单元 D）

- [ ] 4.1 本机拉 pinned 快照后**裁出需刷新的技能子集**，不传整个仓库。
- [ ] 4.2 打 tar 流，经 `ConnectedRemoteTarget::run_command_with_stdin_bytes_cancellable`
      （`remote.rs:125`）管进远端 `tar -x` 到 staging 目录，不先落远端磁盘再解压。
- [ ] 4.3 **显式检查项**：确认 4.2 所用方法的 `ProcessPolicy` 是 `bulk_transfer()`。
      若它实际是 `standard()`（120s），大快照会超时——
      此时改用带 bulk 策略的路径，或在 `targets/` 补一个 bulk 变体。
      不要默认它已经是对的。
- [ ] 4.4 锁顺序：lease → **网络准备（4.1、4.2 的本机部分）** → 远端 target guard
      → guard 下 recheck → journal。网络准备排在 guard 之前，与本机 apply 一致
      （`apply.rs:208` 取快照、`:239-245` 才取 guard）。
- [ ] 4.5 journal 沿用 `apply.rs:45-53` 的九个阶段常量，**不改名**。
      远端语义：`backups_staged` = 远端 canonical 已备份；
      `cleanup_pending` = 远端 staging 待清理。
      `cli_started` 是历史遗留命名（apply 本就不 spawn CLI），保留以兼容 DB schema。
- [ ] 4.6 `updates/apply/recover.rs` 支持远端续做：重连远端后按 journal 阶段恢复。
- [ ] 4.7 进度事件**零新增**：复用 `UPDATE_PROGRESS_EVENT`（`updates/mod.rs:32`）
      与 `AppUpdateProgress`（`commands/skills_cli.rs:483-486`），phase 字符串不变。

验证：`cargo test -p skillport skills_cli`

## 段 5 — `install_origin` 与能力矩阵（回滚单元 E）

- [ ] 5.1 远端 placement 的 `install_origin` 返回 `None`，**不实现猜测逻辑**（design §2.7）。
- [ ] 5.2 能力矩阵翻闸：`PreviewSource` / `AddGlobal` / `CheckUpdates` /
      `UpdateInventory` / `VerifyUpdateBaseline` / `ApplyUpdates` / `RetryUpdateRecovery`。
- [ ] 5.3 能力矩阵新增一行记录 `install_origin` 远端不支持（fail-closed），
      并在 spec 同步。

验证：`cargo test -p skillport`

## 段 6 — 测试

- [ ] 6.1 AC1：远端 argv 表测试断言含 `--yes`、PIN spec、`-g -y -a -s`；
      断言**不含** `--all`、`*`、`npx.cmd`、`cmd /c`。
- [ ] 6.2 AC2：`&|^%!` 等来源在远端路径同样被拒绝，且在发往远端之前被拒
      （断言远端命令未被构造）。
- [ ] 6.3 AC3：远端 add 超时返回 `skills_cli.timeout`；
      stdout 上限经 `ProcessPolicy::for_tests`（`runner.rs:80-90`，`#[cfg(test)]`）等价手段验证。
- [ ] 6.4 AC4：模拟远端无外网（CLI 非零退出）→ `skills_cli.cli_failed`，
      断言远端 FS 零变更且未写 journal。
- [ ] 6.5 AC5：远端更新检测对 direct_copy 与 conflict 拓扑 fail-closed；
      本地已修改 canonical → `update_local_modified`。
- [ ] 6.6 AC6：**凭据边界**。断言 GitHub token 不出现在
      （a）任何远端命令 argv、（b）远端环境变量、（c）任何远端落盘内容。
      特别加一条负向断言：远端不存在 `curl.conf` 一类携带
      `Authorization` 的文件（防止有人照抄 `github_import/remote.rs:206-212`）。
- [ ] 6.7 AC7：在 `prepared` / `cli_started` / `db_committed` 各注入一次故障，
      断言远端 recovery 可重试并收敛；apply 后 lock 与 placement 一致。
- [ ] 6.8 AC8：远端 placement 的 `install_origin` 为 `None`，不携带猜测标注。
- [ ] 6.9 AC9：远端 apply 过程发出 `skills-cli://update-progress`，前端渲染进度。
- [ ] 6.10 AC10：植入 stderr 与 URL 哨兵 token，断言不出现在 IPC message 与操作日志。

验证：`cargo test -p skillport`

## 段 7 — 收尾

- [ ] 7.1 AC11：i18n en/zh parity 通过。
- [ ] 7.2 确认命令签名未变 → `pnpm docs:gen:check` 无 diff。
      若段 4 迫使 apply 签名变化，则跑 `pnpm docs:gen` 并同步 `ipc_registry` 日志策略。
- [ ] 7.3 spec 同步：远端 install/update 行、`install_origin` 远端行、能力矩阵七行。
- [ ] 7.4 全量：`just ci`。真实 SSH 主机与真实 npm/GitHub 外网行为标记 `UNVERIFIED`。

## 风险文件与回滚点

回滚单元见 `design.md` §6。

| 文件 | 风险 | 回滚单元 |
| --- | --- | --- |
| 远端 argv 构造 | 手工插值会绕过 `shell_quote`，产生远端命令注入面 | A |
| 快照下发路径 | 用错 `ProcessPolicy` 会让大快照静默超时（段 4.3 的显式检查项） | D |
| 凭据处理 | 任何把 token 传给远端的写法都是安全回归，且 `github_import` 里就有一份可被误抄的先例 | D |
| `updates/apply.rs` + `recover.rs` | journal 阶段顺序改动会破坏 recovery 收敛 | D |
| 能力矩阵 + spec | 翻闸早于实现即为 PAC4 违规的矛盾中间态 | E |

## 前置检查

- [ ] `08-27-skills-cli-remote-mutate` 已合入 `dev`。
- [ ] 确认 `08-26-observability-governance-integration` 未在同一工作树改
      `ipc_registry.rs` 或日志策略。
- [ ] 工作树干净。
