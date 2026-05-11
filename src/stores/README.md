# Store action conventions

This directory owns app state and Tauri IPC orchestration. React components
should call store actions instead of calling `invoke()` directly.

## Error signaling

Some mutation actions intentionally use a double-signal error pattern:

```ts
try {
  await invoke("some_command");
} catch (err) {
  set({ error: String(err), isSaving: false });
  throw err;
}
```

The two channels have different consumers:

- `set({ error })` updates durable store state for page-level banners, disabled
  states, retry affordances, and test assertions.
- `throw err` lets the caller show an action-local toast, restore focus, close a
  dialog conditionally, or run rollback logic that only the caller knows about.

Callers that catch a throwing store action must avoid also showing a second
toast from the same `state.error`. Prefer one user-facing surface per failure:

- page/bootstrap loads usually set store `error` and do **not** throw;
- button/dialog mutations may set store `error` and throw for local handling;
- fire-and-forget callers must catch thrown errors if no local UI should surface
  them.

Do not remove either channel from an existing action unless the call sites and
tests are updated to prove the banner/toast behavior remains intentional.
