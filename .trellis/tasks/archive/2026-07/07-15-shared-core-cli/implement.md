# 实施计划

## 1. Shared Façade

- [x] 确认 identity/lock 子任务已验收，读取其 `SkillRef` 和 guard contract。
- [x] 新增 `cli_api` 模块、`CliContext` 与 CLI-facing DTO/error codes。
- [x] CLI 与 Tauri commands 调用同一 marketplace/GitHub import/installation/central-skills service use case，不复制 commands 编排。
- [x] CLI 使用现有 optional progress seam（`AppHandle = None`），binary 不持有 `AppHandle`。

## 2. Binary And Commands

- [x] 增加 Clap/Tokio features 与 `skillport-cli` binary。
- [x] 实现 global `--json`、`--lang`、versioned envelope、stderr 和 exit-code mapping。
- [x] 实现 `skills list/show` 与 uid/slug/name resolver。
- [x] 实现 skills.sh `search`。
- [x] 实现 skills.sh shorthand/GitHub URL `install`、preview、`--replace`。
- [x] 实现 `install --sync` 与 `skills sync refs|--all`、agent/method/dry-run。
- [x] CLI mutation 写现有脱敏 operation log，并复用 shared lock typed busy/timeout。

## 3. Tests

- [x] Parser/source classification table tests；禁止 path-existence guessing。
- [x] JSON schema、human renderer catalog 和 exit-code tests。
- [x] duplicate/no-replace、explicit replace selection 与 partial Agent result 路径由 CLI/service tests 覆盖。
- [x] 临时 HOME 空 DB smoke 与离线 snapshot exact-shorthand fixture E2E，不访问公网。
- [x] GUI service regression tests，证明 façade 增量没有改变现有 IPC 行为。

定向验证：

```powershell
cd src-tauri; rtk cargo test cli_api
cd src-tauri; rtk cargo test --bin skillport-cli
cd src-tauri; rtk cargo test marketplace
cd src-tauri; rtk cargo test github_import
cd src-tauri; rtk cargo test installation
```

## 4. Scripts And Docs

- [x] 增加 Windows-safe `npm run cli --`、CLI build/install scripts。
- [x] 更新 README/README_CN 与 `--help` examples，明确 Local-only、duplicate safety 和 GUI refresh 语义。
- [x] 记录 `cargo install --path src-tauri --bin skillport-cli --locked --force`。

## 5. Full Gate

```powershell
rtk pnpm typecheck
rtk pnpm lint
rtk pnpm test -- --run
rtk just ci
rtk pnpm tauri build
```

- [x] 验证 `src-tauri/target/release/skillport-cli.exe` 实际生成并能运行 `--version`/`--help`。
- [x] 使用 exact shorthand 完成离线等价 fixture E2E：resolve → install → list/show → duplicate preview → dry-run sync。
- [x] `git diff --check` 通过，无 Git backup/snapshot/merge 命令或文档残留。

## 6. Rollback

- [x] binary、CLI scripts 和 façade 均为增量层，Tauri commands 仍通过原共享 use case 工作。
- [x] CLI mutation 遇到 lock/service failure 停止，不存在 direct-fs fallback。

## 7. Verification Evidence (2026-07-15)

- `rtk just ci`: 123 frontend files / 1346 tests; Rust clippy and 780 tests passed.
- `npm run cli -- --help`, `--version`, `skills --help` and temporary-HOME JSON list passed.
- Offline exact shorthand fixture passed shared candidate resolution, staged import, stable list/show, duplicate preview and dry-run sync.
- Windows `pnpm tauri build` passed with `mainBinaryName=skillport` and `beforeBundleCommand=pnpm build:cli`.
- Final artifacts: `skillport.exe`, `skillport-cli.exe`, and `SkillPort_0.10.13_x64-setup.exe`.
