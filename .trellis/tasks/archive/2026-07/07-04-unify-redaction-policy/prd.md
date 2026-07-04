# 统一 Redaction policy：两个日志层共用一份敏感字段契约

## Goal

把「什么算敏感字段、如何打码」收敛为一个 redaction policy module，Operation Log 与 Runtime Log 作为同一 seam 上的两个调用方；闭合已发生的 passphrase 泄漏。

## 背景与证据（2026-07-04 架构评审，已人工复核）

同一概念存在三份 implementation，且词表已经漂移：

- `src-tauri/src/operation_log.rs:36-46` — 敏感词表 9 词，**含 `passphrase`**，打码标记 `[redacted]`；`:296-329` `sanitize_details_value`。
- `src-tauri/src/logging.rs:702-736` — `is_sensitive_key` 8 词，**无 `passphrase`**，标记 `[REDACTED]`。
- `src-tauri/src/logging.rs:623-648` — `redact_sensitive_line` 正则版，同样无 `passphrase`。
- `src-tauri/src/targets/model.rs:101` — `passphrase: Option<String>` 字段真实存在 → SSH passphrase 进 Runtime Log（`skillport-YYYY-MM-DD.log`）会**原文泄漏**，在 Operation Log 则被脱敏。标记大小写分裂（`[redacted]`/`[REDACTED]`）证明两套独立演化。

现有测试：`operation_log.rs:360-392`（`sanitize_details_redacts_*`）、`logging/tests.rs:149,204`——两边各测各的。

## Requirements

1. 敏感词表与打码规则在仓库中只有一处定义；Operation Log 与 Runtime Log 都消费这一处。
2. 补上词表分叉缺口（至少 `passphrase`），并集中登记 CONTEXT.md 要求的全部类别：password、token、PAT、API key、secret、private key、credential。
3. 打码标记是否统一为一种（`[redacted]` vs `[REDACTED]`）在 design 阶段裁决——若既有日志消费方依赖现有字面量，可保持各自标记但由同一 module 输出。
4. 现有两边的 redaction 测试迁移或改写为针对新 module 的测试，不允许净减少覆盖。
5. **（design 阶段发现的线上缺陷，纳入范围）**修复 `"pat"` needle 的 substring 误伤：`is_sensitive_detail_key("path") == true` 导致 `commands/settings.rs:183,269,283` 的扫描目录操作日志把 `path` 持久化为 `[redacted]`。统一后的匹配语义须让 `path`/`pattern` 类 key 不再误伤、`pat`/`github_pat` 仍命中（语义细则见 design.md D3）。
6. **（design 阶段发现，纳入范围）**前端第四份词表 `src/lib/runtimeLogger.ts:17-18`（IPC 传输前的预脱敏防线）同步补 `passphrase`；不做前后端共享词表的过度工程。

## Constraints

- **不做 Operation Log DSL**（CONTEXT.md 明令）：新 module 的 interface 控制在两三个 redact 函数量级，目标是 locality，不是扩大 interface。
- Operation Log 与 Runtime Log 的数据源、生命周期、清理语义保持分离（CONTEXT.md Observability Console 约束）。
- 域错误契约不变；`#[error(...)]` 文案逐字保留（`.trellis/spec/backend/domain-error-enums.md`）。

## Acceptance Criteria

- [x] 新增测试证明：含 `passphrase` 字段的载荷在 Operation Log 与 Runtime Log 两条路径都被脱敏。
- [x] grep 验证：敏感词表（needle 列表）在 `src-tauri/src/` 内只有一处定义；`operation_log.rs` 与 `logging.rs` 不再各自维护词表。
- [x] `cd src-tauri && cargo test` 全部通过；`cargo clippy -- -D warnings` 通过。
- [x] 回归测试证明：`path`/`pattern`/`central_path` 类 key 不再被误脱敏，`pat`/`github_pat` 仍被脱敏。
- [x] 前端 `runtimeLogger` 测试含 `passphrase → "[REDACTED]"` 用例；`pnpm test` 全绿。
- [x] 两层日志的既有对外行为，除「词表补齐（passphrase、api-key/private-key dash 变体）」与「`pat` 误伤修复」外无变化（行为变化全清单见 design.md §5；现有测试仅允许迁移位置，不允许语义删改）。

## Notes

- 复杂度：complex（安全相关 + 跨两个横切模块）→ `task.py start` 前需补 `design.md` + `implement.md`。
- 评审 Top recommendation：改动半径最小的 Strong 候选，两个真实调用方已在场（two adapters = real seam）。
