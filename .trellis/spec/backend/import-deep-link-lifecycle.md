# Import Deep-Link Lifecycle Contract

## 1. Scope / Trigger

修改 `skillport://` scheme、Tauri plugin 顺序、cold/warm instance 处理、native import-intent queue 或 ready command 时适用。深链只传递 GitHub 导入意图，不是 preview/import command channel。

## 2. Signatures

```rust
parse_import_deep_link(raw: &str) -> Result<ImportIntent, DeepLinkError>
parse_import_intent_from_argv(argv: &[String]) -> Result<ImportIntent, DeepLinkError>
submit_import_intent(app: &AppHandle, state: &ImportIntentState, intent: ImportIntent) -> Result<(), DeepLinkError>
mark_import_intent_frontend_ready(app: AppHandle, state: State<'_, ImportIntentState>) -> Result<(), String>
```

```json
{ "event": "skillport://import-intent", "payload": { "source": "https://github.com/owner/repo" } }
```

## 3. Contracts

- Canonical public URI is exactly `skillport://import?source=<percent-encoded HTTPS GitHub URL>` and is limited to 4096 UTF-8 bytes.
- The parser reuses `normalize_github_source_url`; native handlers never call preview/import commands, access PATs, or install skills.
- `ImportIntentState` is managed before plugins. Builder plugin order is fixed: `tauri-plugin-single-instance` first, `tauri-plugin-deep-link` second, then sql/fs/dialog/shell/process/updater.
- Approved versions are `tauri-plugin-single-instance = 2.4.3` and `tauri-plugin-deep-link = 2.4.9`, both `Apache-2.0 OR MIT`. Do not enable single-instance's optional `deep-link` feature.
- Cold start uses `DeepLinkExt::get_current`; warm start accepts exactly executable + one URI from single-instance argv. Both call the same parser/queue.
- Before frontend ready, queue at most 8 normalized sources, deduplicate, and drop the oldest on overflow. The first ready transition flushes FIFO once; repeated ready calls do not replay.
- Warm handling restores/shows/focuses `main` before submitting the intent. Logs contain only stable error codes, counts, and capacity, never URI/source/argv.
- Windows may normalize the transport URL to `skillport://import/?...`. Only the OS-boundary parser may remove that single root slash before calling the strict canonical parser; the public parser must still reject `/` and every other path.
- On Windows, call Tauri `set_focus()` first, then use the dependency-free `AttachThreadInput`/`BringWindowToTop`/`SetForegroundWindow` fallback for user-initiated warm activation. The fallback is `cfg(target_os = "windows")` and must verify the foreground handle.
- `tauri.conf.json` registers only `plugins.deep-link.desktop.schemes = ["skillport"]`; no JavaScript deep-link package, guest capability, or custom NSIS template is required.

## 4. Validation & Error Matrix

| Input/state | Required behavior |
| --- | --- |
| HTTP, non-GitHub, file/UNC, userinfo, port | reject with typed safe error |
| unknown action/path/fragment/parameter or duplicate source | reject fail-closed |
| token/auth/target/overwrite/auto/command | reject without payload in error/log |
| control/backslash/encoded traversal/source query or fragment | reject before GitHub normalization |
| warm argv missing URI or containing extra args | reject; log only code and argument count |
| OS transport adds exactly one root slash | remove at OS boundary, then re-run canonical parser |
| queue full | remove oldest, enqueue newest, log stable overflow code |
| event emit/queue lock failure | typed `EventDelivery`/`QueueUnavailable`; command stringifies only at IPC boundary |

## 5. Good / Base / Bad Cases

- Good: installed Windows app receives a warm canonical URI, focuses the existing window, emits one normalized intent, and leaves Preview untouched.
- Base: cold URI arrives before React; native queues it, listener registers, ready command flushes it exactly once.
- Bad: enabling the single-instance `deep-link` feature, consuming plugin raw events, logging argv, or invoking `preview_github_repo_import` from Rust.

## 6. Tests Required

- Pure Rust parser: valid repo/branch/subpath plus every rejection row and redacted `Display` output.
- Queue: FIFO, normalized dedupe, capacity 8/drop-oldest, ready idempotence, immediate post-ready delivery.
- Warm argv: exact two-item shape, missing/extra/malformed arguments and redaction.
- Windows lifecycle: minimize primary window, activate URI, assert same PID, restored state, and foreground PID ownership.
- Gate: `cargo test deep_link`, `cargo clippy -- -D warnings`, `just ci`, `pnpm tauri build`.
- Windows evidence: real NSIS install, registry command, cold/warm `Start-Process`, PID/window focus, and proof that no automatic Preview/Confirm/import occurred.

## 7. Wrong vs Correct

```rust
// Wrong: duplicate raw event path and automatic work.
plugin_handle.on_open_url(|urls| preview_github_repo_import(urls[0]));

// Correct: one parser/queue and an intent-only typed event.
let intent = parse_import_deep_link(raw)?;
submit_import_intent(app, state, intent)?;
```
