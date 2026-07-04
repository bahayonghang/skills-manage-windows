# Implement：统一 Redaction policy module

> 前置：`prd.md`（需求与 AC）、`design.md`（决策 D1–D7、迁移映射、测试设计）。按步序执行，每步一个提交（即回滚点），每步末尾跑该步的验证命令，绿了才进下一步。

## Step 1 — 新建 redaction module（含全部策略测试）

- [ ] 新建 `src-tauri/src/redaction.rs`：
  - 私有：`SENSITIVE_KEY_NEEDLES`（8 长词，含 `passphrase`）、`TOKEN_BOUNDED_NEEDLES`（`pat`）、`OPERATION_MARKER`/`RUNTIME_MARKER`、`is_sensitive_key`（design §4.2 语义：lowercase + `-`→`_` 归一、长词 substring、`pat` token 边界）、`redact_value(Value, marker)` 递归 walker、`REDACTION_PATTERNS: OnceLock<Vec<Regex>>`（自 `logging.rs:624-635` 迁入，两个 alternation 组各补 `passphrase`）。
  - 公开（仅 3 个）：`redact_operation_details` / `redact_runtime_json` / `redact_runtime_line`。
  - 模块头 doc 注释：声明本模块是敏感字段脱敏的唯一策略点，两个调用方（Operation Log / Runtime Log）。
- [ ] `lib.rs` 追加 `pub mod redaction;`（按现有 mod 列表字母序插入）。
- [ ] 内联测试（design §6 全清单）：词表逐类命中、`path`/`pattern`/`central_path` 不脱敏回归、`pat`/`github_pat` 命中、三入口标记正确、**parity 守卫测试**、行正则 JSON 风格 + kv 风格（含 passphrase 新用例）。
- [ ] 验证：`cd src-tauri && cargo test redaction`，全绿。
- [ ] 提交（回滚点 1）：`feat(redaction): 新增统一脱敏策略模块`——此时旧实现仍在，双轨共存，行为零变化。

## Step 2 — 迁移 Operation Log 调用方

- [ ] `operation_log.rs:141` 改调 `crate::redaction::redact_operation_details`。
- [ ] 删除 `operation_log.rs` 的 `SENSITIVE_DETAIL_KEY_NEEDLES`（:36-46）、`is_sensitive_detail_key`（:324-329）、`sanitize_details_value`（:296-314）及其 doc（D7：不留转发壳）。
- [ ] 迁移测试：`:360-405` 三个 `sanitize_details_*` 测试移入 redaction.rs（断言改经 `redact_operation_details`；Step 1 若已按测试设计写全则此处仅删除旧测试）；`:444` builder 链路测试**原地保留**（wiring 证明）。
- [ ] 更新模块头注释（:1-20）：sanitize 职责改述为「details 经 redaction seam 脱敏后持久化」。
- [ ] 验证：`cd src-tauri && cargo test operation_log && cargo test redaction`，全绿；确认 `event_builder_chains_optional_fields` 仍断言 `token → [redacted]`。
- [ ] 提交（回滚点 2）：`refactor(operation-log): 脱敏改走 redaction seam`。

## Step 3 — 迁移 Runtime Log 调用方

- [ ] `logging.rs` 五处改调：`:359,516,677,698` → `redaction::redact_runtime_line`；`:689` → `redaction::redact_runtime_json`。
- [ ] 删除 `logging.rs` 的 `redact_sensitive_line`（:623-648）、`redact_json_value`/`redact_json_map`（:702-720）、`is_sensitive_key`（:722-736）、`REDACTION_PATTERNS` OnceLock（:14）及相应 `use`（Regex/Captures/Map 若再无他用）。
- [ ] `logging/tests.rs` 原样保留（`:85-117,149-163,204-223` 是经公共 interface 的 wiring 证明）；如需可在 `:149` 导出用例语料中追加一行 `passphrase=x` 断言（补齐生效的 E2E 旁证）。
- [ ] 验证：`cd src-tauri && cargo test logging && cargo test`，全绿（全量跑一次，确认无其它模块引用被删函数）。
- [ ] 提交（回滚点 3）：`refactor(logging): 脱敏改走 redaction seam`。

## Step 4 — 前端词表同步（防线补齐）

- [ ] `src/lib/runtimeLogger.ts:17-18` `SENSITIVE_KEY_PATTERN` 补 `passphrase`（`/password|passphrase|token|…/i`）。
- [ ] `src/test/runtimeLogger.test.ts` 补用例：`{ passphrase: "x" }` → `"[REDACTED]"`。
- [ ] 验证：`pnpm test -- src/test/runtimeLogger.test.ts`，全绿。
- [ ] 提交（回滚点 4）：`fix(runtime-logger): 前端预脱敏词表补 passphrase`。

## Step 5 — 全量门禁 + AC 复核（最后一轮全范围检查）

- [ ] `cd src-tauri && cargo test` 全绿；`cargo clippy -- -D warnings` 通过。
- [ ] `pnpm test`、`pnpm typecheck`、`pnpm lint` 全绿。
- [ ] grep 复核（AC 证据，逐条记录到任务笔记）：
  - `grep -rn "SENSITIVE" src-tauri/src --include="*.rs"` → 词表仅存在于 `redaction.rs`；
  - `grep -rn "\[redacted\]\|\[REDACTED\]" src-tauri/src --include="*.rs"`（排除测试断言）→ 标记仅由 `redaction.rs` 产出；
  - `grep -n "redact\|sanitize_details" src-tauri/src/operation_log.rs src-tauri/src/logging.rs` → 只剩对 `redaction::` 的调用。
- [ ] 对照 `prd.md` AC 清单逐项勾选；行为变化对照 `design.md` §5 无超纲项。
- [ ] 完成后进入 Trellis Phase 3：spec 更新（建议新增 `.trellis/spec/backend/redaction-policy.md`，登记「新增敏感类别只改 redaction.rs 词表 + parity 测试自动守卫」的契约）→ 提交收尾。

## 回滚策略

- 每步独立提交且独立可编译：任一步出问题 `git revert` 该步即可，不影响已合入的前序步骤。
- Step 1 是纯新增（双轨期），风险为零；Step 2/3 是删旧接新，回滚即恢复旧实现；Step 4 独立于后端。

## 审查门

- **门 1（Step 1 后）**：redaction.rs 的语义测试全绿即视为策略定版；若审查对 D3 匹配语义有异议，此时改动成本最低。
- **门 2（Step 5）**：全量门禁 + grep 证据齐备后，才允许宣称完成（verification-before-completion）。
