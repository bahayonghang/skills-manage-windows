# Runtime 诊断设计

## Backend Authority

Extend the IPC boundary through the frozen observability interface. On error it normalizes to `IpcError`, ensures a
correlation ID, emits one allowlisted `skillport::ipc` event and returns the same envelope. It records module/policy action
instead of serializing command args or source errors.

## Frontend View

`invoke.ts` continues to normalize rejection, then calls recorder with command plus normalized envelope. `runtimeLogger`
uses the backend correlation when present; legacy rejection gets a frontend-only ID and explicit frontend origin. Arguments
are nesting-preserved with every object key ordinalized and every string value replaced, matching redaction spec. The
correlation origin is explicitly `backend` or `frontend`.

Global browser events use static messages. Only allowlisted `Error.name` and safe line/column numbers may remain;
`message`, dynamic code, stack, filename and unknown rejection strings are omitted before IPC.

## Runtime DTO

Add optional `operationId` and `eventSource` to parsed Runtime line. Parser reads controlled fields from tracing text after
redaction. Existing raw lines without fields remain readable. Store supports exact operation ID filter; console child owns
cross-navigation.

## Duplicate Semantics

Backend and frontend events are two evidence sources, not deduplicated storage. UI can group by operation ID and source;
tests require exactly one event per source per rejection. Recorder retries never create recursive `ipc.failure`.
