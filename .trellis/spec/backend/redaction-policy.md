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
9. **Update apply 单项诊断使用 allowlist**：Operation Log `failureItems` 只允许 `step`、安全逻辑 `identifier`、受控 `phase`、稳定 `errorCode` 和稳定 `errorCategory`，最多 50 项并记录截断数。Runtime apply 事件只记录排序去重的 code/category 与 phase counts。两层都不得记录 item `error`、manifest、完整路径、URL、repository source path 或命令输出。`step` 必须收敛到固定动作表；`identifier` 只保留有限长度的 ASCII 逻辑 ID（含 `agent_id::skill_id` 和 repository 前缀），不符合约束的动态输入统一降级为 `batch`。
10. **Refresh/retry 仓库诊断使用同一 allowlist**：Operation Log 只允许安全 `repositoryId` 和静态 code/category，最多 50 项；Runtime Log 只允许聚合 code/category 与 retry 数字。持久化历史值在日志前重新校验；URL、owner/repo/ref、HTTP status/detail、响应正文、reqwest Display 与 failed row `error` 不得成为日志输入。
11. **不可变库存身份不属于诊断载荷**：pending additions 可持久化完整 commit SHA 与 repository digest 作为 Apply 权威，但两者不得进入 IPC error、Operation Log、Runtime Log、telemetry 或 portable export。GitHub 401/403 必须由 typed `used_auth` 事实映射为匿名 `access_denied` 或 `configured_token_failed`；禁止记录 token、Authorization header 或为分类解析动态 Display。

## Scenario: Structured IPC Error Payload

### 1. Scope / Trigger

- command error mapper、`IpcError`、frontend failure recorder 或状态导出发生变化时适用。

### 2. Signatures

```text
IpcError { code: String, message: String, retryable: bool, correlationId?: UUID }
failure recorder { command, sanitized args, normalized public error }
```

### 3. Contracts

- payload 只允许稳定 code、已审查 public message、retryable 和可选 UUID correlationId；不附带 source/details。
- PAT、AI key、SSH password/private key、绝对/相对路径、命令/env、stdout/stderr、
  snapshot token/digest 和文件内容不得进入 IPC error、failure recorder 或状态导出。
- Archive redirect rejection 的 Operation Log 只记录静态
  `errorCode=github_import.archive_redirect_rejected` 与
  `phase=repository_snapshot`；Runtime failure recorder 只记录同一固定 public
  code/message。`Location`、完整 URL、owner/repo/ref、header、响应正文不得进入
  任一日志。`UpdateCommandError::Display` 继续使用通用固定文本。
- `GithubImportError::ipc_error_code` 是 IPC envelope、Operation Log `errorCode` 与
  Runtime Log 的唯一码表，全部为 `&'static str` 字面量。一个失败不得在某个面上有稳定
  码、在另一个面上退化为 `internal.unexpected`。
- 未被分类的失败仍必须记录静态 `errorCategory`（域 + 变体族），使 Operation Log 不会只
  剩固定摘要。category 同样是 `&'static str`，不得由 Display 派生。
- Update Center 命令失败必须在 Rust 边界写 Runtime Log：只允许 action、error code、
  error category、phase、duration 这些静态或数值字段。缺少这一条时「See runtime logs
  for details」会指向只有前端通用重复记录的文件。
- frontend recorder 保留对象/数组 shape 与非字符串 scalar，所有字符串参数替换为
  `[REDACTED]`；未知 rejection 的 message 固定化。

### 4. Validation & Error Matrix

| Input                                  | Required output                                                               |
| -------------------------------------- | ----------------------------------------------------------------------------- |
| reviewed stable domain variant         | fixed code/message                                                            |
| archive redirect is rejected           | fixed code + phase in Operation Log; fixed public code/message in Runtime Log |
| GitHub network family failure          | same fixed `github_import.*` code on IPC, Operation Log, and Runtime Log      |
| domain failure without a reviewed code | static `errorCategory` in Operation Log; IPC still `internal.unexpected`      |
| known legacy coded family              | canonical message; raw details dropped                                        |
| unknown Display/string/object          | `internal.unexpected` fixed message                                           |
| args containing any string             | same shape, string replaced                                                   |

### 5. Good / Base / Bad Cases

- Good: mapper selects a static public summary and logs diagnostics only through the existing redaction boundary.
- Base: unknown helper error loses historical text and fails closed.
- Bad: `IpcError { message: error.to_string(), .. }` or recorder stores raw args.

### 6. Tests Required

- Serialize adversarial seeds and assert exact seed absence.
- Assert recorder args contain no original string at any nesting depth.
- Assert existing coded AI/GitHub/local-archive behavior retains only canonical public meaning.
- Assert archive redirect adversarial PAT/URL/path/body seeds are absent from IPC,
  Operation Log details, and Runtime failure records while the stable code remains visible.

### 7. Wrong vs Correct

```rust
// Wrong
IpcError::new("storage.unavailable", Box::leak(error.to_string().into_boxed_str()), false)

// Correct
IpcError::new("storage.unavailable", "Storage is unavailable.", false)
```

## 来源

任务 `07-04-unify-redaction-policy`（2026-07-04，架构深化专项子任务 1）。
