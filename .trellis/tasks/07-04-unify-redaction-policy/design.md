# Design：统一 Redaction policy module

> 任务：`07-04-unify-redaction-policy` · 依据：prd.md + 2026-07-04 设计走查（本文档所有行号已对照当前 `dev` 分支源码核实）

## 1. 目标与非目标

**目标**：把「什么算敏感字段 + 如何打码」收敛为一个 crate 根部的 redaction policy module（deep module：小 interface、全部策略在 implementation 里），Operation Log 与 Runtime Log 成为同一 seam 上的两个调用方。

**非目标**：

- 不做通用 redaction DSL / 可配置策略框架（CONTEXT.md 明令）。
- 不改变两层日志的数据源、生命周期、清理语义（Observability Console 约束）。
- 不引入「后端 tracing 写入时脱敏」——Runtime Log 现行模型是**读取/导出时脱敏**（磁盘原文），改成写入时脱敏是性能敏感的行为变更，超出本任务（记录为已知事实，不动）。
- 不统一前后端为一份代码（前端是 TS，只做词表同步，见 D6）。

## 2. 现状剖析：一个概念、四份 implementation、三种语义

| #   | 位置                                                                             | 形式                               | 词表 | passphrase | dash 变体 (api-key) | "path" 误伤                      | 标记         |
| --- | -------------------------------------------------------------------------------- | ---------------------------------- | ---- | ---------- | ------------------- | -------------------------------- | ------------ |
| 1   | `operation_log.rs:36-46,296-329`                                                 | JSON 递归 walker（substring 匹配） | 9 词 | ✓          | ✗ 不识别            | **✗ 误伤**                       | `[redacted]` |
| 2   | `logging.rs:702-736`（`redact_json_value`/`redact_json_map`/`is_sensitive_key`） | JSON 递归 walker（substring 匹配） | 8 词 | **✗ 泄漏** | ✗ 不识别            | **✗ 误伤**                       | `[REDACTED]` |
| 3   | `logging.rs:623-648`（`redact_sensitive_line` + `REDACTION_PATTERNS` OnceLock）  | 正则行脱敏（`\b` 词边界）          | 8 词 | **✗ 泄漏** | ✓ `api[_-]?key`     | ✓ 不误伤                         | `[REDACTED]` |
| 4   | `src/lib/runtimeLogger.ts:17-18`（`SENSITIVE_KEY_PATTERN`）                      | TS 正则（substring）               | 8 词 | **✗ 缺失** | ✓                   | ✗ 误伤（无关紧要，前端仅防运输） | `[REDACTED]` |

**已核实的两个缺陷**：

1. **passphrase 泄漏**（PRD 主诉求）：`targets/model.rs:101` 存在 `passphrase: Option<String>`；#2/#3/#4 词表均无 `passphrase`。
2. **`"pat"` substring 误伤（设计走查新发现）**：`is_sensitive_detail_key("path")` 返回 true（`"path".contains("pat")`）。`commands/settings.rs:183,269,283` 往 Operation Log details 写 `"path"` key → 扫描目录 add/remove 的操作日志**当前把路径持久化为 `"[redacted]"`**，用户在 /logs 看不到操作对象。同理波及 `pattern`、`*_path` 等一切含 `pat` 子串的 key；而 #3 正则版因 `\b` 词边界不误伤——同一概念第三种语义分叉。

**Runtime Log 的调用链**（迁移映射的依据）：

- 读取：`parse_runtime_log_line`（`logging.rs:516`）→ `redact_sensitive_line`。
- 导出：`export_runtime_log_file_from_dir`（`:359`）→ `redact_sensitive_line`。
- 前端事件落盘：`sanitize_frontend_runtime_log_payload`（`:677` message、`:689` details JSON walker、`:698` details 序列化后再过一遍行正则）。

**Operation Log 的调用链**：`OperationLogEvent::details()`（`operation_log.rs:141`）→ `sanitize_details_value`，全仓唯一入口（grep 已核实无其它消费方）。

## 3. 决策记录

| #   | 决策                                                                                                                                         | 理由                                                                                                                                                                                            |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | 新建 crate 根模块 `src-tauri/src/redaction.rs`，与 `paths.rs`/`fs_util.rs` 同级                                                              | 两个调用方（operation_log、logging）互为 peer，策略放任何一方都会造成 peer 间依赖；横切策略归 crate 根是本仓库既有惯例                                                                          |
| D2  | 标记**不统一**：Operation Log 保持 `[redacted]`、Runtime Log 保持 `[REDACTED]`，由 module 内部封装                                           | PRD AC 要求既有对外行为不变；DB 历史行与前端 fixture（`runtimeLogStore.ts:65` 等）都引用现有字面量；统一是零收益的持久化文本 churn。marker 是 implementation 细节，不进 interface               |
| D3  | 词表匹配语义：**长 needle 维持 substring**（lowercase + `-`→`_` 归一后匹配），**唯独 `pat` 要求 token 边界**（两侧为字符串端点或非字母数字） | `pat` 是唯一高碰撞 needle（path/pattern/dispatch…）；`token`/`secret` 等 substring 保留可继续命中 `accessToken`→`accesstoken` 这类 camelCase 压扁的复合词。最小语义修正，不引入过度聪明的分词器 |
| D4  | JSON walker 词表补齐 dash 变体：归一化 key（lowercase、`-`→`_`）后用 `api_key`/`private_key` 命中 `api-key`/`private-key`                    | 行正则（#3）早已覆盖 dash 变体，JSON 侧没有——属于「词表补齐」授权范围内的收敛                                                                                                                   |
| D5  | 行正则整体搬入 `redaction.rs`，仅在两个 alternation 组里补 `passphrase`，其余 pattern 原样保留                                               | 正则语义已被 `logging/tests.rs:85-117,149-163` 锁定，最小改动                                                                                                                                   |
| D6  | 前端 `runtimeLogger.ts` 的 `SENSITIVE_KEY_PATTERN` 补 `passphrase`，其余不动                                                                 | 前端是防「敏感值过 IPC」的第一道防线（backend 落盘前会再脱敏一次，backend 是权威）；一词同步成本极低。跨语言共享一份词表源文件属于过度工程，明确拒绝                                            |
| D7  | `operation_log.rs` 与 `logging.rs` 中被替代的函数**删除**而非保留转发                                                                        | deletion test：转发壳是 pass-through；`sanitize_details_value` 虽为 pub 但无外部消费方（grep 核实）                                                                                             |

## 4. 模块设计

### 4.1 Interface（全部公开面，共 3 个函数）

```rust
// src-tauri/src/redaction.rs
//! Redaction policy module —— SkillPort 敏感字段脱敏的唯一策略点。
//! Operation Log 与 Runtime Log 是本 seam 的两个调用方；
//! 词表、匹配语义、打码标记全部是 implementation，不对调用方暴露。

/// Operation Log details 专用：递归脱敏 JSON，标记 "[redacted]"。
pub fn redact_operation_details(value: Value) -> Value;

/// Runtime Log JSON 载荷专用（前端 details）：递归脱敏，标记 "[REDACTED]"。
pub fn redact_runtime_json(value: Value) -> Value;

/// Runtime Log 文本行专用（读取/导出/前端 message）：正则脱敏，标记 "[REDACTED]"。
pub fn redact_runtime_line(raw: &str) -> String;
```

调用方无需知道词表、标记或匹配规则——interface 只回答「这是哪一层日志的什么形态」。不暴露 `is_sensitive_key`（无外部需求，暴露即扩大 interface）。

### 4.2 Implementation（模块私有）

```rust
const SENSITIVE_KEY_NEEDLES: &[&str] = &[
    "password", "passphrase", "token", "api_key", "apikey",
    "secret", "private_key", "privatekey", "credential",
];            // 长 needle：归一化后 substring 匹配
              // （privatekey 对齐行正则 private[_-]?key 的无分隔命中，见 §5-2）
const TOKEN_BOUNDED_NEEDLES: &[&str] = &["pat"]; // 短 needle：要求 token 边界

const OPERATION_MARKER: &str = "[redacted]";
const RUNTIME_MARKER: &str = "[REDACTED]";

fn is_sensitive_key(key: &str) -> bool;                    // D3/D4 语义
fn redact_value(value: Value, marker: &str) -> Value;      // 共用递归 walker
static REDACTION_PATTERNS: OnceLock<Vec<Regex>>;           // 自 logging.rs 迁入，补 passphrase
```

`is_sensitive_key` 语义（唯一定义点）：

1. `normalized = key.to_lowercase().replace('-', '_')`；
2. 任一长 needle 是 `normalized` 的子串 → true；
3. `pat` 在 `normalized` 中以 token 形式出现（左右均为端点或非字母数字）→ true；
4. 否则 false。

### 4.3 调用方迁移映射（全量）

| 调用点                                  | 现在                                                                            | 迁移后                                                                |
| --------------------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `operation_log.rs:141` `details()`      | `sanitize_details_value(details)`                                               | `crate::redaction::redact_operation_details(details)`                 |
| `operation_log.rs:36-46,294-329`        | 本地词表 + `is_sensitive_detail_key` + `sanitize_details_value`                 | **删除**（D7），模块头注释同步改写（职责改为「委托 redaction seam」） |
| `logging.rs:359` 导出                   | `redact_sensitive_line`                                                         | `crate::redaction::redact_runtime_line`                               |
| `logging.rs:516` 读取解析               | 同上                                                                            | 同上                                                                  |
| `logging.rs:677` 前端 message           | 同上                                                                            | 同上                                                                  |
| `logging.rs:698` details 字符串二次过滤 | 同上                                                                            | 同上                                                                  |
| `logging.rs:689` 前端 details JSON      | `redact_json_value`                                                             | `crate::redaction::redact_runtime_json`                               |
| `logging.rs:623-648,702-736,14`         | `redact_sensitive_line`/`redact_json_*`/`is_sensitive_key`/`REDACTION_PATTERNS` | **删除**（正则与 OnceLock 迁入 redaction.rs）                         |
| `src/lib/runtimeLogger.ts:17-18`        | 8 词正则                                                                        | 补 `passphrase`（一词）                                               |
| `lib.rs:1-10`                           | —                                                                               | 追加 `pub mod redaction;`                                             |

## 5. 行为变化清单（供审阅，除此以外 byte 级不变）

1. ✅ 补齐：`passphrase` 在 Runtime Log 三条路径（行/JSON/前端预脱敏）开始被脱敏——本任务主诉求。
2. ✅ 补齐：`api-key`/`private-key` dash 变体与 `privateKey` 无分隔驼峰在两个 JSON walker 路径开始被脱敏（对齐行正则 `api[_-]?key`/`private[_-]?key` 的既有行为）。
3. ✅ 修复：`path`/`pattern`/`*_path` 等含 `pat` 子串的 key 不再被 JSON walker 误脱敏——`settings.rs:183,269,283` 的扫描目录操作日志恢复显示真实路径（DB 既有历史行不动）。
4. 标记、两层日志语义、正则其余行为、错误枚举、IPC 契约：零变化。

## 6. 测试设计

**新增 `redaction.rs` 内联测试**（策略的唯一测试点）：

- 词表逐类命中：password/passphrase/token/api_key/apiKey/api-key/apikey/secret/private_key/private-key/credential/PAT/github_pat。
- 回归（新语义）：`path`、`paths`、`pattern`、`central_path` **不**被脱敏；`pat`、`github_pat` 被脱敏。
- 三个入口的标记正确性：operation → `[redacted]`；runtime json/line → `[REDACTED]`。
- **奇偶校验（parity）**：同一嵌套 payload 分别过 `redact_operation_details` 与 `redact_runtime_json`，被脱敏的 key 集合完全一致（仅标记不同）——防止未来词表再漂移的守卫测试。
- 行正则：JSON 风格 `"passphrase":"x"` 与 kv 风格 `passphrase=x` 两种；既有 token/api_key 用例自 logging 测试语料复制。

**迁移的测试**：`operation_log.rs:360-405` 三个 `sanitize_details_*` 测试迁至 redaction.rs（断言改经新入口）；`operation_log.rs:444`（builder 链路含 `token → [redacted]`）**原地保留**——它证明 wiring 而非策略。

**保留的测试**：`logging/tests.rs:85-117`（读取脱敏）、`:149-163`（导出脱敏）、`:204-223`（前端 payload 脱敏）原样保留——经公共 interface 证明 wiring，是「测试面 = interface」的正确形态。

**前端**：`src/test/runtimeLogger.test.ts` 补一个 `passphrase → "[REDACTED]"` 用例。

## 7. 风险与回滚

- 风险低：纯函数迁移 + 全量 parity 测试；无 DB schema、无 IPC 契约、无异步语义变化。
- 唯一语义风险点是 D3（`pat` 边界化）：理论上存在「全小写无分隔的复合 key」（如 `mypat`）从命中变漏网——真实代码库 key 均为 snake/camel 风格，风险接受并已用测试锁定语义。
- 回滚：单点 revert（实现按 implement.md 分三步提交，每步独立可编译、可 revert）。

## 8. 替代方案（design-it-twice，均已否决）

| 方案                                       | 否决理由                                                                                                      |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `RedactionPolicy` 结构体 + 可配置词表/标记 | 只有一份策略存在；配置=扩大 interface，正面违反 CONTEXT.md no-DSL 约束                                        |
| 统一标记为 `[REDACTED]`                    | 打破 PRD 零行为变化 AC；持久化文本 churn 零 locality 收益（见 D2）                                            |
| 策略放进 `logging.rs`，operation_log 引用  | 制造 operation_log→logging 的 peer 依赖；两个调用方 + 独立 seam 才是 two-adapters-justify-the-seam 的正确形态 |
| 完整 camel/snake 分词 token-sequence 匹配  | 过度聪明：会把 `accesstoken` 这类压扁复合词从命中变漏网，换取的精确度没有真实用例支撑（见 D3 的最小修正）     |
| 前后端共享一份词表源（codegen/JSON）       | 两行正则的同步成本远低于引入 codegen 链路；后端权威、前端防线的分层已足够                                     |
