# 实施与验证结论

## 已实现的不变量

- Refresh 对每个 GitHub repository 先解析完整 commit SHA，再下载 pinned snapshot；cache entry 同时持有 commit、repository digest 与 bytes。
- 新 migration 6 为 pending additions 追加 nullable `resolved_commit_sha` / `snapshot_digest`，旧 row 保持 `NULL` 并在 Apply 时要求 Refresh。
- Apply 先按 repository 合并 selections，并只信任 selected pending rows 中唯一、格式有效的 immutable identity。
- Local exact cache hit复用原 bytes且不触网；miss 只使用持久化 full SHA，摘要不一致在 mutation 前返回 `central_updates.snapshot_changed`。
- SSH / WSL workspace 使用 full SHA，校验完整 remote repository manifest，并持久化同算法的 per-candidate content digest。
- GitHub 401/403 保留 `used_auth`：匿名拒绝为 `github_import.access_denied`，已认证拒绝为 `github_import.configured_token_failed`；限流仍为 `github_import.rate_limited`。
- Apply failure 只序列化安全 identifier、固定 code/category/phase 与固定 public message；动态 HTTP/path/token detail 不进入 UI 或日志载荷。

## 验证证据

- `cargo check --manifest-path src-tauri/Cargo.toml --locked --tests`：通过，0 warnings。
- `cargo test ... db::migrations::tests`：9 passed。
- `cargo test ... services::central_updates::inventory::tests::apply_`：16 passed。
- `cargo test ... refresh_writes_pending_additions_for_remote_added`：1 passed。
- `cargo test ... services::central_updates::snapshots::tests`：13 passed。
- `cargo test ... services::github_import::error::`：3 passed。
- `cargo test ... remote_inventory_digest_matches_the_local_snapshot_digest`：1 passed。
- `pnpm exec vitest run`（backend error、i18n、Update Center、Operation Log）：4 files / 17 tests passed。
- `cargo clippy --all-targets --locked -- -D warnings`：通过。
- `pnpm typecheck`、`pnpm lint`、`pnpm docs:gen:check`、`pnpm ipc:codegen:check`、`pnpm sizecheck`：通过。
- `just ci`：通过，包括完整前端、文档、Rust Clippy 与 locked tests。
- `git diff --check`：通过。

## 外部与独立门禁

- 未读取真实 GitHub token，未访问真实 GitHub，未修改用户 Central 或 live inventory。
- 未执行真实 SSH / WSL 端到端导入；固定 ref、remote manifest 和 local/remote digest parity 由纯测试覆盖，真实目标仍受项目既有 transport-seam 限制。
- `just audit` 未通过：两个既有 exception 已于 2026-08-11 过期，并发现未批准的 `npm:GHSA-qwww-vcr4-c8h2`、`cargo:RUSTSEC-2026-0258`、`cargo:RUSTSEC-2023-0071`。本任务未修改依赖或 lockfile，该供应链基线需独立处理。
- 本机 Node 为 v26.7.0，而仓库要求 Node 22.x；pnpm 在前端命令中输出 engine warning，但 `just ci` 完整通过。
