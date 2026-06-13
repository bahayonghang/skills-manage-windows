# Runtime Observability

Runtime observability is the diagnostic layer behind the `/logs` console. It keeps user operation history and developer diagnostics separate so each layer can use the storage, retention, and privacy policy that matches its purpose.

## ADR: Two log layers

**Status:** accepted

**Decision:** SkillPort uses a two-layer log model:

- **Operation Log** stays in SQLite (`operation_logs`). It records user-visible actions such as installs, uninstalls, scans, settings changes, target switches, imports, and exports.
- **Runtime Log** uses bounded daily files named `skillport-YYYY-MM-DD.log`. It records backend tracing and frontend diagnostic events such as `error`, `unhandledrejection`, explicit `frontend.runtime` events, and IPC failures.

These layers meet in the Observability Console UI, but they do not share storage or lifecycle rules.

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

| Mode | Source | Main use | Clear semantics |
| --- | --- | --- | --- |
| Operation | SQLite `operation_logs` | User action history and audit trail | Existing manual Operation Log clear flow |
| Runtime | Daily `skillport-YYYY-MM-DD.log` files | Frontend/backend diagnostics | Delete selected matching runtime file or bounded runtime files |

The Runtime mode supports file selection, query / level / source filters, tail reads, raw line details, copy, export, and clear confirmation. Runtime export applies the same sensitive-value redaction policy as read output.

## Privacy and retention

Both layers redact sensitive fields such as password, token, PAT, API key, secret, private key, and credential. Runtime Log has stricter lifecycle limits:

- startup cleanup removes runtime files older than 14 days;
- manual clear only touches whitelisted runtime log file names;
- Runtime Log v1 deliberately does not proxy all `console.*` output to avoid noisy and private captures.

Last reviewed: 2026-06-03
