# Implementation Plan: SkillPort import deep link

## 1. 前置检查

1. 确认 `07-18-unified-skill-import` 已归档且存在可调用 intent controller。
2. 阅读当前 Tauri 2 官方 deep-link/single-instance 文档，记录所选版本、Windows 行为、许可，以及 single-instance 必须作为 builder 首个插件注册的依据。
3. 用户批准新增生产依赖后才修改 Cargo/package/config。

## 2. 测试与实现顺序

1. 写 Rust parser table tests 和 redaction tests。
2. 实现 typed `ImportIntent`、bounded/deduplicated native queue。
3. 写 frontend controller tests：route/prefill、dirty wizard、pending、duplicate、invalid event。
4. 先把 single-instance 注册为 Tauri builder 的第一个插件并接入 warm-instance argv forwarding，再在其后注册 deep-link plugin 与现有插件；两条入口复用同一 parser/queue。
5. 接入 cold-start argv、frontend-ready handshake、focus/unminimize；验证 ready 前只入队且 ready 后只消费一次。
6. 同步 capability/config/i18n，补普通 GitHub flow 回归测试。
7. 跑 Tauri Windows bundle，安装实际 NSIS，执行 cold/warm 手工验收。
8. 更新 frontend/backend spec，记录 canonical URI 和 intent-only contract。

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

## 4. 风险文件

- `src-tauri/Cargo.toml` / `Cargo.lock`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- app router / root event bootstrap
- unified import intent controller
- i18n 和 frontend tests

## 5. 回滚点

- Commit 1：parser/queue/controller + tests，不注册 scheme。
- Commit 2：plugin/config/single-instance + bundle validation。
- 若 bundle 或 warm-instance 行为不可靠，只回滚 Commit 2，保留纯 intent/controller 供未来使用。
