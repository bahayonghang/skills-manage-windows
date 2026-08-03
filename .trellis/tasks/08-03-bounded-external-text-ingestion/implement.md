# Implementation Plan: Bounded external and text ingestion

## Step 1 - Inventory and red tests

- [x] 写 task-local `research/ingestion-inventory.md`，列出所有 production response/file reads和现有 cap。
- [x] 添加 chunked oversize HTTP、Local grow-after-stat、remote over-output、invalid UTF-8 tests。
- [x] 添加中文/emoji边界 panic regression。
- [x] 添加 SSE idle/total/wire/event/output matrix，使用 paused Tokio time和小 test policy。

## Step 2 - Shared mechanisms

- [x] 实现 bounded HTTP bytes/text reader与 checked accumulator。
- [x] 实现 Local bounded bytes/text reader。
- [x] 实现 UTF-8 char truncation和byte-prefix helper。
- [x] 为 remote target增加 bounded read operation并纳入process output policy。

Gate: helper purity、overflow、TOCTOU和redaction tests通过。

## Step 3 - Finite HTTP responses

- [x] Git tree 16 MiB body切换到bounded reader。
- [x] AI one-shot/tagging/connection success body切换到1 MiB policy，error details切64 KiB。
- [x] 统一AI client connect/header/body deadlines，保留auth/429/fallback分类。
- [x] 确认 Marketplace direct install downloader已由P0 task删除；无则不新增替代。

## Step 4 - SSE

- [x] 用byte buffer增量解析，处理跨chunk UTF-8和CRLF/event delimiter。
- [x] 加wire/event/output cap、idle/total deadline和checked计数。
- [x] 消除逐行remainder clone与完成时full-text clone。
- [x] 只在完整成功时cache并emit complete；失败emit一次typed error。

## Step 5 - File call sites

- [x] Central main SKILL与arbitrary file切换bounded Local/remote reader。
- [x] scanner和AI tagging file fallback在分配前使用1 MiB cap，并避免不必要content clone。
- [x] 检查其他inventory hits；迁移或记录有证据的exemption。

## Step 6 - Docs and validation

- [x] 更新resource budget/domain specs与inventory。
- [x] focused `ai_provider`、`ai_tagging`、`github_import`、`central_skills`、`scanner`、`targets` tests。
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [x] IPC/schema未变化；`pnpm docs:gen:check` 由 `just ci` 验证通过。
- [x] Node 22.23.2 环境下 `just ci`
- [x] 最终重新生成ingestion inventory并确认无未解释的post-allocation cap。

## Rollback points

- Helpers/tests独立。
- Finite HTTP、SSE、file domains分阶段，单阶段失败不需要撤回其他已验证边界。
- Remote bounded read涉及共享transport contract，必须与所有调用方和FakeRunner测试一起提交/回滚。
