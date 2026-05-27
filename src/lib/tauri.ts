import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type TauriWindow = Window & {
  __TAURI__?: unknown;
  __TAURI_INTERNALS__?: unknown;
};

export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const tauriWindow = window as TauriWindow;
  return Boolean(tauriWindow.__TAURI__ || tauriWindow.__TAURI_INTERNALS__);
}

export const invoke = tauriInvoke;
export const listen = tauriListen;

let hasRequestedMainWindowShow = false;

export async function showMainWindowWhenReady(): Promise<void> {
  if (hasRequestedMainWindowShow || !isTauriRuntime()) {
    return;
  }

  hasRequestedMainWindowShow = true;
  await getCurrentWindow().show();
}

export function __resetMainWindowReadyForTest(): void {
  hasRequestedMainWindowShow = false;
}
