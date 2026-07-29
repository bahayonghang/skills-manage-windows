# Typed IPC 与结构化 IpcError 设计

## 1. Baseline 与边界

本设计同时处理两个相关但独立的契约：

1. 全部 Tauri command rejection 从 raw string 迁为结构化 `IpcError`。
2. 当前 89 项 allowlist 中的首批 42 项从 Rust/Serde 生成 args/result 类型。

冻结基线：

| 集合 | 数量 | 说明 |
| --- | ---: | --- |
| Rust registered handlers | 184 | annotation 与 runtime handler 当前完全一致 |
| `Result<_, String>` handlers | 180 | 本任务必须清零 |
| non-`Result` handlers | 4 | `get_startup_status`、`get_app_runtime_info`、`record_frontend_runtime_log`、`exit_startup` |
| Frontend contract commands | 177 | 88 typed + 89 untyped |
| Backend-only handlers | 7 | 已注册但当前无 frontend literal caller |
| First generated batch | 42 | 完成后 typed 130 / untyped 47 |

7 个 backend-only handler 固定为：`detect_agents`、`get_active_target`、
`get_central_skills_page`、`read_skill_content`、`suggest_skill_tags`、`sync_registry`、
`sync_registry_with_options`。parity contract 将验证集合关系，不要求 184 与 177 相等。

## 2. End-to-end data flow

```text
Rust domain/service error
  -> command-boundary mapper
  -> IpcError { code, message, retryable }
  -> Tauri rejected JSON payload
  -> @/lib/ipc normalizeIpcRejection
  -> IpcInvokeError extends Error
  -> store/action (code branch where behavior differs)
  -> component/toast (message/i18n display)

Rust command signatures + Serde metadata
  -> feature-gated tauri-specta/Specta collection
  -> repository AdapterContractExporter
  -> checked generated command map
  -> existing @/lib/ipc invoke overloads
  -> existing fixture/failure-recorder runtime path
```

`@/lib/ipc` remains the only frontend runtime entry. Generated code contains phantom
args/result types and command names only; it never imports or calls `@tauri-apps/api/core`.

## 3. Rust error contract

Add a backend boundary module owning the only desktop IPC error envelope:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

pub type IpcResult<T> = Result<T, IpcError>;
```

Under `ipc-codegen`, `IpcError` also derives `specta::Type`. The module provides explicit
constructors/mappers, not a public arbitrary-details bag. Commands return `IpcResult<T>`;
payload-internal per-item failures such as `FailedInstall.error` remain strings.

### 3.1 Code taxonomy

Codes use lower-case snake segments separated by dots. Existing stable families are retained:

- `ai.*`
- `github_import.*`
- `local_archive.*`

The common fallback families are:

| Family | Meaning | Default retryable |
| --- | --- | --- |
| `input.*` | invalid request or unsupported value | false |
| `resource.not_found` / `resource.conflict` | stable resource state | false |
| `permission.denied` | authz denial | false |
| `credential.*` | missing/unavailable protected secret | false |
| `operation.busy` / `operation.cancelled` | lifecycle state | false |
| `transport.timeout` / `transport.unavailable` | transient transport | false unless mapper proves pre-mutation safety |
| `storage.unavailable` | DB/filesystem unavailable | false unless operation is read-only |
| `internal.unexpected` | unclassified safe fallback | false |

UI-dependent codes frozen by this task:

- `operation.cancelled`
- `portable_state.invalid_manifest_json`
- `portable_state.unsupported_export_kind`
- `portable_state.unsupported_export_version`
- `github_import.rate_limited`
- `github_import.access_denied`
- `github_import.configured_token_failed`
- `credential.ssh_password_unavailable`

Existing `github_import.preview_*`, `local_archive.*` and `ai.*` codes keep their current i18n
keys. A strict legacy `code:message` parser exists only for transition/transport compatibility;
new Rust command boundaries construct the object directly.

### 3.2 Retryability

`retryable` is a conservative UI/API fact, not permission to retry automatically.

- Default is `false`.
- Cancellation is `false`: restarting is a new user action, not retrying the rejected request.
- Validation, conflict, missing credential and access denial are `false`.
- Rate-limit, timeout, offline and busy may be `true` only when the mapper proves the rejected
  command performed no mutation or the operation is explicitly idempotent/recoverable.
- No frontend automatic retry is added in this task.

### 3.3 Redaction and safe messages

The boundary mapper must choose a stable public message per domain variant. Safe historical
messages remain byte-compatible where possible; variants containing machine-local or attacker-
controlled details use a fixed safe summary. Raw `.to_string()` is not a general public escape
hatch.

The backend redaction policy gains an IPC-error entry point and tests that seed:

- GitHub PAT, AI key, SSH password and private-key material;
- Windows and POSIX absolute paths;
- executed command/environment text;
- captured stdout/stderr;
- snapshot token/digest and file content.

None may survive in serialized `IpcError`. Raw diagnostic sources may be logged only through
the existing redaction boundary; they are not stored in the error object, operation export or
frontend failure recorder.

## 4. Frontend normalization and compatibility

Add `src/lib/ipc/errors.ts` with:

- `IpcErrorPayload` runtime guard;
- `IpcInvokeError extends Error` exposing `code` and `retryable`;
- `normalizeIpcRejection(error)`;
- a structured fixture-error factory.

`IpcInvokeError.toString()` returns `message` so existing `String(err)`, toast and state fields
do not become `[object Object]` or `IpcInvokeError: ...`. `instanceof Error` remains true.

Normalization rules:

1. Existing `IpcInvokeError` passes through.
2. A valid `{ code, message, retryable }` payload becomes `IpcInvokeError`.
3. A strict legacy `code:message` string becomes the same wrapper and keeps its code.
4. A plain legacy string uses `internal.unexpected`, preserves only the adapter-approved public
   message, and remains non-retryable.
5. Frontend-local Errors such as `IpcFixtureMissingError` remain diagnosable and are not
   misrepresented as backend objects.
6. Unknown transport values become a fixed safe `internal.unexpected` wrapper.

`src/lib/backendError.ts` accepts `IpcInvokeError`/payloads first, then its legacy coded-string
path. It returns `retryable` in addition to code/message/details; legacy details are display-only
compatibility and are never added to the new Rust payload.

`invoke` normalizes rejection before returning it and before passing it to the failure recorder.
`invokeRaw` remains the documented runtime-logger bootstrap exception.

### 4.1 Message classification removal

The following behavior branches move to code checks:

- four portability status branches in `centralSkillsStore.updateSlice.ts`;
- `isManifestPreviewError` in `statePortabilityDialogUtils.ts`;
- GitHub auth/rate-limit/configured-token guidance in `githubImportWizardUtils.ts`;
- SSH password repair in `githubImportWizardActions.ts`.

Plain `String(err)` used only for display may remain because the wrapper guarantees message
semantics. Tests that currently reject raw strings or `Error` for backend cases migrate to
structured fixture payloads; transport-compatibility tests retain representative legacy cases.

## 5. Rust-derived codegen

### 5.1 Exact dependencies and feature boundary

Add an `ipc-codegen` feature with exact direct pins:

- `tauri-specta = =2.0.0-rc.25`
- `specta = =2.0.0-rc.25`
- `specta-typescript = =0.0.12`
- `specta-serde = =0.0.12` and `specta-util = =0.0.12` only if used directly by the exporter

All are optional and enabled only by the codegen binary/check. Normal `cargo build`, desktop
runtime and release bundle do not compile or mount a tauri-specta runtime handler. `Cargo.lock`
is committed and CI uses `--locked`.

Migrated commands use `#[cfg_attr(feature = "ipc-codegen", specta::specta)]`; referenced custom
types use feature-gated `specta::Type` derives. Serde remains authoritative for wire naming.

### 5.2 Registry ownership

Refactor the 184 runtime command paths into one declarative registry macro/list used to produce:

- the existing `tauri::generate_handler!` runtime handler;
- a compile-time full handler-name inventory for parity checks.

A second declarative list owns the 42 generated commands and feeds
`tauri_specta::collect_commands!`. Tests require it to be a subset of the full registry and to
equal the generated command-name set. This avoids parsing Rust source text to infer types or
pretending a name-only scan proves signature parity.

### 5.3 Adapter-compatible exporter

A dedicated codegen binary builds a tauri-specta `BuilderConfiguration` for the 42 commands and
passes it to a repository-owned `LanguageExt` exporter. The exporter traverses structured
`Function`/`DataType` metadata and uses Specta TypeScript rendering; it does not post-process the
standard generated client with string replacement.

For each command it must:

- omit Tauri-injected `State`, `AppHandle`, `Window`, `WebviewWindow` and channel parameters using
  tauri-specta metadata semantics;
- use the Serde deserialize phase for args and serialize phase for success results;
- unwrap `Result<T, IpcError>` into `result: T` while asserting the error side is `IpcError`;
- render no-arg commands as `args: undefined`;
- emit a deterministic sorted map and command-name tuple;
- fail on duplicate command names or an unsupported type rather than degrading to `unknown`.

The checked artifact exports only phantom contract values/types compatible with the existing
`IPC_COMMANDS` map. It contains no invoke function and no Tauri runtime import.

## 6. First generated batch

The 42-command batch is frozen below. Completion reduces 89 untyped commands to 47.

### Secret and credential operations (8)

`clear_ai_api_key`, `clear_github_pat`, `get_ai_api_key_state`, `get_github_pat`,
`set_ai_api_key`, `set_github_pat`, `test_ai_connection`, `test_github_pat`.

### Import and install operations (10)

`batch_install_central_skills`, `batch_install_collection`, `batch_install_to_agents`,
`import_collection`, `import_obsidian_skill_to_central`,
`import_obsidian_skill_to_platform`, `install_from_skills_sh`,
`install_marketplace_skill`, `install_skill_to_agent`, `install_skill_to_project`.

### Destructive operations (10)

`delete_central_skill`, `delete_central_skills`, `delete_collection`,
`delete_skill_repository`, `remove_project`, `remove_registry`, `remove_scan_directory`,
`remove_skill_from_collection`, `unassign_skill_tags`, `uninstall_skill_from_project`.

### Central store, update and sync operations (14)

`apply_central_repository_sync`, `apply_central_store_location_change`,
`apply_local_remote_sync`, `clear_skill_update_inventory`,
`force_mirror_central_repositories`, `force_update_central_skills`,
`get_central_skill_update_states`, `get_skill_update_inventory`,
`keep_remote_missing_central_skills`, `preview_central_store_location_change`,
`preview_local_remote_sync`, `refresh_skill_update_inventory`,
`scan_deleted_platform_copies`, `scan_platform_duplicate_skills`.

## 7. Frontend map integration

Split `commandMap.ts` conceptually into:

- the generated 42-entry map;
- the existing 88 handwritten typed entries;
- the remaining 47-entry `UNTYPED_IPC_COMMANDS` ratchet.

`IPC_COMMANDS` merges generated and handwritten entries. Type-level and runtime tests reject
overlap. Call sites for the 42 commands remove explicit generics and use the existing overload;
fixtures gain compile-time args/result checking from the generated map.

The seven backend-only commands do not enter `IPC_COMMANDS` until a real frontend caller exists.

## 8. CI and drift checks

Add a deterministic generate command and a check mode that writes to a temporary file and diffs
against the checked artifact. The checks prove independently:

1. generated artifact is current;
2. generated names equal the 42-command registry;
3. generated registry is a subset of the 184 runtime registry;
4. all 177 frontend contract names exist in the runtime registry;
5. runtime minus frontend equals the explicit seven-item backend-only set;
6. typed/untyped sets are disjoint and totals are 130/47/177;
7. no annotated command returns `Result<_, String>`;
8. every `IpcResult` rejection serializes the three-field camelCase contract.

The codegen check joins `just ci`; Windows `pnpm tauri build` proves the optional feature does not
change the production bundle.

## 9. Migration and rollback

Implementation proceeds in reversible layers:

1. Add Rust/TypeScript error primitives and compatibility tests.
2. Migrate command boundaries and UI code branches domain by domain; legacy adapter fallback keeps
   the frontend usable during the local worktree transition.
3. Add registry/codegen feature and generate the 42-command artifact.
4. Migrate call sites/fixtures, shrink allowlist, enable CI ratchets.
5. Update specs and run full verification/bundle.

Rollback before release is mechanical: restore the 42 entries to the allowlist and handwritten
map/call-site types, remove the generated overlay and `ipc-codegen` feature, and keep the adapter's
legacy normalization. The structured Rust error migration is not partially rolled back after its
180-command gate passes; if a domain mapper regresses, revert that mapper and its command batch
together. No DB or user-data rollback is required.

## 10. Trade-offs and risks

- The selected structured-error scope is much larger than args/result codegen alone. Batching and
  count-based gates prevent a half-migrated boundary from being declared complete.
- tauri-specta/Specta remain RC. Exact pins, feature isolation and a checked artifact contain the
  risk; normal runtime does not depend on generated command wrappers.
- A local exporter is owned by this repository, but it consumes structured metadata and is tested
  against Serde phases/injected arguments. This is narrower and safer than parsing Rust source or
  bypassing `@/lib/ipc`.
- Generic error conversion tends either to leak details or erase useful messages. Explicit domain
  mapping plus safe fallback is intentionally more work and is required by the credential/path
  boundary.
