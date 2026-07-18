# Tauri 2 Windows deep-link 依赖核查

## 1. 结论快照

核查时间：2026-07-18。来源只使用 Tauri 官方文档、`tauri-apps/plugins-workspace`、`tauri-apps/tauri`、crates.io 与 npm registry 官方元数据。

建议批准以下两个生产 Rust 依赖：

```toml
tauri-plugin-deep-link = "2.4.9"
tauri-plugin-single-instance = "2.4.3"
```

两者均为当前 crates.io 最新非 yanked 稳定版，许可均为 `Apache-2.0 OR MIT`，最低 Rust 为 1.77.2，依赖 Tauri `^2.10`。当前仓库锁定 `tauri 2.11.0`，本机 `rustc 1.97.0`，因此版本区间兼容。`deep-link 2.4.9` tag 为 `5c7668b6bb7c9a509f394d584568b3a922161e50`；`single-instance 2.4.3` tag 为 `cad301fcc1f3ebad1eaef552c886b0bc8580c3fe`。

本任务的 native parser/queue 自己处理 cold argv 与 warm callback，并只向前端发送自定义 typed event，因此**不需要**新增 `@tauri-apps/plugin-deep-link` JavaScript 依赖，也不需要 single-instance JavaScript 包（官方没有发布该 npm 包）。现有 `@tauri-apps/api` 的 event listener 足够。若改成直接从前端调用 `getCurrent()` / `onOpenUrl()`，才需要 `@tauri-apps/plugin-deep-link@2.4.9` 和相应 capability；这不是当前设计。

建议暂不启用 single-instance 的 `deep-link` Cargo feature。该 feature 会在 single-instance callback 之前自动调用 deep-link 插件的 `handle_cli_arguments`，而本任务要求 cold/warm 多入口统一进入自有 parser/queue；不启用可避免一条未经该 parser 的额外 `deep-link://new-url` 事件。single-instance callback 无论是否启用 feature，都会收到第二实例完整 argv 和 cwd。

## 2. 版本、许可与兼容性

| 依赖 | 选定版本 | 官方状态 | 许可 | 平台 / Tauri 兼容性 |
| --- | --- | --- | --- | --- |
| `tauri-plugin-deep-link` | `2.4.9` | crates.io 最新、非 yanked；tag commit `5c7668b...` | `Apache-2.0 OR MIT` | metadata 将 Windows 标为 full support；Tauri `^2.10`；Rust >=1.77.2 |
| `tauri-plugin-single-instance` | `2.4.3` | crates.io 最新、非 yanked；tag commit `cad301f...` | `Apache-2.0 OR MIT` | Windows/Linux/macOS full，mobile none；Tauri `^2.10`；Rust >=1.77.2 |
| `@tauri-apps/plugin-deep-link` | 不新增 | npm 最新为 `2.4.9` | `MIT OR Apache-2.0` | 依赖 `@tauri-apps/api ^2.11.0`；仅直接使用 guest bindings 时需要 |
| `@tauri-apps/plugin-single-instance` | 不存在 / 不新增 | npm registry 返回 404；官方文档关闭 JS links | 不适用 | 插件只有 Rust API，不需要 capability |

准确依据：

- deep-link 版本和 Windows support：[`plugins/deep-link/Cargo.toml` tag `deep-link-v2.4.9`, lines 1-20](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/Cargo.toml#L1-L20)。
- single-instance 版本、平台支持和可选 deep-link feature：[`plugins/single-instance/Cargo.toml` tag `single-instance-v2.4.3`, lines 1-17, 19-46](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/Cargo.toml#L1-L46)。
- 两个 tag 的 workspace 统一声明 Tauri `2.10`、许可、Rust 1.77.2：[`Cargo.toml` at `cad301f`, lines 12-33](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/Cargo.toml#L12-L33)。
- 双许可证 SPDX 声明：deep-link [`LICENSE.spdx`, lines 1-9](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/LICENSE.spdx#L1-L9)，single-instance [`LICENSE.spdx`, lines 1-9](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/LICENSE.spdx#L1-L9)。
- crates.io 固定版本元数据：[`tauri-plugin-deep-link/2.4.9`](https://crates.io/api/v1/crates/tauri-plugin-deep-link/2.4.9) 与 [`tauri-plugin-single-instance/2.4.3`](https://crates.io/api/v1/crates/tauri-plugin-single-instance/2.4.3)。
- deep-link JS 包版本、许可及 `@tauri-apps/api ^2.11.0`：[`package.json` at `5c7668b`, lines 1-31](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/package.json#L1-L31)；npm 固定版本元数据：[`@tauri-apps/plugin-deep-link/2.4.9`](https://registry.npmjs.org/@tauri-apps/plugin-deep-link/2.4.9)。
- single-instance 官方文档只给 Cargo 安装，并明确无 JavaScript API / 无 capability：[`single-instance.mdx` docs commit `b1f7fda`, lines 22-66](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/single-instance.mdx#L22-L66) 与 [lines 201-204](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/single-instance.mdx#L201-L204)。

版本新鲜度风险较低但不能忽略：`single-instance 2.4.3` 发布于 2026-07-13，变更仅修复 macOS 阻塞线程；Windows 路径未变。其 2.4.2 已把可选 deep-link 依赖升级到 2.4.9：[`CHANGELOG.md` at `cad301f`, lines 1-12](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/CHANGELOG.md#L1-L12)。deep-link 2.4.9 的发布变更是 iOS custom scheme 修复，Windows 路径未变：[`CHANGELOG.md` at `5c7668b`, lines 1-6](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/CHANGELOG.md#L1-L6)。仍必须以本任务要求的真实 Windows NSIS cold/warm 验收为准。

## 3. Rust / JavaScript 依赖边界

官方通用 setup 同时展示 Rust core plugin 和 JS guest bindings，因为文档示例从前端调用 `getCurrent` / `onOpenUrl`：[`deep-linking.mdx`, lines 23-78](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L23-L78)。但插件也明确“同时可在 JavaScript 和 Rust 使用”，Rust 可直接调用 `get_current` 和 `on_open_url`：[`deep-linking.mdx`, lines 260-318](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L260-L318)。

本任务应采用：

1. Rust deep-link 插件负责静态 scheme/bundler 集成。
2. Rust single-instance callback 提供 warm argv。
3. 自有 Rust parser/queue 处理 cold `std::env::args()` 和 warm argv。
4. 前端只通过现有 `@tauri-apps/api/event` 监听应用自定义 typed event。

因此 package.json 无新增项。若后续改成直接监听插件 `deep-link://new-url` 或调用 `getCurrent`，必须重新评估：JS 包会增加一条插件原始事件入口，并需要 `deep-link:default`。该默认 permission 只允许 `get_current`：[`deep-link/permissions/default.toml`, lines 1-4](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/permissions/default.toml#L1-L4)；动态 `register` / `unregister` 需要更高权限，本任务不应授予。

## 4. tauri.conf、capability 与 NSIS 注册

### 4.1 必需配置

`src-tauri/tauri.conf.json` 需要增加唯一静态 desktop scheme：

```json
{
  "plugins": {
    "deep-link": {
      "desktop": {
        "schemes": ["skillport"]
      }
    }
  }
}
```

官方 desktop 配置形状见 [`deep-linking.mdx`, lines 200-258](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L200-L258)。`skillport://import?...` 中 `import` 是 URI host/action，不应再注册成第二个 scheme，也不需要配置 path variants。

### 4.2 capability

本方案预期**不改 capability**：single-instance 没有 JS API；deep-link 的 `get_current/register/unregister/is_registered` 都不从 webview 调用；自定义 event listener 只需要 core event permission。当前仓库已有 `core:default`，其生成 manifest 包含 `core:event:default`。官方 event 默认 permission 包含 listen/unlisten/emit/emit-to：[`tauri event permission reference` at `499df79`, lines 1-10](https://github.com/tauri-apps/tauri/blob/499df79be65ef8c0670abc0207cd9e37b55d8491/crates/tauri/permissions/event/autogenerated/reference.md#L1-L10)。

官方 deep-link 文档仅在直接使用 guest API 时示例添加 `core:event:default` 和 `deep-link:default`：[`deep-linking.mdx`, lines 448-465](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L448-L465)。不应为本任务无条件扩大 webview 权限。

### 4.3 Windows / NSIS

无需自定义 NSIS script，也没有单独的 `bundle.windows.nsis.scheme` 配置。deep-link plugin 的静态 desktop schemes 会进入 Tauri bundler 的 `deep_link_protocols`：[`tauri-bundler NSIS mod.rs` at CLI 2.11.2 commit `499df79`, lines 519-527](https://github.com/tauri-apps/tauri/blob/499df79be65ef8c0670abc0207cd9e37b55d8491/crates/tauri-bundler/src/bundle/windows/nsis/mod.rs#L519-L527)。官方 NSIS 模板安装时写：

- `Software\Classes\skillport` 的 `URL Protocol`；
- `DefaultIcon`；
- `shell\open\command = "<installed skillport.exe>" "%1"`。

源码依据：[`installer.nsi`, lines 664-670](https://github.com/tauri-apps/tauri/blob/499df79be65ef8c0670abc0207cd9e37b55d8491/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi#L664-L670)。卸载时只有当 command 仍指向本安装目录才删除 scheme，避免误删被其他程序接管的关联：[`installer.nsi`, lines 799-805](https://github.com/tauri-apps/tauri/blob/499df79be65ef8c0670abc0207cd9e37b55d8491/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi#L799-L805)。

当前仓库 NSIS `installMode` 是 `currentUser`；Tauri 模板在该模式设置 current-user shell/registry context：[`utils.nsh`, lines 1-8](https://github.com/tauri-apps/tauri/blob/499df79be65ef8c0670abc0207cd9e37b55d8491/crates/tauri-bundler/src/bundle/windows/nsis/utils.nsh#L1-L8)。因此验收时应检查 `HKCU\Software\Classes\skillport`，不是假定 HKLM。

官方同时说明：桌面静态 deep link 默认在应用安装时注册，开发态可用 `register_all()` 临时注册；正式验收仍必须安装真实 bundle：[`deep-linking.mdx`, lines 351-369](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L351-L369) 与 [lines 371-423](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L371-L423)。本任务不应在 release build 动态注册 scheme；NSIS 注册是验收对象。

## 5. cold / warm Windows 行为

### 5.1 Cold start

Windows 把已注册 URI 作为唯一 CLI 参数启动新进程。deep-link 插件在 setup 时读取 `std::env::args()`：[`deep-link/src/lib.rs` at `5c7668b`, lines 73-84](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/src/lib.rs#L73-L84)。它只接受“可执行文件名 + 恰好一个参数”，解析 URL 后还要求 scheme 匹配静态配置；命中后更新 current 并 emit：[`deep-link/src/lib.rs`, lines 184-220](https://github.com/tauri-apps/plugins-workspace/blob/5c7668b6bb7c9a509f394d584568b3a922161e50/plugins/deep-link/src/lib.rs#L184-L220)。

该 setup 可能早于前端 listener ready，官方因此要求 app start 用 `getCurrent/get_current`，运行期再监听 open URL：[`deep-linking.mdx`, lines 264-315](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L264-L315)。本任务已有更严格的 native bounded queue/ready handshake，应该直接把 cold argv 送入同一纯 parser/queue，不依赖早期 plugin event 是否被 webview 接住。

### 5.2 Warm start

官方文档明确 Windows/Linux 会启动第二个应用进程并把 deep link 作为 CLI 参数；若要主实例接收，需要 single-instance：[`deep-linking.mdx`, lines 160-188](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L160-L188)。

Windows 实现的实际顺序是：

1. plugin setup 用 bundle identifier 创建 mutex。
2. 第二实例看到 `ERROR_ALREADY_EXISTS`，收集 `current_dir` 与完整 `std::env::args()`。
3. 通过隐藏窗口的 `WM_COPYDATA` 发给主实例。
4. 第二实例执行 cleanup 后 `exit(0)`。
5. 主实例反序列化 cwd/argv 并调用注册 callback。

源码依据：[`single-instance/windows.rs` at `cad301f`, lines 54-117](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/src/platform_impl/windows.rs#L54-L117) 和 [lines 131-167](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/src/platform_impl/windows.rs#L131-L167)。callback 的 `Vec<String>` 含可执行文件名在内的完整 argv；自有入口应只接受该形状中的唯一 URI，并复用 cold parser/queue。

single-instance 默认不会显示或聚焦窗口。官方要求在 callback 中显式 `set_focus()`：[`single-instance.mdx`, lines 102-124](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/single-instance.mdx#L102-L124)。本任务还需按自身窗口状态显式 unminimize/show，再 focus；不能把“只保证单实例”误写成“自动恢复窗口”。

### 5.3 single-instance 必须第一个注册

官方依据有三处，结论一致：

- single-instance 当前文档：必须第一个注册，确保它在其他插件干扰前运行：[`single-instance.mdx`, lines 73-77](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/single-instance.mdx#L73-L77)。
- deep-link desktop 集成示例：single-instance “should always be the first plugin”，随后才注册 deep-link：[`deep-linking.mdx`, lines 165-188](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L165-L188)。
- 插件 tag README：插件按加入 builder 的顺序运行，因此 single-instance 必须 first：[`plugins/single-instance/README.md`, lines 49-60](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/README.md#L49-L60)。

实施时应形成可审查的 builder 顺序：

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(/* argv -> common parser/queue */))
    .plugin(tauri_plugin_deep_link::init())
    // existing sql/fs/dialog/shell/updater/process plugins follow
```

不能把 single-instance 放进 `.setup()` 动态注册；文档的 setup 写法只用于 cfg 示例，且顺序要求仍然成立。对本任务，builder 第一项最清晰、可静态审查，也满足 PRD 门禁。

## 6. 风险与实施约束

| 风险 | 官方事实 | 本任务约束 |
| --- | --- | --- |
| 伪造 CLI deep link | 官方警告用户可手工把 URL 放进 argv，scheme match 不是业务校验 | 所有 cold/warm argv 必须进入 canonical parser；不能把 plugin event 当可信 intent |
| 事件过早 | deep-link setup 在 frontend ready 前即可 emit | native queue 等 ready；ready 后只消费一次 |
| 重复入口 | single-instance `deep-link` feature 会先触发插件，再执行 callback | 不启用该 feature；cold/warm 都只进自有 parser/queue |
| 原始 payload 日志 | plugin debug path 会在 scheme 不匹配时打印 URL；业务层若直接打印 argv 风险更高 | release/业务日志只记脱敏 error code，不打印 URI/argv/source |
| Windows argv transport | `WM_COPYDATA` 实现用 `|` 拼接 cwd 与 args，再 split | canonical URI 不允许裸敏感/异常参数；拆分后不满足唯一 URI 形状即安全拒绝 |
| scheme 只在安装后可靠注册 | 官方桌面文档说明默认安装时注册 | 必须安装实际 NSIS，再验证 HKCU、cold、warm；仅 `tauri dev` 不构成证据 |
| 窗口不会自动聚焦 | single-instance 默认无 focus 动作 | callback 显式 show/unminimize/focus，并证明无残留第二实例 |
| 权限面扩大 | guest bindings 需要 deep-link permission，Rust-only 不需要 | 不新增 JS 包，不授予 register/unregister capability |

伪造 argv 的官方警告与静态 scheme 校验边界：[`deep-linking.mdx`, lines 191-198](https://github.com/tauri-apps/tauri-docs/blob/b1f7fda386732374f0b8e12bdb4af9871fff39fd/src/content/docs/plugin/deep-linking.mdx#L191-L198)。single-instance 可选 feature 在 callback 前调用 deep-link handler 的源码：[`single-instance/src/lib.rs`, lines 41-77](https://github.com/tauri-apps/plugins-workspace/blob/cad301fcc1f3ebad1eaef552c886b0bc8580c3fe/plugins/single-instance/src/lib.rs#L41-L77)。

## 7. 审批后的预期文件改动

依赖审批仅覆盖以下最小变更面；具体产品文件仍以 task start 后 `trellis-before-dev` 与 red tests 为准：

| 文件 | 预期改动 |
| --- | --- |
| `src-tauri/Cargo.toml` | 添加 deep-link 2.4.9、single-instance 2.4.3 Rust 依赖 |
| `src-tauri/Cargo.lock` | 锁定两个插件及 Windows transitive dependencies |
| `src-tauri/src/lib.rs` | single-instance 作为第一个 builder plugin；deep-link 其次；cold/warm 接统一 native parser/queue |
| `src-tauri/tauri.conf.json` | `plugins.deep-link.desktop.schemes = ["skillport"]` |
| `src-tauri/capabilities/default.json` | 预计不改；只有实际选择 guest bindings 时才需重新审批权限面 |
| `package.json` / `pnpm-lock.yaml` | 预计不改；当前架构不需要 JS deep-link bindings |

最终依赖审批建议表述为：批准新增 `tauri-plugin-deep-link 2.4.9` 与 `tauri-plugin-single-instance 2.4.3`（双许可 `Apache-2.0 OR MIT`），不新增 JS 包、不启用 single-instance `deep-link` feature、不扩大 capability；批准后再启动子任务、修改依赖/配置并以真实 Windows NSIS cold/warm 证据收口。
