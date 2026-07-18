# Design: SkillPort import deep link

## 1. 契约

Canonical form：

```text
skillport://import?source=https%3A%2F%2Fgithub.com%2Fowner%2Frepo%2Ftree%2Fmain%2Fskills%2Fdemo
```

`host = import`，query 只有一个 `source`。不要同时支持多种 path/host 变体；一个 canonical contract 更容易验证和长期兼容。

## 2. Native flow

```text
OS scheme activation
  -> deep-link plugin validates configured scheme
  -> cold: get_current() / warm: single-instance callback argv
  -> parse_import_deep_link(raw)
  -> bounded PendingImportIntent queue
  -> frontend-ready signal
  -> emit skillport://import-intent { source }
  -> warm callback show/unminimize/focus main window
```

builder 顺序固定为：single-instance 第一、deep-link 第二、sql/fs/dialog/shell/process/updater 在后。bounded queue state 在 plugin setup 前由 builder manage，确保主实例初始化期间到达的 warm callback 也能入队。Windows 第二实例把完整 argv/cwd 送给主实例；callback 从“binary + 唯一 URI”形状提取 URI、调用 parser/queue，再恢复/聚焦窗口。cold `get_current()` 也调用同一 parser/queue。

不启用 single-instance 的可选 `deep-link` feature。该 feature 会在用户 callback 前调用 deep-link `handle_cli_arguments` 并 emit plugin event；本任务不让 frontend 或第二个 native listener消费该原始 event，以免同一 warm argv 形成两条业务入口。所有业务 event 只能由自有 queue 发出。

parser 是纯 Rust 函数，返回 typed `ImportIntent` 或稳定、脱敏的 error code。它先验证 outer URI 与 source 凭据/参数边界，再调用现有 GitHub source parser/normalizer；需要把该 pure helper 收窄地提升到 `pub(crate)`，不复制 branch/subpath 逻辑。native 层不访问 DB/PAT，不调用 GitHub preview/import。

## 3. Frontend flow

新增全局 typed import-intent store/controller，并让 Central/Marketplace 的 GitHub launcher 状态都接入它：

1. 监听 typed Tauri event。
2. 防御性校验 `{ source: string }` 后导航 `/central`，调用 `openImportIntent({ kind: "github", source })`。
3. 若无 dirty import session，设置 source 并打开现有 GitHub wizard；launcher 自身仍不调用 IPC。
4. 若已有 dirty wizard，显示非破坏性 pending prompt；保留当前输入，新 intent 进入最多 8 条的 FIFO。
5. wizard 关闭后由用户选择消费 FIFO 首项或丢弃；不自动替换、Preview 或 Confirm。

dirty 至少包含：非空 source、已有/正在进行 preview、preview error、confirm/import/result 状态。这样 Central 与 Marketplace 路由切换时不会因为 URL/open flag 是页面局部 state 而丢失保护。controller 同时对 active/pending source 去重，并拒绝非字符串、空值、控制字符或非规范 HTTPS GitHub event payload。

事件 payload 只有规范化 source。组件不直接调用 `invoke()`。

## 4. Queue policy

native 最多 8 条 FIFO，并按规范化 source 去重。超过上限丢弃最旧且只记录 error code/queue length 的 warning；ready command 是幂等 transition，首次 transition 后按 FIFO emit，每项仅一次。

frontend 也使用最多 8 条 pending FIFO，而不是原设计的单槽。原因是 cold queue ready 后可能连续 emit 多项；单槽会覆盖中间 intent，违反 FIFO AC。UI 始终只处理一个 wizard，不形成多个 modal；关闭后逐项 consume/discard。

## 5. 插件与配置

已核对 2026-07-18 Tauri 官方发布与源码，实施采用：

- `tauri-plugin-deep-link = "2.4.9"`，`Apache-2.0 OR MIT`，Windows full support，Rust 1.77.2 / Tauri 2.10+。
- `tauri-plugin-single-instance = "2.4.3"`，`Apache-2.0 OR MIT`，Windows full support，Rust 1.77.2 / Tauri 2.10+；不启用可选 `deep-link` feature。
- 仓库当前 Tauri 2.11.0、Rust 1.97.0，兼容。
- 不新增 `@tauri-apps/plugin-deep-link`：native Rust API 已覆盖 cold/warm，frontend 复用现有 `@tauri-apps/api` custom event。single-instance 官方没有 JavaScript API。
- `tauri.conf.json` 添加 `plugins.deep-link.desktop.schemes = ["skillport"]`。Tauri CLI/bundler 自动把它写入 NSIS 注册/卸载脚本，不需要自定义 NSIS template。
- 不新增 `deep-link:default` capability，因为 frontend 不调用 deep-link guest commands；现有 `core:default` 覆盖 event listen。若实现改为 guest API，必须回到审批/规划门重新说明 npm 与 capability 增量。
- single-instance 首位注册解决 warm-instance 捕获顺序；frontend-ready handshake 另行解决事件消费时机，两者不能互相替代。native queue 等待显式 `frontend_ready` command/event 后再 emit。

不要把敏感 URI 写入普通日志。只记录 action、normalized owner/repo（如允许）或 error code。

## 6. Security matrix

| 输入 | 行为 |
| --- | --- |
| valid GitHub HTTPS | 打开预填 UI |
| token/userinfo/credential query | 拒绝并脱敏 |
| file/UNC/http/javascript | 拒绝 |
| token/auth/overwrite/target/auto 或任意未知参数 | 拒绝且不回显 payload |
| source 自带 query/fragment、端口或 userinfo | 拒绝 |
| encoded control/traversal | 拒绝 |
| app ready 前 | 入有界 native queue |
| wizard dirty | 不覆盖，提示 pending |

## 7. 回滚

移除 scheme/plugin/single-instance registration、native listener/queue 与 config；保留已抽取的普通 UI typed launcher 时，Central/Marketplace GitHub wizard 仍可通过手工入口运行。NSIS 卸载脚本只在 registry command 仍指向本安装目录时删除 scheme。回滚不需要数据库迁移。
