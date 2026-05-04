# IPC Commands

Every Rust function annotated with `#[tauri::command]` is a callable from the frontend via `invoke('name', args)`. The table below is regenerated from source — never edit by hand.

## How the dictionary is built

```text
[scripts/build-ipc-dict.mjs] ── reads src-tauri/src/**/*.rs
                                    │
                                    ▼
                            extract #[tauri::command]
                                    │
                                    ▼
                  group by module (commands::*, services::*…)
                                    │
                                    ▼
        write docs/architecture/_generated/ipc-commands.md
```

Run `pnpm docs:gen` to refresh. CI runs the same script before `pnpm docs:build`, so a stale table fails the documentation pipeline.

## Calling Convention

- **Naming.** Snake-case Rust function names map 1:1 to the JS `invoke()` argument: `invoke('scan_all_skills', {})`.
- **Inputs.** Tauri serializes camelCase JS keys to snake-case parameters via serde. Pass a plain object.
- **Returns.** All commands return `Result<T, String>`. The frontend treats the string as a user-visible error message; rich diagnostics go through `operation_logs` instead.
- **Injected parameters.** `State<AppState>`, `Window`, `AppHandle`, and `Emitter` are injected by Tauri and do not appear in the JS payload.

## Source of truth

The generated dictionary lives at `docs/architecture/_generated/ipc-commands.md`. It includes:

- Module path (`commands::scanner`, `services::installation::centralize`)
- Command name
- Business inputs (Tauri-injected parameters filtered out)
- Return type
- The first paragraph of `///` docs above the function

<!--@include: ./_generated/ipc-commands.md-->

Last reviewed: 2026-05-04
