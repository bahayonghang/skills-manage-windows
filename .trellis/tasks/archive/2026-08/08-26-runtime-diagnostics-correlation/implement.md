# Runtime 诊断实施计划

## Steps

1. Characterize current backend-missing evidence, frontend raw global error data and recursion protections.
2. Add a named backend IPC boundary that resolves `CommandPolicyEntry` and emits failure evidence using operation/
   runtime-only/excluded correlation semantics; retain the unnamed macro only as a migration adapter.
3. Tighten frontend runtime payload construction; remove message/stack/filename/raw reason and preserve stable code/ID.
4. Extend Runtime parser/types/store filters for operation ID/source; keep legacy lines compatible.
5. Add exactly-once-per-source, renderer-absent, recorder-failure, redaction and retention/export tests.
6. Publish the named-boundary migration pattern for coverage children; do not edit their owned command files.

## Validation

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked logging
cargo test --manifest-path src-tauri/Cargo.toml --locked ipc_error
pnpm exec vitest run src/test/runtime/ipc.test.ts src/test/runtime/runtimeLogger.test.ts src/test/stores/runtimeLogStore.test.ts
pnpm typecheck
pnpm lint
cargo fmt --all -- --check
git diff --check
```

## Rollback

Keep optional correlation field compatible. If frontend recording regresses, disable only the frontend adapter; backend
Runtime evidence remains authoritative. Never delete existing Runtime files during rollback.
