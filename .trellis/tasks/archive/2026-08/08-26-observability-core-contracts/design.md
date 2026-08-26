# 核心契约设计

## Module Shape

`src-tauri/src/observability/` owns policy types, operation IDs, lifecycle execution, safe result/failure builders and
best-effort recording. `operation_log.rs` becomes a temporary compatibility adapter and is removed only by the final
integration child.

External interface:

```rust
run_operation(state, definition, operation_context, success_builder, operation) -> IpcResult<R>
record_terminal(pool, definition, operation_context, safe_result) -> OperationId
record_runtime_failure(runtime_failure_context, ipc_error) -> IpcError
```

Callers never learn DB insert/update details, redaction vocabulary, UUID generation or tracing fields.
`OperationContext` owns allowlisted target/subject identity and an optional `OperationBatchId`; batch grouping stays
separate from the row/correlation ID. `RuntimeFailureContext` accepts any `CommandPolicyEntry`: Operation reuses its row
ID, Runtime-only creates an ephemeral ID and Excluded remains silent.

## Policy Generation

Extend the existing runtime command macro syntax with a policy tag. Separate callbacks generate Tauri handler,
command names and `CommandPolicyEntry[]`; no second handwritten command list is allowed. `Excluded` reasons are enum
variants such as `SelfLogging` or `FrontendReadyBridge`, not arbitrary text.

## Persistence

- caller-supplied UUID row ID;
- insert started or terminal row;
- update by ID to a terminal status and duration;
- startup sweep `started -> interrupted` before normal operation handling;
- the sweep runs only after the process installs `AppState` for the first time, so startup retry cannot mark current-process
  rows interrupted;
- exact-ID filter/search; existing schema columns are sufficient.

The in-memory SQLite fixture exercises the real repository; no public repository trait is introduced.

## Diagnostic Contract

`ReviewedDiagnostic` is fully static except operation ID. Unknown errors receive fixed `internal.unexpected`, the
operation definition's category/default phase and retryable=false. Public messages come from existing reviewed IPC
registry/domain mappers. Raw source errors never enter the observability interface.

## Compatibility

`IpcError.correlationId` is optional and omitted when absent. Existing three-field deserializers remain compatible;
frontend types accept the optional field. Old Operation rows already have UUID ids and need no migration.
The legacy unnamed `ipc_boundary!(expression)` cannot select a precise command policy. The Runtime child owns the named
boundary, and each coverage child migrates the command files it owns.
