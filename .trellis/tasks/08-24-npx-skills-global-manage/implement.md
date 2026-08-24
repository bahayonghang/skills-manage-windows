# Skills CLI global 实施清单

## 0. Before start

- [ ] 规划摘要已获用户对 **本修订版** 的明确批准。
- [ ] 已读 jsonl 中的 spec，尤其是 `central-mutation-lock.md`、`exclusive-job-lifecycle.md`、`target-context.md`、`process-supervision.md`、`domain-error-enums.md`。
- [ ] 不把 `~/.agents/skills/` 整棵树当 CLI 所有权；不默认 `--all` / `--agent '*'`；不用 `npx.cmd` 当 program。

## 1. Backend domain

- [ ] `services/skills_cli/`：`error.rs`（含 R12 表）、source grammar、argv builder、PIN `skills@1.5.23`、完整 agent map 闭包、lock 所有权、Local node runner。
- [ ] Program = `node.exe`/`node` + npx JS CLI；argv 以 `npx --yes --package=skills@1.5.23 -- skills` 为前缀。
- [ ] `ProcessRequest` + Job Object + Standard/BulkTransfer。
- [ ] `commands/skills_cli.rs`：`resolve_target_context`；非 Local 立即失败。
- [ ] add/remove：exclusive job `skills_cli` → `acquire_target_mutation_guard(Local)` → spawn。
- [ ] leftover **本地** apply 在删除前取同一 target mutation lock（`leftover_cleanup.rs`）。
- [ ] leftover 扫描：仅 Local 用 lock 排除；不按 canonical 根一刀切（R7/AC10）。
- [ ] origin：lock + 解析后的 symlink/junction 目标。
- [ ] `ipc_error.rs` 登记 `skills_cli.*` public message。
- [ ] `ipc_registry`、`IPC_COMMANDS`、operation log redaction、`pnpm docs:gen`。

## 2. Frontend

- [ ] `/skills-cli` + 侧栏；非 Local 隐藏。
- [ ] `skillsCliStore`；组件不 `invoke()`。
- [ ] 安装流：source 白名单 → preview 多选 → 检测平台多选（默认 enabled）→ add。
- [ ] `UnifiedSkillCard` `skillsCli` variant + 负例。
- [ ] 卸载确认。
- [ ] fixture + 中英 i18n。

## 3. Spec and docs

- [ ] `platform-origin-classification.md`：symlink ≠ 一律 Central。
- [ ] `skill-card-scenarios.md`：新 variant。
- [ ] `CONTEXT.md`、README、README_CN。

## 4. Tests

对应 AC：

- [ ] AC4 argv 表：npx `--yes`、PIN、skills `-g -y -a -s`；禁止 `--all`/`*`/`npx.cmd`。
- [ ] AC3/AC5 targets 默认 detected∩enabled∩mapped；空选择拒绝。
- [ ] AC8 doctor：无 node / 过旧 / 无 npx JS。
- [ ] AC2 非 Local IPC 拒绝；远程 leftover 不被本机 lock 排除。
- [ ] AC9/AC10 leftover：lock 命中排除；无 lock 的 Universal 根内副本仍列入。
- [ ] AC11 origin：CLI junction/symlink vs Central symlink vs copy。
- [ ] AC12 取消 add：假 runner 收到 cancel，返回 cancelled。
- [ ] AC13 超时与 stdout cap（policy::for_tests）。
- [ ] AC14 source `&|^%!` 拒绝；stderr 不出现在 IpcError.message。
- [ ] AC15 交叉：持有 CLI add lease+lock 时 install_skill 与 leftover 本地 apply Busy/Timeout。
- [ ] R4 表驱动：每个 seed builtin id 已映射或明确不支持。
- [ ] Vitest：列表、默认勾选、改选 payload、卸载确认、非 Local 隐藏、npx 缺失。
- [ ] 不访问公网；node/npx 用假 runner。

定向验证：

```powershell
pnpm test -- src/test/pages/SkillsCliView.test.tsx
pnpm test -- src/test/stores/skillsCliStore.test.ts
pnpm test -- src/test/lib/platformSkillViewModel.test.ts
cd src-tauri; cargo test skills_cli --locked
cd src-tauri; cargo test central_updates::inventory --locked
cd src-tauri; cargo test installation::install --locked
```

## 5. Gate

```powershell
pnpm typecheck
pnpm lint
pnpm test
cd src-tauri; cargo test --locked
cd src-tauri; cargo clippy -- -D warnings
just ci
```

新增 IPC 后 `pnpm docs:gen`。改 capabilities 才需要 `pnpm tauri build`。

## 6. Risky files

- `leftover_cleanup.rs`：补 Local mutation lock
- `scan.rs`：lock 证据排除，带 ActiveTarget
- `platformSkillViewModel.ts`：origin
- `ipc_error.rs`：新 code
- Local process：禁止 `npx.cmd`

## 7. Rollback

增量页面可关。leftover lock 排除单独保留更安全。mutation lock 补到 leftover 是正确收紧，回滚页面时不必撤回，除非它引起安装超时回归。
