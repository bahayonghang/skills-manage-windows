# 设计：GitHub 网络边界收敛

## 1. 边界与不变量

本任务只收敛 `services/github_import` 及其 Tauri Markdown 预览入口。核心不变量是：renderer 只能选择已经由 GitHub preview 返回的仓库与仓库内相对路径，不能选择 HTTP scheme、host、port、IP、redirect target 或认证目标。

现有 `GITHUB_MIRROR_ENDPOINTS` 继续作为唯一网络出口清单。API/raw URL 由服务层使用固定 endpoint 构造；PAT 仅在 endpoint label 为 `github` 且 URL 未与 mirror 共用时发送。Marketplace 组件中的浏览器 `fetch()`、AI provider 自定义 endpoint 及全局 NetworkGateway 均不在本任务内。

## 2. IPC 与数据流

本地 preview 流程：

1. `GitHubRepoImportWizard` 从 `GitHubRepoPreview` 取得 `repo` 与当前 `sourcePath`。
2. Zustand store 调用 `fetch_github_skill_markdown({ repo, sourcePath, previewWorkspaceId: null })`，不再发送 `downloadUrl`。
3. command 校验 `sourcePath`，组合为 `<sourcePath>/SKILL.md`，并调用结构化 service helper。
4. service helper 只通过 direct GitHub raw endpoint 与既有 mirror fallback 构造 URL，使用共享 client 与 PAT 规则。
5. body 通过预算化 stream helper 读取，超限立即返回 `GithubImportError::Budget`。

远端 preview 流程保持 `previewWorkspaceId + sourcePath`，直接从已下载的远端 workspace 读取，不进入 HTTP helper。`GitHubSkillPreview.downloadUrl` 继续作为展示/兼容 DTO 字段，但不再是该 IPC 的权威输入。

## 3. URL 构造与校验

- `repo.owner`、`repo.repo`、`repo.branch` 与 `sourcePath` 都视为不可信 IPC 数据。
- `sourcePath` 通过现有 `normalize_repo_path` 校验；最终文件路径由 `join_repo_path(sourcePath, "SKILL.md")` 构造。
- owner/repo/branch 必须是非空单段值，不含 `/`、`\\`、控制字符、`.` 或 `..`；`normalizedUrl` 不参与请求构造。
- endpoint 必须由静态 `GITHUB_MIRROR_ENDPOINTS` 提供。请求发送前做防御性 URL 校验：HTTPS、无 userinfo、无 fragment、默认端口、host 精确匹配 endpoint host。
- client 禁止自动重定向。3xx 不会触发第二个未校验地址请求。

固定 host + HTTPS 证书校验比“先 DNS resolve 检查、后 reqwest 再 resolve”更强地避免 renderer 驱动的 DNS rebinding TOCTOU；测试矩阵验证危险 URL 无法通过结构化构造器表达。

## 4. Client 与流式预算

`github_client()` 继续使用 `OnceLock`，builder 增加：

- `connect_timeout(Duration::from_secs(5))`
- `timeout(Duration::from_secs(30))`
- `redirect(reqwest::redirect::Policy::none())`

测试环境继续 `no_proxy()`，保证本地 fixture 不受系统代理影响。

新增内部 budgeted response reader：

- 有 `content_length` 时先调用现有 `reject_raw_bytes_budget` 快速失败。
- 使用 `bytes_stream()` 逐 chunk 读取。
- 用 `checked_add` 计算累计长度；溢出按 budget 失败处理。
- 每个 chunk 追加前执行对应 metadata/repository-file budget 校验。
- 返回 `Vec<u8>`，不改变上层 UTF-8/parse contract。

该 helper 只替换 raw metadata/repository file 的 `.bytes()` 路径；archive 有独立 archive budget 与解包 contract，不在此子任务中重写。

## 5. 错误契约

- URL/结构化输入错误使用 `GithubImportError` 的语义化变体，不在 service 层返回 `String`。
- 路径错误复用 `UnsupportedRepoPath`；仓库字段错误新增明确的 GitHub source 变体，command 边界再 `.to_string()`。
- HTTP timeout/redirect/status 保持 `Http` 类别；budget 超限保持 `Budget`，便于现有 fallback 逻辑按类型分支。
- 保留既有 UI 可见错误文案，新增错误只覆盖此前可被伪造的输入。

## 6. 兼容性与回滚

这是 Tauri IPC 参数的内部前后端同步变更，不影响持久化 schema、CLI 或公开文件格式。前端 store、command 和测试必须同一提交更新。回滚时可整体恢复旧 IPC 参数，但不得单独恢复任意 URL service fallback。

风险点是禁用 redirect 或 30 秒总超时可能暴露某个 mirror/archive 的隐式依赖。测试需覆盖 direct/mirror fallback；若真实受控 endpoint 必须 redirect，应新增显式静态 endpoint，而不是恢复自动 redirect。

## 7. 受影响文件

- `src-tauri/src/commands/github_import.rs`
- `src-tauri/src/services/github_import/{raw_http.rs,pat.rs,error.rs,tests.rs}`
- `src/stores/marketplaceStore.githubImportSlice.ts`
- `src/stores/marketplaceStore.types.ts`
- 对应 frontend store/component tests（按现有布局定位）
- `.trellis/spec/backend/github-import-preview-contract.md`（完成后记录结构化 Markdown 读取契约）
