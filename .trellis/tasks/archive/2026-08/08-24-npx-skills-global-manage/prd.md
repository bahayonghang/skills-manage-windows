# Manage npx skills global installs

## Goal

SkillPort 增加独立页面，作为官方 Skills CLI（npm 包 `skills`）的**全局（`-g`）**管理界面：列出已装技能、从界面执行 `npx skills add` 安装、执行完整卸载。

用户价值：用桌面完成现在只能在终端做的全局 Skills CLI 生命周期，并避免 Update Center leftover 清理删掉 CLI 真实源。

## Background

`npx skills` 来自 npm 包 `skills`（https://github.com/vercel-labs/skills）。全局安装把真实文件写到 `~/.agents/skills/<name>/`，lock 在 `~/.agents/.skill-lock.json`（或 `$XDG_STATE_HOME/skills/.skill-lock.json`）。这与 SkillPort Central（`~/.skillsmanage/skills/`）不是同一套真实源。十个 Universal Agents 的平台目录也是 `~/.agents/skills/`（`src-tauri/src/db/types.rs:70-81`），因此该目录里同时可能有 CLI 技能、SkillPort 安装和手工副本。

SkillPort 用 `is_detected`（`global_skills_dir` 或其父目录存在）判断平台已安装，用 `is_enabled` 判断用户是否启用（`src-tauri/src/commands/agents/mod.rs:127-137,175-184`）。`detect_agents` 按活动目标分流 Local/SSH/WSL（`agents/mod.rs:395-399`）。

非交互命令见 `research/npx-skills-global-feasibility.md` 第 6 节。Codex 审阅处置见 `research/codex-review-disposition.md`。

## Decisions

- Skills CLI 拥有 lock 中记录的全局技能。不把 `~/.agents/skills/` 整棵树收编为 Central，也不仅凭「位于该目录」判定 CLI 所有权。
- 列表、预览、安装、卸载 spawn 官方 CLI；不在 Rust 里重写 add/remove/lock 写入。
- MVP 只管理全局 `-g`，且只操作 **Local target**。
- 新建独立页面 + 侧栏入口。远程目标下隐藏入口；IPC 拒绝非 Local。
- 卸载对齐 `skills remove --global <name> -y`（canonical + 各 agent 链接 + lock）。带确认。不碰 Central 同名技能。
- 安装目标：已检测且可映射的 SkillPort 平台；默认勾选其中已启用项；用户可改技能多选和平台多选。
- npx 层与 skills 层旗标分离：`npx --yes --package=skills@1.5.23 -- skills …`，add/remove 另加 skills `-y`。
- 平台目录写入与 Central install/uninstall/leftover apply 共享 Local `acquire_target_mutation_guard`。exclusive job family 只用于 npx 取消与进度，不充当文件系统互斥。
- 不把 CLI 技能导入 Central。不 spawn `find` / `use` / `init` / `update`。不默认 `--all` 或 `--agent '*'`。

## Requirements

- **R1** 页面：新路由与侧栏入口。列出全局 Skills CLI 技能：名称、canonical 路径、lock source、CLI 报告的 agents。卡片走 `UnifiedSkillCard` 新场景。文案走 `src/i18n/`。组件不直接 `invoke()`。
- **R2** 列表：刷新走冻结版本的 `skills ls -g --json`。没有 Node/npx、Node 低于 22.20.0、或 PIN 包无法执行时，页面展示可操作错误；安装/卸载不可用；不回退到 Central 安装器。
- **R3** 安装预览：用户输入通过白名单的 source（`owner/repo`、`owner/repo@skill`、GitHub/GitLab HTTPS、`git@github.com:owner/repo.git`）。先执行 `skills add <source> --list`，展示可装技能。用户多选技能（默认可全选）。`owner/repo@skill` 可预勾该 skill。`--list` 解析失败返回 typed 错误，不把原始 stderr 送进 UI。
- **R4** 安装目标：打开安装流时做 **Local** 平台检测。候选 = `is_detected` 且映射表为「已映射」。默认勾选候选中 `is_enabled`。用户可改选；不能选未检测、无映射或不支持的平台；不能发 `--agent '*'`。技能或平台空选择禁止安装。每个 SkillPort builtin id 必须在映射表里是「已映射」或「明确不支持」。
- **R5** 安装执行：spawn 冻结版本 CLI：npx 层 `--yes --package=skills@1.5.23`，skills 层 `add <source> -s … -g -a … -y`。禁止省略 npx `--yes`、skills `-g`/`-y`/`-a`。禁止默认 `--all`。
- **R6** 卸载：确认后 spawn `skills remove --global <name> -y`（同一 npx 前缀）。确认文案写明删除 canonical、平台链接和 lock 条目。不做 `remove --all`。
- **R7** leftover 保护（Local only）：Update Center leftover 扫描与一键清理不得删除 **lock 证明** 的 Skills CLI global 技能，也不得删除指向其 canonical 的 symlink/junction。无 npx 时仍有效。不得仅因路径位于 `~/.agents/skills/` 就排除整棵 Universal 根，以免挡住合法 leftover。
- **R8** 来源分类：平台视图不得把「symlink/junction 目标是 lock 拥有的 CLI canonical」标成 SkillPort Central 安装。
- **R9** 进程：本机执行必须有超时、取消、输出上限、Windows Job Object。Windows 可执行文件是 `node.exe`，不是 `npx.cmd`。argv 分槽。source 拒绝 `& | ^ % ! < > " \n \r`。stdout/stderr/URL 不得进入 IPC message 或未脱敏日志。
- **R10** 本地 target：`skills_cli_*` IPC 在 ActiveTarget 不是 Local 时返回固定错误，不 spawn、不读本机 lock 去保护远程 leftover。
- **R11** 目标级互斥：CLI add/remove 在 spawn 会写平台目录的命令前获取 Local `acquire_target_mutation_guard`，并保持到进程结束。leftover **本地** 删除同样获取该 lock。锁顺序：exclusive job lease → target mutation guard。不得依赖独立 job family 来互斥文件系统。
- **R12** 错误信封：每个 `SkillsCliError` 变体映射固定 `skills_cli.*` code、审阅过的 public message、`retryable`。登记 `ipc_error.rs` allowlist。未知 Display 不得把 stderr 带过 IPC。

## Acceptance Criteria

- **AC1** [R1] 侧栏在 Local target 能进入 `/skills-cli`；页面列出冻结版本 `ls -g --json` 的全局技能。
- **AC2** [R1][R10] 活动目标为 SSH/WSL 时侧栏不进入该页；对应 IPC 返回 `skills_cli.local_target_only`，不 spawn。
- **AC3** [R3][R4][R5] 安装流展示已检测且已映射平台；默认勾选其中已启用项；用户改技能集合和平台集合后执行 add。
- **AC4** [R4][R5] 发出的 add argv 含 npx `--yes`、`--package=skills@1.5.23`、skills `-g`、`-y`、至少一个 `-a` 与 `-s`；安装后列表刷新可见新技能。
- **AC5** [R4][R5] 未选技能或未选平台时不 spawn add。
- **AC6** [R6] 卸载确认后执行 `remove --global <name> -y`；之后 ls 不再包含该技能，canonical 与 lock 条目消失。
- **AC7** [R6] 未确认卸载时不 spawn remove。
- **AC8** [R2] 没有 Node、Node 过旧、或 npx/PIN 包不可用时，页面显示可操作错误，不调用 Central 安装器。
- **AC9** [R7] leftover 一键清理不删除 lock 中的 Skills CLI global 技能及其 agent 链接；npx 不可用时该保护仍成立。
- **AC10** [R7] `~/.agents/skills/` 下 **无 lock 条目** 的可写 leftover 仍可出现在清理列表（不因位于 canonical 根而被全部屏蔽）。
- **AC11** [R8] 平台视图把指向 lock 拥有 CLI canonical 的 symlink/junction 标成 Skills CLI 来源，不标成 SkillPort 中央安装。
- **AC12** [R9] 取消正在进行的 add/remove：lease 取消，子进程树在 Job Object 策略下终止，返回 `operation.cancelled` 或 `skills_cli.cancelled`。
- **AC13** [R9] preview/list 超过 Standard 时限或 add/remove 超过 BulkTransfer 时限时终止进程并返回超时错误；超 cap 的 stdout/stderr 不进入 buffer。
- **AC14** [R9][R12] IPC 错误 message 与 Operation Log 不含 raw stdout/stderr、token、完整命令行 URL。source 含 `&|^%!` 等字符时拒绝 spawn。
- **AC15** [R11] CLI add/remove 与 Central install/uninstall、leftover 本地 apply 争用同一 Local target mutation lock：一方持有时另一方得到 Busy/Timeout，不并行写同一 agent 路径。测试覆盖这三组交叉。
- **AC16** [R1][R12] 文案中英走 i18n；状态走 Zustand；IPC 走 `@/lib/ipc`。`just ci` 通过。新增 IPC 后 `pnpm docs:gen` 纳入提交。

## Out Of Scope

- 项目级 `npx skills add`（无 `-g`）。
- `find` / `use` / `init` / `update` / `experimental_*`。
- 导入或收编进 Central；Central 同名合并或覆盖。
- 在 SSH/WSL 上执行 `npx skills`。
- 在 Rust 中重实现 add/remove/lock 写入。
- 捆绑 Node 进安装包。
- 一键 `remove --all` 或默认 `add --all` / `--agent '*'`。
- 官方 CLI 73+ agent 全量选择器。
- 自动升级 `skills` npm PIN。

## Notes

- 复杂任务。规划摘要再次批准后才可 `task.py start`。
- 领域词：**Skills CLI global**。实施时写入 `CONTEXT.md`。
- PIN：`skills@1.5.23`。映射表与不支持名单在 design.md。
- leftover 本地 apply 补 target mutation lock 是本任务范围，因为否则 R11 无法成立。
