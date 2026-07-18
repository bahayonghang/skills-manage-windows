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
  -> tauri deep-link plugin callback / argv
  -> parse_import_deep_link(raw)
  -> bounded PendingImportIntent queue
  -> single-instance forwards argv to primary instance
  -> frontend-ready signal
  -> emit skillport://import-intent { source }
  -> focus/unminimize window
```

parser 是纯 Rust 函数，返回 typed `ImportIntent` 或脱敏 error code。native 层不访问 DB/PAT，不调用 GitHub preview/import。

## 3. Frontend flow

在 app shell 建一个轻量 import-intent controller：

1. 监听 typed Tauri event。
2. 若无 active import session，导航 `/central`，调用 unified launcher 的 GitHub prefill intent。
3. 若已有 dirty wizard，显示非破坏性 prompt/toast；保留当前输入，新 intent 进入单槽 pending 状态。
4. wizard 关闭后由用户选择打开 pending intent；不自动替换。

事件 payload 只有规范化 source。组件不直接调用 `invoke()`。

## 4. Queue policy

推荐最多 8 条 FIFO，并按规范化 source 去重。超过上限丢弃最旧且记录 warning；这比 last-only 更适合用户连续打开多个 repo，同时保持内存有界。frontend dirty-session 再使用单槽 pending，避免多 modal 排队。

## 5. 插件与配置

实施阶段核对 Tauri 2 官方版本并最小化变更：

- Rust/plugin dependency 与 `.plugin(...)` 初始化。
- `tauri-plugin-single-instance` 必须是 builder 上第一个 `.plugin(...)` 注册；Tauri 插件按注册顺序运行，Windows 第二实例 argv 依赖这一顺序。随后再注册 deep-link 与当前已有插件。
- scheme/bundle 配置和 capability。
- Windows single-instance forwarding，在 callback 中复用相同 parser。
- single-instance 首位注册解决 warm-instance 捕获顺序；frontend-ready handshake 另行解决事件消费时机，两者不能互相替代。native queue 等待显式 `frontend_ready` command/event 后再 emit。

不要把敏感 URI 写入普通日志。只记录 action、normalized owner/repo（如允许）或 error code。

## 6. Security matrix

| 输入 | 行为 |
| --- | --- |
| valid GitHub HTTPS | 打开预填 UI |
| token/userinfo/credential query | 拒绝并脱敏 |
| file/UNC/http/javascript | 拒绝 |
| overwrite/target/auto 参数 | 拒绝未知参数 |
| encoded control/traversal | 拒绝 |
| app ready 前 | 入有界 native queue |
| wizard dirty | 不覆盖，提示 pending |

## 7. 回滚

移除 scheme/plugin/single-instance registration 和 native listener；frontend unified launcher 与普通 GitHub wizard 完全保留。回滚不需要数据库迁移。
