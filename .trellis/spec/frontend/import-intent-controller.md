# Import Intent Controller Contract

## 1. Scope / Trigger

修改 Central/Marketplace GitHub launcher、native import-intent event、wizard open/source ownership、dirty protection 或 pending UI 时适用。普通按钮和 deep link 必须进入同一 controller/store。

## 2. Signatures

```ts
type ImportIntent = { kind: "github"; source?: string };
openImportIntent(intent: ImportIntent): "opened" | "pending" | "duplicate" | "invalid";
listen<{ source: string }>("skillport://import-intent", handler): Promise<UnlistenFn>;
invoke("mark_import_intent_frontend_ready"): Promise<void>;
```

Shared Zustand state is `githubWizardOpen`, `githubSource`, and `pendingSources` (FIFO, maximum 8).

## 3. Contracts

- `ImportIntentController` is mounted once in `AppShell`. The application-level listener is StrictMode-safe: register listener first, then invoke ready once.
- A valid native event navigates to `/central` and calls `openImportIntent`; opening/prefilling never calls Preview, Confirm, or Import IPC.
- Central launcher, repository-sync entry, Marketplace CTA, and deep link share the same wizard open/source state. Local ZIP remains separate.
- Dirty means non-empty source or any preview/loading/error/import/result state. A new source never overwrites a dirty session; it enters a deduplicated FIFO of at most 8.
- Pending UI shows count. The user closes the current wizard before consuming the FIFO head, or discards an item. Consume clears the old GitHub session but does not preview.
- Event payload must be exactly `{ source: string }`; reject extra keys, non-HTTPS/non-GitHub credentials/ports/query/fragment, controls, backslashes, and encoded traversal.
- Components do not call Tauri directly; listener/ready use `@/lib/ipc`, and the ready command is registered in `IPC_COMMANDS`.

## 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| clean session + valid source | set source, open wizard, navigate Central |
| dirty Central or Marketplace session | preserve source/state, enqueue and show pending count |
| duplicate active or pending source | ignore without queue growth |
| ninth distinct pending source | drop oldest, retain newest eight FIFO |
| invalid payload or source | ignore without navigation or malicious payload feedback |
| consume while wizard open | no-op; user must close first |
| listener setup failure | do not send ready before a listener exists |

## 5. Good / Base / Bad Cases

- Good: Marketplace has typed input; warm deep link navigates Central, preserves that input, and shows one pending request.
- Base: no active import; cold event opens the existing input step with normalized source.
- Bad: route-local open/source state, `useEffect` ready before `listen`, last-write-wins pending slot, or auto-preview after prefill.

## 6. Tests Required

- Controller: listener-before-ready, StrictMode singleton, route/prefill, invalid event, zero preview/import IPC.
- Store/UI: dirty session preservation, duplicate, FIFO overflow, pending count, close/consume/discard.
- Page regressions: Central and Marketplace normal GitHub flow, Preview/Confirm/import, SSH/WSL password/target behavior, result/reset.
- Test support that renders these pages must reset `useImportIntentStore` between cases.

## 7. Wrong vs Correct

```ts
// Wrong: route-local overwrite and automatic network work.
setGithubRepoUrl(event.payload.source);
await previewGitHubRepoImport(event.payload.source);

// Correct: one intent boundary; Preview stays user initiated.
openImportIntent({ kind: "github", source: event.payload.source });
navigate("/central");
```
