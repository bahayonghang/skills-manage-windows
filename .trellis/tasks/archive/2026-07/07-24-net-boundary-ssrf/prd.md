# 封堵任意 URL SSRF 与 HTTP client 超时/流式限额

## Goal

消除 GitHub 内容获取链路的 SSRF 面、无超时请求和整体缓冲内存峰值。对应审计 P1-03（🟠 客观缺陷）与 QW-01。

## 核对证据（2026-07-24 dev 分支）

- `src-tauri/src/commands/github_import.rs:91-115`：`fetch_github_skill_markdown(download_url: String)` 接受前端任意字符串 URL。
- `src-tauri/src/services/github_import/raw_http.rs:61-65`：URL 不是 `raw.githubusercontent.com` 形式时**原样请求**，无 scheme/host/IP/port 校验。
- `src-tauri/src/services/github_import/pat.rs:38-48`：`github_client()` 仅设置 user-agent，无 `timeout`/`connect_timeout`/redirect policy（reqwest 默认跟随 10 次 redirect）。
- `src-tauri/src/services/github_import/raw_http.rs:86-94`：仅当响应带 content_length 才预检预算，随后 `.bytes()` 整体缓冲后再复检——chunked/无长度响应可先占满内存。

## Requirements

1. **结构化 IPC**：本地 `fetch_github_skill_markdown` 不再接受 renderer 提供的 `download_url`，改为接受 preview 已返回的 `GitHubRepoRef` 与 `source_path`。后端只用固定的 GitHub/raw mirror endpoint 构造请求；远端 preview workspace 分支继续使用 `workspace_id + source_path`，不发起本地 HTTP 请求。
2. **URL policy**：所有 raw/API 请求只能由固定 `GITHUB_MIRROR_ENDPOINTS` 与经过 `normalize_repo_path` 校验的仓库相对路径构造；拒绝 userinfo、fragment、非标准端口、非 HTTPS scheme 和未列入 allowlist 的 host。renderer 无法选择 scheme、host、port、IP 或 redirect target。
3. **地址边界**：任意 URL、IP literal、混淆 IP 表示和 `github.com.evil.example` 都不能进入 HTTP 请求构造器。固定 HTTPS host 配合证书校验消除 renderer 控制的 DNS/地址输入；不引入先解析后由 reqwest 再解析的 TOCTOU DNS 检查。
4. **redirect policy**：共享 GitHub client 使用 `redirect::Policy::none()`；3xx 响应按 HTTP 失败或既有 mirror fallback 处理，不跟随到未校验地址。
5. **超时**：共享 client 增加 5 秒 `connect_timeout` 与 30 秒总 `timeout`。该总超时覆盖 API、raw、archive、Marketplace 和 Central Update 的单次请求；镜像 fallback 的每次尝试各自受限。
6. **流式限额**：`bytes_stream()` 累积读取，读取前检查 `content_length`，逐 chunk 使用 checked addition 累计，超过对应 `ResourceBudget` 时立即丢弃 response/stream，取代"整体缓冲后检查"。
7. **凭据收敛**：bearer token 仅发送给 GitHub 官方 endpoint（现有 mirror 判断逻辑保留并测试覆盖）。

## Acceptance Criteria

- [ ] 审计 §7.2 SSRF 测试矩阵落地并全部拒绝或在类型边界上不可表达：loopback IPv4/IPv6、RFC1918、link-local、169.254.169.254、redirect-to-private、`file://`/`ftp://`、`github.com.evil.example`
- [ ] 无 content-length 的超大流在 cap 处中断，内存占用有界（测试证明）
- [ ] 所有经 `github_client()` 的请求具备连接与总超时（现有 mirror fallback 行为回归通过）
- [ ] 本地 GitHub wizard 只提交 `repo + sourcePath`；伪造 `downloadUrl` 不再是 IPC contract 的一部分，远端 workspace Markdown 读取保持可用
- [ ] `cd src-tauri && cargo test github_import` 全绿；`just ci` 通过

## 非目标 / 依赖

- 不在本任务内建全局 NetworkGateway（长期 L 项）；AI provider 端点的 SSRF policy 另行评估。
- 无前置依赖，可立即执行。错误处理遵守 `domain-error-enums.md`（新增语义化变体，禁止字符串嗅探）。
- 不在本任务内修改 `GitHubSkillPreview.downloadUrl` 展示字段或 Marketplace renderer 直接 `fetch()` 链路；本任务封堵的是 Tauri backend 的任意 URL fetch 面。
