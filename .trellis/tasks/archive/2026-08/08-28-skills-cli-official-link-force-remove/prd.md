# Skills CLI 识别官方相对链接并支持强制卸载

## Goal

远端 SSH 上由官方 `npx skills` 以 symlink 装到 Claude Code 的全局技能，必须被识别为受管链接。用户能卸载：删除 `~/.agents/skills/<name>`、删除指向它的平台符号链接、删除 lock 行。对仍无法按受管链接处理的槽位提供强制卸载/取消链接。同一任务内取消 `skills@1.5.23` PIN，改用未钉死的 `npx skills`，默认 symlink 安装。远端 doctor/launcher 必须能解析到用户登录壳里的 Node（本机证据：Linuxbrew `v26.7.0`），而不是 SSH 非交互 PATH 上可能存在的另一个 `node`。

## Background

卸载 `ask-matt`：1 个规范文件夹、0 个受管链接、Claude Code `wrong_link_target`、Uninstall 禁用。`~/.claude/skills/<name>` 的 `readlink` 为 `../../.agents/skills/<name>`，即 Universal 规范目录。其它平台多为 DirectCopy。分类失败原因见 `research/official-relative-symlink-misclassify.md`。

用户登录壳：`/home/linuxbrew/.linuxbrew/bin/node` → **v26.7.0**。先前「Node 20」来自桌面截图 OCR，不是在该主机上执行的 `node --version`，作废。SkillPort 远端 doctor 使用 SSH 脚本里的 `command -v node`（`transport.rs` `DOCTOR_PROBE_SCRIPT`），不加载 Linuxbrew。`npx-cli.js` 候选也不含 Linuxbrew 的 `../lib/node_modules/npm/bin/npx-cli.js`。见 `research/remote-node-linuxbrew-path.md`。

## Decisions

- 相对 `readlink` 经 POSIX 词法折叠 `.` / `..` 后与 canonical 相等 → `managed_link`。不在本任务改全局 `remote_join`。
- 常规卸载：canonical + 受管链接 + lock。DirectCopy 普通目录保留。
- 强制卸载/强制取消链接：只对符号链接执行 `rm -f`（Unix）/ junction `rmdir`（Windows 远端既有脚本）；禁止删除普通目录；禁止跟随链接删除 Central。
- 取消 PIN：`--package=skills`（npm 默认 dist-tag，不写 `@1.5.23`）。启动器仍为 `node` + `npx-cli.js`。安装 argv **不得**带 `--copy`；`-y` 只跳过确认。官方默认 symlink。
- 「npx skills + operator」= SkillPort 作为官方 CLI 操作层，不是另装 operator 包，也不是 `bash -lc` 用户 profile。
- 远端 Node：在 SSH 脚本 PATH 前插入已评审的 brew 前缀（含 Linuxbrew 与 Homebrew），禁止无界 login shell。npx 相对候选增加 `$PREFIX/bin` → `../lib/node_modules/npm/bin/npx-cli.js`。
- 卸载仍由 SkillPort 域实现，不 spawn `skills remove`（官方 `--force` 未纳入 argv）。安装/预览 spawn 未钉死的 `skills`。
- 若 latest lock 不是 v3 或 add 默认变成 copy：fail-closed，写入 research，不得靠猜测补旗标。

## Requirements

- **R1** 远端与本机：符号链接在词法折叠 `readlink` 相对 slot 解析后等于 canonical 则为 `managed_link`，不得标 `wrong_link_target`。
- **R2** 卸载预览：R1 链接计入 managed，不进 conflicts；无其它真实冲突时 `confirmable`。
- **R3** 常规卸载：删 lock 拥有的 canonical、已验证受管链接（含相对链接）、lock 行。DirectCopy 保留。
- **R4** 强制路径只删符号链接 + canonical + lock。指向 Central 的链接只摘链。普通目录 skipped，不 `rm -rf`。
- **R5** 存在冲突时卸载对话框提供强制确认（二次文案列出将删与将保留）。详情抽屉对冲突链接提供强制取消链接。不得只灰掉按钮。
- **R6** `SKILLS_CLI_NPM_SPEC` 改为 `skills`。doctor / 安装 argv / README / i18n 预览命令不再出现 `skills@1.5.23`。禁止 `npx.cmd`。默认 symlink，argv 不含 `--copy`。所有权仍为 lock v3 名字。
- **R7** 文案走 `src/i18n/`。reason code 对用户可理解。Remote 不可 Reveal。
- **R8** 本机绝对链接 / junction 行为不变。折叠后不相等仍为 `wrong_link_target`。断链 `broken_link`。文件槽 `not_a_directory`。
- **R9** 远端 doctor 与 launcher probe 使用同一套 PATH 增强：至少 `/home/linuxbrew/.linuxbrew/bin`、`$HOME/.linuxbrew/bin`、`/opt/homebrew/bin`、`/usr/local/bin`，再接 SSH 原 PATH。`command -v node` 必须能解析到 Linuxbrew Node（用户证据路径）。npx 候选包含 Homebrew/Linuxbrew 的 `../lib/node_modules/npm/bin/npx-cli.js`。不使用 `bash -lc` / `zsh -lic` 作为解析器。
- **R10** IPC 变更后 `pnpm docs:gen`。用户可见 README / README_CN 与 PIN 文案对齐。

## Acceptance Criteria

- **AC1** [R1] 远端 Claude Code `readlink` = `../../.agents/skills/ask-matt` 且 canonical = `$HOME/.agents/skills/ask-matt` → 受管链接，非冲突。
- **AC2** [R2][R3] 仅相对链接 + DirectCopy、无其它冲突：预览至少 1 managed link、冲突空、Uninstall 可确认；确认后 canonical 与该链接消失，lock 无名，DirectCopy 仍在。
- **AC3** [R4][R5] 目标折叠后仍非 canonical 的冲突：常规卸载仍阻止；强制卸载删 canonical + lock，只摘该符号链接，不删链接目标树、不删 DirectCopy。
- **AC4** [R4] 强制取消链接只作用于符号链接；普通目录不可当链接删除。
- **AC5** [R8] 测试：折叠后相等 → managed；不相等 → conflict。远端 probe 不以 `readlink -f` 作为唯一比较。
- **AC6** [R6] doctor.npmSpec 与 add argv 为 `--package=skills`，无 `1.5.23`；add argv 无 `--copy`，仍有 `-g`、`-y`、至少一个 `-a` 与 `-s`。安装预览字符串为 `npx skills add …`。
- **AC7** [R7] 中英强制/冲突文案经 i18n；不渲染空的 `wrong_link_target:`。
- **AC8** [R9] 远端 doctor/launcher 脚本在 `command -v node` 前导出增强 PATH；单测 stdin 含 Linuxbrew bin 与 `../lib/node_modules/npm/bin/npx-cli.js`。不出现 `bash -lc`。
- **AC9** [R10] 新增/变更 IPC 后生成文档纳入提交；README 不再写死 `skills@1.5.23`。

## Out of scope

- 一键 `rm -rf` DirectCopy 或整棵 `~/.agents/skills/`。
- 删除 Central 技能树。
- 项目级（非 `-g`）安装。
- Reveal 远端文件夹。
- 默认 `--all` / `--agent '*'`。
- 修改全局 `remote_join`。
- 无界 login shell 解析 Node。
- 捆绑 Node。
- 凭据进 SQLite / 日志。
- Windows x64 安装包（UNVERIFIED，除非本任务明确纳入）。

## Notes

- 复杂任务。规划摘要批准后才可 `task.py start`。
- 实施中必须对当前 npm `skills` latest 做一次 argv/lock/默认 symlink 探测，写入 `research/latest-cli-probe.md`；与 R6 冲突则停写产品代码并升级 PRD。
