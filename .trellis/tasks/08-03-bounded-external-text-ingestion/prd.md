# 统一外部响应与文本读取的有界输入契约

## Goal

对所有纳入范围的 HTTP response、SSE、Local/SSH/WSL 文本读取实施“分配前限额”，并修复 UTF-8 byte slicing panic。现有 `ResourceBudget` 的单项 cap 要成为 reader 的输入，而不是完整读取后的审计检查；AI 长流另外具备 idle/total deadline 和输出上限。

## Evidence

- `ai_provider/stream.rs:133-138,255-363` 仅有 connect timeout，SSE wire/buffer/full text 无界。
- `ai_provider/stream.rs:208-236`、`ai_provider/claude.rs:170-199,279-304` 和 `ai_tagging/prompt.rs:114-141` 对响应直接 `.text()`。
- `ai_tagging/mod.rs:231-242` 的 client 无 timeout。
- `github_import/tree_manifest.rs:320-331` 只预检 optional Content-Length，chunked body 在 16 MiB 检查前完整分配。
- `central_skills/files.rs:49-85,394-400` 的主 SKILL 和 remote file 在 transport 读取后才检查或完全不检查。
- `scanner/mod.rs:91-93`、`ai_tagging/prompt.rs:19-28` 完整读文件后才解析/截断。
- `ai_provider/prompt.rs:85-90` 的 `&content[..8000]` 与 `claude.rs:199,304` 的 500-byte slice 可在多字节 UTF-8 边界 panic。

## Requirements

1. 在 `resource_budget` 邻近位置提供共享 bounded readers，不把 HTTP/target/domain 错误耦合进 budget struct：
   - HTTP bytes/text：Content-Length 快速拒绝 + chunk checked-add + limit+1 早停。
   - Local file：metadata preflight + `Read::take(limit + 1)`，抵御 read 前文件增长。
   - Remote file：transport API 接受 max bytes，在远端 size/read 或 process output cap 层拒绝，不能先把完整 stdout/bytes 分配进调用方。
2. 保留既有 `DEFAULT_FILE_BYTES = 1 MiB` 与 `DEFAULT_TREE_RESPONSE_BYTES = 16 MiB`，并让主 SKILL、任意 skill file、scanner frontmatter 和 Git tree 实际使用这些 cap。
3. AI non-stream response正文最多 1 MiB，错误/诊断 body 最多 64 KiB；超限返回 typed response-too-large，公共错误不包含截断 body。
4. AI SSE 初始 policy：wire bytes 4 MiB、单 event/未换行 buffer 256 KiB、decoded explanation 1 MiB、idle 30 秒、total 5 分钟。connect timeout 10 秒保留；limits/deadlines集中为命名 policy 并可在测试注入小值。
5. SSE parser 不得通过每行复制整个 remainder 形成 O(n^2)；用 bounded byte buffer/drain 或等价增量解析。完成事件不再 clone 整个 explanation，只共享/移动 ownership 或让 cache/result引用同一内容。
6. `truncate_content` 按 8,000 Unicode scalar values 截断；AI tagging summary 按 4,000 chars；错误摘要按不超过指定 bytes 的最后合法 char boundary 截断。任何合法 UTF-8 都不 panic。
7. `ai_tagging`、one-shot AI、connection test 和 streaming request 都有明确 header/body deadline；429/auth/fallback 现有分类和用户文案保持。
8. P0 Marketplace 任务删除的 direct downloader不在本任务重建。实施前做全仓 `.text()`/`.bytes()`/`read_to_string`/remote `read_file` inventory；未迁移 call site 必须记录已有前置 cap 或明确非外部/小数据理由。
9. resource/timeout errors 进入各 domain typed enum 和 stable IPC code；URL、API key、response body、absolute path、remote output 不泄漏。
10. 不增加新的 production HTTP/SSE dependency；若后续认为必须引入，按仓库规则另行请求批准。

## Acceptance Criteria

- [x] chunked oversized Git tree、AI JSON/error body 与 remote file 在 limit+1 后停止读取，测试证明没有完整 body 分配。
- [x] SSE 无换行超大 event、持续小 chunk 超总量、30 秒无进展、总期限、正常 fragmented UTF-8 event 都有 deterministic paused-time tests。
- [x] 8,000-byte 边界附近的中文、emoji、combining text、ASCII 和空内容不 panic，truncate 结果为合法 UTF-8 且满足 char/byte契约。
- [x] Local 文件在 metadata 后增长、remote 报告小但输出超限、缺失/invalid UTF-8 都返回 typed error。
- [x] 主 `SKILL.md` 与 arbitrary skill file 的 Local/SSH/WSL budget parity通过；正常 1 MiB 以下行为/文案不变。
- [x] AI success/auth/429/provider fallback/cache/event sequence保持；超限/timeout 不写 partial explanation cache或 complete event。
- [x] inventory 文档列出所有生产 `.text()`/`.bytes()`/`read_to_string`/remote read call site及其 cap/exemption。
- [x] focused AI/GitHub/Central/scanner/targets tests、Rust fmt、all-targets locked Clippy、locked tests和 `just ci` 通过。

## Non-Goals

- 不更改用户配置的 AI endpoint allowlist/SSRF policy、模型选择或 prompt 产品内容。
- 不改变 archive expanded/file count budgets；只修 reader 在 budget 生效前完整分配的问题。
- 不把 usage 历史、SQLite payload 和用户显式打开的 arbitrary file 强行设为同一个小 cap；不同信任/用途使用显式 policy。

## Dependency

Marketplace call site处理必须后于 `08-03-marketplace-install-central-contract`，预期结果是确认 direct downloader 已删除。其余 AI/tree/file work可独立规划。
