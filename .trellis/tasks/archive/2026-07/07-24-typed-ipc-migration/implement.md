# Implementation Plan

## Preconditions and frozen gates

- [ ] Confirm task status is still `planning`, branch is `dev`, and unrelated dirty files remain
      untouched.
- [ ] Re-run the inventory helpers and confirm 184 registered handlers, 180 raw string-error
      handlers, 4 non-`Result` handlers, 88 typed, 89 untyped and 177 frontend commands.
- [ ] Run `task.py start 07-24-typed-ipc-migration` only after the user approves this exact plan.
- [ ] Load `trellis-before-dev` specs before product edits.

## Phase A: Structured Rust error boundary

- [ ] Add the backend `IpcError`/`IpcResult<T>` module with camelCase serialization, stable code
      validation, conservative retryability constructors and unit tests.
- [ ] Add explicit safe-summary/redaction integration; seed credential, Windows/POSIX path,
      command/env, stdout/stderr, token/digest and file-content leak tests.
- [ ] Preserve existing coded families (`ai.*`, `github_import.*`, `local_archive.*`) as object
      fields instead of `code:message` strings.
- [ ] Add explicit mappings for the UI-dependent codes: cancellation, portable manifest errors,
      GitHub rate-limit/access/configured-token failures and missing SSH password.
- [ ] Migrate the 180 annotated command signatures to `IpcResult<_>` in bounded domain batches:
      startup/bootstrap/logs; targets/agents/settings; skills/linker/metadata; update/store/sync;
      import/marketplace/projects/collections; saved views/tag groups/usage.
- [ ] Keep the four non-`Result` command signatures unchanged.
- [ ] Replace command-boundary `.to_string()`/raw `Err(String)` exits with reviewed domain mappers
      or fixed safe fallback. Do not change payload-internal `error: String` fields.
- [ ] Add a Rust contract test/inventory gate proving no `#[tauri::command]` returns
      `Result<_, String>` and all fallible commands use `IpcResult`.
- [ ] Run after each Rust batch:
      `cargo fmt --all -- --check`, focused module tests, then
      `cargo check --all-targets --locked` from `src-tauri`.

Rollback point A: revert the current domain batch as a unit. Do not leave mixed raw/object errors
inside one command family.

## Phase B: Frontend adapter, fixtures and behavioral branches

- [ ] Add `src/lib/ipc/errors.ts` with `IpcErrorPayload`, runtime guard,
      `IpcInvokeError`, normalization and fixture helper.
- [ ] Normalize Tauri/fixture rejection in `invoke` before it reaches callers or the failure
      recorder; keep `invokeRaw` and `IpcFixtureMissingError` behavior intact.
- [ ] Extend `backendError.ts` to read object/wrapper code/message/retryable first and preserve the
      strict legacy coded-string fallback.
- [ ] Convert expected backend failures in `fixtures/skills.ts`, `targets.ts`, `savedViews.ts`,
      `tagGroups.ts` and `platform.ts` to the structured helper.
- [ ] Replace portability cancellation message sniffing in
      `centralSkillsStore.updateSlice.ts` with `operation.cancelled` checks.
- [ ] Replace manifest JSON/kind/version message sniffing in
      `statePortabilityDialogUtils.ts` with the three portable-state codes.
- [ ] Replace GitHub auth/rate-limit/configured-token regex classification and SSH-password regex
      classification with code sets in wizard utils/actions.
- [ ] Update focused store/component tests and backend-error/i18n tests to use structured payloads;
      retain explicit legacy string/Error/unknown transport compatibility cases.
- [ ] Verify `String(new IpcInvokeError(...))` equals only the historical message and never
      `[object Object]`.
- [ ] Run focused Vitest:
      `src/test/runtime/ipc.test.ts`, `src/test/lib/backendError.test.ts`, browser fixture tests,
      portability tests, GitHub wizard tests, and affected store tests.
- [ ] Run `pnpm typecheck` and `pnpm lint`.

Rollback point B: adapter accepts both forms, so an affected UI/fixture batch can be reverted
without reverting completed Rust error mappings. The final gate still requires no raw Rust
command boundary.

## Phase C: Runtime registry and Rust-derived codegen

- [ ] Move all 184 runtime command paths behind one declarative registry used by
      `tauri::generate_handler!` and the full handler-name inventory.
- [ ] Add the separate frozen 42-command generated registry and subset/equality tests.
- [ ] Add optional exact-version dependencies and the `ipc-codegen` feature; keep them disabled in
      default/runtime/release builds.
- [ ] Add feature-gated `specta::specta` annotations to the 42 commands and feature-gated
      `specta::Type` derives to all transitively referenced args/results and `IpcError`.
- [ ] Implement the repository `AdapterContractExporter` over tauri-specta/Specta structured
      metadata. Cover Tauri injected args, Serde phases, optional/null, nested enum/struct, no-arg,
      `Result<T, IpcError>` unwrap and deterministic ordering with unit/golden tests.
- [ ] Add a codegen binary with generate and `--check`/temporary-diff modes. Generation must fail
      on duplicate commands, unsupported types, unexpected error type or `unknown` degradation.
- [ ] Generate and check in the adapter-compatible TypeScript artifact; assert it contains no
      Tauri runtime imports or callable command client.
- [ ] Add a mutation fixture/test that renames a Rust parameter or Serde field and proves the
      checked-artifact gate fails before application startup.
- [ ] Add `just`/package CI entry points and join the codegen drift check to `just ci` without
      changing the normal Tauri handler.
- [ ] Run codegen generate once, codegen check twice, then Rust fmt/clippy/test with `--locked`.

Rollback point C: remove the generated overlay/feature/dependencies and restore the 42 commands to
the allowlist. The structured error contract remains independently valid.

## Phase D: Frontend generated map migration

- [ ] Merge the generated 42-entry map with the 88 existing handwritten typed entries and reject
      overlap at type/runtime test layers.
- [ ] Remove exactly the frozen 42 entries from `UNTYPED_IPC_COMMANDS`, leaving exactly 47.
- [ ] Remove explicit invoke generics from the 42 call sites and resolve all generated args/result
      mismatches at the Rust or caller source, never by widening to `unknown`.
- [ ] Convert corresponding fixtures to typed registration where needed.
- [ ] Extend IPC contracts to prove 130 typed / 47 untyped / 177 frontend total; every frontend
      command is a runtime handler; runtime-minus-frontend is exactly the seven-item backend-only
      set; generated names equal the 42-item registry.
- [ ] Run `pnpm typecheck`, `pnpm lint` and all IPC/caller/fixture Vitest suites.

## Phase E: Specs and final verification

- [ ] Update `.trellis/spec/frontend/ipc-adapter.md` for object normalization, fixture errors and
      generated-map ownership.
- [ ] Update `.trellis/spec/backend/domain-error-enums.md` from `Result<T, String>` to
      `IpcResult<T>` and document mapper/retryability rules.
- [ ] Update `.trellis/spec/backend/redaction-policy.md` with the IPC error payload boundary.
- [ ] Update `.trellis/spec/quality/ci-quality-gate.md` with codegen/parity/error ratchets.
- [ ] Run focused checks first, then the required full gates:

```powershell
pnpm typecheck
pnpm lint
pnpm test
Push-Location src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
Pop-Location
just ci
pnpm tauri build
```

- [ ] Confirm the Windows NSIS installer actually exists under
      `src-tauri/target/release/bundle/nsis/`.
- [ ] Inspect `git diff --check`, final diff/stat and status; prove unrelated Trellis runtime,
      `.gitattributes`, parent/sibling tasks and audit report were not included.
- [ ] Run `python ./.trellis/scripts/task.py validate 07-24-typed-ipc-migration`.
- [ ] Present verification evidence and request commit confirmation before using the Chinese emoji
      `[AI]` commit flow. Do not push.

## Completion invariants

- 180 raw string command errors -> 0.
- 88 typed / 89 untyped -> 130 typed / 47 untyped.
- Runtime 184 = frontend 177 + explicit backend-only 7.
- Generated 42 is an exact handler/caller subset and checked artifact is clean.
- No credential/path/command/output detail crosses structured IPC errors.
- `just ci` and Windows Tauri bundle both pass.
