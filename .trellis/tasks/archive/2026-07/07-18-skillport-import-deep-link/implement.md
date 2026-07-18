# Implementation Plan: SkillPort import deep link

## 1. 前置检查

1. `07-18-unified-skill-import` 已归档，但当前只有 `SkillImportLauncher.onOpenIntent` 回调，没有可调用 controller；本任务先以测试补齐 `openImportIntent({ kind: "github", source? })`，不修改已归档工件。
2. 官方依赖研究记录在 `research/tauri-deep-link-dependencies.md`：选择 Rust `tauri-plugin-deep-link 2.4.9` 与 Rust `tauri-plugin-single-instance 2.4.3`，均 `Apache-2.0 OR MIT`；不启用可选 `deep-link` feature，不新增 JavaScript 依赖或 deep-link capability。
3. 用户批准上述两个 Rust 生产依赖后，仅运行一次 `python ./.trellis/scripts/task.py start 07-18-skillport-import-deep-link`。启动前后都不得启动父任务。
4. task status 变为 `in_progress` 后，写代码前使用 `trellis-before-dev` 读取 frontend/backend/Tauri 相关 spec。

## 2. 测试与实现顺序

1. 先写 Rust parser table tests：valid repo/branch/subpath，以及 unknown action/parameter、duplicate/missing source、HTTP/non-GitHub/file/UNC、userinfo/port、source query/fragment、token/auth、control/backslash、encoded traversal、4096-byte boundary；另写 error/log redaction tests。
2. 实现 pure `ImportIntent` parser，并把现有 GitHub parser/normalizer 的必要 pure surface 收窄提升到 `pub(crate)`；禁止网络请求、preview/import command 和规则复制。
3. 先写 native queue tests：FIFO、normalized dedupe、capacity 8/drop-oldest、ready 前无 emit、ready 幂等/只 flush 一次；再实现 shared state、`frontend_ready` command 与 custom typed event。
4. 先写 `src/test/ImportIntentController.test.tsx`：route/prefill、Central/Marketplace dirty wizard、pending FIFO consume/discard、duplicate、invalid event、zero preview/import IPC；再实现全局 store/controller 和 `openImportIntent({ kind: "github", source? })`。
5. 把 Central/Marketplace 的 GitHub open/source state 接入统一 controller；普通 launcher、repository sync 和 Marketplace CTA 复用同一 action。wizard 关闭时保留明确的 pending consume/discard UX，同步中英文 i18n。
6. 获批后修改 Cargo：single-instance 为 builder 首个 plugin，deep-link 为第二个，现有插件依次在后；queue state 在 plugin setup 前 manage。cold `get_current` 与 warm callback argv 复用 parser/queue；callback 随后 show/unminimize/focus 主窗口，不消费或转发 deep-link plugin 原始 event。
7. `tauri.conf.json` 添加唯一 desktop scheme `skillport`。frontend 不用 guest API，因此不改 `package.json`/`pnpm-lock.yaml`，也不增加 `deep-link:default`；检查生成 schema/capability 仍覆盖 custom event listener。
8. 补普通 Central/Marketplace GitHub flow、SSH/WSL target 回归测试，更新 frontend/backend spec，记录 canonical URI、intent-only、queue、dirty/pending 与日志脱敏契约。
9. 跑真实 Windows bundle，安装最新 NSIS；读取 `HKCU:\Software\Classes\skillport` 与 `...\shell\open\command`，再完成 cold/warm 手工验收并把命令、产物路径、脱敏日志、进程/窗口观察和截图保存到本任务 `research/`。

## 3. 定向验证

```powershell
pnpm vitest run src/test/ImportIntentController.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx
pnpm typecheck
pnpm lint
cd src-tauri; cargo test deep_link
cd src-tauri; cargo clippy -- -D warnings
git diff --check
just ci
pnpm tauri build
```

安装后手工验证示例：

```powershell
Start-Process 'skillport://import?source=https%3A%2F%2Fgithub.com%2Fowner%2Frepo'
```

分别在应用未运行和已运行时执行，确认只预填且未自动发起 import；warm 路径同时核对主实例收到 argv，且 single-instance 在 builder 中保持首个插件注册。

额外记录：

```powershell
Get-ItemProperty 'HKCU:\Software\Classes\skillport'
Get-ItemProperty 'HKCU:\Software\Classes\skillport\shell\open\command'
Get-Process skillport
```

cold 前确认无 `skillport` 进程；warm 前后记录 PID 集合，证明没有残留第二实例。截图必须同时显示已预填 URL 与仍停留在 input step；只有用户主动点击 Preview 后才允许出现 `preview_github_repo_import` IPC。

## 4. 风险文件

- `src-tauri/Cargo.toml` / `Cargo.lock`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/src/services/deep_link*` / command registration
- app shell/root event bootstrap 与 typed IPC map
- unified import intent store/controller
- Central/Marketplace GitHub wizard state bindings
- i18n 和 frontend tests

## 5. 回滚点

- Commit 1：Rust parser/queue/lifecycle、approved dependencies/config 与 tests。
- Commit 2：frontend controller、Central/Marketplace bindings、pending UX/i18n 与 tests。
- Commit 3：frontend/backend spec 与真实 Windows/NSIS evidence。
- Trellis archive 与 journal 按仓库既有生命周期单独提交。若 bundle 或 warm-instance 行为不可靠，不归档；回滚 native scheme/plugin commit 时保留普通 UI controller 仅需先证明 Central/Marketplace 回归仍通过。
