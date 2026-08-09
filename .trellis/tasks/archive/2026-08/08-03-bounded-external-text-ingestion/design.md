# Design: Bounded external and text ingestion

## 1. Policy 与 mechanism 分离

```rust
struct ReadLimit { label: &'static str, max_bytes: u64 }
struct StreamPolicy {
    wire_bytes: u64,
    event_bytes: usize,
    output_bytes: usize,
    idle_timeout: Duration,
    total_timeout: Duration,
}
```

`ResourceBudget`/domain module决定数值和 label；reader只负责 checked incremental consumption。Domain 将 `BudgetExceeded`/timeout/UTF-8 映射到自己的 error enum。

## 2. HTTP reader

API 形状：

```rust
async fn read_response_bytes_bounded(
    response: reqwest::Response,
    limit: ReadLimit,
) -> Result<Bytes, BoundedReadError>;
```

算法：

1. 若 Content-Length > max，读取任何 body 前拒绝。
2. `bytes_stream()` 每 chunk 使用 `checked_add`；new_total > max 时立即 drop stream并报错。
3. 预分配 capacity 只使用 `min(content_length, max)`，不能信任超大 header。
4. UTF-8 decode 在 bounded bytes 上执行；错误不复制 body。
5. 错误 body 的读取预算为 64 KiB；用户可见摘要继续使用现有更小上限，并由 `safe_utf8_prefix_bytes` 在合法 char boundary 截断。原始 body 不进入公共错误。

Git tree直接传 16 MiB policy；AI JSON传 1 MiB，AI error details传 64 KiB。

## 3. File/target reader

Local：`metadata.len` 快速拒绝，随后 `File::open` + `.take(max + 1)`；读到 max+1 即拒绝。避免 `read_to_string` 先完整分配。

Remote：扩展 target transport operation 为 bounded read。远端脚本先 inspect regular file/size，再最多输出 `max + 1` bytes；本地 process policy 的 stdout cap同样设为 `max + protocol overhead`。最终 caller 再检查一次长度和 UTF-8。TOCTOU 文件增长在第二层被捕获。

所有 path containment仍先于 read；bounded reader不成为 path policy authority。

## 4. SSE state machine

```text
send/header deadline
  -> per-next idle timeout
  -> checked wire total
  -> append bounded buffer
  -> extract complete lines/events without remainder clone
  -> checked decoded output total
  -> emit chunk
  -> cache + complete only on valid terminal success
```

- 使用 absolute total deadline + 每次 `stream.next()` idle timeout。
- buffer超过 256 KiB 且没有 event delimiter直接报 protocol/budget error。
- `full_text` 预留小容量并限制 1 MiB。完成 payload可移动到 event/cache orchestration，或使用 `Arc<str>` 内部共享；IPC serialization最终仍会分配一次，但 service不得额外深 clone。
- timeout、budget、transport和provider status保持可区分 typed variants。

## 5. UTF-8 helpers

- Prompt按 chars：`content.chars().take(max_chars).collect()`，仅在确有更多 chars 时追加截断标记。
- Byte summary先 `min(len,max_bytes)`，再向前寻找合法 char boundary；不使用任意 `&s[..N]`。
- SSE chunk可能切在 UTF-8 code point中间，buffer以 bytes 保存，只对完整 data line做 UTF-8 decode；不能对每个 chunk做 lossy conversion，否则分片字符会被替换。

## 6. Error/redaction

`BoundedReadError` 只包含静态 label、actual/limit、timeout phase；domain error决定公共 code。不要保存 response body、URL 或 path作为 `source` debug field。既有 redaction仍覆盖operation log，但不是泄漏后的补救。

## 7. Inventory gate

实现结束前搜索：

```text
reqwest Response.text/bytes/bytes_stream
std::fs::read/read_to_string
tokio::fs::read/read_to_string
ConnectedRemoteTarget.read_file
String byte slices with dynamic limits
```

每个 production hit 要么使用 bounded helper，要么有靠近 call site 的可信内存来源/上游硬 cap注释和测试。

## 8. Rollback

Shared helper和UTF-8 tests先落地；按 Git tree -> non-stream AI -> SSE -> Local/remote files切换。每个 domain可独立回滚到 helper前，但绝不能保留“post-allocation check”并宣称已满足预算。
