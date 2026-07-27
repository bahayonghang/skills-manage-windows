# Redaction Policy（敏感字段脱敏约定）

## 契约

1. **唯一策略点**：敏感字段脱敏的词表、匹配语义、打码标记全部在 `src-tauri/src/redaction.rs` 内部。禁止在其它模块私建词表或脱敏正则（历史教训：`operation_log.rs` 与 `logging.rs` 曾各持一份，`passphrase` 漂移导致 Runtime Log 泄漏，`"pat"` 子串匹配误伤 `path` 类 key）。
2. **调用方只挑入口**，不感知策略：
   - Operation Log details JSON → `redaction::redact_operation_details`（标记 `[redacted]`，持久化前脱敏）；
   - Runtime Log JSON 载荷 → `redaction::redact_runtime_json`（标记 `[REDACTED]`）；
   - Runtime Log 文本行（读取/导出/前端 message）→ `redaction::redact_runtime_line`（标记 `[REDACTED]`，覆盖 `"key":"value"` 与 `key=value` 两种形态）。
3. **匹配语义**（单一定义点）：key 归一化（lowercase、`-` 折叠为 `_`）后，长 needle 以 substring 匹配（保证 `accessToken` 这类驼峰压扁复合词命中）；短 needle（`pat`）要求 token 边界（两侧为端点或非字母数字），避免 `path`/`pattern` 误伤。
4. **新增敏感类别**：只改 `redaction.rs` 的词表并在该模块补一类测试；`operation_and_runtime_redact_the_same_keys` parity 测试自动守卫两个 JSON 入口不再漂移。若行正则也需覆盖新类别，两个 regex 的 alternation 组同步补词。
5. **标记不统一是有意为之**：`[redacted]`（Operation Log，DB 历史行沿用）与 `[REDACTED]`（Runtime Log，前端 fixture 依赖）由模块内部封装，调用方与前端不得依赖对方层的标记字面量。
6. **前端防线**：`src/lib/runtimeLogger.ts` 的 `SENSITIVE_KEY_PATTERN` 在敏感值过 IPC 前预脱敏，词表须与后端保持同步（后端权威，前端 belt-and-suspenders）。
7. **两层日志模型勿混**：Operation Log 是持久化前脱敏；Runtime Log 是读取/导出时脱敏（磁盘文件保留原文）。改动脱敏时机属于行为变更，需独立评审。
8. **Recovery journal 是第三类受控存储**：`fs_db_operations.manifest_json` 可保存恢复所需完整路径和 fingerprint，但不得进入 Operation Log、Runtime Log、IPC summary、状态导出或 telemetry。IPC/Operation Log 仅暴露 operation/target/kind/phase、稳定 error code 与 `CentralOperationError::redacted_message()`；tracing 禁止格式化含 source/path 的原始 recovery error。

## 来源

任务 `07-04-unify-redaction-policy`（2026-07-04，架构深化专项子任务 1）。
