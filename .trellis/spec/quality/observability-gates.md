# Observability Coverage And Privacy Gates

## Executable source of truth

- `src-tauri/src/ipc_registry.rs` owns command membership and exactly one `operation`, `runtime-only` or `excluded` policy.
- Contract tests compare sets and unique membership. They must not freeze hand-maintained command totals.
- Every fallible production command uses a named IPC boundary; excluded commands still name the boundary so their typed
  exclusion reason is selected deliberately.
- `pnpm docs:gen` derives the command/action, policy, category, phase, lifecycle/reason and audit-evidence matrix from the
  registry. `pnpm docs:gen:check` is read-only and fails on drift.

## Evidence layers

| Gate                | Required evidence                                                                                                |
| ------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Registry            | Unique handler/name/policy parity; no unnamed fallible boundary                                                  |
| Operation lifecycle | Success/failure plus applicable partial/cancel; started updates the same UUID; stale started becomes interrupted |
| Runtime rejection   | Reviewed code/category/phase/retryable/duration/source/correlation; typed target kind when known                 |
| Privacy             | Adversarial secret/host/path/URL/raw error/stack/args absent at persistence, read, export, DOM and clipboard     |
| Cross-layer         | One valid UUID filters and navigates both Operation and backend/frontend Runtime evidence                        |
| Compatibility       | Historical rows parse safely; invalid IDs match nothing and are not copied                                       |
| UI                  | Centered dialog, one scroll owner, keyboard/focus behavior, narrow viewport and i18n parity                      |

Native Windows Tauri/WebView2 layout, real log rotation/clear and abnormal-process recovery require native evidence. When
they cannot be executed, report `UNVERIFIED`; frontend unit tests do not upgrade them to PASS.
