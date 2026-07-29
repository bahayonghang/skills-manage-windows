# Typed IPC codegen options (2026-07-28)

## Question

How can the remaining IPC commands derive their TypeScript argument and result
contracts from Rust without bypassing the repository's `@/lib/ipc` adapter?

## Current repository evidence

- `src/lib/ipc/commandMap.ts` currently contains 88 typed and 89 untyped
  commands (177 total). The July 24 baseline of 104 untyped commands is stale.
- `src/test/contracts/ipcCommandCoverage.test.ts` checks only frontend literal
  command registration, overlap, and zombie entries. It does not compare Rust
  parameter names/types or the `tauri::generate_handler!` registry.
- `src-tauri/src/lib.rs` registers commands through one
  `tauri::generate_handler!` invocation.
- `.trellis/spec/frontend/ipc-adapter.md` requires all frontend IPC to pass
  through `@/lib/ipc`, which owns browser fixtures and failure recording.
- `.trellis/spec/backend/domain-error-enums.md` currently defines the command
  boundary as `Result<T, String>`.

## Primary-source findings

### tauri-specta

- crates.io reports `2.0.0-rc.25` as the latest non-yanked release on
  2026-07-28. The crate is still an RC, so all related versions must be pinned
  exactly rather than floated.
  Source: <https://crates.io/api/v1/crates/tauri-specta>
- The v2 README maps Tauri 2 to tauri-specta 2 / Specta 2.
  Source: <https://github.com/specta-rs/tauri-specta/blob/main/README.md>
- The tagged crate docs explicitly call v2 beta, require
  `#[specta::specta]` on commands and `specta::Type` on custom types, and
  support Serde-aware serialize/deserialise phases. This is important for the
  repository's `rename_all`, optional fields, and custom deserializers.
  Source: <https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/lib.rs>
- `collect_commands!` combines Tauri's handler and Specta's function metadata;
  `Builder::commands` owns one command set, and `Builder::invoke_handler`
  exposes that set as the application handler.
  Sources:
  <https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/macros.rs>,
  <https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/builder.rs>
- The standard TypeScript exporter hardcodes an import of
  `invoke as __TAURI_INVOKE` from `@tauri-apps/api/core`. Using its generated
  `commands` object directly would bypass this repository's adapter, fixtures,
  and failure recorder.
  Source:
  <https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/lang/js_ts.rs>

## Options

### 1. Standard tauri-specta runtime bindings

Pros: least custom generator code; Rust signatures and Serde types are the
source of truth.

Cons: generated calls bypass `@/lib/ipc`; replacing the single Tauri handler
incrementally is not the documented path; adopting all commands at once would
require a broad derive/annotation migration. Reject for this task.

### 2. Parse Rust/TypeScript source with repository scripts

Pros: no new Rust dependency.

Cons: a source parser must reproduce Tauri injection, Serde rename/phase rules,
generic result flattening, and type rendering. A name-only parser would not
prove shape parity. Reject because it recreates the hard part of Specta.

### 3. Pinned tauri-specta metadata with a local adapter-compatible exporter

Use exact RC versions behind an `ipc-codegen` feature. Annotate only the
migrated commands/types, collect their function metadata, and export a checked
TypeScript contract shaped for the existing `@/lib/ipc` adapter. Keep the
application's existing `tauri::generate_handler!` as the runtime handler during
the staged migration. CI regenerates to a temporary path and fails on diff.

Pros: Rust/Serde remains authoritative; staged migration is possible; browser
fixtures and failure recording stay intact; no runtime dependency is needed in
normal builds.

Cons: the exporter bridge is repository-owned and must be contract-tested; the
upstream stack is still RC and must be pinned; migrated nested types need
feature-gated `specta::Type` derives.

## Recommendation

Choose option 3. Treat generated output as a checked build artifact, never as a
second runtime invoke layer. Add three independent ratchets:

1. generated Rust command names equal the migrated command set;
2. migrated names exist in `tauri::generate_handler!` and in the frontend
   caller inventory;
3. `UNTYPED_IPC_COMMANDS` may only shrink from the live baseline of 89.

The user selected the cross-layer structured-error option on 2026-07-28. The
implementation therefore adds `IpcError { code, message, retryable }` across all
Rust command boundaries and uses that type as the generated command error
contract. The adapter-compatible exporter recommendation is unchanged.
