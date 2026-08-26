# IPC Commands

Every Rust function annotated with `#[tauri::command]` is a callable from the frontend via `invoke('name', args)`. The table below is regenerated from source — never edit by hand.

## How the dictionary is built

```text
[scripts/docs/build-ipc-dict.mjs] ── reads src-tauri/src/commands/**/*.rs
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

Run `pnpm docs:gen` to refresh and commit the generated file with its Rust source. CI runs the read-only `pnpm docs:gen:check` through `pnpm docs:build`, so a stale table fails without rewriting the checkout.

## Calling Convention

- **Naming.** Snake-case Rust function names map 1:1 to the JS `invoke()` argument: `invoke('scan_all_skills', {})`.
- **Inputs.** Tauri serializes camelCase JS keys to snake-case parameters via serde. Pass a plain object.
- **Returns.** Fallible commands return the structured `IpcResult<T>` envelope with a reviewed code, fixed public message,
  retryable flag and optional correlation UUID. Rich diagnostic evidence goes through Runtime Log; raw source errors do
  not become user-visible messages or Operation Log fields.
- **Injected parameters.** `State<AppState>`, `Window`, `AppHandle`, and `Emitter` are injected by Tauri and do not appear in the JS payload.

## Source of truth

The generated dictionary lives at `docs/architecture/_generated/ipc-commands.md`. It includes:

- Module path (`commands::scanner`, `services::installation::centralize`)
- Command name
- Business inputs (Tauri-injected parameters filtered out)
- Return type
- The first paragraph of `///` docs above the function
- The registry-derived log policy, Operation category/default phase/lifecycle, or explicit runtime/exclusion reason
- The audit-evidence contract for Operation, Runtime-only and excluded commands

<!--@include: ./_generated/ipc-commands.md-->

Last reviewed: 2026-08-27
