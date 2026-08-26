# 核心契约实施计划

## Preconditions

- Parent final plan explicitly approved.
- Read parent PRD/design and all manifests; preserve unrelated 08-26 changes.

## Steps

1. Add characterization tests for current registry counts, raw Display path, row-id generation and old-row DTO.
2. Add policy metadata types and extend `ipc_registry.rs` macro; classify every command without wiring domain logs yet.
3. Create observability module with controlled category/action/phase/status types, `OperationId`, reviewed diagnostic
   and lifecycle interface.
4. Extend DB repo for caller-supplied ID, started insert, terminal update, interrupted sweep and ID filtering.
5. Add optional `IpcError.correlationId`, TS/generated types and compatibility tests.
6. Implement best-effort fallback and remove raw Display from the new interface; retain a documented temporary adapter.
7. Run focused tests and codegen/docs checks; freeze interface for dependent children.

## Validation

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked observability
cargo test --manifest-path src-tauri/Cargo.toml --locked operation_log
cargo test --manifest-path src-tauri/Cargo.toml --locked operation_logs
pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts src/test/runtime/ipc.test.ts
pnpm ipc:codegen:check
pnpm docs:gen:check
cargo fmt --all -- --check
git diff --check
```

## Rollback

Keep old recorder adapter until all children migrate. Revert macro metadata, optional IPC field and repo behavior as one
unit if compatibility tests fail; do not delete or rewrite existing log data.
