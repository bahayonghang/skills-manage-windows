# Implementation Plan: Bounded external and text ingestion

## Step 1 - Inventory and red tests

- [ ] 写 task-local `research/ingestion-inventory.md`，列出所有 production response/file reads和现有 cap。
- [ ] 添加 chunked oversize HTTP、Local grow-after-stat、remote over-output、invalid UTF-8 tests。
- [ ] 添加中文/emoji边界 panic regression。
- [ ] 添加 SSE idle/total/wire/event/output matrix，使用 paused Tokio time和小 test policy。

## Step 2 - Shared mechanisms

- [ ] 实现 bounded HTTP bytes/text reader与 checked accumulator。
- [ ] 实现 Local bounded bytes/text reader。
- [ ] 实现 UTF-8 char truncation和byte-prefix helper。
- [ ] 为 remote target增加 bounded read operation并纳入process output policy。

Gate: helper purity、overflow、TOCTOU和redaction tests通过。

## Step 3 - Finite HTTP responses

- [ ] Git tree 16 MiB body切换到bounded reader。
- [ ] AI one-shot/tagging/connection success body切换到1 MiB policy，error details切64 KiB。
- [ ] 统一AI client connect/header/body deadlines，保留auth/429/fallback分类。
- [ ] 确认 Marketplace direct install downloader已由P0 task删除；无则不新增替代。

## Step 4 - SSE

- [ ] 用byte buffer增量解析，处理跨chunk UTF-8和CRLF/event delimiter。
- [ ] 加wire/event/output cap、idle/total deadline和checked计数。
- [ ] 消除逐行remainder clone与完成时full-text clone。
- [ ] 只在完整成功时cache并emit complete；失败emit一次typed error。

## Step 5 - File call sites

- [ ] Central main SKILL与arbitrary file切换bounded Local/remote reader。
- [ ] scanner和AI tagging file fallback在分配前使用1 MiB cap，并避免不必要content clone。
- [ ] 检查其他inventory hits；迁移或记录有证据的exemption。

## Step 6 - Docs and validation

- [ ] 更新resource budget/domain specs与inventory。
- [ ] focused `ai_provider`、`ai_tagging`、`github_import`、`central_skills`、`scanner`、`targets` tests。
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] IPC变化时 `pnpm docs:gen` + `pnpm docs:gen:check`。
- [ ] `just ci`
- [ ] 最终重新生成ingestion inventory并确认无未解释的post-allocation cap。

## Rollback points

- Helpers/tests独立。
- Finite HTTP、SSE、file domains分阶段，单阶段失败不需要撤回其他已验证边界。
- Remote bounded read涉及共享transport contract，必须与所有调用方和FakeRunner测试一起提交/回滚。
