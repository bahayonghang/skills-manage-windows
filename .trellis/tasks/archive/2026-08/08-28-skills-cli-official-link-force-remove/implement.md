# 实施清单 — 官方相对链接、强制卸载、未钉死 npx skills

批准规划并 `task.py start` 之后按段执行。每段可单独编译测试；不要先改 PIN 再放着分类不管。

## 段 0 — latest CLI 探测（R6 门闩）

- [ ] 用本机 Node 对 npm `skills` latest 采集 `add --help` / 默认 symlink 证据，写入 `research/latest-cli-probe.md`（包版本、是否 `-y` 仍 copy、lock 是否 v3）。
- [ ] 与 R6 冲突则停，升级 PRD，不改 `argv.rs` PIN。

## 段 1 — 词法折叠分类（R1, R8, AC1, AC5）

- [ ] `probe.rs`：`probe_resolves_to_canonical` 在比较前折叠 `.` / `..`。
- [ ] 单测：相对 `../../.agents/skills/ask-matt` → managed；指向其它绝对路径 → conflict。
- [ ] 不改 `paths.rs` `remote_join`。

## 段 2 — 强制卸载 / unlink（R3–R5, AC2–AC4, AC7）

- [ ] `SkillsCliRemovePlan` 保持 conflicts 列表；`remove_global` + unlink IPC 增加 `force: bool`。
- [ ] 本机/远端：force 仅删除符号链接槽位；Dir/File 跳过。
- [ ] `SkillsCliUninstallDialog` + 详情抽屉：强制确认与 i18n；reason 空后缀修复。
- [ ] store 不让组件 `invoke()`。`pnpm docs:gen`。
- [ ] 测试：对话框强制可点；force 不删 DirectCopy；指向 Central 的链接只摘链。

## 段 3 — Linuxbrew PATH 与 npx 布局（R9, AC8）

- [ ] doctor 脚本与 `build_remote_launcher_probe_script` 共用 PATH 前缀。
- [ ] `NPX_JS_POSIX_RELATIVE` 增加 `../lib/node_modules/npm/bin/npx-cli.js`。
- [ ] 单测 stdin 含 `/home/linuxbrew/.linuxbrew/bin` 与该相对路径；断言无 `bash -lc`。

## 段 4 — 取消 PIN 与默认 symlink（R6, R10, AC6, AC9）

- [ ] `SKILLS_CLI_NPM_SPEC = "skills"`；add argv 无 `--copy`。
- [ ] 前端预览、fixture、README、README_CN、CONTEXT.md、`skills-cli-global.md`。
- [ ] 更新既有 argv 断言（`install_update_tests.rs`、`skillsCliInstallViewModel.test.ts` 等）。

## 段 5 — 门闩

- [ ] 定向：`cargo test` skills_cli probe/remove/argv；`pnpm test` UninstallDialog / store / install view model。
- [ ] `just ci`。跳过的外部证据（真 SSH Linuxbrew、真 `npx skills add`）在完成说明里写 UNVERIFIED。

## 风险文件

- `src-tauri/src/services/skills_cli/probe.rs`
- `src-tauri/src/services/skills_cli/argv.rs`
- `src-tauri/src/services/skills_cli/transport.rs`（doctor 脚本）
- `src-tauri/src/services/skills_cli/remote_scripts.rs`
- `src-tauri/src/services/skills_cli/remove.rs` 与 `remove/remote.rs`
- `src-tauri/src/services/skills_cli/link.rs`
- `src-tauri/src/commands/skills_cli.rs`
- `src/components/skillsCli/SkillsCliUninstallDialog.tsx`
- `src/stores/skillsCliStore.ts` / `placementSlice.ts`
- `src/i18n/locales/zh.json` `en.json`
- `README.md` `README_CN.md` `CONTEXT.md` `.trellis/spec/backend/skills-cli-global.md`

## 回滚

段 1 可单独保留。段 4 回滚 PIN。段 2 去掉 `force` 即回到冲突零写入。
