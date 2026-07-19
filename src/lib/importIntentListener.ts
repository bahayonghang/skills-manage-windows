import { invoke, listen } from "@/lib/ipc";

const IMPORT_INTENT_EVENT = "skillport://import-intent";

let activeHandler: ((payload: unknown) => void) | null = null;
let listenerPromise: Promise<void> | null = null;
let listenerCleanup: (() => void | Promise<void>) | null = null;

export function setImportIntentHandler(
  handler: ((payload: unknown) => void) | null,
) {
  activeHandler = handler;
}

export function ensureImportIntentListener(): Promise<void> {
  if (listenerPromise) return listenerPromise;

  listenerPromise = listen<unknown>(IMPORT_INTENT_EVENT, (event) => {
    activeHandler?.(event.payload);
  })
    .then(async (cleanup) => {
      listenerCleanup = cleanup;
      await invoke("mark_import_intent_frontend_ready");
    })
    .catch(() => {
      listenerPromise = null;
    });
  return listenerPromise;
}

export function resetImportIntentListenerForTest() {
  if (listenerCleanup) {
    void Promise.resolve(listenerCleanup()).catch(() => undefined);
  }
  activeHandler = null;
  listenerCleanup = null;
  listenerPromise = null;
}
