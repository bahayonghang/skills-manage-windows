# Runtime Observability

Runtime observability is the diagnostic layer behind the `/logs` console. It keeps user operation history and developer diagnostics separate so each layer can use the storage, retention, and privacy policy that matches its purpose.

## ADR: Two log layers

**Status:** accepted

**Decision:** SkillPort uses a two-layer log model:

- **Operation Log** stays in SQLite (`operation_logs`). It records user-visible actions such as installs, uninstalls, scans, settings changes, target switches, imports, and exports.
- **Runtime Log** uses bounded daily files named `skillport-YYYY-MM-DD.log`. It records backend tracing and frontend diagnostic events such as `error`, `unhandledrejection`, explicit `frontend.runtime` events, and IPC failures.

These layers meet in the Observability Console UI, but they do not share storage or lifecycle rules.

## Command policy and correlation

`src-tauri/src/ipc_registry.rs` is the source of truth for every runtime command and its one log policy: `operation`,
`runtime-only`, or `excluded`. The generated [IPC command dictionary](./ipc-commands.md) is the current command audit
matrix; it is generated from the registry and intentionally does not rely on a hand-maintained command count.

```text
frontend IPC call
  └─ valid correlation UUID
      ├─ Operation Log row (for user-visible side effects)
      ├─ backend Runtime rejection (safe reviewed diagnostic)
      └─ frontend Runtime rejection (renderer perspective)
```

Operation commands use their Operation row UUID as the correlation ID. Runtime-only failures receive the same UUID
format without creating business audit history. Backend and frontend Runtime events are separate viewpoints and expose
their event source explicitly.

## Why SQLite for Operation Log

Operation Log entries are long-lived product data. They need structured filters, stable pagination, target/category/action fields, and user-facing export semantics. SQLite also keeps them close to other local-first metadata in `~/.skillsmanage/db.sqlite`.

Keeping Operation Log in SQLite avoids treating diagnostic noise as audit history. It also preserves the existing `operation_logs` table contract for list, detail, clear, and export commands.

## Why files for Runtime Log

Runtime Log entries are short-lived diagnostic traces. File logging is a better fit because:

- the Rust backend can write tracing output before the database or UI is ready;
- frontend diagnostics can be appended without schema migrations;
- daily files are easy to inspect, copy, redact, export, and delete;
- retention can be bounded by deleting matching files older than 14 days.

Only files matching `skillport-YYYY-MM-DD.log` are listed, read, exported, or deleted. The backend rejects arbitrary file names so the IPC surface cannot traverse outside the log directory.

## Observability Console contract

`/logs` is a two-mode console:

| Mode      | Source                                 | Main use                            | Clear semantics                                                |
| --------- | -------------------------------------- | ----------------------------------- | -------------------------------------------------------------- |
| Operation | SQLite `operation_logs`                | User action history and audit trail | Existing manual Operation Log clear flow                       |
| Runtime   | Daily `skillport-YYYY-MM-DD.log` files | Frontend/backend diagnostics        | Delete selected matching runtime file or bounded runtime files |

The Runtime mode supports file selection, query / level / source / operation-ID / event-source filters, tail reads, safe
line details, copy, export, and clear confirmation. Runtime lines pass through the same fail-closed redaction policy before
disk persistence and again on read/export, including lines assembled from multiple writer fragments. If a writer flushes
an incomplete line, the sink persists only a redaction marker and discards that line's continuation through the next
newline, so separate flushes cannot reassemble a secret on disk.

Operation detail opens as a centered, compact, viewport-safe dialog. The primary view uses localized status, reviewed
reason, next action, safe diagnostic keys and bounded failure items; safe structured details remain collapsed. A valid
correlation UUID can jump to matching Runtime evidence. Runtime evidence can jump back to the exact Operation row.

## Privacy and retention

New events are safe by construction: codes, categories, actions, phases, statuses and sources come from controlled sets;
dynamic values are limited to validated UUID/logical IDs, numbers and booleans. Passwords, tokens, PATs, API keys, SSH
credentials, host/user values, paths, URLs, refs/SHAs, commands/environment, output, stacks and raw source errors are not
log inputs. Pre-persistence redaction is a final sink guard; read/export redaction remains a compatibility defense for
historical files, not permission to construct raw events.

Runtime Log has stricter lifecycle limits:

- startup cleanup removes runtime files older than 14 days;
- manual clear only touches whitelisted runtime log file names;
- Runtime Log deliberately does not proxy all `console.*` output to avoid noisy and private captures.

## Troubleshooting workflow

1. Start from the failed Operation row and copy its correlation UUID.
2. Open matching Runtime evidence and compare the backend and frontend source views.
3. Use the reviewed code, phase and retryable flag to decide the next action; do not infer from raw JSON.
4. If no Operation row is expected for a read/preview, filter Runtime directly by the returned correlation UUID.
5. A `started` row after abnormal termination becomes `interrupted` during the next one-time startup audit sweep.

Last reviewed: 2026-08-27
