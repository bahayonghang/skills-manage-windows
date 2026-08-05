# 检查更新失败根因诊断

## 结论

根因不是新 GitHub PAT 无效。根因是共享 GitHub HTTP client 在安全加固后全局禁止重定向，但 Central Update 的仓库快照仍通过 GitHub API archive endpoint 获取 tarball。该 endpoint 正常返回 `302 Found` 并重定向到 `codeload.github.com`；当前实现既不跟随，也没有受信任 redirect handler，因此把正常成功流程当作 `HTTP 302` 错误并终止整个 inventory refresh。

另有一个独立的可观测性缺陷：真实 `HTTP 302` 在 Update Center command、Operation Log、IPC 和前端 runtime log 之间被两次通用化。UI 要求查看 runtime logs，但 runtime 文件并未记录底层 Rust 错误。

## 运行时证据

- `C:\Users\lyh\.skillsmanage\logs\skillport-2026-08-03.log:11` 对应截图时刻 `2026-08-03T06:16:31.687910Z`，只记录前端 `IpcInvokeError` 和通用消息。
- PAT protected fallback 文件在 `2026-08-03 14:16:08 +08:00` 更新；日志显示应用于 `14:16:25` 重新初始化，并在 `14:16:31` 再次失败。因此不是“保存后未重启导致旧内存 token”。
- 通过同一 DPAPI CurrentUser 解密边界加载 PAT，只输出响应状态的 `/rate_limit` 探针返回 HTTP 200，`core.limit=5000`、`core.remaining=4991`。PAT 已成功保存、可读取且可认证。
- 未认证 `/rate_limit` 当时为 `60/60` 已耗尽并返回 403，但这只是环境现象，不是最终根因；带 PAT 的请求可正常认证。
- 对 `https://api.github.com/repos/jakubkrehel/skills/tarball/main` 使用该 PAT 且 `AllowAutoRedirect=false`，实际返回 `302 Found`，`Location` host 为 `codeload.github.com`。该探针与生产 client 的 redirect policy 和 archive URL 一致。
- `skill_update_inventory_runs`、`skill_update_inventory_entries` 和 `skill_update_states` 均为 0 行，说明 refresh 在最终 inventory/state persistence 前 fail-fast。
- 141 个 Central skills 中只有 7 个属于 `github:jakubkrehel-skills-main`，其余 134 个是未知来源。未知来源在 `core/state.rs` 中返回 `Ok(None)`，不会导致批次失败；7 个已关联技能共同触发一次仓库 snapshot 下载。

## 源码因果链

1. `src-tauri/src/commands/skill_update_inventory.rs:114` 从 `state.secrets` 读取 PAT，并在 `:124` 传入 inventory service。
2. `src-tauri/src/services/central_updates/inventory/mod.rs:164` 把 PAT 传给 snapshot preparation。
3. `src-tauri/src/services/central_updates/snapshots.rs:415` 调用 `download_repo_snapshot`。
4. `src-tauri/src/services/github_import/archive.rs:37` 构造 `/repos/{owner}/{repo}/tarball/{branch}` 请求，并把 `Failed to download GitHub repository archive` 作为 request helper 的 failure prefix。
5. `src-tauri/src/services/github_import/pat.rs:41-47` 的共享 client 使用 `redirect::Policy::none()`。
6. `src-tauri/src/services/github_import/raw_http.rs:369-456` 没有 3xx 的受信任处理分支；302 不是 success、404 或 mirror-retry status，最终直接返回 `Failed to download GitHub repository archive: HTTP 302`。因此该响应甚至到不了 `archive.rs:55-70` 的后续分类逻辑。
7. `git blame` 指向提交 `35e0c086`（2026-07-26，`close raw fetch SSRF boundary`）引入全局 no-redirect policy；archive endpoint 自 2026-05-02 起就依赖 GitHub tarball API。安装包构建于 2026-07-29，包含该回归。
8. `.trellis/spec/backend/github-import-preview-contract.md:52-53` 要求共享 client 禁止所有 redirect，同时同一规范继续要求 archive fallback，形成契约冲突。
9. 定向运行 `cargo test --locked github_client_does_not_follow_redirects -- --nocapture` 通过（1 passed，1117 filtered）；该测试只证明 302 不会被跟随，没有覆盖 GitHub tarball 的合法 302 后仍能完成 snapshot。

## 错误为何不可见

- `src-tauri/src/commands/skill_update_inventory.rs:60-63` 让 `UpdateCommandError::Display` 固定输出 `Update Center action failed`。
- `src-tauri/src/operation_log.rs:175-178` 使用 `Display` 写 failure，因此 Operation Log 丢失内部 `HTTP 302`。
- command 在 `src-tauri/src/commands/skill_update_inventory.rs:131-135` 把内部字符串送到 IPC；`src-tauri/src/ipc_error.rs:38-49` 只允许白名单 legacy family，archive 302 不匹配，最终由 `:318-328` 变成 `internal.unexpected`。
- Rust boundary 没有在通用化前 `tracing` 原始的已脱敏错误；文件 runtime log 因此只有前端再次记录的通用 `IpcInvokeError`。

## 已排除与仍需区分

- 已排除：PAT 未保存、PAT 不可读取、PAT 无效、应用未重启、基础 DNS/TLS/仓库不存在、Central 扫描失败、134 个未知来源技能直接报错、SQLite 持久化阶段失败。
- 已证实：archive endpoint 正常 302 与全局 no-redirect policy 冲突；当前分支和已安装 0.10.14 均包含该策略。
- 次要环境因素：未认证 GitHub API 配额当时耗尽；它会让无 PAT 请求返回 403，但有效 PAT 已证明可用，且即使认证成功仍会撞上 302 回归。

## 修复边界

### 最小永久修复

- 保留共享 client 的全局 `Policy::none()`；不要恢复任意自动重定向。
- 仅在 archive acquisition 中显式处理 GitHub 官方 redirect：验证 `Location` 为 HTTPS、443、无 userinfo/fragment，host 精确等于 `codeload.github.com`，路径与结构化 owner/repo/ref 匹配，然后发起第二个有界请求。
- 不把 Bearer header 自动转发到不同 host；如私有仓库依赖签名 Location，使用 GitHub 返回的受信任 URL且不附带原 PAT。
- 保留现有 archive byte/file/expanded/entry budgets 和 mirror auth 隔离。

### 可观测性修复

- 为 archive redirect failure 增加稳定 domain/IPC code 与安全的用户消息，不把 raw URL 或 token 写入 Operation Log。
- 在 `UpdateCommandError` 通用化之前把已脱敏的 domain error 写入 Runtime Log；Operation Log 只保留安全 code、phase、HTTP status 和 repository id。
- 修正 UI 文案或实现，使“See runtime logs”确实能找到同一 operation 的底层错误。

### 回归测试

- production-shaped fixture：direct archive endpoint 返回 302 到受信任 codeload URL，第二跳 200 tar.gz，snapshot 构建成功。
- 拒绝矩阵：HTTP、userinfo、非 443、fragment、lookalike host、loopback/private/link-local、owner/repo/ref/path 不匹配。
- 证明 Bearer 只发往 direct GitHub endpoint，第二跳不含 Authorization。
- Update Center service test：一个 repository-backed skill 能完成 refresh 并持久化 inventory/state；302 不再令整个批次失败。
- observability test：安全 domain code 能进入 IPC/Operation Log，底层已脱敏诊断能进入 Runtime Log。

## 临时处置

重复更新 PAT、重启应用或等待未认证 rate limit 重置都不能解决该回归。修复版本发布前，没有可靠的 Update Center UI 绕过方式；可用 `git ls-remote` 等只读 Git 命令人工确认远端状态，但不要安装 2026-07-26 之前的旧包来规避，因为那会重新打开已修复的 SSRF 边界。
