# Operation Observability And Correlation Contract

## 1. Scope / Trigger

Apply this contract when adding or changing a runtime IPC command, Operation Log event, backend Runtime failure,
operation-log query/export, IPC rejection, or startup audit recovery. The canonical code owners are
`__skillport_runtime_commands` and `command_policy` in `src-tauri/src/ipc_registry.rs`, plus the public seam exported by
`src-tauri/src/observability/mod.rs`.

## 2. Signatures

```rust
pub enum CommandLogPolicy {
    Operation(OperationDefinition),
    RuntimeOnly(RuntimeOnlyReason),
    Excluded(ExclusionReason),
}

pub fn command_policy(command: &str) -> Option<&'static CommandPolicyEntry>;

pub async fn run_operation<R, F, Fut, BuildSuccess, Context>(
    state: &AppState,
    definition: OperationDefinition,
    context: Context,
    build_success: BuildSuccess,
    operation: F,
) -> IpcResult<R>;

pub async fn record_terminal(
    pool: &DbPool,
    definition: OperationDefinition,
    context: impl Into<OperationContext>,
    result: SafeOperationResult,
) -> OperationId;

pub fn record_runtime_failure(
    context: RuntimeFailureContext,
    error: IpcError,
) -> IpcError;

pub struct IpcError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub correlation_id: Option<String>,
}
```

Repository lifecycle entry points are `insert_operation_log_with_id`, `update_operation_log`,
`mark_started_operation_logs_interrupted`, and exact `OperationLogFilter.operation_id` filtering. The existing
`insert_operation_log` remains the compatibility path for callers not yet migrated.

## 3. Contracts

- Every runtime command is declared once in `__skillport_runtime_commands` with its handler and exactly one log policy.
  Handler, command-name and policy inventories are generated from that declaration. Do not copy command lists or counts
  into hand-maintained docs; `ipcCommandCoverage.test.ts` and the Rust registry tests report the current inventory.
- `Operation` is for user-visible writes, state changes and external side effects. `RuntimeOnly` is for successful pure
  reads, previews and reviewed internal refreshes; their failures still receive backend Runtime evidence. `Excluded` is
  limited to typed recursion/readiness reasons and emits no Runtime event.
- A command boundary must supply its command name or `CommandPolicyEntry`. The legacy `ipc_boundary!(expression)` cannot
  select an exact policy and is migration-only; new or migrated commands use the named boundary owned by the Runtime
  failure layer.
- `OperationId` is generated before execution and is the Operation Log row UUID plus cross-layer correlation ID.
  `OperationBatchId` is a separate UUID that groups attempts and must never replace a row ID.
- `StartedThenTerminal` inserts one `started` row and updates that same row to its terminal outcome. If the started insert
  was unavailable, terminal recording may insert the final row with the same ID. `TerminalOnly` inserts one terminal row.
- After `AppState` is successfully installed once, startup changes prior-process `started` rows to `interrupted`. Retry or
  re-entry must not rerun the sweep. This is audit truth only and never rewrites `fs_db_operations` recovery evidence.
- `OperationContext`, `SafeIdentifier`, `SafeOperationResult`, `ReviewedDiagnostic` and typed enums are the only inputs to
  new audit events. Subject/target IDs use the allowlist; labels, hosts, IPs, paths, arbitrary JSON and raw errors are not
  accepted. `OperationTarget` validates the target kind's stable identity prefix.
- Operation details contain only controlled counts, booleans, static values, safe identifiers, phase, stable error code/
  category and retryable. They pass through `redact_operation_details` before persistence.
- Operation Log writes are best effort and never replace the business result. A started or terminal write failure emits a
  static Runtime warning containing operation ID and phase only; it must not format the database error.
- `record_runtime_failure` reuses an operation ID for Operation commands and creates a same-format ephemeral ID for
  Runtime-only failures. It returns the same safe `IpcError` with optional `correlationId`; excluded self-logging remains
  silent. Backend and frontend Runtime events are distinct sources sharing the ID, not storage duplicates to delete.
- Existing Operation rows, missing optional IPC fields, filters and exports remain readable. No schema migration is needed
  because historical operation rows already use UUID IDs.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Command missing or duplicating policy | Registry/coverage contract test fails |
| Successful read or preview | No Operation row |
| Failed read or preview | Safe backend Runtime event with correlation ID |
| Operation succeeds/fails/cancels/partials | One terminal row with the pre-generated row/correlation ID |
| Process exits after `started` | Next one-time startup sweep marks the row `interrupted` |
| Started insert fails, operation succeeds | Business success preserved; terminal fallback attempted; static warning visible |
| Terminal write fails | Business result preserved; static warning visible |
| Invalid subject/target/identifier | Fixed safe fallback; no dynamic input in log fields |
| Raw path, host, credential, command output or source error | Absent from IPC, Operation and Runtime evidence |
| Excluded self-logging command fails | No recursive Runtime event |
| Legacy IPC rejection lacks `correlationId` | Frontend remains compatible and normalizes safely |

## 5. Good / Base / Bad Cases

- Good: a command resolves its registry policy, creates an `OperationContext`, maps its typed domain result to a
  `SafeOperationResult`, and calls `run_operation`.
- Good: a failed read calls the named Runtime boundary and returns the same reviewed `IpcError` carrying the generated ID.
- Base: a compatibility caller still uses `insert_operation_log`; the final governance task removes the adapter only after
  every owned command family migrates.
- Bad: invent category/action strings at the call site, use `error.to_string()` as `error_summary`, reuse `batch_id` as the
  operation ID, or write a Runtime event for an excluded self-logging command.

## 6. Tests Required

- Registry parity: names, handler and policy inventory have identical unique membership; representative read/preview/
  operation/excluded policies retain their class.
- Lifecycle repository: caller UUID insert, same-row terminal update, missing-start terminal fallback, exact ID query and
  startup `started -> interrupted` behavior.
- Module behavior: success/failure/partial/cancel, terminal-only/started-terminal, safe subject/batch persistence, and
  `batch_id != operation_id`.
- Best effort: missing schema or injected write failure preserves the business result; a test subscriber observes one
  static warning per failed lifecycle write and no raw SQLite text.
- Privacy: adversarial PAT/key/password/path/host/IP/URL/command/output/source text is absent from serialized IPC,
  Operation details, Runtime fields and frontend fixtures.
- Cross-layer compatibility: Rust/TypeScript optional correlation round-trip, invalid correlation rejection, legacy
  payload without the field, IPC codegen parity and generated-document drift checks.

## 7. Wrong vs Correct

```rust
// Wrong: no command policy, dynamic strings, and raw error persistence.
with_operation_log(state, operation, |error| {
    OperationLogEvent::new("updates", "refresh", "failed", "Refresh failed")
        .error(error.to_string())
})

// Correct: registry policy plus typed, allowlisted context and diagnostics.
let entry = command_policy("refresh_skill_update_inventory").expect("registered command");
let CommandLogPolicy::Operation(definition) = entry.policy else { unreachable!() };
run_operation(
    state,
    definition,
    OperationContext::new(target),
    |result| SafeOperationResult::succeeded("Update inventory refreshed.")
        .count(SafeDetailKey::AffectedCount, result.len() as u64),
    || async { service_call().await.map_err(map_reviewed_failure) },
).await
```
