# 设计 — 官方相对链接、强制卸载、未钉死 npx skills

对应 `prd.md` R1–R10。不改全局 `remote_join`。

## 1. 相对链接等价（R1, R8, AC1, AC5）

远端 `readlink` 原文经 `remote_parent(slot)` + `remote_join` 得到未折叠路径。在 **Skills CLI probe 比较层**增加 POSIX 词法折叠（`.` 丢弃、`..` 弹出一段、不越过 `/`），再与 canonical 做现有 `normalize_compare`。

入口：`probe_resolves_to_canonical`（`probe.rs:182`）。单测用 `ask-matt` 工作例（`research/official-relative-symlink-misclassify.md`）。禁止把远端比较改成只跑 `readlink -f`（可把指向别处、经中间链接绕回的目标误判为 managed）。

本机继续 `paths_equivalent`（已 canonicalize）。两侧对「相对链接指向 canonical」应对齐为 `managed_link`。

SkillPort 自建远端 `ln -s` 仍可用绝对 canonical（`remote_scripts.rs:260-263`）。可选后续：改为相对目标以与官方一致；**本任务不要求改创建端**，识别端必须先对。

## 2. 常规卸载（R2, R3, AC2）

分类修复后，相对 Claude 链接进入 `managed_placements`。现有 `execute_remove` / `remove_global_remote` 已删 managed 链接 + canonical + lock。DirectCopy 仍走 retained。不 spawn `skills remove`。

## 3. 强制卸载 / 强制 unlink（R4, R5, AC3, AC4）

### 3.1 IPC

- `skills_cli_remove_global` 增加 `force: bool`（默认 false，specta + `pnpm docs:gen`）。
- `skills_cli_unlink_platform` / batch：冲突且 `force` 时，若槽位为符号链接则走已有 verified link remove 脚本；普通目录仍 `skipped_not_link` / `direct_copy_not_toggleable`。
- preview 保持只读：仍返回 conflicts。前端用 conflicts 决定展示强制按钮，不另开 preview 命令。

常规路径：`conflicts` 非空 → 零写入（现约 `remove.rs:341-342`）。

`force=true`：把 **Conflict 且远端 probe 为 Link**（或本机 `is_symlink`）的槽位当作可摘链接；仍不把 Dir/File 当链接删。然后与常规相同：有 canonical 则备份删除、CAS 删 lock 行。链接目标是 Central 时只 `rm` 槽位链接。

### 3.2 UI

`SkillsCliUninstallDialog`：`impact.conflicts.length > 0` 时主按钮改为「强制卸载」，需勾选或二次确认（i18n：将删规范目录、符号链接、lock；列出保留的 DirectCopy）。无冲突时保持现「卸载」。

详情抽屉冲突行：强制取消链接，走 `force` unlink。

Store：`removeGlobalBatch(names, { force })`。组件不 `invoke()`。

## 4. 未钉死 CLI + 默认 symlink（R6, AC6）

`SKILLS_CLI_NPM_SPEC = "skills"`。`NodeLauncher::npx_argv_prefix` 变为 `--package=skills`。`build_add_global_argv` **不得**追加 `--copy`。保留 `-g -y -a -s`。禁止 `--all` / `--agent '*'`。

实施第一步：对 npm 当前 `skills` latest 探测 add `--help`、默认是否 symlink、lock 是否仍 v3，写入 `research/latest-cli-probe.md`。若 latest 在 `-y` 下仍 copy 且无官方 symlink 旗标：停止改 argv，升级 PRD，不得发明未文档化旗标。

`SKILLS_CLI_MIN_NODE` 保持 22.20.0，除非探测显示 latest `engines.node` 更高。用户登录 Node 为 26.7.0，满足下限。

Doctor 仍只跑 `node --version`（加 R9 PATH），`npmSpec` 显示 `skills`。不恢复 `skills --help` 进 doctor。

前端安装预览：`npx skills add …`（`skillsCliInstallViewModel`）。README / CONTEXT.md / `skills-cli-global.md` PIN 句一并改。

更新 apply 仍 fail-closed 于未验证的 CLI `--force` / copy refresh（现约）。

## 5. 远端 Node / Linuxbrew（R9, AC8）

Doctor 脚本与 launcher probe **同一** PATH 前缀（常量列表，禁止 profile）：

1. `/home/linuxbrew/.linuxbrew/bin`
2. `$HOME/.linuxbrew/bin`（`$HOME` 来自已有 probe 的 `HOME=`）
3. `/opt/homebrew/bin`
4. `/usr/local/bin`
5. 原 `$PATH`

`export PATH="<prefixes>:$PATH"` 后 `command -v node`。

`NPX_JS_POSIX_RELATIVE` 增加 `../lib/node_modules/npm/bin/npx-cli.js`（Homebrew/Linuxbrew：`$PREFIX/bin/node` → `$PREFIX/lib/node_modules/npm/bin/npx-cli.js`）。可选 well-known：`/home/linuxbrew/.linuxbrew/lib/node_modules/npm/bin/npx-cli.js`。

禁止 `bash -lc` / `zsh -lic`。

## 6. 数据流

```
SSH probe readlink → 词法折叠 → 五态 placements
                 ↘ remove plan (conflicts / managed / retained)
UI: confirmable → remove_global
    conflicts → 强制确认 → remove_global(force=true)
Install: PATH 增强 → node + npx-cli.js --yes --package=skills -- skills add … -g -y -a -s
```

## 7. 风险

| 风险 | 处理 |
| --- | --- |
| latest CLI 改 lock / 默认 copy | 探测文档；fail-closed |
| 强制摘错链接 | 仅 `[ -L ]` 槽位；二次确认 |
| SSH PATH 仍找不到 node | doctor `node_missing`；文案保持现码 |
| 词法折叠越过 root | 停在 `/` |

## 8. 回滚

还原 `SKILLS_CLI_NPM_SPEC`、去掉 `force` 参数、去掉 PATH 前缀与相对 npx 候选。相对链接折叠可单独保留（安全修复）。
