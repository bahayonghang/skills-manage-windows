# Notes — Skill Usage 页加载性能优化

## 调查确认的事实（实施前验证）

- 本机数据规模（2026-08-15 实测）：codex 5.2G（2554 文件）、claude 500M、grok 314M、
  opencode 298M；`~/.factory/sessions` 不存在（droid 无本机数据，预过滤只能 fixture 级验证）。
- portable_state 导出完全不读取 usage 相关表 → 新增 `skill_call_file_cache`（含文件路径）
  不会进入导出/日志通道（构造层面保证）。
- `schema/usage.rs` 被 `migrations/versions/v1.rs` 以 `include_str!` 计入 migration 1
  checksum → 新表只能放 `versions/v5.rs`，绝不能动 `schema/usage.rs`。

## R1 预过滤 needle 真实数据验证（verify_prefilter.py）

方法：对本机真实日志逐行做「无过滤完整 JSON DOM 解析」，凡会产生解析效果的行
（codex: type==session_meta，或 response_item 中 input_text 含 `<skill>` 且
`<name>X</name>` 命中且非 builtin；grok updates: user_message_chunk text 命中
`<command-name>` 正则），断言原始行文本包含对应 needle。

- codex：全量 2554 文件 / 1,264,057 行 / 3,985 行有解析效果 / **0 违例**
  （needle = `<skill>` 或 `session_meta`）。
- grok updates.jsonl：74 文件 / 32,963 行 / 0 行有解析效果 / **0 违例**
  （needle = `<command-name>`；本机 grok 的 skill 调用全部来自 chat_history.jsonl，
  该路径已有 `command-name` 预过滤）。
- droid：本机无数据；needle = `session_start` 或 `is now active`（正则
  `Skill "X" is now active` 的命中必然包含后者；session_start 行提供 session_id/project，
  不可被过滤掉）。fixture 级等价测试兜底。

## 基准测量

环境：release profile（opt-level 3）单测二进制直跑 `usage_bench`（`--ignored`），
内存池 DB + 真实 home 目录。峰值内存用 PowerShell `PeakWorkingSet64` 采样。

| 轮次 | cold 全量 | steady-state 增量 | 峰值工作集 |
| --- | --- | --- | --- |
| 基线（改动前） | 10.95s（2109 calls，4 providers） | 10.96s（全量重扫，无增量路径） | 5,640,335,360 B ≈ 5.25 GiB |
| 改动后 | 4.91s（2109 calls，4 providers） | 153ms（2109 calls，指纹全命中、零重解析） | 284,405,760 B ≈ 0.26 GiB |

改动后测量于 2026-08-15（同一台机器、同一份真实 home 数据、同一采样脚本）。
cold 提速 ~2.2× 来自 R1 行级预过滤（codex 1.26M 行仅 3,985 行需要 JSON 解析）+
R3 流式逐文件处理；steady-state ~71× 来自 R7 指纹缓存；峰值内存 ~20× 下降
来自流式处理（旧路径 `read_many_to_strings` 把 5.2G 语料整体读进内存，已消除）。
两轮 `calls_written` 完全一致（2109=2109）——真实数据上增量与全量等价。
