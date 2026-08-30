# Design: 风险导向测试覆盖补强

## Architecture And Boundaries

本任务沿生产责任边界补测，不建立独立 coverage framework：

1. Rust service/database tests 验证文件系统、SQLite、SecretStore/credential backend 和 target config 的原子业务结果。
2. React/Zustand store tests 通过现有 IPC mock 和 deferred promises 验证 mutation 后刷新失败、并发完成顺序、错误传播与 stale-write 隔离。
3. 既有质量契约继续承担 Tauri capability、IPC code/redaction 和测试发现；本任务只补这些契约未覆盖的业务组合。

## Test Seams

### Filesystem + SQLite

- 使用 `fresh_db()` 和临时目录构造真实 repository/service state。
- 使用 SQLite trigger 在指定 insert/delete/第二条 batch write 处确定性失败。
- 断言错误前后完整状态快照：FS entry、canonical bytes、installation/member/skip rows、update state。
- 若测试首先失败，则最小修复优先选择 validation-before-write、单一顶层 transaction 或显式 compensation；不改变 schema。

### Target + Credential

- 使用 `MemoryCredentialBackend`，不触达系统 keyring。
- 通过现有 settings repository/trigger seam 注入分阶段失败，并比较 SSH/WSL lists、active target、credential 和 pool ownership。
- 前端复用现有 target store IPC mock，分别覆盖 mutation 首命令失败和 mutation 成功后 refresh 失败；保留“后端可能已变更”的显式 error/reload-required 语义。
- 使用 secret sentinel 递归检查 Zustand state、错误和普通诊断；不改变尚未定义的 connection-test 成功载荷契约。

### Zustand Async Jobs

- 通过现有 `mockIPC` 按命令顺序返回 success、reject 或 deferred promise。
- 对每次 await 后的 store write 使用已有 generation/job correlation 语义；reset/target change 后旧完成和旧错误均不得写回。
- 部分成功测试分别记录 backend mutation 已发生与 renderer terminal state，避免把整个动作误报为“从未发生”。

### Credential Settings

- 将 secret command 与普通 settings command 的调用参数分别断言，递归检查 renderer state、错误和 runtime diagnostics 不含 sentinel secret。
- `set_ai_api_key` 成功而普通设置失败时，保留明确失败/重试状态并清除 renderer plaintext；provider switch 的前置 flush 失败后短路后续 provider 操作。
- 并发持久化顺序不在本任务定义；没有已有 spec 支持时不擅自增加队列或 latest-edit-wins 语义。

## Compatibility And Trade-offs

- 不新增依赖和 coverage 配置，保持 Node/pnpm/Rust pinned toolchain 不变。
- 测试默认放在现有拥有者测试模块，避免扩大 `pub(crate)` 或创建新的测试 taxonomy。
- 测试优先保护安全不变量；不通过断言固化已知的 partial mutation、secret leakage 或 stale state。
- symlink 变体受 Windows privilege 影响；copy 路径是必需证据，链接路径只在现有 seam 可确定执行时纳入通过门禁。

## Rollback

- 无数据库 migration、依赖或公共 API 变更，回滚以模块级测试/最小修复为单位。
- 若某高风险 invariant 需要超出模块边界的架构重写，停止该项并记录为独立缺陷，不以弱化测试完成任务。

## Deferred Evidence

- 没有数值 line/branch coverage 结论。
- 没有真实 SSH、系统 keyring、provider、原生 GUI 或发布环境验证。
